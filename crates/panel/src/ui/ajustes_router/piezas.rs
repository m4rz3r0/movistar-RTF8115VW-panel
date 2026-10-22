//! Piezas comunes de un ajuste: cabecera, ayuda, detalles técnicos y valores.
//!
//! Las usan el detalle de un elemento ([`super::detalle`]), las tarjetas de
//! los temas que aún no van en rejilla ([`super::heredada`]) y los
//! componentes de Red local y Puertos. Los detalles técnicos plegados son
//! lo único que nombra la API.

use super::campo::{es_recomendado, etiqueta, texto_legible, valor_resumido};
use crate::ajustes_router::{AjusteVista, CampoVista, DetallesTecnicos};
use crate::t;
use crate::ui::comun::Insignia;
use crate::ui::explorador::RiesgoCambio;
use crate::ui::i18n::traducir;
use ajustes::Nivel;
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

/// Etiqueta de nivel: nada en lo esencial; «Avanzado» u «Oculto» (con lo
/// que significa) en lo demás.
pub(crate) fn nivel(n: Nivel) -> Option<AnyView> {
    match n {
        Nivel::Basico => None,
        Nivel::Avanzado => Some(
            view! { <span class="chip chip-nivel">{t!("ajustes-nivel-avanzado")}</span> }
                .into_any(),
        ),
        // Lo que significa va una sola vez en la página (`NOTA_OCULTO`). Un
        // enlace a esa nota: un lector de pantalla anuncia su descripción
        // (en un `<span>` sin rol, no siempre) y lleva a ella.
        Nivel::Oculto => Some(
            view! {
                <a
                    class="chip chip-nivel chip-oculto"
                    href=format!("#{}", super::NOTA_OCULTO)
                    aria-describedby=super::NOTA_OCULTO
                >
                    {t!("ajustes-nivel-oculto")}
                </a>
            }
            .into_any(),
        ),
    }
}

pub(crate) fn riesgo(r: RiesgoCambio) -> AnyView {
    let (texto, desc) = match r {
        RiesgoCambio::Bajo => (t!("ajustes-riesgo-bajo"), t!("ajustes-riesgo-bajo-desc")),
        RiesgoCambio::Medio => (t!("ajustes-riesgo-medio"), t!("ajustes-riesgo-medio-desc")),
        RiesgoCambio::Alto => (t!("ajustes-riesgo-alto"), t!("ajustes-riesgo-alto-desc")),
    };
    view! {
        <Insignia estado=r.estado() texto=texto />
        <span class="solo-lector">{desc}</span>
    }
    .into_any()
}

/// Cabecera común: título (enfocable, para volver a él tras guardar),
/// nivel, riesgo si se puede cambiar y la descripción corta.
pub(crate) fn cabecera(a: &AjusteVista, editable: bool) -> impl IntoView + use<> {
    let titulo_id = format!("{}-titulo", dom(a));
    view! {
        <header class="ajuste-cabecera">
            <h2 id=titulo_id tabindex="-1">{traducir(&format!("{}-titulo", a.clave), None)}</h2>
            <div class="ajuste-etiquetas">
                {nivel(a.nivel)}
                {editable.then(|| riesgo(a.riesgo))}
            </div>
        </header>
        <p class="ajuste-desc">{traducir(&format!("{}-desc", a.clave), None)}</p>
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

/// Motivos (sin repetir) de los campos que solo se pueden consultar.
pub(crate) fn motivos(campos: &[CampoVista]) -> Option<impl IntoView + use<>> {
    let claves = sin_repetidos(campos.iter().filter_map(|c| c.solo_lectura.as_ref()));
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
