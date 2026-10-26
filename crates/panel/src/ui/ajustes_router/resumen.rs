//! Cabecera de la lectura («Leído hace N s», «Volver a leer») y franja de resumen.

use crate::ajustes_router::DatoResumen;
use crate::t;
use crate::ui::i18n::traducir;
use leptos::prelude::*;

/// La franja de resumen (nada si el tema no la tiene). El dato que pide
/// atención (recomendaciones pendientes) va en ámbar.
#[component]
pub(crate) fn ResumenTema(datos: Vec<DatoResumen>) -> impl IntoView {
    (!datos.is_empty()).then(|| {
        let datos = datos
            .into_iter()
            .map(|d| {
                view! {
                    <div class="resumen-dato" class:pendiente=d.atencion>
                        <p>{traducir(&d.clave, None)}</p>
                        <strong>{d.cifra}</strong>
                        <span>{d.detalle}</span>
                    </div>
                }
            })
            .collect_view();
        view! { <section class="resumen" aria-label=t!("ajustes-resumen")>{datos}</section> }
    })
}

/// «Leído hace N s» (o «… min») a partir de `edad_s` segundos.
fn leido_hace(s: u64) -> String {
    if s < 60 {
        t!("ajustes-leido-hace", s = s)
    } else {
        t!("ajustes-leido-hace-min", min = s / 60)
    }
}

/// Cuánto hace que se leyó del router lo que se ve (`edad_s` al llegar la
/// lectura; cuenta cada segundo en el navegador) y «Volver a leer», que lo
/// lee de nuevo sin la caché. Con `foto`, a la izquierda de la misma barra,
/// el enlace a lo que ha cambiado desde la instalación: una fila en vez de
/// dos encima del resumen.
#[component]
pub(crate) fn LecturaDelTema(
    edad_s: u64,
    volver_a_leer: Callback<()>,
    #[prop(optional)] foto: bool,
) -> impl IntoView {
    let pasados = RwSignal::new(0u64);
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
    view! {
        <div class="tema-lectura">
            {foto.then(|| view! {
                <a class="tema-foto" href=super::RUTA_FOTO>{t!("ajustes-foto-enlace")}</a>
            })}
            <span>{move || leido_hace(edad_s + pasados.get())}</span>
            <button type="button" class="b-sec" on:click=move |_| volver_a_leer.run(())>
                {t!("ajustes-volver-a-leer")}
            </button>
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
