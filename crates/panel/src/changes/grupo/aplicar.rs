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
                if self.funcion_escribible_para(&p.req)?.tipo == Tipo::Accion {
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
        let mut huella = Vec::with_capacity(g.pasos.len());
        for (indice, p) in g.pasos.iter().enumerate() {
            let (diff, sin_cambios) = match self.planificar(&p.req, explicito).await {
                Ok(plan) => {
                    riesgo = riesgo.max(Some(plan.funcion.riesgo));
                    huella.push(Some(cola::Huella::de(&plan)));
                    (
                        para_mostrar(&plan.funcion, &self.state.mascara, &plan.campos),
                        false,
                    )
                }
                Err(CambioError::SinCambios) => {
                    huella.push(None);
                    (Diff::default(), true)
                }
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
        let vigila = self
            .se_vigila(
                pasos
                    .iter()
                    .filter(|p| !p.sin_cambios)
                    .map(|p| p.funcion.as_str()),
            )
            .await;
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
                usuario: None,
                huella,
            },
        );
        Ok(PreparadoGrupo {
            token,
            pasos,
            riesgo,
            vigila,
        })
    }

    /// Aplica el grupo preparado con `token` sin los pasos de `desmarcados`
    /// (índices): escribe y verifica cada paso en orden y deja la vigilancia
    /// en marcha. Devuelve el id del cambio (en `vigilando`, o `en_cola` si
    /// otro se estaba aplicando o vigilando: ver `cola.rs`). Si un paso
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
        // Cada petición entera: la regla «sin ping» del cortafuegos solo se
        // escribe con su carga exacta (ver `especiales`).
        let peticiones: Vec<ChangeRequest> = {
            let p = lock(&self.pendientes);
            match p.get(token) {
                // Sin ligar o de otro usuario del panel: no vale y no se gasta.
                Some(v) if v.usuario.as_deref() != Some(usuario) => {
                    return Err(CambioError::TokenInvalido);
                }
                Some(v) if v.caduca > Instant::now() => {
                    v.pasos.iter().map(|p| p.req.clone()).collect()
                }
                _ => return Err(CambioError::TokenInvalido),
            }
        };
        for req in &peticiones {
            let f = self.funcion_escribible_para(req)?;
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
        // El hueco o, con otro cambio en curso, un sitio en la cola.
        let entrada = self.entrar()?;
        let Some(pendiente) = lock(&self.pendientes).remove(token) else {
            if let cola::Entrada::Cola = entrada {
                self.devolver_sitio();
            }
            return Err(CambioError::TokenInvalido);
        };
        let turno = cola::Turno {
            fila: None,
            pendiente,
            desmarcados: desmarcados.to_vec(),
            usuario: usuario.to_owned(),
        };
        let reserva = match entrada {
            cola::Entrada::Ahora(r) => r,
            cola::Entrada::Cola => return self.encolar(turno).await,
        };
        let (tx, rx) = oneshot::channel();
        self.lanzar(reserva, turno, Some(tx));
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
    /// Devuelve lo necesario para vigilar el grupo. `fila`: la de un cambio
    /// que esperó en la cola (antes de escribir se comprueba que el router
    /// sigue como cuando se revisó).
    pub(in crate::changes) async fn aplicar_grupo(
        &self,
        p: &Pendiente,
        desmarcados: &[usize],
        usuario: &str,
        reserva: &Reserva,
        fila: Option<i64>,
    ) -> Result<Ejecucion, CambioError> {
        let mut pausa = Some(self.pausar().await?);
        if fila.is_some() {
            self.comprobar_huella(p, desmarcados).await?;
        }
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
        // Antes de escribir nada: después, un cambio de la WiFi podría haber
        // dejado al panel sin saber por dónde llega.
        let mut vigilar = self
            .se_vigila(
                (i0..n)
                    .filter(|i| !omitir(*i))
                    .map(|i| p.pasos[i].req.funcion.as_str()),
            )
            .await;
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
                fila,
            })
            .await
            .map_err(|e| CambioError::Router(format!("no se puede guardar el cambio: {e}")))?;
        reserva.fijar(id);
        tracing::info!(
            cambio = id,
            pasos = n,
            funcion = %plan0.funcion.name,
            instancia = crate::changes::especiales::instancia_en_registro(&plan0),
            usuario,
            diff = %resumen(&plan0.campos),
            "cambio aplicándose"
        );
        let mut escritos: Vec<PasoEscrito> = Vec::new();
        let mut notas: Vec<Nota> = Vec::new();
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
                        let paso = &p.pasos[i];
                        return Err(self
                            .fallo_en_paso(
                                id,
                                i,
                                (&paso.req.funcion, paso.etiqueta.as_deref()),
                                None,
                                fallo,
                                escritos,
                                multi,
                            )
                            .await);
                    }
                },
            };
            match self
                .escribir_paso(id, i, &mut plan, &mut pausa, multi)
                .await
            {
                Ok((nota, confirmado)) => {
                    // Sin confirmar lo escrito, se vigila igualmente.
                    vigilar |= !confirmado;
                    self.marcar_paso(id, i, EstadoPaso::Aplicado, None).await;
                    if let Some(nota) = nota {
                        notas.push(if multi { nota.del_paso(i) } else { nota });
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
                    let etiqueta = p.pasos[i].etiqueta.as_deref();
                    return Err(self
                        .fallo_en_paso(
                            id,
                            i,
                            (&funcion, etiqueta),
                            Some(plan),
                            fallo,
                            escritos,
                            multi,
                        )
                        .await);
                }
            }
        }
        let ej = Ejecucion {
            id,
            pasos: escritos,
            base,
            nota: Nota::juntar(notas),
            riesgo,
            multi,
            vigilar,
        };
        // Lo que no puede cortar el acceso queda aplicado ya.
        if ej.sin_vigilancia() {
            let nota = Nota::juntar(
                ej.nota
                    .clone()
                    .into_iter()
                    .chain([Nota::clave(nota::SIN_VIGILANCIA)]),
            );
            self.marcar(id, Estado::Aplicado, None, nota.as_ref()).await;
        } else {
            self.marcar(id, Estado::Vigilando, None, ej.nota.as_ref())
                .await;
        }
        drop(pausa);
        Ok(ej)
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
            instancia = crate::changes::especiales::instancia_en_registro(&plan),
            diff = %resumen(&plan.campos),
            "paso aplicándose"
        );
        Ok(Some(plan))
    }

    /// Escribe un paso y lo verifica. Devuelve la nota del paso y si la
    /// relectura confirmó lo escrito (si no, el cambio se vigila).
    async fn escribir_paso(
        &self,
        id: i64,
        orden: usize,
        plan: &mut Plan,
        pausa: &mut Option<PauseGuard>,
        multi: bool,
    ) -> Result<(Option<Nota>, bool), FalloPaso> {
        let mut nota = None;
        let comprobacion = match self.enviar(plan).await {
            Ok(()) => self.comprobar(plan, Lado::Nuevo).await,
            // El router rechazó la escritura: no se aplicó nada.
            Err(e @ hgu::Error::CallFailed { .. }) => {
                return Err(FalloPaso::no_aplicado(CambioError::Router(e.to_string())));
            }
            Err(e) if plan.funcion.name == REBOOT => {
                nota = Some(Nota::clave(nota::REINICIO_SIN_RESPUESTA).tecnico(e.to_string()));
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
        let confirmado = matches!(comprobacion, Comprobacion::Coincide(_));
        match comprobacion {
            Comprobacion::Coincide(nueva) => {
                if let Some(n) = nueva {
                    self.fijar_instancia(id, orden, &n).await;
                    plan.instancia = Some(n);
                }
            }
            Comprobacion::Limitada => {
                nota = Some(nota.unwrap_or_else(|| Nota::clave(nota::VERIFICACION_LIMITADA)));
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
        Ok((nota, confirmado))
    }
}
