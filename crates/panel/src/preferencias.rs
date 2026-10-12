//! Server functions de «Recomendaciones → Tus preferencias»
//! (`/recomendaciones/preferencias`): los parámetros de la instancia que usan
//! prestablecidos y recomendaciones (DNS preferidos, dispositivos de
//! confianza, horario de mantenimiento y servidor propio).
//!
//! Se guardan con la misma función que «Ajustes del panel»
//! ([`crate::panel_ajustes::guardar_instancia`]), mandando solo estos campos.

use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};

/// Equipo que conoce el router, para elegir los de confianza.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipoLan {
    pub mac: String,
    pub nombre: String,
}

/// Lo que muestra «Tus preferencias».
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferencias {
    pub dns_preferidos: Vec<String>,
    pub dispositivos_confianza: Vec<String>,
    pub mantenimiento: Option<String>,
    pub servidor_propio: Option<String>,
    /// Equipos de la última lectura del router.
    pub equipos: Vec<EquipoLan>,
}

/// Las preferencias guardadas y los equipos de la última lectura del sondeo
/// (sin lectura aún, ninguno: nunca se pregunta al router desde la página).
#[server(endpoint = "leer_preferencias", input = Json)]
pub async fn leer_preferencias() -> Result<Preferencias, ServerFnError> {
    servidor::leer().await
}

#[cfg(feature = "ssr")]
mod servidor {
    use super::*;
    use crate::instancia::{self, claves};
    use crate::panel_ajustes::{
        normalizar_mac,
        servidor::{lista, preparar, texto},
    };

    pub(super) async fn leer() -> Result<Preferencias, ServerFnError> {
        let (state, _) = preparar().await?;
        let db = &state.db;
        let mut equipos: Vec<EquipoLan> = Vec::new();
        let foto = state.poller.latest();
        for h in foto.iter().flat_map(|s| &s.hosts) {
            if let Some(mac) = normalizar_mac(&h.mac)
                && !equipos.iter().any(|e| e.mac == mac)
            {
                equipos.push(EquipoLan {
                    mac,
                    nombre: h.hostname.clone(),
                });
            }
        }
        Ok(Preferencias {
            dns_preferidos: lista(instancia::leer(db, claves::DNS_PREFERIDOS).await),
            dispositivos_confianza: lista(
                instancia::leer(db, claves::DISPOSITIVOS_CONFIANZA).await,
            ),
            mantenimiento: texto(instancia::leer(db, claves::MANTENIMIENTO).await),
            servidor_propio: texto(instancia::leer(db, claves::SERVIDOR_PROPIO).await),
            equipos,
        })
    }
}
