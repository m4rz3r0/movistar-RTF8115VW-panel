//! Insignia de estado: un punto del color del estado y su texto.

use super::EstadoVisual;
use leptos::prelude::*;

/// Insignia de estado («Encendida», «Apagada», «Aplicando…»): el punto y el
/// fondo llevan el color del estado; el texto lo dice.
#[component]
pub fn Insignia(estado: EstadoVisual, #[prop(into)] texto: String) -> impl IntoView {
    view! { <span class=format!("ins ins-{}", estado.sufijo())>{texto}</span> }
}
