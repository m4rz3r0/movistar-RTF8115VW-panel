//! Modo de visualización de cada usuario del panel: [`ModoVista`] y sus
//! server functions ([`leer_vista`], [`guardar_vista`]).
//!
//! Se guarda en la tabla de usuarios (`modo_vista`; `NULL` es
//! [`ModoVista::Mixto`]). El servidor lo pone en `<html data-vista>` y la
//! hidratación lo lee de ahí (como la zona horaria en `data-zona`), así que
//! el servidor y el navegador pintan lo mismo. La interfaz (contexto,
//! selectores) está en [`crate::ui::vista`].
//!
//! Sin features: compila igual para el servidor y para el navegador.

use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};

/// Cómo se presentan los datos con gráficas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModoVista {
    /// Solo texto: ningún gráfico; los indicadores como pares nombre/valor y
    /// los históricos como tablas de último, mínimo, media y máximo.
    Texto,
    /// Las gráficas primero y a todo el ancho; los indicadores, compactos y
    /// sin minigráfica.
    Grafica,
    /// Indicadores con minigráfica y gráficas del histórico.
    #[default]
    Mixto,
}

impl ModoVista {
    /// Los tres modos, en el orden de los selectores.
    pub const TODOS: [ModoVista; 3] = [ModoVista::Texto, ModoVista::Grafica, ModoVista::Mixto];

    /// Código del modo (el de la base de datos, de `data-vista` y de la
    /// server function).
    pub fn codigo(self) -> &'static str {
        match self {
            ModoVista::Texto => "texto",
            ModoVista::Grafica => "grafica",
            ModoVista::Mixto => "mixto",
        }
    }

    /// Modo de un código; `None` si no es ninguno.
    pub fn desde_codigo(codigo: &str) -> Option<Self> {
        ModoVista::TODOS.into_iter().find(|m| m.codigo() == codigo)
    }
}

/// Modo de visualización del usuario de la sesión.
#[server(endpoint = "leer_vista", input = Json)]
pub async fn leer_vista() -> Result<ModoVista, ServerFnError> {
    Ok(crate::auth::require_user().await?.modo_vista)
}

/// Guarda el modo de visualización del usuario de la sesión (no pide volver
/// a escribir la contraseña: solo cambia cómo ve el panel).
#[server(endpoint = "guardar_vista", input = Json)]
pub async fn guardar_vista(modo: ModoVista) -> Result<(), ServerFnError> {
    crate::auth::guardar_modo_vista(modo).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codigos_de_ida_y_vuelta() {
        for m in ModoVista::TODOS {
            assert_eq!(ModoVista::desde_codigo(m.codigo()), Some(m));
            // Las server functions lo mandan con el mismo código.
            #[cfg(feature = "ssr")]
            assert_eq!(
                serde_json::to_string(&m).unwrap(),
                format!("\"{}\"", m.codigo())
            );
        }
        assert_eq!(ModoVista::desde_codigo("otro"), None);
        assert_eq!(ModoVista::default(), ModoVista::Mixto);
    }
}
