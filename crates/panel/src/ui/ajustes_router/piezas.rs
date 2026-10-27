//! Piezas comunes de un ajuste: ayuda, detalles técnicos y valores.
//!
//! Las usan el detalle de un elemento ([`super::detalle`]) y las listas de
//! Red local y Puertos ([`super::lista`]). Los detalles técnicos plegados son
//! lo único que nombra la API.

use super::campo::{es_recomendado, etiqueta, texto_legible, valor_resumido};
use crate::ajustes_router::{AjusteVista, CampoVista, DetallesTecnicos};
use crate::t;
use crate::ui::i18n::traducir;
use leptos::prelude::*;

/// Prefijo de los ids de la tarjeta en la página (`ajuste-wifi-red_24`).
pub(crate) fn dom(a: &AjusteVista) -> String {
    format!("ajuste-{}", a.id.replace('.', "-"))
}

/// Enfoca el elemento `id` de la página, si existe.
pub(crate) fn enfocar(id: &str) {
    use leptos::wasm_bindgen::JsCast;
    if let Some(e) = document()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok())
    {
        let _ = e.focus();
    }
}

/// «¿Qué hace esto?»: la ayuda larga, plegada.
pub(crate) fn ayuda(a: &AjusteVista) -> impl IntoView + use<> {
    view! {
        <details class="ajuste-ayuda">
            <summary>{t!("ajustes-que-hace")}</summary>
            <p>{traducir(&format!("{}-ayuda", a.clave), None)}</p>
        </details>
    }
}

/// Funciones y parámetros del router detrás del ajuste, plegados. Lo único
/// de la página con nombres de la API.
pub(crate) fn detalles_tecnicos(clave: &str, d: &DetallesTecnicos) -> impl IntoView + use<> {
    let parametros = d
        .parametros
        .iter()
        .map(|(campo, param)| {
            view! {
                <tr>
                    <th scope="row">{traducir(&format!("{clave}-{campo}"), None)}</th>
                    <td><code class="mono">{param.clone()}</code></td>
                </tr>
            }
        })
        .collect_view();
    let escrituras = (!d.escrituras.is_empty()).then(|| {
        let lista = d
            .escrituras
            .iter()
            .map(|e| view! { <code class="mono">{e.clone()}</code> " " })
            .collect_view();
        view! {
            <div>
                <dt>{t!("ajustes-detalles-escrituras")}</dt>
                <dd>{lista}</dd>
            </div>
        }
    });
    let con_parametros = !d.parametros.is_empty();
    view! {
        <details class="detalles-tecnicos">
            <summary>{t!("ajustes-detalles-tecnicos")}</summary>
            <p class="ayuda">{t!("ajustes-detalles-tecnicos-desc")}</p>
            <dl class="detalles-lista">
                <div>
                    <dt>{t!("ajustes-detalles-lectura")}</dt>
                    <dd><code class="mono">{d.lectura.clone()}</code></dd>
                </div>
                {escrituras}
            </dl>
            {con_parametros.then(|| view! {
                <div class="tabla-scroll">
                    <table class="tabla">
                        <caption class="solo-lector">{t!("ajustes-detalles-parametros")}</caption>
                        <thead>
                            <tr>
                                <th scope="col">{t!("ajustes-grupo-col-dato")}</th>
                                <th scope="col">{t!("ajustes-detalles-parametros")}</th>
                            </tr>
                        </thead>
                        <tbody>{parametros}</tbody>
                    </table>
                </div>
            })}
        </details>
    }
}

/// `claves` sin repetir, en el orden en que aparecen por primera vez.
fn sin_repetidos<'a>(claves: impl IntoIterator<Item = &'a String>) -> Vec<&'a String> {
    let mut vistas = std::collections::HashSet::new();
    claves.into_iter().filter(|k| vistas.insert(*k)).collect()
}

/// Motivos (sin repetir) de los campos que solo se pueden consultar. Lo
/// que no se escribe por no estar comprobado en el router real no lleva
/// nota: solo se ve su valor.
pub(crate) fn motivos(campos: &[CampoVista]) -> Option<impl IntoView + use<>> {
    let claves = sin_repetidos(
        campos
            .iter()
            .filter_map(|c| c.solo_lectura.as_ref())
            .filter(|k| *k != crate::validacion_real::SIN_COMPROBAR),
    );
    (!claves.is_empty()).then(|| {
        claves
            .into_iter()
            .map(|k| view! { <p class="nota-solo-lectura">{traducir(k, None)}</p> })
            .collect_view()
    })
}

/// Una fila de la vista resumida: etiqueta, valor (un secreto, censurado con
/// el ojo) y, si no es el recomendado, cuál lo es (cuando coincide no se
/// marca: menos ruido). La marca se despliega con para quién es y sus pros
/// y contras.
pub(crate) fn fila(c: &CampoVista) -> impl IntoView + use<> {
    let marca = (es_recomendado(c) == Some(false)).then(|| {
        let r = c.recomendado.clone().unwrap_or_default();
        let chip = t!("ajustes-recomendado-valor", valor = texto_legible(c, &r));
        match &c.motivo_recomendado {
            Some(m) => view! {
                <details class="recomendacion-resumen">
                    <summary class="chip chip-sugerencia">{chip}</summary>
                    <p>{traducir(m, None)}</p>
                </details>
            }
            .into_any(),
            None => view! { <span class="chip chip-sugerencia">{chip}</span> }.into_any(),
        }
    });
    view! {
        <div class="ajuste-fila">
            <dt>{etiqueta(c)}</dt>
            <dd>{valor_resumido(c)} {marca}</dd>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::sin_repetidos;

    #[test]
    fn sin_repetidos_aunque_no_vayan_seguidos() {
        let (a, b) = ("motivo-a".to_owned(), "motivo-b".to_owned());
        assert_eq!(sin_repetidos([&a, &b, &a, &b, &a]), [&a, &b]);
    }
}
