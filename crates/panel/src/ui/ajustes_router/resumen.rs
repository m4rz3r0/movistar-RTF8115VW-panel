//! Franja de resumen de un tema.

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
