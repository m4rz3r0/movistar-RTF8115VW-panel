//! Preparar, confirmar y aplicar un grupo de pasos.

use super::*;
use crate::changes::persistencia::{NuevoGrupo, nuevo_token};
use crate::poller::PauseGuard;

impl ChangeManager {
    /// Lee el router y calcula el diff de cada paso del grupo. Devuelve un
    /// token para [`ChangeManager::confirmar_grupo`]. Los pasos que no
    /// cambian nada se marcan `sin_cambios`; si ninguno cambia nada,
    /// [`CambioError::SinCambios`]. Cualquier paso bloqueado o no válido
    /// rechaza el grupo entero. Con el panel en solo lectura no prepara nada.
    pub async fn preparar_grupo(&self, g: GrupoRequest) -> Result<PreparadoGrupo, CambioError> {
        self.preparar_grupo_con(g, false, None).await
    }

    /// Única entrada de las escrituras (también las de un paso y deshacer).
    pub(in crate::changes) async fn preparar_grupo_con(
        &self,
        g: GrupoRequest,
        explicito: bool,
        deshace: Option<i64>,
    ) -> Result<PreparadoGrupo, CambioError> {
        if crate::instancia::solo_lectura(&self.state.db).await {
            return Err(CambioError::Bloqueada(BLOQUEO_SOLO_LECTURA.into()));
        }
        if g.pasos.is_empty() {
            return Err(CambioError::Invalida("el cambio no tiene pasos".into()));
        }
        if g.pasos.len() > 1 {
            for p in &g.pasos {
                if self.funcion_escribible(&p.req.funcion)?.tipo == Tipo::Accion {
                    return Err(CambioError::Invalida(format!(
                        "{}: una acción solo puede ir sola (no se podría revertir si falla otro \
                         paso)",
                        p.req.funcion
                    )));
                }
            }
        }
        let mut pasos = Vec::with_capacity(g.pasos.len());
        let mut riesgo: Option<Riesgo> = None;
        for (indice, p) in g.pasos.iter().enumerate() {
            let (diff, sin_cambios) = match self.planificar(&p.req, explicito).await {
                Ok(plan) => {
                    riesgo = riesgo.max(Some(plan.funcion.riesgo));
                    (visible(&plan.campos), false)
                }
                Err(CambioError::SinCambios) => (Diff::default(), true),
                Err(e) => return Err(e),
            };
            pasos.push(PasoPreparado {
                indice,
                diff,
                sin_cambios,
                funcion: p.req.funcion.clone(),
                etiqueta: p.etiqueta.clone(),
            });
        }
        let Some(riesgo) = riesgo else {
            return Err(CambioError::SinCambios);
        };
        let token = nuevo_token();
        let mut pend = lock(&self.pendientes);
        let ahora = Instant::now();
        pend.retain(|_, v| v.caduca > ahora);
        pend.insert(
            token.clone(),
            Pendiente {
                sin_cambios: pasos.iter().map(|p| p.sin_cambios).collect(),
                pasos: g.pasos,
                titulo: g.titulo,
                caduca: ahora + TOKEN_TTL,
                explicito,
                deshace,
            },
        );
        Ok(PreparadoGrupo {
            token,
            pasos,
            riesgo,
        })
    }

    /// Aplica el grupo preparado con `token` sin los pasos de `desmarcados`
    /// (índices): escribe y verifica cada paso en orden y deja la vigilancia
    /// en marcha. Devuelve el id del cambio (en `vigilando`). Si un paso
    /// falla, revierte los anteriores antes de volver con el error.
    ///
    /// Las acciones piden palabra: van por [`ChangeManager::confirmar`].
    pub async fn confirmar_grupo(
        self: &Arc<Self>,
        token: &str,
        desmarcados: &[usize],
        usuario: &str,
    ) -> Result<i64, CambioError> {
        self.confirmar_con(token, None, desmarcados, usuario).await
    }

    /// Desde la reserva del hueco, el trabajo corre en una tarea propia: si
    /// este futuro se abandona, el cambio sigue hasta un estado final.
    pub(in crate::changes) async fn confirmar_con(
        self: &Arc<Self>,
        token: &str,
        palabra: Option<&str>,
        desmarcados: &[usize],
        usuario: &str,
    ) -> Result<i64, CambioError> {
        let funciones: Vec<String> = {
            let p = lock(&self.pendientes);
            match p.get(token) {
                Some(v) if v.caduca > Instant::now() => {
                    v.pasos.iter().map(|p| p.req.funcion.clone()).collect()
                }
                _ => return Err(CambioError::TokenInvalido),
            }
        };
        for nombre in &funciones {
            let f = self.funcion_escribible(nombre)?;
            if let Some(esperada) = palabra_requerida(f)
                && palabra != Some(esperada)
            {
                return Err(CambioError::PalabraIncorrecta);
            }
        }
        // Un cambio nuevo no puede adelantarse a uno pendiente de retomar.
        let mut listo = self.listo.subscribe();
        if tokio::time::timeout(self.tiempos.pausa_max, listo.wait_for(|v| *v))
            .await
            .is_err()
        {
            return Err(CambioError::Router(
                "el gestor aún está retomando cambios pendientes; inténtalo de nuevo".into(),
            ));
        }
        let reserva = self.reservar()?;
        let Some(pendiente) = lock(&self.pendientes).remove(token) else {
            return Err(CambioError::TokenInvalido);
        };
        let (tx, rx) = oneshot::channel();
        let m = self.clone();
        let usuario = usuario.to_owned();
        let desmarcados = desmarcados.to_vec();
        let h = tokio::spawn(async move {
            match m
                .aplicar_grupo(&pendiente, &desmarcados, &usuario, &reserva)
                .await
            {
                Ok(ej) => {
                    let _ = tx.send(Ok(ej.id));
                    m.seguir(ej).await;
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                }
            }
            drop(reserva);
        });
        *lock(&self.tarea) = Some(h.abort_handle());
        rx.await.unwrap_or_else(|_| {
            Err(CambioError::Router(
                "la aplicación del cambio se interrumpió".into(),
            ))
        })
    }

    /// Pausa el sondeo (como mucho `pausa_max`).
    async fn pausar(&self) -> Result<PauseGuard, CambioError> {
        tokio::time::timeout(self.tiempos.pausa_max, self.state.poller.pause())
            .await
            .map_err(|_| {
                CambioError::Router("el sondeo no se detuvo a tiempo; no se ha escrito nada".into())
            })
    }

    /// Escribe y verifica los pasos en orden (con el sondeo en pausa).
    /// Devuelve lo necesario para vigilar el grupo.
    async fn aplicar_grupo(
        &self,
        p: &Pendiente,
        desmarcados: &[usize],
        usuario: &str,
        reserva: &Reserva,
    ) -> Result<Ejecucion, CambioError> {
        let mut pausa = Some(self.pausar().await?);
        if let Some(o) = p.deshace
            && let Some(e) = self.ya_deshecho(o).await
        {
            return Err(CambioError::Invalida(e));
        }
        let n = p.pasos.len();
        let omitir = |i: usize| desmarcados.contains(&i) || p.sin_cambios.get(i) == Some(&true);
        let multi = (0..n).filter(|i| !omitir(*i)).count() > 1;
        // El primer paso con cambios se planifica antes de guardar nada: si
        // no hay ninguno o falla, no queda nada en el historial.
        let mut estados = vec![EstadoPaso::Pendiente; n];
        let mut primero = None;
        for (i, estado) in estados.iter_mut().enumerate() {
            if omitir(i) {
                *estado = EstadoPaso::Omitido;
                continue;
            }
            match self.planificar(&p.pasos[i].req, p.explicito).await {
                Ok(plan) => {
                    primero = Some((i, plan));
                    break;
                }
                Err(CambioError::SinCambios) => *estado = EstadoPaso::Omitido,
                Err(e) => return Err(e),
            }
        }
        let Some((i0, mut plan0)) = primero else {
            return Err(CambioError::SinCambios);
        };
        estados[i0] = EstadoPaso::Aplicando;
        for (i, estado) in estados.iter_mut().enumerate().skip(i0 + 1) {
            if omitir(i) {
                *estado = EstadoPaso::Omitido;
            }
        }
        let c = &self.state.hgu;
        let no_responde =
            |e: hgu::Error| CambioError::Router(format!("el router no responde: {e}"));
        let base = if plan0.funcion.name == REBOOT {
            Base::Uptime(read::summary(c).await.map_err(no_responde)?.uptime_s)
        } else {
            Base::Wan(self.state.wan_vigilada().await.map_err(no_responde)?.1)
        };
        if plan0.operacion == Operacion::Crear && matches!(plan0.lectura, Lectura::Xml { .. }) {
            plan0.ids_antes = Some(self.ids_de_la_lista(&plan0).await.map_err(no_responde)?);
        }
        // Riesgo del grupo: el mayor de los pasos que se van a intentar.
        let riesgo = (i0..n)
            .filter(|i| !omitir(*i))
            .filter_map(|i| self.state.catalogo.funcion(&p.pasos[i].req.funcion))
            .map(|f| f.riesgo)
            .fold(plan0.funcion.riesgo, Riesgo::max);
        let id = self
            .insertar(&NuevoGrupo {
                principal: (i0, &plan0),
                pasos: &p.pasos,
                estados: &estados,
                titulo: p.titulo.as_deref(),
                riesgo,
                usuario,
                base,
                deshace: p.deshace,
            })
            .await
            .map_err(|e| CambioError::Router(format!("no se puede guardar el cambio: {e}")))?;
        reserva.fijar(id);
        tracing::info!(
            cambio = id,
            pasos = n,
            funcion = %plan0.funcion.name,
            instancia = plan0.instancia.as_deref().unwrap_or("-"),
            usuario,
            diff = %resumen(&plan0.campos),
            "cambio aplicándose"
        );
        let mut escritos: Vec<PasoEscrito> = Vec::new();
        let mut notas: Vec<String> = Vec::new();
        let mut siguiente = Some(plan0);
        for i in i0..n {
            let mut plan = match siguiente.take() {
                Some(plan) => plan,
                None if omitir(i) => continue,
                None => match self.preparar_paso(id, i, p, &mut pausa).await {
                    Ok(Some(plan)) => plan,
                    Ok(None) => continue,
                    Err(fallo) => {
                        // Las reversiones pausan el sondeo en cada intento.
                        drop(pausa.take());
                        let funcion = &p.pasos[i].req.funcion;
                        return Err(self
                            .fallo_en_paso(id, i, funcion, None, fallo, escritos, multi)
                            .await);
                    }
                },
            };
            match self
                .escribir_paso(id, i, &mut plan, &mut pausa, multi)
                .await
            {
                Ok(nota) => {
                    self.marcar_paso(id, i, EstadoPaso::Aplicado, None).await;
                    if let Some(nota) = nota {
                        notas.push(if multi {
                            format!("{}: {nota}", nombre_paso(i, &plan.funcion.name))
                        } else {
                            nota
                        });
                    }
                    escritos.push(PasoEscrito {
                        orden: i,
                        plan,
                        verificado: true,
                    });
                }
                Err(fallo) => {
                    drop(pausa.take());
                    let funcion = plan.funcion.name.clone();
                    return Err(self
                        .fallo_en_paso(id, i, &funcion, Some(plan), fallo, escritos, multi)
                        .await);
                }
            }
        }
        let nota = (!notas.is_empty()).then(|| notas.join("; "));
        self.marcar(id, Estado::Vigilando, None, nota.as_deref())
            .await;
        drop(pausa);
        Ok(Ejecucion {
            id,
            pasos: escritos,
            base,
            nota,
            riesgo,
            multi,
        })
    }

    /// Planifica el paso `i` (no el primero) y lo guarda en `aplicando`.
    /// `None` si ya no cambia nada (queda `omitido`).
    async fn preparar_paso(
        &self,
        id: i64,
        i: usize,
        p: &Pendiente,
        pausa: &mut Option<PauseGuard>,
    ) -> Result<Option<Plan>, FalloPaso> {
        // La relectura tras una escritura sin respuesta pudo soltar la pausa.
        if pausa.is_none() {
            *pausa = Some(self.pausar().await.map_err(FalloPaso::no_aplicado)?);
        }
        let mut plan = match self.planificar(&p.pasos[i].req, p.explicito).await {
            Ok(plan) => plan,
            Err(CambioError::SinCambios) => {
                self.marcar_paso(id, i, EstadoPaso::Omitido, None).await;
                return Ok(None);
            }
            Err(e) => return Err(FalloPaso::no_aplicado(e)),
        };
        if plan.operacion == Operacion::Crear && matches!(plan.lectura, Lectura::Xml { .. }) {
            let ids = self.ids_de_la_lista(&plan).await.map_err(|e| {
                FalloPaso::no_aplicado(CambioError::Router(format!("el router no responde: {e}")))
            })?;
            plan.ids_antes = Some(ids);
        }
        self.guardar_paso(id, i, &plan).await.map_err(|e| {
            FalloPaso::no_aplicado(CambioError::Router(format!(
                "no se puede guardar el paso: {e}"
            )))
        })?;
        tracing::info!(
            cambio = id,
            paso = i + 1,
            funcion = %plan.funcion.name,
            instancia = plan.instancia.as_deref().unwrap_or("-"),
            diff = %resumen(&plan.campos),
            "paso aplicándose"
        );
        Ok(Some(plan))
    }

    /// Escribe un paso y lo verifica. Devuelve la nota del paso.
    async fn escribir_paso(
        &self,
        id: i64,
        orden: usize,
        plan: &mut Plan,
        pausa: &mut Option<PauseGuard>,
        multi: bool,
    ) -> Result<Option<String>, FalloPaso> {
        let mut nota = None;
        let comprobacion = match self.enviar(plan).await {
            Ok(()) => self.comprobar(plan, Lado::Nuevo).await,
            // El router rechazó la escritura: no se aplicó nada.
            Err(e @ hgu::Error::CallFailed { .. }) => {
                return Err(FalloPaso::no_aplicado(CambioError::Router(e.to_string())));
            }
            Err(e) if plan.funcion.name == REBOOT => {
                nota = Some(format!(
                    "el router no respondió a la orden de reinicio ({e})"
                ));
                self.comprobar(plan, Lado::Nuevo).await
            }
            // Sin respuesta (tiempo agotado, conexión cortada, respuesta
            // rara): la escritura puede haber llegado. Se relee.
            Err(e) => match self.tras_escritura_sin_respuesta(plan, &e, pausa).await {
                SinRespuesta::Aplicada(c, n) => {
                    nota = Some(n);
                    c
                }
                SinRespuesta::Fallido(e) => {
                    return Err(FalloPaso::no_aplicado(CambioError::Router(e)));
                }
                // En un grupo, lo que pudo crearse se intenta revertir: si no
                // se sabe qué instancia es, el grupo no puede darse por
                // revertido entero.
                SinRespuesta::CreacionIncierta(e) => {
                    return Err(FalloPaso {
                        error: CambioError::Router(e),
                        revertir: multi,
                    });
                }
                SinRespuesta::OtrosValores(e) => {
                    return Err(FalloPaso {
                        error: CambioError::Router(e),
                        revertir: true,
                    });
                }
            },
        };
        match comprobacion {
            Comprobacion::Coincide(nueva) => {
                if let Some(n) = nueva {
                    self.fijar_instancia(id, orden, &n).await;
                    plan.instancia = Some(n);
                }
            }
            Comprobacion::Limitada => {
                nota = Some(nota.unwrap_or_else(|| {
                    "verificación limitada: el router no muestra los campos escritos".into()
                }));
            }
            Comprobacion::Difiere(v) => {
                return Err(self
                    .fallo_de_verificacion(plan, CambioError::Verificacion(v), multi)
                    .await);
            }
            Comprobacion::SinLectura(e) => {
                return Err(self
                    .fallo_de_verificacion(plan, CambioError::Verificacion(vec![e]), multi)
                    .await);
            }
        }
        Ok(nota)
    }
}
