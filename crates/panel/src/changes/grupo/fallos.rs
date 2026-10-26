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
    ///
    /// `(funcion, etiqueta)`: la función del paso (solo para el registro) y
    /// la clave de su nombre, que es lo que dice el Historial.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn fallo_en_paso(
        &self,
        id: i64,
        k: usize,
        (funcion, etiqueta): (&str, Option<&str>),
        plan: Option<Plan>,
        fallo: FalloPaso,
        mut escritos: Vec<PasoEscrito>,
        multi: bool,
    ) -> CambioError {
        let texto = Nota::de_error(&fallo.error);
        let texto = if multi {
            texto.del_paso_nombrado(k, etiqueta)
        } else {
            texto
        };
        tracing::warn!(
            cambio = id,
            paso = k + 1,
            funcion,
            "paso fallido: {}",
            texto.para_registro()
        );
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
        tracing::warn!(cambio = id, "revirtiendo lo aplicado");
        let (fallos, aviso) = self.revertir_pasos(id, &escritos).await;
        let (estado, e) = if fallos.is_empty() {
            (Estado::Fallido, texto.y(Nota::clave(nota::ERROR_REVERTIDO)))
        } else {
            let estado = if multi {
                Estado::RevertidoParcial
            } else {
                Estado::Fallido
            };
            (estado, texto.y(detalle_reversion(multi, &fallos)))
        };
        self.marcar(id, estado, Some(&e), aviso.as_ref()).await;
        self.cerrar_sin_revertir(id, &escritos, &fallos).await;
        CambioError::Router(e.para_registro())
    }

    /// Revierte `pasos` en orden inverso y marca cada uno. Devuelve los que
    /// no se pudieron revertir y los avisos para la nota del cambio de lo
    /// revertido o de lo que llegó a escribirse al intentarlo (un usuario
    /// que vuelve con una contraseña que el panel no guarda).
    pub(in crate::changes) async fn revertir_pasos(
        &self,
        id: i64,
        pasos: &[PasoEscrito],
    ) -> (Vec<(usize, String, ErrorReversion)>, Option<Nota>) {
        let mut fallos = Vec::new();
        let mut avisos = Vec::new();
        for p in pasos.iter().rev() {
            let (resultado, aviso) = self.revertir_plan(id, &p.plan).await;
            avisos.extend(aviso);
            match resultado {
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
                    self.marcar_paso(id, p.orden, estado, Some(&e.nota())).await;
                    fallos.push((p.orden, p.plan.funcion.name.clone(), e));
                }
            }
        }
        (fallos, Nota::juntar(avisos))
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
    /// durante `alto`; los deterministas terminan en el acto. Con un aviso
    /// para el Historial si lo revertido lo necesita
    /// ([`nota::USUARIO_REVERTIDO`]), también si falla tras llegar a
    /// escribir.
    async fn revertir_plan(
        &self,
        id: i64,
        plan: &Plan,
    ) -> (Result<(), ErrorReversion>, Option<Nota>) {
        let Some(mut req) = inversa(plan) else {
            // Sin inversa, vale si el router ya muestra lo de antes (p. ej.
            // una creación sin instancia conocida que no llegó a hacerse).
            let r = match plan.operacion {
                Operacion::Accion => Err(ErrorReversion::SinInversa),
                _ => match self.comprobar(plan, Lado::Anterior).await {
                    Comprobacion::Coincide(_) => Ok(()),
                    _ if plan.operacion == Operacion::Crear => Err(ErrorReversion::Fallo(
                        "no se sabe qué instancia se creó; revisa la lista en el router".into(),
                    )),
                    _ => Err(ErrorReversion::SinInversa),
                },
            };
            return (r, None);
        };
        // Un usuario borrado vuelve con una contraseña nueva aleatoria (la
        // de antes no se puede leer). Aquí no hay cambio de deshacer que la
        // guarde para el ojo: el Historial avisa de que hay que cambiarla.
        let recrea = crate::changes::especiales::con_contrasena_nueva(&mut req);
        let fin = Instant::now() + self.tiempos.alto;
        // Plan del último intento que llegó a escribir (o pudo llegar).
        let mut enviado = None;
        let resultado = loop {
            let error = match self.revertir_una_vez(&req, plan, &mut enviado).await {
                Ok(()) => break Ok(()),
                Err(FalloReversion::Definitivo(e)) => break Err(ErrorReversion::Fallo(e)),
                Err(FalloReversion::Reintentar(e)) => e,
            };
            tracing::warn!(cambio = id, "reversión: {error}; se reintenta");
            if Instant::now() >= fin {
                break Err(ErrorReversion::Fallo(error));
            }
            tokio::time::sleep(self.tiempos.intervalo).await;
        };
        // Solo si llegó a escribirse (aunque luego fallara: el alta pudo
        // aplicarse); si el usuario ya estaba, sigue con su contraseña.
        let aviso = (recrea && enviado.is_some()).then(|| Nota::clave(nota::USUARIO_REVERTIDO));
        (resultado, aviso)
    }

    /// Vigila un grupo aplicado hasta un estado final, que devuelve.
    pub(in crate::changes) async fn seguir(&self, ej: Ejecucion) -> Estado {
        let id = ej.id;
        if ej.sin_vigilancia() {
            // Ya quedó aplicado al escribirlo.
            return Estado::Aplicado;
        }
        match ej.base {
            Base::Uptime(antes) => match self.esperar_vuelta(antes).await {
                Ok(()) => {
                    self.marcar(id, Estado::Aplicado, None, ej.nota.as_ref())
                        .await;
                    Estado::Aplicado
                }
                Err(e) => {
                    self.marcar(id, Estado::Fallido, Some(&e), None).await;
                    Estado::Fallido
                }
            },
            Base::Wan(wan) => {
                let nota = if wan {
                    ej.nota.clone()
                } else {
                    let extra = Nota::clave(nota::WAN_CAIDA);
                    Some(match &ej.nota {
                        Some(n) => n.clone().y(extra),
                        None => extra,
                    })
                };
                let dur = self.tiempos.vigilancia(ej.riesgo);
                match self.vigilar(id, dur, wan).await {
                    Ok(None) => {
                        self.marcar(id, Estado::Aplicado, None, nota.as_ref()).await;
                        Estado::Aplicado
                    }
                    Ok(Some((usuario, responder))) => {
                        tracing::info!(cambio = id, usuario, "vigilancia terminada a mano");
                        let fin = Nota::con(nota::VIGILANCIA_TERMINADA, &[("usuario", usuario)]);
                        let nota = Nota::juntar(nota.into_iter().chain([fin]));
                        self.marcar(id, Estado::Aplicado, None, nota.as_ref()).await;
                        let _ = responder.send(Ok(()));
                        Estado::Aplicado
                    }
                    Err(motivo)
                        if ej
                            .pasos
                            .iter()
                            .any(|p| p.plan.operacion == Operacion::Accion) =>
                    {
                        let e = motivo.y(Nota::clave(nota::ERROR_ACCION));
                        self.marcar(id, Estado::Fallido, Some(&e), nota.as_ref())
                            .await;
                        Estado::Fallido
                    }
                    Err(motivo) => {
                        self.revertir_grupo(&ej, motivo, nota.as_ref(), Estado::Revertido)
                            .await
                    }
                }
            }
        }
    }

    /// Revierte el grupo entero en orden inverso: `ok` si todo vuelve; si
    /// no, `revertido_parcial` (o `fallido` con un solo paso). Devuelve el
    /// estado en que queda.
    async fn revertir_grupo(
        &self,
        ej: &Ejecucion,
        motivo: Nota,
        nota: Option<&Nota>,
        ok: Estado,
    ) -> Estado {
        tracing::warn!(cambio = ej.id, "{}; revirtiendo", motivo.para_registro());
        let (fallos, aviso) = self.revertir_pasos(ej.id, &ej.pasos).await;
        let nota = Nota::juntar(nota.cloned().into_iter().chain(aviso));
        let nota = nota.as_ref();
        if fallos.is_empty() {
            let e = motivo.y(Nota::clave(nota::ERROR_REVERTIDO));
            self.marcar(ej.id, ok, Some(&e), nota).await;
            ok
        } else {
            let estado = if ej.multi {
                Estado::RevertidoParcial
            } else {
                Estado::Fallido
            };
            let e = motivo.y(detalle_reversion(ej.multi, &fallos));
            self.marcar(ej.id, estado, Some(&e), nota).await;
            self.cerrar_sin_revertir(ej.id, &ej.pasos, &fallos).await;
            estado
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
                EstadoPaso::Pendiente
                | EstadoPaso::Fallido
                | EstadoPaso::Revertido
                | EstadoPaso::Cancelado => completo = false,
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
                    Comprobacion::Limitada => {
                        notas.push(Nota::clave(nota::VERIFICACION_LIMITADA).del_paso(p.orden))
                    }
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
            notas.insert(0, Nota::clave(nota::RETOMADA));
            let nota = Nota::juntar(notas);
            self.marcar(id, Estado::Vigilando, None, nota.as_ref())
                .await;
            let ej = Ejecucion {
                id,
                pasos: escritos,
                base: Base::Wan(wan_antes),
                nota,
                riesgo,
                multi: true,
                vigilar: true,
            };
            self.vigilar_en_segundo_plano(ej, reserva);
            return;
        }
        let m = self.clone();
        let h = tokio::spawn(async move {
            let motivo = || Nota::clave(nota::ERROR_INTERRUMPIDO);
            let (mut fallos, aviso) = m.revertir_pasos(id, &escritos).await;
            let aviso = aviso.as_ref();
            fallos.extend(sin_plan);
            if fallos.is_empty() && escritos.is_empty() {
                let e = motivo().y(Nota::clave(nota::ERROR_NINGUN_PASO));
                m.marcar(id, Estado::Fallido, Some(&e), aviso).await;
            } else if fallos.is_empty() {
                // Cortado mientras se vigilaba: como una vigilancia fallida.
                let ok = if aplicando {
                    Estado::Fallido
                } else {
                    Estado::Revertido
                };
                let e = motivo().y(Nota::clave(nota::ERROR_REVERTIDO));
                m.marcar(id, ok, Some(&e), aviso).await;
            } else {
                let e = motivo().y(detalle_reversion(true, &fallos));
                m.marcar(id, Estado::RevertidoParcial, Some(&e), aviso)
                    .await;
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
        // Mientras se vigila, otro cambio puede esperar en la cola.
        lock(&self.hueco).con_cola = true;
        let m = self.clone();
        let h = tokio::spawn(async move {
            let estado = m.seguir(ej).await;
            m.seguir_la_cola(reserva, estado).await;
        });
        *lock(&self.tarea) = Some(h.abort_handle());
    }
}
