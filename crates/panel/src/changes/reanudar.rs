//! Reanudación al arrancar de los cambios que quedaron a medias.

use super::*;

impl ChangeManager {
    /// Retoma los cambios que quedaron en `aplicando`/`vigilando` (llamar al
    /// arrancar, con el sondeo ya en marcha). Si el router muestra los
    /// valores nuevos, la vigilancia empieza de cero; si muestra los
    /// anteriores, `fallido` ("interrumpido antes de aplicar"); si no se
    /// puede saber, `fallido` con el motivo.
    ///
    /// Solo actúa la primera vez; hasta entonces `confirmar` espera.
    pub async fn reanudar_pendientes(self: &Arc<Self>) {
        if self
            .reanudando
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return;
        }
        // En su propia tarea: si entra en pánico se registra, y `listo` se
        // marca al salir pase lo que pase (también si se abandona este
        // futuro: la tarea sigue y lo marca al terminar).
        struct ListoAlSalir(Arc<ChangeManager>);
        impl Drop for ListoAlSalir {
            fn drop(&mut self) {
                self.0.listo.send_replace(true);
            }
        }
        let m = self.clone();
        let h = tokio::spawn(async move {
            let _listo = ListoAlSalir(m.clone());
            m.reanudar_filas().await;
        });
        if let Err(e) = h.await {
            tracing::error!("error interno al retomar los cambios pendientes: {e}");
        }
    }

    async fn reanudar_filas(self: &Arc<Self>) {
        let filas = match sqlx::query(
            "SELECT * FROM changes WHERE estado IN ('aplicando', 'vigilando') ORDER BY id",
        )
        .fetch_all(&self.state.db)
        .await
        {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("no se pueden leer los cambios pendientes: {e}");
                return;
            }
        };
        for fila in filas {
            let id: i64 = fila.get("id");
            // El hueco, antes de leer o tocar nada: lo que lo ocupe (un
            // cambio de credenciales que empezó antes, o el cambio anterior
            // de esta lista) termina antes de seguir. Así ningún grupo se da
            // por cerrado sin intentar revertirlo.
            let reserva = self.esperar_hueco(id).await;
            reserva.fijar(id);
            let aplicando = fila.get::<String, _>("estado") == Estado::Aplicando.as_str();
            // La WAN como estaba antes del cambio (nunca la de ahora).
            let wan_antes = fila.get::<Option<i64>, _>("wan_antes") != Some(0);
            let pasos: Vec<PasoGuardado> = self
                .filas_pasos(id)
                .await
                .iter()
                .map(|f| {
                    let estado = EstadoPaso::parse(&f.get::<String, _>("estado"))
                        .unwrap_or(EstadoPaso::Pendiente);
                    let escrito = matches!(estado, EstadoPaso::Aplicando | EstadoPaso::Aplicado);
                    PasoGuardado {
                        orden: f.get::<i64, _>("orden") as usize,
                        funcion: f.get("funcion"),
                        estado,
                        con_error: f.get::<Option<String>, _>("error").is_some(),
                        plan: if escrito {
                            self.reconstruir(f)
                        } else {
                            Err("sin escribir".into())
                        },
                    }
                })
                .collect();
            let activos: Vec<&PasoGuardado> = pasos
                .iter()
                .filter(|p| p.estado != EstadoPaso::Omitido)
                .collect();
            match activos.as_slice() {
                [] => {
                    let e = "interrumpido; el cambio no tiene pasos";
                    self.marcar(id, Estado::Fallido, Some(e), None).await;
                }
                [_] => {
                    let p = pasos.into_iter().find(|p| p.estado != EstadoPaso::Omitido);
                    let Some(p) = p else { continue };
                    match (p.estado, p.plan) {
                        (EstadoPaso::Aplicando | EstadoPaso::Aplicado, Ok(plan)) => {
                            self.reanudar(id, p.orden, plan, aplicando, wan_antes, reserva)
                                .await
                        }
                        (EstadoPaso::Aplicando | EstadoPaso::Aplicado, Err(e)) => {
                            self.marcar_paso(id, p.orden, EstadoPaso::Fallido, Some(&e))
                                .await;
                            self.marcar(id, Estado::Fallido, Some(&e), None).await;
                        }
                        // Sin escribir, fallido o ya revertido: nada aplicado
                        // que retomar.
                        _ => {
                            let e = "interrumpido: el cambio no quedó aplicado";
                            self.marcar(id, Estado::Fallido, Some(e), None).await;
                        }
                    }
                }
                _ => {
                    self.reanudar_grupo(id, pasos, aplicando, wan_antes, reserva)
                        .await
                }
            }
        }
    }

    /// Retoma un cambio de un paso (`orden`); `aplicando`: se cortó antes de
    /// terminar de escribir y verificar. `reserva`: el hueco, ya fijado.
    async fn reanudar(
        self: &Arc<Self>,
        id: i64,
        orden: usize,
        mut plan: Plan,
        aplicando: bool,
        wan_antes: bool,
        reserva: Reserva,
    ) {
        const RETOMADA: &str = "vigilancia retomada tras un reinicio del panel";
        let fallido = |e: String| async move {
            self.marcar_paso(id, orden, EstadoPaso::Fallido, Some(&e))
                .await;
            self.marcar(id, Estado::Fallido, Some(&e), None).await;
        };
        let c = &self.state.hgu;
        if plan.funcion.name == REBOOT {
            match read::summary(c).await {
                // No se sabe si la orden llegó al router.
                Ok(_) if aplicando => {
                    fallido("interrumpido: no se sabe si el router llegó a reiniciarse".into())
                        .await;
                }
                Ok(_) => {
                    self.marcar(id, Estado::Aplicado, None, Some(RETOMADA))
                        .await
                }
                Err(e) => {
                    fallido(format!(
                        "el router no responde tras reiniciar el panel: {e}"
                    ))
                    .await
                }
            }
            return;
        }
        let nota = match self.comprobar(&plan, Lado::Nuevo).await {
            Comprobacion::Coincide(nueva) => {
                if let Some(n) = nueva
                    && plan.instancia.is_none()
                {
                    self.fijar_instancia(id, orden, &n).await;
                    plan.instancia = Some(n);
                }
                RETOMADA.to_owned()
            }
            Comprobacion::SinLectura(e) => {
                fallido(format!(
                    "interrumpido; no se puede comprobar el estado: {e}"
                ))
                .await;
                return;
            }
            Comprobacion::Limitada => format!("{RETOMADA}; verificación limitada"),
            Comprobacion::Difiere(v) => {
                let anterior = self.comprobar(&plan, Lado::Anterior).await;
                let e = if anterior == Comprobacion::Coincide(None) {
                    "interrumpido antes de aplicar: el router tiene los valores anteriores"
                        .to_owned()
                } else {
                    format!(
                        "interrumpido; el router no muestra el cambio: {}",
                        v.join("; ")
                    )
                };
                fallido(e).await;
                return;
            }
        };
        self.marcar_paso(id, orden, EstadoPaso::Aplicado, None)
            .await;
        self.marcar(id, Estado::Vigilando, None, Some(&nota)).await;
        let ej = Ejecucion {
            id,
            riesgo: plan.funcion.riesgo,
            pasos: vec![grupo::PasoEscrito {
                orden,
                plan,
                verificado: true,
            }],
            base: Base::Wan(wan_antes),
            nota: Some(nota),
            multi: false,
        };
        self.vigilar_en_segundo_plano(ej, reserva);
    }
}
