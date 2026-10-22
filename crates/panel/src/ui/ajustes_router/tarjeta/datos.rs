//! Datos de la tarjeta: hasta seis, en una cuadrícula de tres columnas.

use super::super::campo::valor_resumido;
use crate::ajustes_router::{CampoVista, DatoVista, ElementoVista};
use crate::t;
use crate::ui::i18n::traducir;
use ajustes::ValorLegible;
use leptos::prelude::*;

/// El campo de `e` del que sale el dato `d`, si es un campo (no un
/// derivado).
fn campo<'e>(e: &'e ElementoVista, d: &DatoVista) -> Option<&'e CampoVista> {
    e.ajustes
        .iter()
        .flat_map(|a| a.campos.iter())
        .find(|c| c.clave == d.clave)
}

/// El valor de un dato: el de su campo como en el resumen de siempre (un
/// secreto, censurado con el ojo) o el de un derivado tal cual.
fn valor(e: &ElementoVista, d: &DatoVista) -> AnyView {
    if let Some(c) = campo(e, d) {
        return valor_resumido(c);
    }
    match &d.valor {
        ValorLegible::Texto(t) | ValorLegible::Opcion(t) => {
            view! { <span>{t.clone()}</span> }.into_any()
        }
        ValorLegible::Desconocido(x) => {
            view! { <span>{t!("ajustes-valor-desconocido", valor = x.clone())}</span> }.into_any()
        }
        ValorLegible::Booleano(b) => {
            view! { <span>{if *b { t!("comun-si") } else { t!("comun-no") }}</span> }.into_any()
        }
        ValorLegible::Secreto => ().into_any(),
    }
}

/// Los datos de la tarjeta de `e`; el que no tiene el valor recomendado, en
/// ámbar.
pub(super) fn datos(e: &ElementoVista) -> Option<impl IntoView + use<>> {
    (!e.datos.is_empty()).then(|| {
        let filas = e
            .datos
            .iter()
            .map(|d| {
                view! {
                    <div>
                        <dt>{traducir(&d.clave, None)}</dt>
                        <dd class:atencion=d.atencion>{valor(e, d)}</dd>
                    </div>
                }
            })
            .collect_view();
        view! { <dl class="tarjeta-datos">{filas}</dl> }
    })
}
