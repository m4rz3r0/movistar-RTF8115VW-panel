//! Página Mejoras (`/mejoras`): los lotes de ajustes recomendados, cada uno en su tarjeta.
//!
//! Una sola lectura ([`leer_mejoras`]) da los lotes con el estado de cada
//! paso. Mientras lee, los esqueletos de las tarjetas; «Volver a leer» pide
//! una lectura que no sale de la caché del servidor. El contador de la barra
//! lateral toma el total de cada lectura ([`CuentaMejoras`]).
//!
//! «Revisar y aplicar» prepara el lote ([`preparar_lote`]) y lo abre en el
//! diálogo de revisión de Ajustes; el cambio se sigue en el aviso de la
//! página, con su «Deshacer». Al terminar, la página vuelve a leer.

mod revision;
mod tarjeta;

use crate::ajustes_router::{OpcionesLote, ProveedorVista, leer_mejoras, preparar_lote};
use crate::t;
use crate::ui::ajustes_router::{AvisoCambio, Borradores, Seguimiento, Seleccion};
use crate::ui::explorador::texto_error;
use crate::ui::layout::{CuentaMejoras, RecargaMarco};
use crate::ui::{Estado, IconoEstado};
use leptos::{prelude::*, task::spawn_local};
use revision::SelectorDns;
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
    let marco = use_context::<RecargaMarco>();
    // Los proveedores de DNS del selector, de la última lectura.
    let proveedores = StoredValue::new(Vec::<ProveedorVista>::new());
    let datos = Resource::new(
        || (),
        move |()| {
            let fresca = fresca.try_update_value(std::mem::take).unwrap_or(false);
            leer_mejoras(fresca)
        },
    );
    // El contador de la barra toma el total de esta lectura: no hace la suya.
    Effect::new(move |_| {
        if let Some(Ok(m)) = datos.get() {
            proveedores.set_value(m.proveedores);
            if let Some(CuentaMejoras(c)) = cuenta {
                c.set(Some(m.pendientes));
            }
        }
    });
    let volver_a_leer = Callback::new(move |()| {
        fresca.set_value(true);
        datos.refetch();
    });
    // Al terminar un cambio (aplicado o deshecho), lo leído ya no vale: la
    // página y el marco vuelven a leer.
    let recargar = Callback::new(move |()| {
        fresca.set_value(true);
        datos.refetch();
        if let Some(RecargaMarco(r)) = marco {
            r.update(|n| *n += 1);
        }
    });
    let seguimiento = Seguimiento::nuevo(recargar, Borradores::nuevos(), Seleccion::nueva());
    provide_context(seguimiento);
    let selector = SelectorDns::nuevo();
    // El lote que se está preparando y el último error al prepararlo (ya
    // aplicado, pasos sin leer, otro cambio en curso): va en su tarjeta, sin
    // abrir el diálogo.
    let preparando = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<(String, String)>);
    let al_revisar = Callback::new(move |lote: String| {
        if preparando.get_untracked().is_some() {
            return;
        }
        preparando.set(Some(lote.clone()));
        error.set(None);
        spawn_local(async move {
            match preparar_lote(lote.clone(), OpcionesLote::default()).await {
                Ok(g) => match selector.extra(lote, &g, proveedores.get_value()) {
                    Some(extra) => seguimiento.revisar_con(g, extra),
                    None => seguimiento.revisar(g, Vec::new()),
                },
                Err(e) => {
                    error.try_set(Some((lote, texto_error(&e))));
                }
            }
            preparando.try_set(None);
        });
    });
    view! {
        <header class="pagina-cabecera">
            <div>
                <h1>{t!("mejoras-titulo")}</h1>
                <p class="pagina-sub">{t!("mejoras-desc")}</p>
            </div>
        </header>
        <AvisoCambio />
        <Transition fallback=esqueleto>
            {move || Suspend::new(async move {
                match datos.await {
                    Ok(m) => {
                        let solo_lectura = m.solo_lectura;
                        let tarjetas = m
                            .lotes
                            .into_iter()
                            .map(|lote| {
                                let (id, id_error) = (lote.id.clone(), lote.id.clone());
                                let preparando = Signal::derive(move || {
                                    preparando.with(|p| p.as_ref() == Some(&id))
                                });
                                let error = Signal::derive(move || {
                                    error.with(|e| {
                                        e.as_ref()
                                            .filter(|(l, _)| *l == id_error)
                                            .map(|(_, m)| m.clone())
                                    })
                                });
                                view! {
                                    <TarjetaLote
                                        lote
                                        solo_lectura
                                        al_revisar
                                        volver_a_leer
                                        preparando
                                        error
                                    />
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
