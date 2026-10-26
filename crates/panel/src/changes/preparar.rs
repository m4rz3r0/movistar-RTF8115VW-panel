//! Preparar, confirmar y deshacer un cambio de un paso.

use super::*;

impl ChangeManager {
    /// Lee el router y calcula el diff de `req`. Devuelve un token para
    /// [`ChangeManager::confirmar`]. Es un grupo de un paso
    /// ([`ChangeManager::preparar_grupo`]). Con el panel en solo lectura
    /// ([`crate::instancia::solo_lectura`]) no prepara nada:
    /// `Bloqueada(`[`BLOQUEO_SOLO_LECTURA`]`)`.
    pub async fn preparar(&self, req: ChangeRequest) -> Result<Preparado, CambioError> {
        self.preparar_con(vec![(req, None)], false, None, None)
            .await
    }

    /// Prepara el grupo de `reqs` (cada paso con su etiqueta) con el título
    /// `titulo` y lo presenta como un [`Preparado`].
    async fn preparar_con(
        &self,
        pasos: Vec<(ChangeRequest, Option<String>)>,
        explicito: bool,
        deshace: Option<i64>,
        titulo: Option<String>,
    ) -> Result<Preparado, CambioError> {
        let reqs: Vec<ChangeRequest> = pasos.iter().map(|(r, _)| r.clone()).collect();
        let g = GrupoRequest {
            pasos: pasos
                .into_iter()
                .map(|(req, etiqueta)| Paso { req, etiqueta })
                .collect(),
            titulo,
        };
        let g = self.preparar_grupo_con(g, explicito, deshace).await?;
        let primero = g.pasos.iter().position(|p| !p.sin_cambios).unwrap_or(0);
        let req = &reqs[primero];
        let palabra = reqs
            .iter()
            .filter_map(|r| self.state.catalogo.funcion(&r.funcion))
            .find_map(palabra_requerida)
            .map(str::to_owned);
        Ok(Preparado {
            token: g.token,
            diff: Diff {
                campos: g
                    .pasos
                    .iter()
                    .flat_map(|p| p.diff.campos.iter().cloned())
                    .collect(),
            },
            riesgo: g.riesgo,
            vigila: g.vigila,
            palabra,
            funcion: req.funcion.clone(),
            instancia: req.instancia.clone(),
            borrar: req.borrar,
            pasos: g.pasos,
        })
    }

    /// Aplica el cambio preparado con `token` (todos sus pasos): escribe,
    /// verifica y deja la vigilancia en marcha. Devuelve el id del cambio
    /// (en `vigilando`). Las acciones exigen `palabra`.
    ///
    /// Desde la reserva del hueco, el trabajo corre en una tarea propia: si
    /// este futuro se abandona, el cambio sigue hasta un estado final.
    pub async fn confirmar(
        self: &Arc<Self>,
        token: &str,
        palabra: Option<&str>,
        usuario: &str,
    ) -> Result<i64, CambioError> {
        self.confirmar_con(token, palabra, &[], usuario).await
    }

    /// Tras una escritura sin respuesta (error que no es `CallFailed`):
    /// relee el router, con reintentos durante la vigilancia del riesgo, para
    /// saber si la escritura llegó. Si el primer intento no consigue leer, se
    /// suelta la pausa del sondeo (`pausa`): el estado del panel refleja el
    /// router mientras se reintenta.
    ///
    /// - Muestra lo nuevo, no se puede comprobar o no se consigue leer ⇒
    ///   `Aplicada`: se vigila como cualquier cambio (si el router sigue sin
    ///   responder, la vigilancia revierte) y deshacer queda disponible. La
    ///   comprobación que se devuelve es `Coincide` o `Limitada`.
    /// - Muestra otros valores: si son los de antes ⇒ `Fallido` (no llegó
    ///   nada); si no son ni los de antes ni los nuevos (a medias),
    ///   `OtrosValores`: no se da por aplicada y se revierte; al crear,
    ///   `Fallido` (sin la instancia no hay nada que revertir).
    pub(super) async fn tras_escritura_sin_respuesta(
        &self,
        plan: &Plan,
        e: &hgu::Error,
        pausa: &mut Option<crate::poller::PauseGuard>,
    ) -> SinRespuesta {
        let sin_confirmar = format!("el router no confirmó la escritura ({e})");
        // El texto del cliente del router (con la función) va a lo técnico.
        let nota = |otra: Nota| {
            Nota::clave(nota::SIN_CONFIRMAR)
                .tecnico(e.to_string())
                .y(otra)
        };
        let fin = Instant::now() + self.tiempos.vigilancia(plan.funcion.riesgo);
        loop {
            match self.comprobar(plan, Lado::Nuevo).await {
                c @ Comprobacion::Coincide(_) => {
                    return SinRespuesta::Aplicada(c, nota(Nota::clave(nota::RELEIDO_APLICADA)));
                }
                Comprobacion::Limitada => {
                    return SinRespuesta::Aplicada(
                        Comprobacion::Limitada,
                        nota(Nota::clave(nota::SIN_CAMPOS_ESCRITOS)),
                    );
                }
                Comprobacion::Difiere(v) => {
                    return match self.comprobar(plan, Lado::Anterior).await {
                        Comprobacion::Coincide(_) => SinRespuesta::Fallido(format!(
                            "{sin_confirmar}; el router sigue con los valores de antes: no se aplicó"
                        )),
                        // Al crear, sin saber cuál es la instancia nueva no
                        // se podría revertir ni deshacer: no se toca nada.
                        _ if plan.operacion == Operacion::Crear => SinRespuesta::CreacionIncierta(
                            format!("{sin_confirmar}; {}", v.join("; ")),
                        ),
                        _ => SinRespuesta::OtrosValores(format!(
                            "{sin_confirmar}; el router muestra otros valores: {}",
                            v.join("; ")
                        )),
                    };
                }
                Comprobacion::SinLectura(l) => {
                    if Instant::now() >= fin {
                        return SinRespuesta::Aplicada(
                            Comprobacion::Limitada,
                            nota(Nota::clave(nota::SIN_RELEER).tecnico(l.to_string())),
                        );
                    }
                    // No congelar el sondeo durante los reintentos.
                    drop(pausa.take());
                    tokio::time::sleep(self.tiempos.intervalo).await;
                }
            }
        }
    }

    /// Relee lo escrito tras una escritura que el router confirmó. Si la
    /// relectura no sale (tras algunos cambios, como el del QoS, el router
    /// rehace su red o su servidor web y tarda en volver), se reintenta cada
    /// `intervalo` durante el plazo de la vigilancia de su riesgo (al menos
    /// una vez: la primera relectura pudo agotar ella sola el plazo), sin
    /// congelar el sondeo; después, lo que dé la última.
    pub(super) async fn releer_tras_escribir(
        &self,
        plan: &Plan,
        pausa: &mut Option<crate::poller::PauseGuard>,
    ) -> Comprobacion {
        let fin = Instant::now() + self.tiempos.vigilancia(plan.funcion.riesgo);
        let mut reintentos = 0u32;
        loop {
            match self.comprobar(plan, Lado::Nuevo).await {
                Comprobacion::SinLectura(e) if reintentos == 0 || Instant::now() < fin => {
                    if reintentos == 0 {
                        tracing::warn!(
                            funcion = %plan.funcion.name,
                            "no se puede releer lo escrito; se reintenta: {e}"
                        );
                    }
                    reintentos += 1;
                    drop(pausa.take());
                    tokio::time::sleep(self.tiempos.intervalo).await;
                }
                c => return c,
            }
        }
    }

    /// Motivo por el que `original` ya no se puede deshacer: lo deshizo otro
    /// cambio que está en curso o aplicado.
    /// Si no se puede consultar, también se rechaza (mejor no deshacer que
    /// deshacer dos veces).
    pub(super) async fn ya_deshecho(&self, original: i64) -> Option<String> {
        let r: Option<(i64, String)> = match sqlx::query_as(
            "SELECT d.id, d.estado FROM changes o JOIN changes d ON d.id = o.deshecho_por \
             WHERE o.id = ?",
        )
        .bind(original)
        .fetch_optional(&self.state.db)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(cambio = original, "no se puede leer deshecho_por: {e}");
                return Some(format!(
                    "no se puede comprobar si el cambio {original} ya se deshizo: {e}"
                ));
            }
        };
        let (d, estado) = r?;
        // Un deshacer que quedó revertido en parte también cuenta: parte de
        // él sigue aplicada y no se deshace dos veces.
        let vigente = [
            Estado::Aplicando,
            Estado::Vigilando,
            Estado::Aplicado,
            Estado::RevertidoParcial,
        ]
        .iter()
        .any(|e| e.as_str() == estado);
        vigente.then(|| format!("el cambio {original} ya se ha deshecho (cambio {d})"))
    }

    /// Peticiones inversas de un cambio `aplicado` que nadie ha deshecho, con
    /// los valores anteriores (secretos incluidos), en orden inverso al de
    /// sus pasos. Es la única comprobación de qué se puede deshacer: la usan
    /// [`ChangeManager::deshacer`] y [`ChangeManager::se_puede_deshacer`].
    /// Pasos inversos de un cambio aplicado, cada uno con la etiqueta del
    /// paso que deshace.
    async fn inversa_de(
        &self,
        change_id: i64,
    ) -> Result<Vec<(ChangeRequest, Option<String>)>, CambioError> {
        let fila = self
            .fila(change_id)
            .await
            .ok_or_else(|| CambioError::Invalida("el cambio no existe".into()))?;
        let estado: String = fila.get("estado");
        if estado != Estado::Aplicado.as_str() {
            return Err(CambioError::Invalida(
                "solo se puede deshacer un cambio aplicado".into(),
            ));
        }
        if let Some(e) = self.ya_deshecho(change_id).await {
            return Err(CambioError::Invalida(e));
        }
        let no_se_puede = || {
            CambioError::Invalida(
                "este cambio no se puede deshacer (acción o valores anteriores desconocidos)"
                    .into(),
            )
        };
        // Solo cambió lo que el router no deja leer (su valor anterior es el
        // vacío de la lectura, que el deshacer omite): no hay nada que volver.
        if let (true, _) = self.tapados_del_cambio(change_id).await {
            return Err(CambioError::Invalida(DESHACER_SOLO_TAPADOS.into()));
        }
        let mut reqs = Vec::new();
        for paso in self.filas_pasos(change_id).await.iter().rev() {
            if paso.get::<String, _>("estado") != EstadoPaso::Aplicado.as_str() {
                continue;
            }
            let plan = self.reconstruir(paso).map_err(CambioError::Invalida)?;
            let etiqueta: Option<String> = paso.get("etiqueta");
            let mut req = inversa(&plan).ok_or_else(no_se_puede)?;
            // Un usuario borrado vuelve con una contraseña nueva aleatoria.
            super::especiales::con_contrasena_nueva(&mut req);
            reqs.push((req, etiqueta));
        }
        if reqs.is_empty() {
            return Err(no_se_puede());
        }
        Ok(reqs)
    }

    /// Prepara el cambio inverso de uno `aplicado` (con los valores
    /// anteriores, secretos incluidos): en un grupo, los pasos inversos en
    /// orden inverso, que se confirman juntos con [`ChangeManager::confirmar`].
    pub async fn deshacer(&self, change_id: i64) -> Result<Preparado, CambioError> {
        let reqs = self.inversa_de(change_id).await?;
        // El deshacer lleva el título del cambio que deshace (el de su
        // ajuste, si se hizo desde Ajustes): así el Historial lo nombra igual.
        let titulo = self.cambio(change_id).await.and_then(|c| c.titulo);
        self.preparar_con(reqs, true, Some(change_id), titulo).await
    }

    /// Los parámetros que el router da tapados al leer y que cambiaron en los
    /// pasos aplicados del cambio `id` (`(función, parámetro)`), y si solo
    /// cambiaron ellos. El deshacer los omite: su valor anterior es el vacío
    /// que dio la lectura.
    pub async fn tapados_del_cambio(&self, id: i64) -> (bool, Vec<(String, String)>) {
        let (mut tapados, mut otros) = (Vec::new(), 0usize);
        for f in self.filas_pasos(id).await {
            if f.get::<String, _>("estado") != EstadoPaso::Aplicado.as_str() {
                continue;
            }
            let funcion: String = f.get("funcion");
            let diff: Vec<CampoDiff> =
                serde_json::from_str(&f.get::<String, _>("diff")).unwrap_or_default();
            let func = self.state.catalogo.funcion(&funcion);
            for d in diff {
                if func
                    .and_then(|f| f.parametro(&d.nombre))
                    .is_some_and(|p| p.tapado)
                {
                    tapados.push((funcion.clone(), d.nombre));
                } else {
                    otros += 1;
                }
            }
        }
        (!tapados.is_empty() && otros == 0, tapados)
    }

    /// ¿Aceptaría [`ChangeManager::deshacer`] este cambio? Solo mira el
    /// historial (no el router): para mostrar o no el botón.
    pub async fn se_puede_deshacer(&self, change_id: i64) -> bool {
        self.inversa_de(change_id).await.is_ok()
    }

    /// Enlaces de deshacer de un cambio: el que lo deshizo y el que él
    /// deshace. Solo guardan el vínculo de la última vez que se deshizo un
    /// cambio.
    pub async fn enlaces_deshacer(&self, id: i64) -> (Option<i64>, Option<i64>) {
        let consulta = |sql: &'static str| async move {
            sqlx::query_scalar::<_, i64>(sql)
                .bind(id)
                .fetch_optional(&self.state.db)
                .await
                .unwrap_or_else(|e| {
                    tracing::error!(cambio = id, "no se pueden leer los enlaces: {e}");
                    None
                })
        };
        (
            consulta("SELECT deshecho_por FROM changes WHERE id = ? AND deshecho_por IS NOT NULL")
                .await,
            consulta("SELECT id FROM changes WHERE deshecho_por = ? ORDER BY id DESC LIMIT 1")
                .await,
        )
    }
}
