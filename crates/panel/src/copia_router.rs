//! «Copia de la configuración» (Ajustes → Sistema): la copia de seguridad de
//! la configuración del router, como la descarga backup_restore.asp.
//!
//! Igual que la web, el panel guarda la configuración (`cbSaveConfig` con
//! `save=1`), genera la copia (`backup=1`) y la descarga (`cbDownloadConfig`)
//! con [`hgu::Client::copia_de_configuracion`], y se la pasa al navegador
//! con el nombre que da el router (`config_<fecha>.tar`).
//!
//! El fichero lleva todas las contraseñas del router: el panel no lo anota,
//! no lo guarda y no enseña su contenido; su depuración ([`CopiaRouter`])
//! solo da el nombre y el tamaño. Como guarda la configuración, cuenta como
//! escritura: exige sesión, no se hace en solo lectura ni con un cambio en
//! curso, y solo con `cbSaveConfig` y `cbDownloadConfig` comprobadas en el
//! router real (la lista blanca del catálogo); si no, 409 con
//! [`crate::validacion_real::SIN_COMPROBAR`].

use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};

/// Las funciones del catálogo que hace la copia: las dos tienen que estar
/// comprobadas en el router real.
pub const FUNCIONES: [&str; 2] = ["cbSaveConfig", "cbDownloadConfig"];

/// La copia descargada.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopiaRouter {
    /// El nombre que da el router (`config_<fecha>.tar`).
    pub nombre: String,
    /// El fichero, en base64 (estándar, con relleno).
    pub datos: String,
}

impl std::fmt::Debug for CopiaRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CopiaRouter")
            .field("nombre", &self.nombre)
            .field("base64", &self.datos.len())
            .finish()
    }
}

/// Por qué no se puede hacer la copia ahora: la clave de su motivo
/// ([`crate::validacion_real::SIN_COMPROBAR`] o `panel-solo-lectura`), o
/// `None` si se puede. Exige sesión.
#[server(endpoint = "estado_copia_router", input = Json)]
pub async fn estado_copia_router() -> Result<Option<String>, ServerFnError> {
    servidor::estado().await
}

/// Guarda la configuración del router, genera la copia y la descarga (ver
/// el módulo). Exige sesión; la respuesta va con `no-store`. 403 en solo
/// lectura, 409 sin comprobar o con un cambio en curso, 503 sin router
/// configurado y 502 si el router no la da.
#[server(endpoint = "descargar_copia_router", input = Json)]
pub async fn descargar_copia_router() -> Result<CopiaRouter, ServerFnError> {
    servidor::descargar().await
}

#[cfg(feature = "ssr")]
mod servidor {
    use super::*;
    use crate::state::AppState;
    use crate::validacion_real::SIN_COMPROBAR;
    use crate::{instalar::servidor::fallo, instancia, panel_ajustes::servidor::preparar, t};
    use axum::http::StatusCode;
    use base64::{Engine, engine::general_purpose::STANDARD};

    /// Clave del motivo del modo solo lectura.
    const SOLO_LECTURA: &str = "panel-solo-lectura";

    /// La clave del motivo por el que no se hace, si lo hay.
    async fn motivo(state: &AppState) -> Option<&'static str> {
        let comprobadas = FUNCIONES
            .iter()
            .all(|n| state.catalogo.funcion(n).is_some_and(|f| f.comprobada()));
        if !comprobadas {
            Some(SIN_COMPROBAR)
        } else if instancia::solo_lectura(&state.db).await {
            Some(SOLO_LECTURA)
        } else {
            None
        }
    }

    pub(super) async fn estado() -> Result<Option<String>, ServerFnError> {
        let (state, _) = preparar().await?;
        Ok(motivo(&state).await.map(str::to_owned))
    }

    pub(super) async fn descargar() -> Result<CopiaRouter, ServerFnError> {
        let (state, _) = preparar().await?;
        match motivo(&state).await {
            Some(SOLO_LECTURA) => {
                return Err(fallo(StatusCode::FORBIDDEN, t!("panel-solo-lectura")));
            }
            Some(_) => {
                return Err(fallo(
                    StatusCode::CONFLICT,
                    crate::ui::i18n::traducir(SIN_COMPROBAR, None),
                ));
            }
            None => {}
        }
        // Mientras viva, el gestor no empieza ningún cambio: que no se
        // guarde en la flash un cambio a medias.
        let _reserva = state
            .cambios()
            .map(|m| m.excluir_cambios())
            .transpose()
            .map_err(|_| fallo(StatusCode::CONFLICT, t!("copia-router-en-curso")))?;
        match state.hgu.copia_de_configuracion().await {
            // Ni el contenido ni su tamaño en el registro: solo que se hizo.
            Ok(c) => {
                tracing::info!("copia de la configuración del router descargada");
                Ok(CopiaRouter {
                    nombre: c.nombre,
                    datos: STANDARD.encode(&c.datos),
                })
            }
            Err(hgu::Error::NoConfigurado) => Err(fallo(
                StatusCode::SERVICE_UNAVAILABLE,
                t!("copia-router-sin-router"),
            )),
            // Los errores del cliente nunca llevan el contenido del fichero.
            Err(e) => {
                tracing::warn!("copia de la configuración del router: {e}");
                Err(fallo(StatusCode::BAD_GATEWAY, t!("copia-router-error")))
            }
        }
    }
}
