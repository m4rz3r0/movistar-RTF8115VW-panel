//! Tarjeta de un elemento de la rejilla: cabecera, datos, recomendación y pie.
//!
//! Toda la tarjeta abre el detalle (con el ratón o con Intro), salvo sus
//! controles: la marca redonda, el interruptor, el ojo y los botones. Un
//! elemento que no se pudo leer dice «Sin leer», con su motivo y «Volver a
//! leer»; los demás siguen funcionando.

mod cabecera;
mod datos;
mod pie;
mod recomendacion;

use super::rejilla::{Pagina, teclas_de_tarjeta};
use super::seleccion::{Marca, Seleccion, nombre};
use crate::ajustes_router::{ElementoVista, EstadoLectura};
use crate::t;
use crate::ui::{Estado, IconoEstado};
use leptos::{ev, prelude::*};

/// Id en la página de la tarjeta del elemento `id`.
pub(crate) fn id_tarjeta(id: &str) -> String {
    format!("elemento-{id}")
}

/// ¿El clic fue en un control de la tarjeta (que hace lo suyo, no abrir)?
fn en_un_control(ev: &ev::MouseEvent) -> bool {
    use leptos::wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<leptos::web_sys::Element>().ok())
        .and_then(|e| {
            e.closest("button, a, input, select, textarea, label, summary")
                .ok()
                .flatten()
        })
        .is_some()
}

/// La tarjeta del elemento `e`.
#[component]
pub(crate) fn Tarjeta(e: ElementoVista) -> impl IntoView {
    let pagina = expect_context::<Pagina>();
    let sel = expect_context::<Seleccion>();
    let id = e.id.clone();
    let nombre = nombre(&e);
    let titulo = format!("{}-titulo", id_tarjeta(&id));
    let titulo_cabecera = titulo.clone();
    let abrir = {
        let id = id.clone();
        Callback::new(move |()| pagina.abrir.run(id.clone()))
    };
    let marcada = {
        let id = id.clone();
        move || sel.marcada(&id)
    };
    let abierta = {
        let id = id.clone();
        move || pagina.abierto.with(|a| a.as_deref() == Some(id.as_str()))
    };
    let cuerpo = match &e.lectura {
        EstadoLectura::Leido => view! {
            {datos::datos(&e)}
            {recomendacion::recomendacion(&e, pagina.solo_lectura)}
        }
        .into_any(),
        EstadoLectura::SinLeer(motivo) => view! {
            <div class="tarjeta-sin-leer">
                <p class="aviso estado-error">
                    <IconoEstado estado=Estado::Error />
                    <span>{motivo.clone()}</span>
                </p>
                <button type="button" class="b-sec" on:click=move |_| pagina.volver_a_leer.run(())>
                    {t!("ajustes-volver-a-leer")}
                </button>
            </div>
        }
        .into_any(),
    };
    let teclas = {
        let id = id.clone();
        move |ev: ev::KeyboardEvent| teclas_de_tarjeta(&ev, &id, pagina, sel)
    };
    view! {
        <article
            id=id_tarjeta(&id)
            class="tarjeta-el"
            class:marcada=marcada
            class:abierta=abierta
            tabindex="0"
            aria-labelledby=titulo
            on:click=move |ev| {
                if !en_un_control(&ev) {
                    abrir.run(());
                }
            }
            on:keydown=teclas
        >
            <Marca id=id.clone() nombre=nombre.clone() />
            {cabecera::cabecera(&e, &nombre, titulo_cabecera)}
            {cuerpo}
            {pie::pie(&e, &nombre, abrir)}
        </article>
    }
}
