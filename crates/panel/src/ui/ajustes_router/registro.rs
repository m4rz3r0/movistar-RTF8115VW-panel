//! «Registro del router», al pie de Sistema: el registro del sistema del
//! router ([`leer_registro_router`]) en un bloque de texto que se desplaza,
//! con lo más nuevo abajo, un filtro de líneas y «Descargar» (el registro
//! entero, como `registro-router-AAAAMMDD-HHMM.log`). Solo se lee al
//! pulsar el botón: cada lectura le pide al router que lo exporte.

use super::pestanas::leido_hace;
use crate::registro_router::{LeerRegistroRouter, RegistroRouter};
use crate::t;
use crate::ui::charts::HoraLocal;
use crate::ui::explorador::texto_error;
use crate::ui::{Estado, IconoEstado};
use leptos::prelude::*;

/// Id del campo del filtro (para su etiqueta).
const ID_FILTRO: &str = "registro-router-filtro";

/// Las líneas de `texto` que contienen cada palabra de `escrito`, sin
/// mayúsculas (como el filtro de las listas). Sin nada escrito, todas.
pub(crate) fn filtrar(texto: &str, escrito: &str) -> String {
    let palabras: Vec<String> = escrito.split_whitespace().map(str::to_lowercase).collect();
    if palabras.is_empty() {
        return texto.to_owned();
    }
    texto
        .lines()
        .filter(|l| {
            let l = l.to_lowercase();
            palabras.iter().all(|p| l.contains(p.as_str()))
        })
        .map(|l| format!("{l}\n"))
        .collect()
}

/// `registro-router-AAAAMMDD-HHMM.log` con la fecha y la hora `h`.
pub(crate) fn nombre_fichero(h: &HoraLocal) -> String {
    format!(
        "registro-router-{:04}{:02}{:02}-{:02}{:02}.log",
        h.anio, h.mes, h.dia, h.hora, h.minuto
    )
}

/// Guarda `texto` en el equipo como el fichero `nombre`.
#[cfg(feature = "hydrate")]
fn descargar(nombre: &str, texto: &str) {
    use leptos::wasm_bindgen::{JsCast, JsValue};
    use leptos::web_sys::{Blob, HtmlAnchorElement, Url, js_sys::Array};
    let Ok(blob) = Blob::new_with_str_sequence(&Array::of1(&JsValue::from_str(texto))) else {
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
fn descargar(_nombre: &str, _texto: &str) {}

/// Segundos Unix de ahora, en el navegador.
fn ahora() -> i64 {
    #[cfg(feature = "hydrate")]
    {
        (leptos::web_sys::js_sys::Date::now() / 1000.0) as i64
    }
    #[cfg(not(feature = "hydrate"))]
    {
        0
    }
}

/// Desplaza el bloque del registro hasta su última línea.
fn al_final() {
    if let Ok(Some(pre)) = document().query_selector("pre.registro-router-texto") {
        pre.set_scroll_top(pre.scroll_height());
    }
}

/// Lo último leído y cuándo llegó (segundos Unix).
#[derive(Clone)]
struct Leido {
    registro: RegistroRouter,
    llegada: i64,
}

/// La sección del registro.
#[component]
pub(crate) fn SeccionRegistro() -> impl IntoView {
    let leer = ServerAction::<LeerRegistroRouter>::new();
    let leido: RwSignal<Option<Leido>> = RwSignal::new(None);
    let filtro = RwSignal::new(String::new());
    let pasados = RwSignal::new(0u64);
    // Una lectura buena sustituye a la anterior; una fallida la deja a la
    // vista, con el error encima.
    Effect::new(move |_| {
        if let Some(Ok(r)) = leer.value().get() {
            pasados.set(0);
            leido.set(Some(Leido {
                registro: r,
                llegada: ahora(),
            }));
        }
    });
    Effect::new(move |_| {
        let h = set_interval_with_handle(
            move || {
                pasados.try_update(|p| *p += 1);
            },
            std::time::Duration::from_secs(1),
        )
        .ok();
        on_cleanup(move || {
            if let Some(h) = h {
                h.clear();
            }
        });
    });
    // Lo más nuevo, abajo: a la vista al llegar.
    Effect::new(move |_| {
        leido.track();
        request_animation_frame(al_final);
    });
    let error = move || {
        leer.value()
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
    let hay = move || leido.with(Option::is_some);
    let visible = move || {
        leido.with(|l| {
            l.as_ref()
                .map(|l| filtro.with(|f| filtrar(&l.registro.texto, f)))
        })
    };
    let bajar = move |_| {
        leido.with_untracked(|l| {
            if let Some(l) = l {
                let h = crate::ui::charts::hora_local(l.llegada);
                descargar(&nombre_fichero(&h), &l.registro.texto);
            }
        });
    };
    view! {
        <section class="tema-nota registro-router" aria-labelledby="registro-router-titulo">
            <h2 id="registro-router-titulo">{t!("registro-router-titulo")}</h2>
            <p>{t!("registro-router-desc")}</p>
            <div class="registro-router-barra">
                <button
                    type="button"
                    class="b-sec"
                    aria-busy=move || leer.pending().get().then_some("true")
                    on:click=move |_| {
                        if !leer.pending().get_untracked() {
                            leer.dispatch(LeerRegistroRouter {});
                        }
                    }
                >
                    {move || {
                        if leer.pending().get() {
                            t!("registro-router-leyendo")
                        } else if hay() {
                            t!("registro-router-volver")
                        } else {
                            t!("registro-router-leer")
                        }
                    }}
                </button>
                <button type="button" class="b-sec" disabled=move || !hay() on:click=bajar>
                    {t!("registro-router-descargar")}
                </button>
                <span class="registro-router-leido">
                    {move || {
                        leido
                            .with(|l| l.as_ref().map(|_| leido_hace(pasados.get())))
                    }}
                </span>
            </div>
            {error}
            <Show when=hay>
                <p class="campo lista-filtro">
                    <label for=ID_FILTRO>{t!("registro-router-filtrar")}</label>
                    <input
                        id=ID_FILTRO
                        type="search"
                        autocomplete="off"
                        prop:value=move || filtro.get()
                        on:input=move |ev| filtro.set(event_target_value(&ev))
                    />
                </p>
                {move || {
                    leido
                        .with(|l| l.as_ref().is_some_and(|l| l.registro.recortado))
                        .then(|| view! { <p class="registro-router-nota">{t!("registro-router-recortado")}</p> })
                }}
                {move || match visible() {
                    Some(v) if v.trim().is_empty() => {
                        let vacio = leido.with(|l| {
                            l.as_ref().is_some_and(|l| l.registro.texto.trim().is_empty())
                        });
                        let msg = if vacio {
                            t!("registro-router-vacio")
                        } else {
                            t!("registro-router-sin-coincidencias")
                        };
                        view! { <p class="registro-router-nota">{msg}</p> }.into_any()
                    }
                    Some(v) => view! {
                        <pre
                            class="registro-router-texto"
                            tabindex="0"
                            role="region"
                            aria-labelledby="registro-router-titulo"
                        >
                            {v}
                        </pre>
                    }
                        .into_any(),
                    None => ().into_any(),
                }}
            </Show>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtra_lineas_por_palabras_sin_mayusculas() {
        let t = "Jan 1 dnsmasq DHCPACK\nJan 1 httpd login\nJan 2 DNSMASQ started\n";
        assert_eq!(filtrar(t, ""), t);
        assert_eq!(
            filtrar(t, "dnsmasq"),
            "Jan 1 dnsmasq DHCPACK\nJan 2 DNSMASQ started\n"
        );
        assert_eq!(filtrar(t, "jan  dhcpack"), "Jan 1 dnsmasq DHCPACK\n");
        assert_eq!(filtrar(t, "nada"), "");
    }

    #[test]
    fn nombre_del_fichero() {
        let h = HoraLocal {
            anio: 2026,
            mes: 3,
            dia: 7,
            hora: 9,
            minuto: 5,
            segundo: 59,
        };
        assert_eq!(nombre_fichero(&h), "registro-router-20260307-0905.log");
    }
}
