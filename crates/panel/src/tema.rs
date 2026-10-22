//! Tema de la interfaz de cada usuario del panel: [`TemaUi`] y sus server
//! functions ([`leer_tema_ui`], [`guardar_tema_ui`]).
//!
//! Se guarda en la tabla de usuarios (`tema`; `NULL` es
//! [`TemaUi::Automatico`]). El servidor lo pone en `<html data-tema>`, así
//! que la página llega pintada con su tema, sin destello; en «automático»
//! deciden el CSS y la preferencia del sistema (`prefers-color-scheme`), sin
//! JavaScript. La interfaz (contexto, selector) está en [`crate::ui::tema`].
//!
//! Sin features: compila igual para el servidor y para el navegador.

use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};

/// Tema de color de la interfaz.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TemaUi {
    /// Sigue al sistema: claro u oscuro según la preferencia del navegador.
    #[default]
    Automatico,
    /// Oscuro, el de referencia.
    Oscuro,
    /// Claro.
    Claro,
}

impl TemaUi {
    /// Los tres temas, en el orden del selector.
    pub const TODOS: [TemaUi; 3] = [TemaUi::Automatico, TemaUi::Oscuro, TemaUi::Claro];

    /// Código del tema (el de la base de datos, de `data-tema` y de la
    /// server function).
    pub fn codigo(self) -> &'static str {
        match self {
            TemaUi::Automatico => "automatico",
            TemaUi::Oscuro => "oscuro",
            TemaUi::Claro => "claro",
        }
    }

    /// Tema de un código; `None` si no es ninguno.
    pub fn desde_codigo(codigo: &str) -> Option<Self> {
        TemaUi::TODOS.into_iter().find(|t| t.codigo() == codigo)
    }
}

/// Tema del usuario de la sesión.
#[server(endpoint = "leer_tema_ui", input = Json)]
pub async fn leer_tema_ui() -> Result<TemaUi, ServerFnError> {
    Ok(crate::auth::require_user().await?.tema)
}

/// Guarda el tema del usuario de la sesión (no pide volver a escribir la
/// contraseña: solo cambia cómo ve el panel).
#[server(endpoint = "guardar_tema_ui", input = Json)]
pub async fn guardar_tema_ui(tema: TemaUi) -> Result<(), ServerFnError> {
    crate::auth::guardar_tema(tema).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codigos_de_ida_y_vuelta() {
        for t in TemaUi::TODOS {
            assert_eq!(TemaUi::desde_codigo(t.codigo()), Some(t));
            #[cfg(feature = "ssr")]
            assert_eq!(
                serde_json::to_string(&t).unwrap(),
                format!("\"{}\"", t.codigo())
            );
        }
        assert_eq!(TemaUi::desde_codigo("rosa"), None);
        assert_eq!(TemaUi::default(), TemaUi::Automatico);
    }
}
