//! Sección «Ajustes» (`/ajustes/<tema>`): los ajustes del router agrupados
//! por temas en lenguaje claro. `/ajustes` lleva a Internet.
//!
//! Cada página tiene las pestañas de los siete temas (por debajo de 640 px,
//! un selector), el aviso de solo lectura si el panel aún no escribe, las
//! tarjetas de lo esencial arriba y el resto bajo «Más opciones». Las
//! tarjetas ([`tarjeta::Tarjeta`]) se editan en el sitio y guardan con el
//! diálogo de grupo. Un tema que no existe es «página no encontrada».

mod campo;
mod tarjeta;

use crate::ajustes_router::{TemaVista, leer_tema};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::i18n::traducir;
use crate::ui::{Estado, IconoEstado};
use ajustes::{Nivel, Tema};
use leptos::prelude::*;
use leptos_router::{
    NavigateOptions,
    components::{Outlet, Redirect},
    hooks::{use_navigate, use_params_map},
};
use tarjeta::Tarjeta;

/// Primera página de la sección (la del menú).
pub const RUTA_AJUSTES: &str = "/ajustes/internet";

/// Id del título que recibe el foco cuando la página vuelve a leer el tema
/// (tras aplicar un cambio, las tarjetas se pintan de nuevo).
#[derive(Clone, Copy)]
pub(crate) struct Foco(pub RwSignal<Option<String>>);

/// Marco de la sección: la página de la ruta hija.
#[component]
pub fn SeccionAjustes() -> impl IntoView {
    view! { <Outlet /> }
}

/// `/ajustes`: lleva a Internet (302 en el servidor). En el navegador
/// sustituye la entrada del historial.
#[component]
pub fn InicioAjustes() -> impl IntoView {
    let options = NavigateOptions {
        replace: true,
        ..NavigateOptions::default()
    };
    view! { <Redirect path=RUTA_AJUSTES options /> }
}

fn ruta(t: Tema) -> String {
    format!("/ajustes/{}", t.slug())
}

fn nombre(t: Tema) -> String {
    traducir(&format!("ajustes-tema-{}", t.slug()), None)
}

/// Pestañas de los temas y, en pantallas estrechas, el selector que navega.
fn pestanas(slug: Memo<String>) -> impl IntoView {
    let enlaces = Tema::TODOS
        .into_iter()
        .map(|t| {
            let actual = move || (slug.get() == t.slug()).then_some("page");
            view! { <a href=ruta(t) aria-current=actual>{nombre(t)}</a> }
        })
        .collect_view();
    let opciones = Tema::TODOS
        .into_iter()
        .map(|t| {
            let elegido = move || slug.get() == t.slug();
            view! { <option value=t.slug() selected=elegido>{nombre(t)}</option> }
        })
        .collect_view();
    let navegar = use_navigate();
    let cambiar = move |ev: leptos::ev::Event| {
        if let Some(t) = Tema::from_slug(&event_target_value(&ev)) {
            navegar(&ruta(t), NavigateOptions::default());
        }
    };
    view! {
        <nav class="temas" aria-label=t!("ajustes-temas")>{enlaces}</nav>
        <p class="campo temas-selector">
            <label for="ajustes-tema">{t!("ajustes-tema-elegir")}</label>
            <select id="ajustes-tema" on:change=cambiar>{opciones}</select>
        </p>
    }
}

/// Página no encontrada dentro del marco (404 en el servidor).
fn no_encontrada() -> impl IntoView {
    #[cfg(feature = "ssr")]
    if let Some(r) = use_context::<leptos_axum::ResponseOptions>() {
        r.set_status(axum::http::StatusCode::NOT_FOUND);
    }
    view! {
        <header class="pagina-cabecera">
            <h1>{t!("comun-no-encontrada")}</h1>
        </header>
        <p>
            <a href=RUTA_AJUSTES>{t!("ajustes-volver")}</a>
        </p>
    }
}

/// `/ajustes/<tema>`.
#[component]
pub fn PaginaTema() -> impl IntoView {
    let params = use_params_map();
    let slug = Memo::new(move |_| params.with(|p| p.get("tema").unwrap_or_default()));
    let existe = Memo::new(move |_| Tema::from_slug(&slug.get()).is_some());
    let datos = Resource::new(
        move || slug.get(),
        |s| async move {
            match Tema::from_slug(&s) {
                Some(_) => leer_tema(s).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let foco = Foco(RwSignal::new(None));
    provide_context(foco);
    let recargar = Callback::new(move |()| datos.refetch());
    // Con la lectura nueva ya pintada, el foco al título pendiente.
    Effect::new(move |_| {
        if datos.with(|d| d.is_some())
            && let Some(id) = foco.0.get_untracked()
        {
            request_animation_frame(move || {
                tarjeta::enfocar(&id);
                foco.0.set(None);
            });
        }
    });
    move || {
        if !existe.get() {
            return no_encontrada().into_any();
        }
        view! {
            <header class="pagina-cabecera">
                <div>
                    <h1>{t!("ajustes-titulo")}</h1>
                    <p class="pagina-sub">{t!("ajustes-desc")}</p>
                </div>
            </header>
            {pestanas(slug)}
            <Transition fallback=|| view! { <p class="estado-espera">{t!("comun-cargando")}</p> }>
                {move || Suspend::new(async move {
                    match datos.await {
                        Ok(Some(t)) => view! { <ContenidoTema t recargar /> }.into_any(),
                        Ok(None) => ().into_any(),
                        Err(e) => {
                            view! {
                                <p class="aviso estado-error" role="alert">
                                    <IconoEstado estado=Estado::Error />
                                    <span>{texto_error(&e)}</span>
                                </p>
                            }
                                .into_any()
                        }
                    }
                })}
            </Transition>
        }
        .into_any()
    }
}

/// Las tarjetas de un tema: lo esencial arriba; lo avanzado y lo oculto,
/// bajo «Más opciones».
#[component]
fn ContenidoTema(t: TemaVista, recargar: Callback<()>) -> impl IntoView {
    let solo_lectura = t.solo_lectura;
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
    let aviso = solo_lectura.then(|| {
        view! {
            <p class="aviso estado-aviso" role="status">
                <IconoEstado estado=Estado::Aviso />
                <span>
                    {t!("ajustes-solo-lectura-aviso")} " "
                    <a href=crate::ui::router::RUTA_CONEXION>{t!("ajustes-solo-lectura-enlace")}</a>
                </span>
            </p>
        }
    });
    let tema = format!("ajustes-tema-{}-desc", t.tema);
    let hay_basicos = !basicos.is_empty();
    let mas = (!otros.is_empty()).then(|| {
        view! {
            <details class="mas-opciones">
                <summary>
                    <span class="mas-opciones-titulo">{t!("ajustes-mas-opciones")}</span>
                    <span class="mas-opciones-desc">{t!("ajustes-mas-opciones-desc")}</span>
                </summary>
                <div class="ajustes-lista">{tarjetas(otros)}</div>
            </details>
        }
    });
    view! {
        <p class="tema-desc">{traducir(&tema, None)}</p>
        {aviso}
        {hay_basicos.then(|| view! { <div class="ajustes-lista">{tarjetas(basicos)}</div> })}
        {mas}
    }
}
