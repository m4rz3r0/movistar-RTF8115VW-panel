//! Interfaz: páginas, marco común y componentes.
//!
//! Aquí viven también las piezas mínimas que comparten tablas, indicadores y
//! páginas: el [`Estado`] (ok, aviso, error, neutro) con su icono e
//! [`Insignia`]. Los colores de estado se reservan para estados y van siempre
//! con icono y texto, nunca solos.

pub mod charts;
pub mod layout;
pub mod login;
pub mod muestra;
pub mod table;

use leptos::prelude::*;

/// Estado con significado (bien, aviso, error) o neutro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    Ok,
    Aviso,
    Error,
    Neutro,
}

impl Estado {
    /// Clase CSS del estado (color del icono y del borde).
    pub fn clase(self) -> &'static str {
        match self {
            Estado::Ok => "estado-ok",
            Estado::Aviso => "estado-aviso",
            Estado::Error => "estado-error",
            Estado::Neutro => "estado-neutro",
        }
    }
}

/// Icono de 16 px del estado (círculo con marca, triángulo, círculo con
/// aspa, círculo con «i»). Decorativo: el texto que lo acompaña da el estado.
#[component]
pub fn IconoEstado(estado: Estado) -> impl IntoView {
    let trazos = match estado {
        Estado::Ok => view! {
            <circle cx="8" cy="8" r="6.25" />
            <path d="M5.2 8.3l1.9 1.9 3.8-4" />
        }
        .into_any(),
        Estado::Aviso => view! {
            <path d="M8 1.9l6.4 11.3H1.6z" />
            <path d="M8 6.4v3.1M8 11.6v.1" />
        }
        .into_any(),
        Estado::Error => view! {
            <circle cx="8" cy="8" r="6.25" />
            <path d="M5.8 5.8l4.4 4.4M10.2 5.8l-4.4 4.4" />
        }
        .into_any(),
        Estado::Neutro => view! {
            <circle cx="8" cy="8" r="6.25" />
            <path d="M8 7.3v3.8M8 4.9v.1" />
        }
        .into_any(),
    };
    view! {
        <svg
            class=format!("icono icono-estado {}", estado.clase())
            viewBox="0 0 16 16"
            width="16"
            height="16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"
        >
            {trazos}
        </svg>
    }
}

/// Insignia de estado: icono y texto.
#[component]
pub fn Insignia(estado: Estado, #[prop(into)] texto: String) -> impl IntoView {
    view! {
        <span class=format!("insignia {}", estado.clase())>
            <IconoEstado estado />
            {texto}
        </span>
    }
}
