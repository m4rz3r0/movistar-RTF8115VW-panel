//! La barra de pestañas de los temas: las pestañas, con un indicador bajo
//! la actual (desplazables si no caben), y a la derecha cuánto hace que se
//! leyó lo que se ve, con el icono para volver a leerlo.

use super::{Precarga, nombre, ruta};
use crate::t;
use ajustes::Tema;
use leptos::prelude::*;

/// La edad de lo que enseña la página del tema, en segundos al llegar la
/// lectura. La pone el contenido al pintarse (`None` sin tema leído) y la
/// pinta la barra, que vive fuera de la lectura.
#[derive(Clone, Copy)]
pub(crate) struct Lectura(RwSignal<Option<u64>>);

impl Lectura {
    pub(crate) fn nueva() -> Lectura {
        Lectura(RwSignal::new(None))
    }

    /// Ha llegado una lectura con `edad_s` segundos. Al montarse lo que la
    /// pinta, en el navegador: el HTML del servidor no la lleva y la
    /// hidratación casa.
    pub(crate) fn llega(edad_s: u64) {
        if let Some(l) = use_context::<Lectura>() {
            Effect::new(move |_| l.0.set(Some(edad_s)));
        }
    }

    /// No hay lectura que enseñar (el tema no se ha podido leer).
    pub(crate) fn falta() {
        if let Some(l) = use_context::<Lectura>() {
            Effect::new(move |_| l.0.set(None));
        }
    }
}

/// «Leído hace N s» (o «… min») a partir de `edad_s` segundos.
pub(super) fn leido_hace(s: u64) -> String {
    if s < 60 {
        t!("ajustes-leido-hace", s = s)
    } else {
        t!("ajustes-leido-hace-min", min = s / 60)
    }
}

/// Lleva la pestaña actual a la vista dentro de la barra (en el móvil no
/// caben todas).
fn mostrar_actual() {
    use leptos::wasm_bindgen::JsCast;
    use leptos::web_sys::HtmlElement;
    let doc = document();
    let como = |s: &str| {
        doc.query_selector(s)
            .ok()
            .flatten()
            .and_then(|n| n.dyn_into::<HtmlElement>().ok())
    };
    let (Some(nav), Some(a)) = (como("nav.temas"), como("nav.temas a[aria-current=page]")) else {
        return;
    };
    let (izq, der) = (a.offset_left(), a.offset_left() + a.offset_width());
    let (desde, ancho) = (nav.scroll_left(), nav.client_width());
    if izq < desde {
        nav.set_scroll_left(izq - 24);
    } else if der > desde + ancho {
        nav.set_scroll_left(der - ancho + 24);
    }
}

/// Cuánto hace que se leyó lo que se ve (cuenta cada segundo) y el icono
/// de volver a leer, que lo lee de nuevo sin la caché. Mientras lee, el
/// icono gira y el botón queda `aria-busy` (y no hace nada).
#[component]
fn LecturaDelTema(
    lectura: Lectura,
    leyendo: RwSignal<bool>,
    volver_a_leer: Callback<()>,
) -> impl IntoView {
    let pasados = RwSignal::new(0u64);
    // Una lectura nueva vuelve a contar desde su edad.
    Effect::new(move |_| {
        lectura.0.track();
        pasados.set(0);
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
    let nombre = t!("ajustes-volver-a-leer");
    // Siempre en la barra (oculta sin lectura): el botón no se vuelve a
    // crear al llegar la lectura nueva y no pierde el foco.
    view! {
        <div class="tema-lectura" hidden=move || lectura.0.with(Option::is_none)>
            <span class="tema-leido">
                {move || lectura.0.get().map(|edad| leido_hace(edad + pasados.get()))}
            </span>
            <button
                type="button"
                class="recargar"
                aria-label=nombre.clone()
                title=nombre
                aria-busy=move || leyendo.get().then_some("true")
                on:click=move |_| {
                    if !leyendo.get_untracked() {
                        volver_a_leer.run(());
                    }
                }
            >
                <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                    <path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3" />
                    <path d="M19.5 4.5v4.2h-4.2" />
                </svg>
            </button>
        </div>
    }
}

/// La barra: las pestañas de los temas y la lectura del actual.
pub(super) fn pestanas(
    slug: Memo<String>,
    precarga: Precarga,
    lectura: Lectura,
    leyendo: RwSignal<bool>,
    volver_a_leer: Callback<()>,
) -> impl IntoView {
    let enlaces = Tema::TODOS
        .into_iter()
        .map(|t| {
            let actual = move || (slug.get() == t.slug()).then_some("page");
            view! {
                <a
                    href=ruta(t)
                    aria-current=actual
                    on:mouseenter=move |_| precarga.programar(t, slug)
                    on:focus=move |_| precarga.programar(t, slug)
                    on:mouseleave=move |_| precarga.cancelar()
                    on:blur=move |_| precarga.cancelar()
                >
                    {nombre(t)}
                </a>
            }
        })
        .collect_view();
    Effect::new(move |_| {
        slug.track();
        request_animation_frame(mostrar_actual);
    });
    view! {
        <div class="temas-barra">
            <nav class="temas" aria-label=t!("ajustes-temas")>{enlaces}</nav>
            <LecturaDelTema lectura leyendo volver_a_leer />
        </div>
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::leido_hace;

    #[test]
    fn leido_hace_en_segundos_y_minutos() {
        assert_eq!(leido_hace(0), "Leído ahora mismo");
        assert_eq!(leido_hace(1), "Leído hace 1 s");
        assert_eq!(leido_hace(42), "Leído hace 42 s");
        assert_eq!(leido_hace(130), "Leído hace 2 min");
    }
}
