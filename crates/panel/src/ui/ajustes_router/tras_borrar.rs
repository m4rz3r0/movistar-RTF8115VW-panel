//! Tras un borrado: el foco va a la fila vecina o, sin ninguna, a «Abrir…/Añadir».

use super::lista::{id_fila, id_nueva};
use super::piezas::enfocar;

/// A dónde va el foco cuando se vuelva a pintar la página tras borrar
/// `borrados` (filas de la lista `lista`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FocoTrasBorrar {
    pub borrados: Vec<String>,
    /// Id del elemento de la lista (`reglas`).
    pub lista: String,
    /// La fila vecina de las borradas, si quedaba alguna.
    pub vecina: Option<String>,
}

/// La fila vecina de las borradas en `orden` (las filas de su lista, como
/// se ven): la primera que queda después de la primera borrada o, si no
/// hay, la última que queda antes. Pura.
pub(crate) fn elegir_vecina(orden: &[String], borrados: &[String]) -> Option<String> {
    let p = orden.iter().position(|id| borrados.contains(id))?;
    let queda = |id: &&String| !borrados.contains(id);
    orden[p..]
        .iter()
        .find(queda)
        .or_else(|| orden[..p].iter().rev().find(queda))
        .cloned()
}

/// Lo que se sabe de la página antes de volver a leerla: la lista de las
/// filas borradas y su vecina. `None` si no son filas de una lista.
pub(crate) fn antes_de_releer(borrados: &[String]) -> Option<FocoTrasBorrar> {
    use leptos::prelude::document;
    let fila = borrados
        .iter()
        .find_map(|id| document().get_element_by_id(&id_fila(id)))?;
    let seccion = fila.closest("section.lista-seccion").ok().flatten()?;
    let lista = seccion
        .get_attribute("aria-labelledby")?
        .strip_prefix("lista-")?
        .strip_suffix("-titulo")?
        .to_owned();
    let filas = seccion.query_selector_all("tr.lista-fila").ok()?;
    let orden: Vec<String> = (0..filas.length())
        .filter_map(|i| filas.item(i))
        .filter_map(|n| {
            use leptos::wasm_bindgen::JsCast;
            let e = n.dyn_into::<leptos::web_sys::Element>().ok()?;
            e.id().strip_prefix("fila-").map(str::to_owned)
        })
        .collect();
    Some(FocoTrasBorrar {
        borrados: borrados.to_vec(),
        vecina: elegir_vecina(&orden, borrados),
        lista,
    })
}

/// Con la página ya pintada de nuevo: el foco a la fila vecina; si ya no
/// está, a la primera de la lista; sin filas, a su botón de «Abrir…» o
/// «Añadir».
pub(crate) fn enfocar_tras_borrar(f: &FocoTrasBorrar) {
    use leptos::prelude::document;
    let existe = |id: &str| document().get_element_by_id(id).is_some();
    if let Some(v) = f.vecina.as_deref().map(id_fila).filter(|id| existe(id)) {
        enfocar(&v);
        return;
    }
    let primera = document()
        .query_selector(&format!(
            "section[aria-labelledby=\"lista-{}-titulo\"] tr.lista-fila",
            f.lista
        ))
        .ok()
        .flatten()
        .map(|e| e.id());
    match primera {
        Some(id) if !id.is_empty() => enfocar(&id),
        _ => enfocar(&id_nueva(&f.lista)),
    }
}

#[cfg(test)]
mod tests {
    use super::elegir_vecina;

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    /// La vecina es la siguiente que queda; si las borradas eran las
    /// últimas, la anterior; si no queda ninguna, ninguna.
    #[test]
    fn la_vecina_de_lo_borrado() {
        let orden = ids(&["1", "2", "3", "4", "5"]);
        assert_eq!(
            elegir_vecina(&orden, &ids(&["2", "3"])).as_deref(),
            Some("4")
        );
        assert_eq!(
            elegir_vecina(&orden, &ids(&["4", "2"])).as_deref(),
            Some("3")
        );
        assert_eq!(
            elegir_vecina(&orden, &ids(&["4", "5"])).as_deref(),
            Some("3")
        );
        assert_eq!(elegir_vecina(&ids(&["1", "2"]), &ids(&["1", "2"])), None);
        assert_eq!(elegir_vecina(&orden, &ids(&["9"])), None);
    }
}
