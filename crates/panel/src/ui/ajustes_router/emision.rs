//! Si cada red WiFi emite, en su tarjeta: la insignia de la cabecera de una
//! red y la nota de la radio encendida que no emite ninguna.

use crate::ajustes_router::{AjusteVista, EmisionRed};
use crate::t;
use crate::ui::{Estado, IconoEstado, Insignia};
use leptos::prelude::*;

/// «Encendida», «Apagada» o «No emite: la radio está apagada», en la
/// cabecera de la tarjeta de una red.
pub(super) fn insignia(a: &AjusteVista) -> Option<impl IntoView + use<>> {
    let (estado, texto) = match a.red_wifi? {
        EmisionRed::Encendida => (Estado::Ok, t!("ajustes-wifi-red-encendida")),
        EmisionRed::Apagada => (Estado::Neutro, t!("ajustes-wifi-red-apagada")),
        EmisionRed::SinRadio => (Estado::Aviso, t!("ajustes-wifi-red-sin-radio")),
    };
    Some(view! { <span class="emision-red"><Insignia estado texto /></span> })
}

/// La nota de una radio encendida sin ninguna red encendida.
pub(super) fn nota_radio(a: &AjusteVista) -> Option<impl IntoView + use<>> {
    a.radio_sin_redes.then(|| {
        view! {
            <p class="aviso estado-aviso nota-radio-sin-redes">
                <IconoEstado estado=Estado::Aviso />
                <span>{t!("ajustes-wifi-radio-sin-redes")}</span>
            </p>
        }
    })
}
