//! «Registro del router» (Ajustes → Sistema): el registro del sistema del
//! router, como lo da syslog.asp en su web avanzada.
//!
//! Solo lectura: pide al router que exporte el registro (`cbSetSyslog` con
//! solo `export=EXPORT`, que no cambia ningún ajuste) y lo lee de
//! `/syslog.log` ([`hgu::Client::registro_del_sistema`]). No pasa por el
//! gestor de cambios ni deja nada en el Historial.
//!
//! TODO(T12): sin comprobar en el router real (la exportación, la ruta del
//! fichero y su tamaño).

use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};

/// El registro leído.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistroRouter {
    /// El texto, con las líneas más nuevas al final.
    pub texto: String,
    /// Era muy largo: solo llegan sus últimas líneas
    /// (`hgu::client::TOPE_REGISTRO`).
    pub recortado: bool,
}

/// Exporta y lee el registro del sistema del router (ver el módulo). Exige
/// sesión; la respuesta va con `no-store`. 503 sin router configurado y 502
/// si el router no lo da.
#[server(endpoint = "leer_registro_router", input = Json)]
pub async fn leer_registro_router() -> Result<RegistroRouter, ServerFnError> {
    servidor::leer().await
}

#[cfg(feature = "ssr")]
mod servidor {
    use super::*;
    use crate::{instalar::servidor::fallo, panel_ajustes::servidor::preparar, t};
    use axum::http::StatusCode;

    pub(super) async fn leer() -> Result<RegistroRouter, ServerFnError> {
        let (state, _) = preparar().await?;
        match state.hgu.registro_del_sistema().await {
            Ok(r) => Ok(RegistroRouter {
                texto: r.texto,
                recortado: r.recortado,
            }),
            Err(hgu::Error::NoConfigurado) => Err(fallo(
                StatusCode::SERVICE_UNAVAILABLE,
                t!("registro-router-sin-router"),
            )),
            Err(e) => {
                tracing::warn!("registro del router: {e}");
                Err(fallo(StatusCode::BAD_GATEWAY, t!("registro-router-error")))
            }
        }
    }
}
