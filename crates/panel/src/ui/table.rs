//! Tabla genérica, densa y legible, con desplazamiento horizontal propio
//! (`.tabla-scroll`) para que la página nunca lo tenga.
//!
//! Los secretos no llegan nunca a la tabla: [`Celda::Secreto`] no lleva valor
//! y se pinta como `<span class="secreto">••••••</span>`.

use crate::ui::{Estado, Insignia};
use leptos::prelude::*;

/// Lo que se muestra en lugar de un valor secreto.
pub const MASCARA: &str = "••••••";

/// Columna: título y alineación (las numéricas, a la derecha y con cifras
/// tabulares).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Columna {
    pub titulo: String,
    pub numerica: bool,
}

impl Columna {
    pub fn new(titulo: impl Into<String>) -> Self {
        Columna {
            titulo: titulo.into(),
            numerica: false,
        }
    }

    pub fn numerica(titulo: impl Into<String>) -> Self {
        Columna {
            titulo: titulo.into(),
            numerica: true,
        }
    }
}

/// Contenido de una celda.
#[derive(Clone, Debug, PartialEq)]
pub enum Celda {
    Texto(String),
    /// Identificadores técnicos (IP, MAC, nombres de función) en monoespaciada.
    Mono(String),
    /// Número ya formateado (cifras tabulares).
    Numero(String),
    /// Valor secreto: no se transporta, solo se indica que existe.
    Secreto,
    /// Estado con icono y texto.
    Estado(Estado, String),
    /// Sin valor («—»).
    Vacia,
}

fn celda(c: Celda) -> AnyView {
    match c {
        Celda::Texto(t) => t.into_any(),
        Celda::Mono(t) => view! { <span class="mono">{t}</span> }.into_any(),
        Celda::Numero(t) => view! { <span class="cifras">{t}</span> }.into_any(),
        Celda::Secreto => view! {
            <span class="secreto">{MASCARA}</span>
            <span class="solo-lector">"(valor oculto)"</span>
        }
        .into_any(),
        Celda::Estado(estado, texto) => view! { <Insignia estado texto /> }.into_any(),
        Celda::Vacia => view! { <span class="vacia" aria-label="sin valor">"—"</span> }.into_any(),
    }
}

/// Tabla con cabecera, filas y, opcionalmente, título (`<caption>`) y texto
/// para cuando no hay filas.
#[component]
pub fn Tabla(
    #[prop(into)] columnas: Signal<Vec<Columna>>,
    #[prop(into)] filas: Signal<Vec<Vec<Celda>>>,
    #[prop(into, optional)] titulo: MaybeProp<String>,
    #[prop(into, optional)] vacia: MaybeProp<String>,
) -> impl IntoView {
    let cabecera = move || {
        columnas
            .get()
            .into_iter()
            .map(|c| {
                let clase = c.numerica.then_some("num");
                view! {
                    <th scope="col" class=clase>
                        {c.titulo}
                    </th>
                }
            })
            .collect_view()
    };
    let cuerpo = move || {
        let numericas: Vec<bool> = columnas.with(|c| c.iter().map(|c| c.numerica).collect());
        let filas = filas.get();
        if filas.is_empty() {
            let texto = vacia.get().unwrap_or_else(|| "Sin datos".to_owned());
            return view! {
                <tr class="fila-vacia">
                    <td colspan=numericas.len().max(1)>{texto}</td>
                </tr>
            }
            .into_any();
        }
        filas
            .into_iter()
            .map(|fila| {
                let celdas = fila
                    .into_iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let clase = numericas.get(i).copied().unwrap_or(false).then_some("num");
                        view! { <td class=clase>{celda(c)}</td> }
                    })
                    .collect_view();
                view! { <tr>{celdas}</tr> }
            })
            .collect_view()
            .into_any()
    };
    view! {
        <div class="tabla-scroll">
            <table class="tabla">
                {move || titulo.get().map(|t| view! { <caption>{t}</caption> })}
                <thead>
                    <tr>{cabecera}</tr>
                </thead>
                <tbody>{cuerpo}</tbody>
            </table>
        </div>
    }
    .into_any()
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    fn html(columnas: Vec<Columna>, filas: Vec<Vec<Celda>>) -> String {
        Owner::new().with(|| view! { <Tabla columnas filas /> }.to_html())
    }

    #[test]
    fn secretos_enmascarados() {
        let h = html(
            vec![Columna::new("Red"), Columna::new("Clave")],
            vec![vec![Celda::Texto("MiWifi".into()), Celda::Secreto]],
        );
        assert!(h.contains("<span class=\"secreto\">••••••</span>"), "{h}");
        assert!(h.contains("MiWifi"), "{h}");
        assert!(h.contains("class=\"tabla-scroll\""), "{h}");
    }

    #[test]
    fn columnas_numericas_alineadas_y_tabla_vacia() {
        let h = html(
            vec![Columna::new("Hora"), Columna::numerica("Bajada")],
            vec![vec![
                Celda::Numero("14:00".into()),
                Celda::Numero("1,5 kbit/s".into()),
            ]],
        );
        assert!(
            h.contains("<th scope=\"col\" class=\"num\">Bajada</th>"),
            "{h}"
        );
        assert!(h.contains("<td class=\"num\">"), "{h}");
        let vacia = html(vec![Columna::new("A")], Vec::new());
        assert!(vacia.contains("Sin datos"), "{vacia}");
    }
}
