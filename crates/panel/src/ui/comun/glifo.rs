//! Glifo de una tarjeta que no es WiFi (reloj, escudo, teléfono…), en color neutro.

use leptos::prelude::*;

/// Trazos (24×24) de cada glifo de `ajustes::ICONOS`; uno desconocido, un
/// círculo.
fn trazos(nombre: &str) -> &'static [&'static str] {
    match nombre {
        // Un globo: el mundo de internet.
        "globo" => &[
            "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18z",
            "M3 12h18",
            "M12 3c3 3.2 3 14.8 0 18c-3-3.2-3-14.8 0-18z",
        ],
        "escudo" => &["M12 3l7 3v5c0 4.5-3 8.2-7 10c-4-1.8-7-5.5-7-10V6z"],
        "llave" => &[
            "M8 8m-4 0a4 4 0 1 0 8 0a4 4 0 1 0-8 0",
            "M11 11l8 8",
            "M16 16l2-2",
            "M18 18l2-2",
        ],
        "personas" => &[
            "M9 8m-3 0a3 3 0 1 0 6 0a3 3 0 1 0-6 0",
            "M3 20c0-3.3 2.7-6 6-6s6 2.7 6 6",
            "M16 5.5a3 3 0 0 1 0 5",
            "M18 14.5c1.8.9 3 2.9 3 5.5",
        ],
        "telefono" => &[
            "M5 4h4l2 5l-2.5 1.5a11 11 0 0 0 5 5L15 13l5 2v4a2 2 0 0 1-2 2A16 16 0 0 1 3 6a2 2 0 0 1 2-2z",
        ],
        "fax" => &[
            "M7 9V3h10v6",
            "M5 9h14a2 2 0 0 1 2 2v6h-4",
            "M7 17H3v-6a2 2 0 0 1 2-2",
            "M7 14h10v7H7z",
        ],
        "reloj" => &["M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18z", "M12 7v5l3 2"],
        "lupa" => &[
            "M10.5 10.5m-6.5 0a6.5 6.5 0 1 0 13 0a6.5 6.5 0 1 0-13 0",
            "M15.5 15.5L21 21",
        ],
        // Una puerta entreabierta con una flecha que entra: puertos abiertos.
        "puerta" => &["M14 4h5v16h-5", "M3 12h10", "M9 8l4 4l-4 4"],
        "casa" => &["M4 11l8-7l8 7", "M6 9.5V20h12V9.5", "M10 20v-5h4v5"],
        // Un ojo: lo que el router cuenta a otros (la privacidad).
        "ojo" => &[
            "M2 12c2.5-4.5 6-7 10-7s7.5 2.5 10 7c-2.5 4.5-6 7-10 7s-7.5-2.5-10-7z",
            "M12 9a3 3 0 1 0 0 6a3 3 0 1 0 0-6z",
        ],
        _ => &["M12 4a8 8 0 1 0 0 16a8 8 0 1 0 0-16z"],
    }
}

/// El glifo `nombre`. Decorativo y neutro: el estado lo dice la insignia.
#[component]
pub fn Glifo(nombre: String) -> impl IntoView {
    let trazos = trazos(&nombre)
        .iter()
        .map(|d| view! { <path d=*d /> })
        .collect_view();
    view! {
        <svg
            class=format!("glifo glifo-{nombre}")
            viewBox="0 0 24 24"
            width="34"
            height="34"
            aria-hidden="true"
            focusable="false"
        >
            {trazos}
        </svg>
    }
}

#[cfg(test)]
mod tests {
    use super::trazos;

    /// Cada glifo que admite el catálogo tiene su dibujo (no el de reserva).
    #[test]
    fn cada_icono_del_catalogo_tiene_su_glifo() {
        let reserva = trazos("no-existe");
        for i in ajustes::ICONOS {
            assert_ne!(trazos(i), reserva, "{i}");
        }
    }
}
