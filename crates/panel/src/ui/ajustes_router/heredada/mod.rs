//! Los temas que aún no van en rejilla: un ajuste, una tarjeta
//! ([`tarjeta::Tarjeta`], que se edita en el sitio), lo esencial arriba y el
//! resto bajo «Más opciones». Los pasan a la rejilla las Tareas 4 y 5 del
//! rediseño; entonces se retira.

mod tarjeta;

use super::{
    DISPOSITIVOS, NOTA_OCULTO, PUERTOS, RESERVAS, dispositivos, introduccion, puertos, reservas,
};
use crate::ajustes_router::{PaginaTema, TemaVista};
use crate::t;
use ajustes::Nivel;
use leptos::prelude::*;
use tarjeta::Tarjeta;

/// Adaptador temporal: la página de elementos como las tarjetas sueltas de
/// siempre (un ajuste, una tarjeta), hasta que el tema pase a la rejilla.
pub(super) fn tarjetas_sueltas(p: PaginaTema) -> TemaVista {
    TemaVista {
        tema: p.tema,
        solo_lectura: p.solo_lectura,
        foto_de_partida: p.foto_de_partida,
        ajustes: p.elementos.into_iter().flat_map(|e| e.ajustes).collect(),
    }
}

/// Las tarjetas de un tema: lo esencial arriba; lo avanzado y lo oculto,
/// bajo «Más opciones».
#[component]
pub(super) fn ContenidoTema(t: TemaVista, recargar: Callback<()>) -> impl IntoView {
    let solo_lectura = t.solo_lectura;
    let tema = t.tema.clone();
    // Los dispositivos y las IP fijas leen la red de casa una vez para los dos.
    let de_red = |c: &Option<String>| matches!(c.as_deref(), Some(DISPOSITIVOS | RESERVAS));
    if t.ajustes.iter().any(|a| de_red(&a.componente)) {
        let existentes = t
            .ajustes
            .iter()
            .find(|a| a.componente.as_deref() == Some(RESERVAS))
            .map(|a| reservas::existentes(&a.instancias))
            .unwrap_or_default();
        provide_context(dispositivos::DatosRed::nuevo(existentes));
    }
    if t.ajustes
        .iter()
        .any(|a| a.componente.as_deref() == Some(PUERTOS))
    {
        provide_context(puertos::DatosPuertos::nuevo());
    }
    let foto = t.foto_de_partida;
    let (basicos, otros): (Vec<_>, Vec<_>) = t
        .ajustes
        .into_iter()
        .partition(|a| a.nivel == Nivel::Basico);
    let tarjetas = move |lista: Vec<_>| {
        lista
            .into_iter()
            .map(|a| view! { <Tarjeta a solo_lectura recargar /> })
            .collect_view()
    };
    // Qué significa «Oculto», una sola vez, delante de las tarjetas ocultas.
    let nota_oculto = |hay: bool| {
        hay.then(|| {
            view! {
                <p class="nota-oculto" id=NOTA_OCULTO>
                    <span class="chip chip-nivel">{t!("ajustes-nivel-oculto")}</span>
                    " "
                    {t!("ajustes-nivel-oculto-desc")}
                </p>
            }
        })
    };
    let hay_oculto = otros.iter().any(|a| a.nivel == Nivel::Oculto);
    // Sin nada esencial (Sistema), plegarlo todo dejaría la página vacía:
    // sus tarjetas se ven directamente, cada una con su etiqueta de nivel.
    let cuerpo = if basicos.is_empty() {
        view! {
            {nota_oculto(hay_oculto)}
            <div class="ajustes-lista">{tarjetas(otros)}</div>
        }
        .into_any()
    } else {
        let mas = (!otros.is_empty()).then(|| {
            view! {
                <details class="mas-opciones">
                    <summary>
                        <span class="mas-opciones-titulo">{t!("ajustes-mas-opciones")}</span>
                        <span class="mas-opciones-desc">{t!("ajustes-mas-opciones-desc")}</span>
                    </summary>
                    {nota_oculto(hay_oculto)}
                    <div class="ajustes-lista">{tarjetas(otros)}</div>
                </details>
            }
        });
        view! {
            <div class="ajustes-lista">{tarjetas(basicos)}</div>
            {mas}
        }
        .into_any()
    };
    view! {
        {introduccion(&tema, foto, solo_lectura)}
        {cuerpo}
    }
}
