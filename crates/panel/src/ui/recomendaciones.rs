//! Sección «Recomendaciones» (`/recomendaciones/*`): volverá en una versión futura con
//! sus tarjetas, que piden en contexto solo lo que necesitan. Mientras tanto
//! no sale en el menú, y sus rutas antiguas (`/recomendaciones` y
//! `/recomendaciones/preferencias`) llevan al Estado para no romper enlaces
//! guardados.

use leptos::prelude::*;
use leptos_router::{NavigateOptions, components::Redirect};

/// Rutas antiguas de la sección: llevan al Estado (302 en el servidor). En
/// el navegador sustituye la entrada del historial: «Atrás» no vuelve a la
/// ruta antigua (que redirigiría otra vez), sino a la página de antes.
#[component]
pub fn RecomendacionesAlEstado() -> impl IntoView {
    let options = NavigateOptions {
        replace: true,
        ..NavigateOptions::default()
    };
    view! { <Redirect path="/" options /> }
}
