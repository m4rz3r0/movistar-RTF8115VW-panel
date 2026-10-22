//! Expansión de cada paso de un lote en los campos que cambia.

use crate::carga::CatalogoAjustes;
use crate::elementos::partir_referencia;
use crate::lotes::{Destino, Lote, PasoExpandido, PasoLote, clave_lote, clave_paso, titulo_lote};
use crate::modelo::{Accion, Ajuste, Campo, Elemento, Tema};

/// Papeles de los elementos que recorre `para_cada = "red_wifi"`.
const PAPELES_DE_RED: [&str; 2] = ["principal", "invitados"];

/// Elementos que recorre `para_cada = "red_wifi"`: las redes WiFi de casa y
/// de invitados, no las radios.
pub(super) fn redes_wifi(c: &CatalogoAjustes) -> impl Iterator<Item = &Elemento> {
    c.elementos_de(Tema::Wifi).filter(|e| {
        e.papel
            .as_deref()
            .is_some_and(|p| PAPELES_DE_RED.contains(&p))
    })
}

/// Ajustes de `e` que tienen el campo `campo`, con el campo.
pub(super) fn campos_del_elemento<'c>(
    c: &'c CatalogoAjustes,
    e: &'c Elemento,
    campo: &'c str,
) -> impl Iterator<Item = (&'c Ajuste, &'c Campo)> {
    e.ajustes
        .iter()
        .filter_map(|id| c.ajuste(id))
        .filter_map(move |a| a.campo(campo).map(|x| (a, x)))
}

/// Acción `id` del tema `tema`.
pub(super) fn accion<'c>(c: &'c CatalogoAjustes, tema: Tema, id: &str) -> Option<&'c Accion> {
    c.acciones_de(tema).find(|a| a.id == id)
}

/// `(campo, valor)` que escribe el paso: los de su acción (si no es de una
/// lista) o los suyos. `None` si escribe por una elección o está mal.
fn campo_y_valor<'c>(c: &'c CatalogoAjustes, p: &'c PasoLote) -> Option<(&'c str, &'c str)> {
    match (&p.accion, &p.campo) {
        (Some(id), None) => {
            let a = accion(c, p.tema, id).filter(|a| a.lista.is_none() && !a.borrar)?;
            Some((a.campo.as_deref()?, a.valor.as_deref()?))
        }
        (None, Some(campo)) => Some((campo, p.valor.as_deref()?)),
        _ => None,
    }
}

fn destino(a: &Ajuste, campo: &str, valor: Option<&str>) -> Destino {
    Destino {
        ajuste: a.id.clone(),
        campo: campo.to_owned(),
        valor: valor.map(str::to_owned),
    }
}

/// Lo que cambia un paso:
/// - con `para_cada = "red_wifi"`, un paso por red, con el ajuste de esa
///   red que tiene el campo y se puede escribir;
/// - con `accion` y sin `para_cada`, un paso con el campo de cada elemento
///   de su tema que lo tiene y se puede escribir;
/// - con `campo`, un paso con ese campo;
/// - con `elegir`, un paso con cada campo que dan las opciones, sin valor.
///
/// Un paso mal formado (lo detecta [`crate::comprobar`]) no cambia nada.
pub fn expandir(c: &CatalogoAjustes, p: &PasoLote) -> Vec<PasoExpandido> {
    let paso = |elemento: Option<&str>, destinos: Vec<Destino>| PasoExpandido {
        paso: p.id.clone(),
        tema: p.tema,
        elemento: elemento.map(str::to_owned),
        destinos,
        si: p.si.clone(),
        opcional: p.opcional,
        elegir: p.elegir.clone(),
    };
    if let Some(e) = &p.elegir {
        let (Some(a), Some(el)) = (p.ajuste.as_deref().and_then(|a| c.ajuste(a)), c.eleccion(e))
        else {
            return Vec::new();
        };
        let destinos = a
            .campos
            .iter()
            .filter(|x| el.opciones.iter().any(|o| o.valores.contains_key(&x.id)))
            .map(|x| destino(a, &x.id, None))
            .collect();
        return vec![paso(None, destinos)];
    }
    let Some((campo, valor)) = campo_y_valor(c, p) else {
        return Vec::new();
    };
    if p.para_cada.is_some() {
        return redes_wifi(c)
            .filter_map(|e| {
                let (a, _) =
                    campos_del_elemento(c, e, campo).find(|(_, x)| x.escribible().is_some())?;
                Some(paso(Some(&e.id), vec![destino(a, campo, Some(valor))]))
            })
            .collect();
    }
    if p.accion.is_some() {
        let destinos = c
            .elementos_de(p.tema)
            .flat_map(|e| campos_del_elemento(c, e, campo))
            .filter(|(_, x)| x.escribible().is_some())
            .map(|(a, _)| destino(a, campo, Some(valor)))
            .collect();
        return vec![paso(None, destinos)];
    }
    match partir_referencia(campo) {
        Some((a, x)) => {
            let destinos = vec![Destino {
                ajuste: a.to_owned(),
                campo: x.to_owned(),
                valor: Some(valor.to_owned()),
            }];
            vec![paso(None, destinos)]
        }
        None => Vec::new(),
    }
}

/// Pasos expandidos de un lote, en su orden.
pub fn pasos_del_lote(c: &CatalogoAjustes, l: &Lote) -> Vec<PasoExpandido> {
    l.pasos
        .iter()
        .filter_map(|id| c.paso(id))
        .flat_map(|p| expandir(c, p))
        .collect()
}

/// Ids de los lotes que cambian el campo `campo` del ajuste `ajuste`, en el
/// orden del catálogo (para avisar junto al campo en Ajustes).
pub fn lotes_del_campo<'c>(c: &'c CatalogoAjustes, ajuste: &str, campo: &str) -> Vec<&'c str> {
    c.lotes()
        .iter()
        .filter(|l| {
            pasos_del_lote(c, l)
                .iter()
                .flat_map(|p| &p.destinos)
                .any(|d| d.ajuste == ajuste && d.campo == campo)
        })
        .map(|l| l.id.as_str())
        .collect()
}

/// Claves de traducción que piden los lotes, sin repetir: nombre, `-desc` y
/// `-titulo` de cada lote, nombre y `-desc` de cada paso,
/// `mejoras-aviso-<id>` y `mejoras-consejo-<id>`.
pub fn claves_de_lotes(c: &CatalogoAjustes) -> Vec<String> {
    let mut claves = Vec::new();
    for l in c.lotes() {
        let k = clave_lote(&l.id);
        claves.push(format!("{k}-desc"));
        claves.push(k);
        claves.push(titulo_lote(&l.id));
        claves.extend(l.avisos.iter().map(|a| format!("mejoras-aviso-{a}")));
        claves.extend(l.consejos.iter().map(|x| format!("mejoras-consejo-{x}")));
        for p in &l.pasos {
            let k = clave_paso(p);
            claves.push(format!("{k}-desc"));
            claves.push(k);
        }
    }
    claves.sort();
    claves.dedup();
    claves
}
