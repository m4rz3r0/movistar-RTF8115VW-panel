//! «De dónde sale»: el bloque plegado con las fuentes de una recomendación, o «Criterio del panel».

use crate::ajustes_router::FuenteVista;
use crate::t;
use crate::ui::i18n::traducir;
use leptos::prelude::*;

/// Una fuente: su título enlazado (se abre aparte y sin referente), quién
/// la publica, su clase y cuándo se consultó. El título no se traduce.
fn fuente(f: FuenteVista) -> impl IntoView {
    view! {
        <li>
            <a href=f.url target="_blank" rel="noopener noreferrer">
                {f.titulo}
            </a>
            <span class="de-donde-datos">
                <span>{f.organismo}</span>
                <span>{traducir(&f.clase, None)}</span>
                <span>{t!("fuentes-consultada", fecha = f.consultada)}</span>
            </span>
        </li>
    }
}

/// «De dónde sale» de una recomendación: el respaldo (las guías coinciden,
/// lo cuentan los foros o es criterio del panel) y la lista de fuentes,
/// plegado. Nada si no tiene ni lo uno ni lo otro.
pub(crate) fn de_donde(
    fuentes: Vec<FuenteVista>,
    respaldo: Option<String>,
) -> Option<impl IntoView + use<>> {
    if fuentes.is_empty() && respaldo.is_none() {
        return None;
    }
    let lista = (!fuentes.is_empty()).then(|| {
        view! { <ul class="de-donde-lista">{fuentes.into_iter().map(fuente).collect_view()}</ul> }
    });
    Some(view! {
        <details class="de-donde">
            <summary>{t!("fuentes-titulo")}</summary>
            {respaldo.map(|r| view! { <p class="de-donde-respaldo">{traducir(&r, None)}</p> })}
            {lista}
        </details>
    })
}
