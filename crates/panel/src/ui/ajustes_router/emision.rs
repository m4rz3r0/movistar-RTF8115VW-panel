//! Si cada red WiFi emite, en su tarjeta: la insignia de la cabecera de una
//! red y la nota de la radio encendida que no emite ninguna o que emite
//! redes que el panel no muestra.

use crate::ajustes_router::{AjusteVista, EmisionRed, NotaRadio};
use crate::t;
use crate::ui::comun::{EstadoVisual, Insignia};
use crate::ui::{Estado, IconoEstado};
use leptos::prelude::*;
use leptos_router::components::A;

/// La Biblioteca de la WiFi: ahí se ven todos los puntos de acceso.
const BIBLIOTECA_WIFI: &str = "/explorador/wifi";

/// «Encendida», «Apagada», «Encendida, pero no emite» o «No emite: la radio
/// está apagada», en la cabecera de la tarjeta de una red.
pub(super) fn insignia(a: &AjusteVista) -> Option<impl IntoView + use<>> {
    let (estado, texto) = match a.red_wifi? {
        EmisionRed::Encendida => (EstadoVisual::Emite, t!("ajustes-wifi-red-encendida")),
        EmisionRed::Apagada => (EstadoVisual::Apagada, t!("ajustes-wifi-red-apagada")),
        EmisionRed::SinRadio => (EstadoVisual::Atencion, t!("ajustes-wifi-red-sin-radio")),
        EmisionRed::NoEmite => (EstadoVisual::Atencion, t!("ajustes-wifi-red-no-emite")),
    };
    Some(view! { <span class="emision-red"><Insignia estado texto /></span> })
}

/// La nota de una radio encendida: no emite ninguna red, o emite algunas
/// que el panel no muestra (con el enlace a la Biblioteca).
pub(super) fn nota_radio(a: &AjusteVista) -> Option<impl IntoView + use<>> {
    let texto = match a.nota_radio? {
        NotaRadio::SinRedes => view! { {t!("ajustes-wifi-radio-sin-redes")} }.into_any(),
        NotaRadio::Ocultas(n) => view! {
            {t!("ajustes-wifi-radio-ocultas", n = n)}
            " "
            <A href=BIBLIOTECA_WIFI>{t!("ajustes-wifi-radio-ocultas-enlace")}</A>
        }
        .into_any(),
    };
    Some(view! {
        <p class="aviso estado-aviso nota-radio">
            <IconoEstado estado=Estado::Aviso />
            <span>{texto}</span>
        </p>
    })
}
