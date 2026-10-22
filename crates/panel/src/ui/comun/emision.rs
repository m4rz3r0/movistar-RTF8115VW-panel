//! Icono de emisión: tres arcos y un punto con el color del estado.

use super::EstadoVisual;
use leptos::prelude::*;

/// Icono de emisión de una red o radio: verde si emite, gris si está
/// apagada, ámbar si requiere atención (una radio sin redes). Decorativo:
/// la insignia que lo acompaña dice el estado.
#[component]
pub fn IconoEmision(estado: EstadoVisual) -> impl IntoView {
    view! {
        <svg
            class=format!("emision emision-{}", estado.sufijo())
            viewBox="0 0 34 34"
            width="34"
            height="34"
            aria-hidden="true"
            focusable="false"
        >
            <path d="M3 12a20 20 0 0 1 28 0" />
            <path d="M7.5 16.5a13.5 13.5 0 0 1 19 0" />
            <path d="M11.5 21a7.5 7.5 0 0 1 11 0" />
            <circle cx="17" cy="26" r="2.5" />
        </svg>
    }
}
