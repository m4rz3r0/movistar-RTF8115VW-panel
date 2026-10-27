//! «Copia de la configuración», al pie de Sistema: un botón que hace que el
//! router guarde su configuración, genere la copia y la descargue
//! ([`descargar_copia_router`]), y se la da al navegador con el nombre del
//! router (`config_<fecha>.tar`). Avisa de que el fichero lleva todas las
//! contraseñas del router. Sin comprobar en el router real, o con el panel
//! en solo lectura, el botón se ve desactivado con el motivo
//! ([`estado_copia_router`]).
//!
//! El contenido no se queda en la página: en cuanto llega se le pasa al
//! navegador y se suelta; solo queda a la vista el nombre y el tamaño.

use crate::copia_router::{DescargarCopiaRouter, estado_copia_router};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::{Estado, IconoEstado};
use leptos::prelude::*;

/// Bytes que ocupa lo codificado en `b64` (base64 estándar con relleno).
pub(crate) fn tamano_b64(b64: &str) -> usize {
    let relleno = b64.bytes().rev().take_while(|&b| b == b'=').count().min(2);
    (b64.len() / 4 * 3).saturating_sub(relleno)
}

/// KB (de 1024 bytes) de `bytes`, redondeados hacia arriba: «1 KB» como
/// poco, para que una copia pequeña no parezca vacía.
pub(crate) fn kb(bytes: usize) -> usize {
    bytes.div_ceil(1024).max(1)
}

/// Guarda en el equipo el fichero `nombre` codificado en base64 en `b64`.
#[cfg(feature = "hydrate")]
fn descargar(nombre: &str, b64: &str) {
    use leptos::wasm_bindgen::JsCast;
    use leptos::web_sys::{
        Blob, HtmlAnchorElement, Url,
        js_sys::{Array, Uint8Array},
    };
    let Ok(binario) = window().atob(b64) else {
        return;
    };
    // `atob` da un carácter por byte (0-255).
    let bytes: Vec<u8> = binario.chars().map(|c| c as u32 as u8).collect();
    let Ok(blob) = Blob::new_with_u8_array_sequence(&Array::of1(&Uint8Array::from(&bytes[..])))
    else {
        return;
    };
    let Ok(url) = Url::create_object_url_with_blob(&blob) else {
        return;
    };
    if let Ok(a) = document().create_element("a") {
        let a: HtmlAnchorElement = a.unchecked_into();
        a.set_href(&url);
        a.set_download(nombre);
        a.click();
    }
    // Después de que el navegador empiece la descarga.
    set_timeout(
        move || {
            let _ = Url::revoke_object_url(&url);
        },
        std::time::Duration::from_secs(10),
    );
}

#[cfg(not(feature = "hydrate"))]
fn descargar(_nombre: &str, _b64: &str) {}

/// La sección de la copia.
#[component]
pub(crate) fn SeccionCopia() -> impl IntoView {
    let accion = ServerAction::<DescargarCopiaRouter>::new();
    let estado = Resource::new(|| (), |_| estado_copia_router());
    // Nombre y tamaño de la última copia descargada.
    let hecha: RwSignal<Option<(String, usize)>> = RwSignal::new(None);
    // En cuanto llega, al navegador; luego se suelta.
    Effect::new(move |_| {
        let llegada = accion.value().with(|v| match v {
            Some(Ok(c)) => {
                descargar(&c.nombre, &c.datos);
                Some((c.nombre.clone(), tamano_b64(&c.datos)))
            }
            _ => None,
        });
        if let Some(h) = llegada {
            hecha.set(Some(h));
            accion.value().set(None);
        }
    });
    let error = move || {
        accion
            .value()
            .with(|v| match v {
                Some(Err(e)) => Some(texto_error(e)),
                _ => None,
            })
            .map(|msg| {
                view! {
                    <p class="aviso estado-error" role="alert">
                        <IconoEstado estado=Estado::Error />
                        <span>{msg}</span>
                    </p>
                }
            })
    };
    let barra = move || {
        estado.get().map(|e| {
            // Sin saber si se puede (la lectura falló), el botón sigue: el
            // servidor lo vuelve a comprobar.
            let motivo = e.ok().flatten();
            let desactivado = motivo.is_some();
            let nota = motivo.map(|m| {
                view! { <p class="copia-router-nota">{crate::ui::i18n::traducir(&m, None)}</p> }
            });
            view! {
                <div class="copia-router-barra">
                    <button
                        type="button"
                        class="b-sec"
                        disabled=desactivado
                        aria-busy=move || accion.pending().get().then_some("true")
                        on:click=move |_| {
                            if !accion.pending().get_untracked() {
                                hecha.set(None);
                                accion.dispatch(DescargarCopiaRouter {});
                            }
                        }
                    >
                        {move || {
                            if accion.pending().get() {
                                t!("copia-router-preparando")
                            } else {
                                t!("copia-router-descargar")
                            }
                        }}
                    </button>
                    <span class="copia-router-hecha" role="status">
                        {move || {
                            hecha
                                .get()
                                .map(|(nombre, bytes)| {
                                    t!("copia-router-hecha", nombre = nombre, kb = kb(bytes))
                                })
                        }}
                    </span>
                </div>
                {nota}
            }
        })
    };
    view! {
        <section class="tema-nota copia-router" aria-labelledby="copia-router-titulo">
            <h2 id="copia-router-titulo">{t!("copia-router-titulo")}</h2>
            <p>{t!("copia-router-desc")}</p>
            <p class="copia-router-aviso">
                <IconoEstado estado=Estado::Aviso />
                <span>{t!("copia-router-secretos")}</span>
            </p>
            <Suspense fallback=|| ()>{barra}</Suspense>
            {error}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tamano_de_lo_codificado() {
        // "", "a", "ab", "abc", "abcd" en base64.
        for (b64, n) in [
            ("", 0),
            ("YQ==", 1),
            ("YWI=", 2),
            ("YWJj", 3),
            ("YWJjZA==", 4),
        ] {
            assert_eq!(tamano_b64(b64), n, "{b64}");
        }
    }

    #[test]
    fn kb_redondea_hacia_arriba() {
        assert_eq!(kb(0), 1);
        assert_eq!(kb(1), 1);
        assert_eq!(kb(1024), 1);
        assert_eq!(kb(1025), 2);
        assert_eq!(kb(40 * 1024 + 3), 41);
    }
}
