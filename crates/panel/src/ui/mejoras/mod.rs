//! Página Mejoras (`/mejoras`): los lotes de ajustes recomendados, cada uno en su tarjeta.
//!
//! Una sola lectura ([`leer_mejoras`]) da los lotes con el estado de cada
//! paso. Mientras lee, los esqueletos de las tarjetas; «Volver a leer» pide
//! una lectura que no sale de la caché del servidor. El contador de la barra
//! lateral toma el total de cada lectura ([`CuentaMejoras`]).

mod tarjeta;

use crate::ajustes_router::leer_mejoras;
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::layout::CuentaMejoras;
use crate::ui::{Estado, IconoEstado};
use leptos::prelude::*;
use tarjeta::TarjetaLote;

/// Mientras se lee: la forma de las dos tarjetas, sin datos.
fn esqueleto() -> impl IntoView {
    let tarjeta = || {
        view! {
            <div class="tarjeta lote-esqueleto">
                <span class="esqueleto-linea esqueleto-titulo"></span>
                <span class="esqueleto-linea"></span>
                <span class="esqueleto-linea esqueleto-corta"></span>
            </div>
        }
    };
    view! {
        <p class="solo-lector" role="status">{t!("comun-cargando")}</p>
        <div class="rejilla-lotes" aria-hidden="true">
            {tarjeta()} {tarjeta()}
        </div>
    }
}

/// `/mejoras`.
#[component]
pub fn PaginaMejoras() -> impl IntoView {
    // «Volver a leer»: la próxima lectura no sale de la caché del servidor.
    let fresca = StoredValue::new(false);
    let cuenta = use_context::<CuentaMejoras>();
    let datos = Resource::new(
        || (),
        move |()| {
            let fresca = fresca.try_update_value(std::mem::take).unwrap_or(false);
            leer_mejoras(fresca)
        },
    );
    // El contador de la barra toma el total de esta lectura: no hace la suya.
    Effect::new(move |_| {
        if let (Some(Ok(m)), Some(CuentaMejoras(c))) = (datos.get(), cuenta) {
            c.set(Some(m.pendientes));
        }
    });
    let volver_a_leer = Callback::new(move |()| {
        fresca.set_value(true);
        datos.refetch();
    });
    // La revisión del lote llega con su diálogo (siguiente tarea).
    let al_revisar = Callback::new(|_lote: String| {});
    view! {
        <header class="pagina-cabecera">
            <div>
                <h1>{t!("mejoras-titulo")}</h1>
                <p class="pagina-sub">{t!("mejoras-desc")}</p>
            </div>
        </header>
        <Transition fallback=esqueleto>
            {move || Suspend::new(async move {
                match datos.await {
                    Ok(m) => {
                        let solo_lectura = m.solo_lectura;
                        let tarjetas = m
                            .lotes
                            .into_iter()
                            .map(|lote| {
                                view! {
                                    <TarjetaLote lote solo_lectura al_revisar volver_a_leer />
                                }
                            })
                            .collect_view();
                        view! { <div class="rejilla-lotes">{tarjetas}</div> }.into_any()
                    }
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
}
