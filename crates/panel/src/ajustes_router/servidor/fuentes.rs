//! De dónde sale cada recomendación: las fuentes del catálogo como [`FuenteVista`], para el campo, la tarjeta y el paso de Mejoras.

use super::*;
use crate::ajustes_router::{FuenteVista, RecomendacionVista};
use ajustes::{CatalogoAjustes, Elemento, PasoExpandido, Respaldo};

/// Las fuentes `ids` del catálogo, en su orden y sin repetir (una que no
/// existe no sale: `ajustes::comprobar` ya la rechaza).
pub(super) fn vistas<'a>(
    cat: &CatalogoAjustes,
    ids: impl IntoIterator<Item = &'a String>,
) -> Vec<FuenteVista> {
    let mut vistas: Vec<FuenteVista> = Vec::new();
    for f in ids.into_iter().filter_map(|id| cat.fuente(id)) {
        if vistas.iter().all(|v| v.url != f.url) {
            vistas.push(FuenteVista {
                titulo: f.titulo.clone(),
                organismo: f.organismo.clone(),
                url: f.url.clone(),
                clase: f.clase.clave(),
                consultada: f.consultada.clone(),
            });
        }
    }
    vistas
}

/// Clave de traducción del respaldo (`fuentes-respaldo-criterio`).
pub(super) fn respaldo(r: Option<Respaldo>) -> Option<String> {
    r.map(Respaldo::clave)
}

/// Fuentes y respaldo de la recomendación del campo `c`: las suyas y las
/// de su opción recomendada (el respaldo de la opción manda).
fn del_campo(cat: &CatalogoAjustes, c: &Campo) -> (Vec<FuenteVista>, Option<String>) {
    let opcion = c.opciones.iter().find(|o| o.recomendada);
    let ids = c
        .fuentes
        .iter()
        .chain(opcion.into_iter().flat_map(|o| &o.fuentes));
    let r = opcion.and_then(|o| o.respaldo).or(c.respaldo);
    (vistas(cat, ids), respaldo(r))
}

/// ¿Tiene el campo una recomendación en el catálogo?
fn recomienda(c: &Campo) -> bool {
    c.recomendado.is_some() || c.opciones.iter().any(|o| o.recomendada)
}

/// Pone las fuentes en los campos de la vista del ajuste `a` (plantilla e
/// instancias). Un campo cuya recomendación no vale ahora (la quitó su
/// `recomendado_si`) se queda sin ellas.
pub(super) fn en_ajuste(cat: &CatalogoAjustes, a: &Ajuste, v: &mut AjusteVista) {
    let campos = v
        .campos
        .iter_mut()
        .chain(v.instancias.iter_mut().flat_map(|i| i.campos.iter_mut()));
    for c in campos {
        let Some(x) = a.campo(&c.id) else { continue };
        if recomienda(x) && c.recomendado.is_none() {
            continue;
        }
        (c.fuentes, c.respaldo) = del_campo(cat, x);
    }
}

/// Pone en la recomendación de la tarjeta del elemento `e` las fuentes del
/// campo al que apunta o, si no sale de un campo (la del cortafuegos), las
/// del propio elemento.
pub(super) fn en_recomendacion(cat: &CatalogoAjustes, e: &Elemento, r: &mut RecomendacionVista) {
    let campo = cat.ajuste(&r.ajuste).and_then(|a| a.campo(&r.campo));
    let (fuentes, respaldo_) = match campo {
        Some(c) if !c.fuentes.is_empty() || c.respaldo.is_some() || recomienda(c) => {
            del_campo(cat, c)
        }
        _ => (vistas(cat, &e.fuentes), respaldo(e.respaldo)),
    };
    r.fuentes = fuentes;
    r.respaldo = respaldo_;
}

/// Fuentes y respaldo de un paso de lote; el de una elección suma las de
/// cada una de sus opciones (la página de cada proveedor de DNS).
pub(super) fn del_paso(
    cat: &CatalogoAjustes,
    p: &PasoExpandido,
) -> (Vec<FuenteVista>, Option<String>) {
    let Some(paso) = cat.paso(&p.paso) else {
        return (Vec::new(), None);
    };
    let opciones = p
        .elegir
        .as_deref()
        .and_then(|id| cat.eleccion(id))
        .into_iter()
        .flat_map(|e| &e.opciones)
        .flat_map(|o| &o.fuentes);
    (
        vistas(cat, paso.fuentes.iter().chain(opciones)),
        respaldo(paso.respaldo),
    )
}
