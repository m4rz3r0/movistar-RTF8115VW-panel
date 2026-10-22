//! El filtro de texto de una lista: solo cuando tiene más de [`MINIMO_FILTRO`] filas.

use super::columnas::Fila;
use crate::t;
use leptos::prelude::*;

/// Con más filas que estas, la lista lleva filtro de texto (spec §4.2).
pub(crate) const MINIMO_FILTRO: usize = 8;

/// ¿Lleva filtro una lista de `n` filas?
pub(crate) fn hay_filtro(n: usize) -> bool {
    n > MINIMO_FILTRO
}

/// ¿Coincide `f` con lo escrito? Busca cada palabra, sin mayúsculas, en el
/// nombre, lo que va debajo, cada celda y su estado.
pub(crate) fn coincide(f: &Fila, escrito: &str) -> bool {
    let texto = std::iter::once(f.nombre.as_str())
        .chain(f.sub.as_deref())
        .chain(f.celdas.iter().map(|c| c.texto.as_str()))
        .chain(f.celdas.iter().filter_map(|c| c.nota.as_deref()))
        .chain(std::iter::once(f.texto_estado.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    escrito
        .split_whitespace()
        .all(|p| texto.contains(&p.to_lowercase()))
}

/// El campo del filtro (`id`, el de su control). Escribir filtra en el
/// momento.
#[component]
pub(crate) fn Filtro(id: String, texto: RwSignal<String>) -> impl IntoView {
    view! {
        <p class="campo lista-filtro">
            <label for=id.clone()>{t!("ajustes-lista-filtrar")}</label>
            <input
                id=id
                type="search"
                autocomplete="off"
                prop:value=move || texto.get()
                on:input=move |ev| texto.set(event_target_value(&ev))
            />
        </p>
    }
}

#[cfg(test)]
mod tests {
    use super::super::columnas::{Celda, Orden};
    use super::*;
    use crate::ui::comun::EstadoVisual;

    fn fila() -> Fila {
        Fila {
            id: "1".into(),
            nombre: "Consola".into(),
            sub: Some("8080 hacia 192.168.1.62".into()),
            sub_mono: false,
            celdas: vec![Celda {
                texto: "TCP".into(),
                nota: Some("host-2".into()),
                mono: false,
                chip: true,
                interruptor: None,
                orden: Orden::Texto("tcp".into()),
            }],
            estado: EstadoVisual::Emite,
            texto_estado: "Abierta".into(),
            activa: Some(true),
        }
    }

    #[test]
    fn filtro_solo_con_mas_de_ocho() {
        assert!(!hay_filtro(8));
        assert!(hay_filtro(9));
    }

    #[test]
    fn coincide_por_palabras_sin_mayusculas() {
        let f = fila();
        assert!(coincide(&f, ""));
        assert!(coincide(&f, "consola"));
        assert!(coincide(&f, "tcp HOST-2"));
        assert!(coincide(&f, "192.168.1.62"));
        assert!(coincide(&f, "abierta"));
        assert!(!coincide(&f, "udp"));
        assert!(!coincide(&f, "consola udp"));
    }
}
