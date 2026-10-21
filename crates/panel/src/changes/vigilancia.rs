//! Vigilancia tras un cambio y reversión.

use super::*;

impl ChangeManager {
    /// Comprueba el router cada `intervalo` durante `dur`. Error con el
    /// motivo tras `fallos_para_revertir` fallos seguidos. Si el plazo acaba
    /// con fallos pendientes, sigue hasta un acierto o la reversión.
    pub(super) async fn vigilar(&self, dur: Duration, wan: bool) -> Result<(), Nota> {
        let fin = Instant::now() + dur;
        let mut seguidos = 0;
        loop {
            tokio::time::sleep(self.tiempos.intervalo).await;
            match self.salud(wan).await {
                Ok(()) => seguidos = 0,
                Err(e) => {
                    seguidos += 1;
                    tracing::warn!("vigilancia: fallo {seguidos}: {}", e.para_registro());
                    if seguidos >= self.tiempos.fallos_para_revertir {
                        return Err(e);
                    }
                }
            }
            if seguidos == 0 && Instant::now() >= fin {
                return Ok(());
            }
        }
    }

    /// El router deja iniciar sesión y (si `wan`) la conexión a internet
    /// está arriba.
    async fn salud(&self, wan: bool) -> Result<(), Nota> {
        let c = &self.state.hgu;
        let no_responde =
            |e: hgu::Error| Nota::clave(nota::ERROR_ROUTER_NO_RESPONDE).tecnico(e.to_string());
        read::summary(c).await.map_err(no_responde)?;
        if !wan {
            return Ok(());
        }
        let (vigilada, arriba) = self.state.wan_vigilada().await.map_err(no_responde)?;
        if !arriba {
            let detalle = match vigilada {
                Some(n) => format!("la WAN ({n}) está caída"),
                None => "la WAN está caída (ninguna conexión de datos arriba)".to_owned(),
            };
            return Err(Nota::clave(nota::ERROR_INTERNET_CAIDA).tecnico(detalle));
        }
        Ok(())
    }

    /// Tras `cbReboot`: espera hasta `alto` a que el router vuelva (responde
    /// y su tiempo encendido es menor que antes, o se le vio caído).
    pub(super) async fn esperar_vuelta(&self, uptime_antes: u64) -> Result<(), Nota> {
        let fin = Instant::now() + self.tiempos.alto;
        let mut caido = false;
        while Instant::now() < fin {
            tokio::time::sleep(self.tiempos.intervalo).await;
            match read::summary(&self.state.hgu).await {
                Ok(s) if caido || s.uptime_s < uptime_antes => return Ok(()),
                Ok(_) => {}
                Err(_) => caido = true,
            }
        }
        Err(Nota::con(
            nota::ERROR_REINICIO_SIN_VUELTA,
            &[("s", format!("{}", self.tiempos.alto.as_secs_f32()))],
        ))
    }

    /// Un intento de reversión de `original` con la petición inversa `req`.
    ///
    /// Idempotente: antes de escribir comprueba si un intento anterior ya
    /// surtió efecto (`enviado`: lo que se escribió; p. ej. la instancia
    /// recreada ya existe) o si el router ya muestra lo anterior (p. ej. la
    /// instancia creada ya no existe). En ese caso termina sin escribir.
    pub(super) async fn revertir_una_vez(
        &self,
        req: &ChangeRequest,
        original: &Plan,
        enviado: &mut Option<Plan>,
    ) -> Result<(), FalloReversion> {
        use FalloReversion::{Definitivo, Reintentar};
        let no_verificada =
            |v: Vec<String>| Definitivo(format!("reversión no verificada: {}", v.join("; ")));
        // El sondeo se pausa si se puede; si no, se revierte igualmente (las
        // peticiones al router van serializadas de todos modos).
        let _pausa = tokio::time::timeout(self.tiempos.pausa_max, self.state.poller.pause())
            .await
            .ok();
        if let Some(previo) = enviado.as_ref() {
            match self.comprobar(previo, Lado::Nuevo).await {
                Comprobacion::Coincide(_) => return Ok(()),
                Comprobacion::SinLectura(e) => return Err(Reintentar(e)),
                Comprobacion::Limitada | Comprobacion::Difiere(_) => {}
            }
        }
        match self.comprobar(original, Lado::Anterior).await {
            Comprobacion::Coincide(_) => return Ok(()),
            Comprobacion::SinLectura(e) => return Err(Reintentar(e)),
            Comprobacion::Limitada | Comprobacion::Difiere(_) => {}
        }
        let plan = match self.planificar(req, true).await {
            Ok(p) => p,
            // Nada que escribir pero el router no muestra lo anterior.
            Err(CambioError::SinCambios) => {
                return match self.comprobar(original, Lado::Anterior).await {
                    Comprobacion::Coincide(_) | Comprobacion::Limitada => Ok(()),
                    Comprobacion::SinLectura(e) => Err(Reintentar(e)),
                    Comprobacion::Difiere(v) => Err(no_verificada(v)),
                };
            }
            Err(CambioError::Router(e)) => return Err(Reintentar(e)),
            Err(e) => return Err(Definitivo(e.to_string())),
        };
        let mut plan = plan;
        if plan.operacion == Operacion::Crear && matches!(plan.lectura, Lectura::Xml { .. }) {
            match self.ids_de_la_lista(&plan).await {
                Ok(ids) => plan.ids_antes = Some(ids),
                Err(e) => return Err(Reintentar(e.to_string())),
            }
        }
        // Desde aquí la escritura puede haber llegado aunque falle la respuesta.
        *enviado = Some(plan.clone());
        match self.enviar(&plan).await {
            Ok(()) => {}
            // El router rechazó los valores: reintentar no cambiaría nada.
            Err(e @ hgu::Error::CallFailed { .. }) => {
                return Err(Definitivo(format!("el router rechazó la reversión: {e}")));
            }
            Err(e) => return Err(Reintentar(e.to_string())),
        }
        match self.comprobar(&plan, Lado::Nuevo).await {
            Comprobacion::Coincide(_) | Comprobacion::Limitada => Ok(()),
            Comprobacion::SinLectura(e) => Err(Reintentar(e)),
            Comprobacion::Difiere(v) => Err(no_verificada(v)),
        }
    }
}
