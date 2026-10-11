//! Cambios por grupos de pasos: el orden de aplicación, la reversión en
//! orden inverso y la vigilancia del grupo como una unidad.
//!
//! - Los pasos se escriben en el orden declarado. Cada uno se planifica justo
//!   antes de escribirlo (con lo que muestra el router tras los anteriores) y
//!   se verifica con su lectura pareada.
//! - Si falla el paso k, se revierten en orden inverso los pasos ya escritos
//!   (y el propio k si pudo quedar aplicado en parte). El grupo queda
//!   `fallido` si todas las reversiones se verifican; si no,
//!   `revertido_parcial`, con el detalle de qué paso no volvió.
//! - Se vigila con el plazo del paso de mayor riesgo; si la vigilancia falla,
//!   se revierte el grupo entero en orden inverso.
//! - Deshacer prepara el grupo inverso (pasos inversos en orden inverso).
//! - Los pasos sin cambios o desmarcados se guardan como `omitido`.
//!
//! Un grupo de un paso se comporta exactamente como un cambio del núcleo
//! ([`ChangeManager::preparar`]/[`ChangeManager::confirmar`] lo usan así).
//! Una acción (p. ej. `cbReboot`) solo puede ir sola: no se puede revertir si
//! falla otro paso.

use super::*;
use crate::poller::PauseGuard;

/// Un paso de un grupo: la escritura y la clave i18n de su resumen humano
/// (`None`: el nombre de la función).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paso {
    pub req: ChangeRequest,
    pub etiqueta: Option<String>,
}

/// Petición de un grupo de pasos. `titulo`: clave i18n del resumen del grupo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrupoRequest {
    pub pasos: Vec<Paso>,
    pub titulo: Option<String>,
}

/// Un paso preparado: su diff (secretos enmascarados) o `sin_cambios` si el
/// router ya está así (se omitirá al confirmar).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasoPreparado {
    /// Posición en el grupo (desde 0); es la que se desmarca al confirmar.
    pub indice: usize,
    pub diff: Diff,
    pub sin_cambios: bool,
    pub funcion: String,
    pub etiqueta: Option<String>,
}

/// Resultado de [`ChangeManager::preparar_grupo`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparadoGrupo {
    /// Token para [`ChangeManager::confirmar_grupo`] (un solo uso, 10 min).
    pub token: String,
    pub pasos: Vec<PasoPreparado>,
    /// El mayor de los pasos con cambios: marca la vigilancia del grupo.
    pub riesgo: Riesgo,
}

/// Estado de un paso de un grupo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstadoPaso {
    /// Aún no se ha escrito (o el grupo se cortó antes de llegar a él).
    Pendiente,
    /// Sin cambios o desmarcado: no se escribe.
    Omitido,
    /// Escrito (o a punto), sin verificar.
    Aplicando,
    /// Escrito y verificado (o sin forma de comprobarlo).
    Aplicado,
    /// No se aplicó; o no se pudo revertir sin saber si se aplicó.
    Fallido,
    /// Revertido y verificado.
    Revertido,
}

impl EstadoPaso {
    /// Nombre en la base de datos.
    pub fn as_str(self) -> &'static str {
        match self {
            EstadoPaso::Pendiente => "pendiente",
            EstadoPaso::Omitido => "omitido",
            EstadoPaso::Aplicando => "aplicando",
            EstadoPaso::Aplicado => "aplicado",
            EstadoPaso::Fallido => "fallido",
            EstadoPaso::Revertido => "revertido",
        }
    }

    pub(super) fn parse(s: &str) -> Option<EstadoPaso> {
        [
            EstadoPaso::Pendiente,
            EstadoPaso::Omitido,
            EstadoPaso::Aplicando,
            EstadoPaso::Aplicado,
            EstadoPaso::Fallido,
            EstadoPaso::Revertido,
        ]
        .into_iter()
        .find(|e| e.as_str() == s)
    }
}

/// Un paso de un cambio del historial (secretos enmascarados).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasoCambio {
    /// Posición en el grupo (desde 0).
    pub orden: i64,
    pub funcion: String,
    pub instancia: Option<String>,
    /// Clave i18n del resumen humano (`None`: la función).
    pub etiqueta: Option<String>,
    pub estado: EstadoPaso,
    pub diff: Diff,
    /// Por qué falló el paso o su reversión.
    pub error: Option<String>,
}

/// Un paso escrito (o que pudo llegar a escribirse) de un grupo en curso.
pub(super) struct PasoEscrito {
    pub(super) orden: usize,
    pub(super) plan: Plan,
    /// Se verificó (`aplicado`); si no, no se sabe si llegó.
    pub(super) verificado: bool,
}

/// Lo que hace falta para vigilar (y, si falla, revertir) un grupo aplicado.
pub(super) struct Ejecucion {
    pub(super) id: i64,
    /// Pasos aplicados, en orden.
    pub(super) pasos: Vec<PasoEscrito>,
    pub(super) base: Base,
    pub(super) nota: Option<String>,
    pub(super) riesgo: Riesgo,
    /// El grupo tiene más de un paso (no omitido): el detalle de un fallo
    /// nombra el paso y una reversión a medias es `revertido_parcial`.
    pub(super) multi: bool,
}

/// Fallo de un paso al aplicarlo.
pub(super) struct FalloPaso {
    pub(super) error: CambioError,
    /// El router pudo quedar con el paso aplicado (en parte): se revierte
    /// con los demás.
    pub(super) revertir: bool,
}

impl FalloPaso {
    /// El paso no se aplicó (no se escribió, el router lo rechazó o sigue
    /// mostrando lo de antes).
    fn no_aplicado(error: CambioError) -> FalloPaso {
        FalloPaso {
            error,
            revertir: false,
        }
    }
}

/// Por qué no se pudo revertir un paso.
pub(super) enum ErrorReversion {
    /// Sin valores anteriores (o sin la instancia creada) no hay inversa.
    SinInversa,
    Fallo(String),
}

impl std::fmt::Display for ErrorReversion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorReversion::SinInversa => {
                f.write_str("no se puede revertir: valores anteriores desconocidos")
            }
            ErrorReversion::Fallo(e) => write!(f, "no se pudo revertir: {e}"),
        }
    }
}

/// «paso 2 (cbSetUpnpCfg)».
pub(super) fn nombre_paso(orden: usize, funcion: &str) -> String {
    format!("paso {} ({funcion})", orden + 1)
}

/// Detalle de las reversiones fallidas: en un grupo de un paso, el motivo
/// tal cual (como en el núcleo); en uno de varios, cada paso con su motivo.
pub(super) fn detalle_reversion(multi: bool, fallos: &[(usize, String, ErrorReversion)]) -> String {
    if !multi && let [(_, _, e)] = fallos {
        return e.to_string();
    }
    fallos
        .iter()
        .map(|(o, f, e)| format!("{}: {e}", nombre_paso(*o, f)))
        .collect::<Vec<_>>()
        .join("; ")
}

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
    pub(super) async fn preparar_grupo_con(
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
    pub(super) async fn confirmar_con(
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

    /// El router no muestra un paso escrito (o no se pudo releer). Un cambio
    /// de un paso no se revierte (lo que el router no aceptó no se toca,
    /// como en el núcleo). En un grupo, si el router no muestra claramente
    /// lo de antes, el paso pudo quedar aplicado en parte y se revierte con
    /// los demás: el grupo queda entero en lo nuevo o entero en lo de antes.
    async fn fallo_de_verificacion(
        &self,
        plan: &Plan,
        error: CambioError,
        multi: bool,
    ) -> FalloPaso {
        let revertir = multi
            && !matches!(
                self.comprobar(plan, Lado::Anterior).await,
                Comprobacion::Coincide(_)
            );
        FalloPaso { error, revertir }
    }

    /// Falla el paso `k`: revierte en orden inverso lo escrito (y `k` si
    /// pudo quedar aplicado) y deja el grupo `fallido` o, si alguna
    /// reversión no se verifica, `revertido_parcial` (en un grupo de un
    /// paso, `fallido`). Devuelve el error para quien confirmó.
    #[allow(clippy::too_many_arguments)]
    async fn fallo_en_paso(
        &self,
        id: i64,
        k: usize,
        funcion: &str,
        plan: Option<Plan>,
        fallo: FalloPaso,
        mut escritos: Vec<PasoEscrito>,
        multi: bool,
    ) -> CambioError {
        let texto = match &fallo.error {
            CambioError::Verificacion(v) => v.join("; "),
            e => e.to_string(),
        };
        let texto = if multi {
            format!("{}: {texto}", nombre_paso(k, funcion))
        } else {
            texto
        };
        match plan {
            Some(plan) if fallo.revertir => escritos.push(PasoEscrito {
                orden: k,
                plan,
                verificado: false,
            }),
            _ => {
                self.marcar_paso(id, k, EstadoPaso::Fallido, Some(&texto))
                    .await
            }
        }
        if escritos.is_empty() {
            self.marcar(id, Estado::Fallido, Some(&texto), None).await;
            return fallo.error;
        }
        tracing::warn!(cambio = id, "{texto}; revirtiendo lo aplicado");
        let fallos = self.revertir_pasos(id, &escritos).await;
        let (estado, e) = if fallos.is_empty() {
            (
                Estado::Fallido,
                format!("{texto}; revertido a los valores anteriores"),
            )
        } else {
            let estado = if multi {
                Estado::RevertidoParcial
            } else {
                Estado::Fallido
            };
            (
                estado,
                format!("{texto}; {}", detalle_reversion(multi, &fallos)),
            )
        };
        self.marcar(id, estado, Some(&e), None).await;
        self.cerrar_sin_revertir(id, &escritos, &fallos).await;
        CambioError::Router(e)
    }

    /// Revierte `pasos` en orden inverso y marca cada uno. Devuelve los que
    /// no se pudieron revertir.
    pub(super) async fn revertir_pasos(
        &self,
        id: i64,
        pasos: &[PasoEscrito],
    ) -> Vec<(usize, String, ErrorReversion)> {
        let mut fallos = Vec::new();
        for p in pasos.iter().rev() {
            match self.revertir_plan(id, &p.plan).await {
                Ok(()) => {
                    self.marcar_paso(id, p.orden, EstadoPaso::Revertido, None)
                        .await
                }
                Err(e) => {
                    // Sigue `aplicado` o `aplicando` (con el motivo): si el
                    // panel se corta antes de cerrar el grupo, al retomar se
                    // vuelve a intentar. [`Self::cerrar_sin_revertir`] lo
                    // pasa a `fallido` una vez cerrado, si no se verificó.
                    let estado = if p.verificado {
                        EstadoPaso::Aplicado
                    } else {
                        EstadoPaso::Aplicando
                    };
                    self.marcar_paso(id, p.orden, estado, Some(&e.to_string()))
                        .await;
                    fallos.push((p.orden, p.plan.funcion.name.clone(), e));
                }
            }
        }
        fallos
    }

    /// Con el grupo ya en su estado final: los pasos sin verificar que no se
    /// pudieron revertir quedan `fallido` (no se sabe si se aplicaron; el
    /// detalle del grupo los nombra).
    pub(super) async fn cerrar_sin_revertir(
        &self,
        id: i64,
        pasos: &[PasoEscrito],
        fallos: &[(usize, String, ErrorReversion)],
    ) {
        for p in pasos {
            if !p.verificado && fallos.iter().any(|(o, _, _)| *o == p.orden) {
                self.marcar_paso(id, p.orden, EstadoPaso::Fallido, None)
                    .await;
            }
        }
    }

    /// Escribe los valores anteriores de `plan` y verifica. Los fallos
    /// transitorios (router sin respuesta, relectura fallida) se reintentan
    /// durante `alto`; los deterministas terminan en el acto.
    async fn revertir_plan(&self, id: i64, plan: &Plan) -> Result<(), ErrorReversion> {
        let Some(req) = inversa(plan) else {
            // Sin inversa, vale si el router ya muestra lo de antes (p. ej.
            // una creación sin instancia conocida que no llegó a hacerse).
            return match plan.operacion {
                Operacion::Accion => Err(ErrorReversion::SinInversa),
                _ => match self.comprobar(plan, Lado::Anterior).await {
                    Comprobacion::Coincide(_) => Ok(()),
                    _ if plan.operacion == Operacion::Crear => Err(ErrorReversion::Fallo(
                        "no se sabe qué instancia se creó; revisa la lista en el router".into(),
                    )),
                    _ => Err(ErrorReversion::SinInversa),
                },
            };
        };
        let fin = Instant::now() + self.tiempos.alto;
        // Plan del último intento que llegó a escribir (o pudo llegar).
        let mut enviado = None;
        loop {
            let error = match self.revertir_una_vez(&req, plan, &mut enviado).await {
                Ok(()) => return Ok(()),
                Err(FalloReversion::Definitivo(e)) => return Err(ErrorReversion::Fallo(e)),
                Err(FalloReversion::Reintentar(e)) => e,
            };
            tracing::warn!(cambio = id, "reversión: {error}; se reintenta");
            if Instant::now() >= fin {
                return Err(ErrorReversion::Fallo(error));
            }
            tokio::time::sleep(self.tiempos.intervalo).await;
        }
    }

    /// Vigila un grupo aplicado hasta un estado final.
    pub(super) async fn seguir(&self, ej: Ejecucion) {
        let id = ej.id;
        match ej.base {
            Base::Uptime(antes) => match self.esperar_vuelta(antes).await {
                Ok(()) => {
                    self.marcar(id, Estado::Aplicado, None, ej.nota.as_deref())
                        .await
                }
                Err(e) => self.marcar(id, Estado::Fallido, Some(&e), None).await,
            },
            Base::Wan(wan) => {
                let nota = if wan {
                    ej.nota.clone()
                } else {
                    let extra = "la WAN ya estaba caída antes del cambio: \
                                 solo se vigila el acceso al router";
                    Some(match &ej.nota {
                        Some(n) => format!("{n}; {extra}"),
                        None => extra.to_owned(),
                    })
                };
                let dur = self.tiempos.vigilancia(ej.riesgo);
                match self.vigilar(dur, wan).await {
                    Ok(()) => {
                        self.marcar(id, Estado::Aplicado, None, nota.as_deref())
                            .await
                    }
                    Err(motivo)
                        if ej
                            .pasos
                            .iter()
                            .any(|p| p.plan.operacion == Operacion::Accion) =>
                    {
                        let e = format!("{motivo}; una acción no se puede revertir");
                        self.marcar(id, Estado::Fallido, Some(&e), nota.as_deref())
                            .await;
                    }
                    Err(motivo) => {
                        self.revertir_grupo(&ej, &motivo, nota.as_deref(), Estado::Revertido)
                            .await
                    }
                }
            }
        }
    }

    /// Revierte el grupo entero en orden inverso: `ok` si todo vuelve; si
    /// no, `revertido_parcial` (o `fallido` con un solo paso).
    async fn revertir_grupo(&self, ej: &Ejecucion, motivo: &str, nota: Option<&str>, ok: Estado) {
        tracing::warn!(cambio = ej.id, "{motivo}; revirtiendo");
        let fallos = self.revertir_pasos(ej.id, &ej.pasos).await;
        if fallos.is_empty() {
            let e = format!("{motivo}; revertido a los valores anteriores");
            self.marcar(ej.id, ok, Some(&e), nota).await;
        } else {
            let estado = if ej.multi {
                Estado::RevertidoParcial
            } else {
                Estado::Fallido
            };
            let e = format!("{motivo}; {}", detalle_reversion(ej.multi, &fallos));
            self.marcar(ej.id, estado, Some(&e), nota).await;
            self.cerrar_sin_revertir(ej.id, &ej.pasos, &fallos).await;
        }
    }

    /// Retoma un grupo de varios pasos cortado por un reinicio del panel.
    ///
    /// Si todos sus pasos se escribieron y el router los muestra, la
    /// vigilancia empieza de cero. Si no (cortado a medias, a mitad de una
    /// reversión, o algún paso no se ve), se revierte lo escrito en orden
    /// inverso, en segundo plano: el router vuelve entero a lo de antes.
    ///
    /// `reserva`: el hueco, ya fijado en `id` antes de leer nada.
    pub(super) async fn reanudar_grupo(
        self: &Arc<Self>,
        id: i64,
        pasos: Vec<PasoGuardado>,
        aplicando: bool,
        wan_antes: bool,
        reserva: Reserva,
    ) {
        const RETOMADA: &str = "vigilancia retomada tras un reinicio del panel";
        let mut escritos = Vec::new();
        let mut sin_plan = Vec::new();
        let mut completo = true;
        for p in pasos {
            if p.con_error {
                completo = false;
            }
            match p.estado {
                EstadoPaso::Omitido => {}
                EstadoPaso::Aplicando | EstadoPaso::Aplicado => match p.plan {
                    Ok(plan) => escritos.push(PasoEscrito {
                        orden: p.orden,
                        plan,
                        verificado: p.estado == EstadoPaso::Aplicado,
                    }),
                    Err(e) => {
                        completo = false;
                        sin_plan.push((p.orden, p.funcion, ErrorReversion::Fallo(e)));
                    }
                },
                EstadoPaso::Pendiente | EstadoPaso::Fallido | EstadoPaso::Revertido => {
                    completo = false
                }
            }
        }
        let mut notas = Vec::new();
        if completo {
            for p in &mut escritos {
                match self.comprobar(&p.plan, Lado::Nuevo).await {
                    Comprobacion::Coincide(nueva) => {
                        if let Some(n) = nueva
                            && p.plan.instancia.is_none()
                        {
                            self.fijar_instancia(id, p.orden, &n).await;
                            p.plan.instancia = Some(n);
                        }
                    }
                    Comprobacion::Limitada => notas.push(format!(
                        "{}: verificación limitada",
                        nombre_paso(p.orden, &p.plan.funcion.name)
                    )),
                    Comprobacion::Difiere(_) | Comprobacion::SinLectura(_) => {
                        completo = false;
                        break;
                    }
                }
            }
        }
        let riesgo = escritos
            .iter()
            .map(|p| p.plan.funcion.riesgo)
            .max()
            .unwrap_or(Riesgo::Bajo);
        if completo {
            for p in &escritos {
                self.marcar_paso(id, p.orden, EstadoPaso::Aplicado, None)
                    .await;
            }
            notas.insert(0, RETOMADA.to_owned());
            let nota = notas.join("; ");
            self.marcar(id, Estado::Vigilando, None, Some(&nota)).await;
            let ej = Ejecucion {
                id,
                pasos: escritos,
                base: Base::Wan(wan_antes),
                nota: Some(nota),
                riesgo,
                multi: true,
            };
            self.vigilar_en_segundo_plano(ej, reserva);
            return;
        }
        let m = self.clone();
        let h = tokio::spawn(async move {
            let motivo = "interrumpido por un reinicio del panel antes de terminar";
            let mut fallos = m.revertir_pasos(id, &escritos).await;
            fallos.extend(sin_plan);
            if fallos.is_empty() && escritos.is_empty() {
                let e = format!("{motivo}; no quedó ningún paso aplicado");
                m.marcar(id, Estado::Fallido, Some(&e), None).await;
            } else if fallos.is_empty() {
                // Cortado mientras se vigilaba: como una vigilancia fallida.
                let ok = if aplicando {
                    Estado::Fallido
                } else {
                    Estado::Revertido
                };
                let e = format!("{motivo}; revertido a los valores anteriores");
                m.marcar(id, ok, Some(&e), None).await;
            } else {
                let e = format!("{motivo}; {}", detalle_reversion(true, &fallos));
                m.marcar(id, Estado::RevertidoParcial, Some(&e), None).await;
                m.cerrar_sin_revertir(id, &escritos, &fallos).await;
            }
            drop(reserva);
        });
        *lock(&self.tarea) = Some(h.abort_handle());
    }

    pub(super) fn vigilar_en_segundo_plano(self: &Arc<Self>, ej: Ejecucion, reserva: Reserva) {
        let m = self.clone();
        let h = tokio::spawn(async move {
            m.seguir(ej).await;
            drop(reserva);
        });
        *lock(&self.tarea) = Some(h.abort_handle());
    }
}

/// Un paso guardado, para retomar un grupo.
pub(super) struct PasoGuardado {
    pub(super) orden: usize,
    pub(super) funcion: String,
    pub(super) estado: EstadoPaso,
    /// Tiene un error guardado: falló, o falló su reversión (el grupo se
    /// estaba revirtiendo y hay que terminar de revertirlo).
    pub(super) con_error: bool,
    /// Plan (solo de los pasos escritos), o por qué no se puede rehacer.
    pub(super) plan: Result<Plan, String>,
}
