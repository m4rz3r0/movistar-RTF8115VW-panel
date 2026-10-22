//! Pie de la tarjeta: información viva y «Abrir».

use crate::ajustes_router::ElementoVista;
use crate::t;
use leptos::prelude::*;
use leptos_router::components::A;

/// La Biblioteca de la WiFi: ahí se ven todos los puntos de acceso.
const BIBLIOTECA_WIFI: &str = "/explorador/wifi";

/// Iniciales de un nombre de dispositivo («Móvil de Ana» → «MA»).
fn iniciales(nombre: &str) -> String {
    nombre
        .split_whitespace()
        .filter(|p| p.chars().next().is_some_and(char::is_alphanumeric))
        .filter(|p| p.chars().count() > 2 || p.chars().all(char::is_uppercase))
        .filter_map(|p| p.chars().next())
        .flat_map(char::to_uppercase)
        .take(2)
        .collect()
}

/// El pie de la tarjeta de `e`: los dispositivos conectados (con sus
/// iniciales), la nota de una radio (con la Biblioteca, donde se ven todas
/// sus redes) y «Abrir».
pub(super) fn pie(e: &ElementoVista, nombre: &str, abrir: Callback<()>) -> impl IntoView + use<> {
    let dispositivos = (!e.pie.dispositivos.is_empty()).then(|| {
        let avatares = e
            .pie
            .dispositivos
            .iter()
            .take(3)
            .map(|d| view! { <span class="tarjeta-av" aria-hidden="true">{iniciales(d)}</span> })
            .collect_view();
        let nombres = e.pie.dispositivos.join(", ");
        view! {
            <div class="tarjeta-disp">
                <span class="tarjeta-avs">{avatares}</span>
                <span>{nombres}</span>
            </div>
        }
    });
    let nota = e.pie.nota.clone().map(|n| {
        view! {
            <span class="tarjeta-nota">{n}</span>
            <A href=BIBLIOTECA_WIFI attr:class="b-txt">{t!("ajustes-wifi-radio-ocultas-enlace")}</A>
        }
    });
    view! {
        <div class="tarjeta-pie">
            {dispositivos}
            {nota}
            <span class="tarjeta-hueco"></span>
            <button
                type="button"
                class="b-sec"
                aria-label=t!("ajustes-abrir-nombre", nombre = nombre.to_owned())
                on:click=move |_| abrir.run(())
            >
                {t!("ajustes-abrir")}
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::iniciales;

    #[test]
    fn iniciales_de_un_nombre() {
        assert_eq!(iniciales("Móvil de Ana"), "MA");
        assert_eq!(iniciales("portátil"), "P");
        assert_eq!(iniciales("TV"), "T");
        assert_eq!(iniciales(""), "");
    }
}
