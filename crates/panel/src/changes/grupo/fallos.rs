//! Fallos y reversión de un grupo, vigilancia y reanudación.

use super::*;

impl ChangeManager {
    /// El router no muestra un paso escrito (o no se pudo releer). Un cambio
    /// de un paso no se revierte (lo que el router no aceptó no se toca,
    /// como en el núcleo). En un grupo, si el router no muestra claramente
    /// lo de antes, el paso pudo quedar aplicado en parte y se revierte con
    /// los demás: el grupo queda entero en lo nuevo o entero en lo de antes.
    pub(super) async fn fallo_de_verificacion(
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
    pub(super) async fn fallo_en_paso(
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
    pub(in crate::changes) async fn revertir_pasos(
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
    pub(in crate::changes) async fn cerrar_sin_revertir(
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
    pub(in crate::changes) async fn seguir(&self, ej: Ejecucion) {
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
    pub(in crate::changes) async fn reanudar_grupo(
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

    pub(in crate::changes) fn vigilar_en_segundo_plano(
        self: &Arc<Self>,
        ej: Ejecucion,
        reserva: Reserva,
    ) {
        let m = self.clone();
        let h = tokio::spawn(async move {
            m.seguir(ej).await;
            drop(reserva);
        });
        *lock(&self.tarea) = Some(h.abort_handle());
    }
}
