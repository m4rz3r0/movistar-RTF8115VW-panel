//! `Minigrafica` e `Indicador`.

use super::{
    escalas::escala,
    geometria::{Tono, Trazo, ruta_area, ruta_linea, segmentos},
};
use crate::ui::{Estado, IconoEstado};
use leptos::prelude::*;

/// Geometría de una minigráfica en un lienzo de `ancho × alto` (sin ejes,
/// desde cero): trazados de línea y de área y último punto.
fn geometria_mini(valores: &[Option<f64>], ancho: f64, alto: f64) -> Trazo {
    let puntos: Vec<(i64, Option<f64>)> = valores
        .iter()
        .enumerate()
        .map(|(i, v)| (i as i64, *v))
        .collect();
    let max = valores
        .iter()
        .flatten()
        .copied()
        .filter(|v| v.is_finite())
        .fold(0.0, f64::max);
    let max = if max > 0.0 { max } else { 1.0 };
    let n = valores.len().saturating_sub(1).max(1) as f64;
    let sx = escala((0.0, n), (0.0, ancho));
    let sy = escala((0.0, max), (alto - 1.0, 3.0));
    let mut trazo = Trazo::vacio("");
    for tramo in segmentos(&puntos) {
        let px: Vec<(f64, f64)> = tramo.iter().map(|&(i, v)| (sx(i as f64), sy(v))).collect();
        if px.len() > 1 {
            trazo.areas.push(ruta_area(&px, alto));
            trazo.lineas.push(ruta_linea(&px));
        }
        trazo.fin = px.last().copied();
    }
    trazo
}

/// Minigráfica decorativa (sin ejes) de los últimos valores, desde cero. El
/// valor lo da el texto que la acompaña.
#[component]
pub fn Minigrafica(
    #[prop(into)] valores: Signal<Vec<Option<f64>>>,
    #[prop(optional)] tono: Tono,
) -> impl IntoView {
    const ANCHO: f64 = 120.0;
    const ALTO: f64 = 32.0;
    let clase = tono.clase();
    let dibujo = move || {
        let t = valores.with(|v| geometria_mini(v, ANCHO, ALTO));
        let areas = t
            .areas
            .into_iter()
            .map(|d| view! { <path class=format!("area {clase}") d=d /> })
            .collect_view();
        let lineas = t
            .lineas
            .into_iter()
            .map(|d| {
                view! {
                    <path class=format!("linea {clase}") d=d vector-effect="non-scaling-stroke" />
                }
            })
            .collect_view();
        // Trazo de longitud cero con extremo redondo: un punto que no se
        // deforma con `preserveAspectRatio="none"`.
        let fin = t.fin.map(|(x, y)| {
            view! {
                <path
                    class=format!("mini-fin {clase}")
                    d=format!("M{x:.1},{y:.1}l0,0")
                    vector-effect="non-scaling-stroke"
                />
            }
        });
        view! {
            {areas}
            {lineas}
            {fin}
        }
    };
    view! {
        <svg
            class="minigrafica"
            viewBox=format!("0 0 {ANCHO} {ALTO}")
            preserveAspectRatio="none"
            aria-hidden="true"
            focusable="false"
        >
            {dibujo}
        </svg>
    }
}

/// Tarjeta de indicador: título, valor destacado, subtítulo opcional, estado
/// opcional (icono junto al valor, que hace de etiqueta) y minigráfica
/// opcional.
#[component]
pub fn Indicador(
    #[prop(into)] titulo: String,
    #[prop(into)] valor: Signal<String>,
    #[prop(into, optional)] sub: MaybeProp<String>,
    #[prop(into, optional)] minigrafica: Option<Signal<Vec<Option<f64>>>>,
    #[prop(into, optional)] estado: MaybeProp<Estado>,
    /// Color de la minigráfica.
    #[prop(optional)]
    tono: Tono,
) -> impl IntoView {
    let icono = move || estado.get().map(|estado| view! { <IconoEstado estado /> });
    let clase = move || match estado.get() {
        Some(e) => format!("indicador {}", e.clase()),
        None => "indicador".to_owned(),
    };
    view! {
        <article class=clase>
            <h3 class="indicador-titulo">{titulo}</h3>
            <p class="indicador-valor">{icono} <span>{valor}</span></p>
            {move || sub.get().map(|s| view! { <p class="indicador-sub">{s}</p> })}
            {minigrafica.map(|valores| view! { <Minigrafica valores tono /> })}
        </article>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minigrafica_con_hueco() {
        let t = geometria_mini(
            &[Some(1.0), Some(2.0), None, Some(1.0), Some(3.0)],
            120.0,
            32.0,
        );
        assert_eq!(t.lineas.len(), 2);
        assert_eq!(t.fin, Some((120.0, 3.0)));
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn indicador_pasa_el_tono_a_la_minigrafica() {
        let html = Owner::new().with(|| {
            view! {
                <Indicador
                    titulo="Subida"
                    valor="1,2 Mbit/s"
                    minigrafica=vec![Some(1.0), Some(2.0)]
                    tono=Tono::Secundario
                />
            }
            .to_html()
        });
        assert!(html.contains("linea tono-2"), "{html}");
        assert!(!html.contains("tono-1"), "{html}");
    }
}
