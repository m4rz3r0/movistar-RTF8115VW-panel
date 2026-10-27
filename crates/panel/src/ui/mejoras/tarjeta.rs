//! Tarjeta de un lote: insignia, para quién es, pasos con su estado, avisos, consejos y «Revisar y aplicar».

use crate::ajustes_router::{EstadoLote, EstadoPaso, LoteVista, PasoLoteVista};
use crate::t;
use crate::ui::ajustes_router::{de_donde, partir_motivo};
use crate::ui::comun::{EstadoVisual, Glifo, Insignia};
use crate::ui::i18n::traducir;
use crate::ui::{Estado, IconoEstado};
use leptos::prelude::*;

/// Color y texto de la insignia del lote.
fn insignia(estado: &EstadoLote) -> (EstadoVisual, String) {
    match estado {
        EstadoLote::Aplicado => (EstadoVisual::Emite, t!("mejoras-estado-aplicado")),
        EstadoLote::Pendientes { pendientes, total } => (
            EstadoVisual::Atencion,
            t!(
                "mejoras-estado-pendientes",
                pendientes = *pendientes,
                total = *total
            ),
        ),
        EstadoLote::SinLeer { .. } => (EstadoVisual::Apagada, t!("mejoras-estado-sin-leer")),
    }
}

/// Icono, clase y texto del estado de un paso.
fn estado_paso(estado: &EstadoPaso) -> (Estado, &'static str, String) {
    match estado {
        EstadoPaso::Hecho => (Estado::Ok, "hecho", t!("mejoras-paso-estado-hecho")),
        EstadoPaso::Pendiente => (
            Estado::Aviso,
            "pendiente",
            t!("mejoras-paso-estado-pendiente"),
        ),
        EstadoPaso::SinLeer { .. } => (
            Estado::Neutro,
            "sin-leer",
            t!("mejoras-paso-estado-sin-leer"),
        ),
        EstadoPaso::NoAplica => (
            Estado::Neutro,
            "no-aplica",
            t!("mejoras-paso-estado-no-aplica"),
        ),
    }
}

/// Un paso de la lista: qué hace, de qué elemento (una red WiFi), si es
/// opcional, si no se pudo leer, por qué, y de dónde sale (plegado). Su nota («DNS propios», o el
/// proveedor con solo su principal), en vez del texto del estado.
fn paso(p: PasoLoteVista) -> impl IntoView {
    let (icono, clase, texto) = estado_paso(&p.estado);
    let mut args = ::i18n::FluentArgs::new();
    if let Some(id) = &p.nota_proveedor {
        args.set("proveedor", traducir(&format!("mejoras-dns-{id}"), None));
    }
    let texto = p
        .nota
        .as_deref()
        .map_or(texto, |k| traducir(k, Some(&args)));
    let motivo = match p.estado {
        EstadoPaso::SinLeer { motivo } => Some(motivo),
        _ => None,
    };
    view! {
        <li class=format!("paso paso-{clase}")>
            <IconoEstado estado=icono />
            <div class="paso-texto">
                <span class="paso-nombre">{traducir(&p.texto, None)}</span>
                {p.detalle.map(|d| view! { <span class="paso-detalle">{traducir(&d, None)}</span> })}
                {p.opcional.then(|| view! { <span class="paso-opcional">{t!("mejoras-opcional")}</span> })}
                {motivo.map(|m| view! { <span class="paso-motivo">{m}</span> })}
                {de_donde(p.fuentes, p.respaldo)}
            </div>
            <span class="paso-estado">{texto}</span>
        </li>
    }
}

/// La tarjeta del lote `lote`. «Revisar y aplicar» (el único botón
/// principal) no sale en solo lectura y está desactivado si ningún paso
/// está pendiente (los opcionales también cuentan). Si el lote no se pudo
/// leer, el motivo y «Volver a leer».
#[component]
pub(super) fn TarjetaLote(
    lote: LoteVista,
    solo_lectura: bool,
    al_revisar: Callback<String>,
    volver_a_leer: Callback<()>,
    /// Se está preparando su revisión.
    preparando: Signal<bool>,
    /// Por qué no se pudo preparar (ya aplicado, pasos sin leer…).
    error: Signal<Option<String>>,
) -> impl IntoView {
    let titulo_id = format!("lote-{}-titulo", lote.id);
    let etiquetado = titulo_id.clone();
    let nombre = traducir(&format!("mejoras-lote-{}", lote.id), None);
    let desc = traducir(&format!("mejoras-lote-{}-desc", lote.id), None);
    let (para_quien, pros) = partir_motivo(&desc, &t!("ajustes-recomendado-marca"));
    let (estado, texto) = insignia(&lote.estado);
    // «Volver a leer» si algo no se pudo leer: el lote entero (con el
    // motivo arriba) o solo algún paso (el motivo va en el paso).
    let algun_paso_sin_leer = lote
        .pasos
        .iter()
        .any(|p| matches!(p.estado, EstadoPaso::SinLeer { .. }));
    let motivo = match &lote.estado {
        EstadoLote::SinLeer { motivo } => Some(motivo.clone()),
        _ => None,
    };
    let sin_leer = (motivo.is_some() || algun_paso_sin_leer).then(|| {
        view! {
            <div class="lote-sin-leer">
                {motivo.map(|m| view! {
                    <p class="aviso estado-neutro">
                        <IconoEstado estado=Estado::Neutro />
                        <span>{m}</span>
                    </p>
                })}
                <button type="button" class="b-sec" on:click=move |_| volver_a_leer.run(())>
                    {t!("mejoras-volver-a-leer")}
                </button>
            </div>
        }
    });
    let hay_pendientes = lote.pasos.iter().any(|p| p.estado == EstadoPaso::Pendiente);
    let pasos = lote.pasos.into_iter().map(paso).collect_view();
    let avisos = lote
        .avisos
        .iter()
        .map(|a| {
            view! {
                <p class="aviso estado-aviso">
                    <IconoEstado estado=Estado::Aviso />
                    <span>{traducir(a, None)}</span>
                </p>
            }
        })
        .collect_view();
    let consejos = (!lote.consejos.is_empty()).then(|| {
        let lista = lote
            .consejos
            .iter()
            .map(|c| view! { <li>{traducir(c, None)}</li> })
            .collect_view();
        view! {
            <details class="lote-consejos">
                <summary>{t!("mejoras-mas-consejos")}</summary>
                <ul>{lista}</ul>
            </details>
        }
    });
    let id = lote.id;
    let pie = (!solo_lectura).then(|| {
        view! {
            <div class="lote-pie">
                {move || error.get().map(|m| view! {
                    <p class="aviso estado-error" role="alert">
                        <IconoEstado estado=Estado::Error />
                        <span>{m}</span>
                    </p>
                })}
                <button
                    type="button"
                    class="b-pri"
                    disabled=move || !hay_pendientes || preparando.get()
                    on:click=move |_| al_revisar.run(id.clone())
                >
                    {move || if preparando.get() { t!("ajustes-grupo-preparando") } else { t!("mejoras-revisar") }}
                </button>
            </div>
        }
    });
    view! {
        <article class="tarjeta-lote tarjeta" aria-labelledby=etiquetado>
            <div class="lote-cabecera">
                <Glifo nombre=lote.icono />
                <h2 id=titulo_id>{nombre}</h2>
                <Insignia estado texto />
            </div>
            <p class="lote-para">{para_quien}</p>
            {pros.map(|p| view! { <crate::ui::comun::ProsContras texto=p /> })}
            {sin_leer}
            <ul class="lote-pasos" aria-label=t!("mejoras-pasos")>
                {pasos}
            </ul>
            {avisos}
            {consejos}
            {pie}
        </article>
    }
}
