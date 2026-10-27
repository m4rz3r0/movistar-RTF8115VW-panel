//! Las IP reservadas de un pool DHCP en el formulario de la Biblioteca
//! (parámetros por ranuras, [`catalogo::Parametro::ranuras`]): una pareja de
//! campos por ranura usada y otra vacía para una nueva, que el gestor pone
//! en la ranura que toca, como la web oficial.

use super::{CampoForm, param_vista};
use crate::changes::plan::ranuras::usadas;
use catalogo::{Funcion, Parametro};
use std::collections::BTreeMap;

/// Campos de las ranuras de `f` al editar, con lo que lee `actual`.
pub(super) fn campos(f: &Funcion, actual: &BTreeMap<String, String>) -> Vec<CampoForm> {
    let ps: Vec<&Parametro> = f
        .parametros
        .iter()
        .filter(|p| p.ranuras.is_some())
        .collect();
    let campo = |p: &Parametro, nombre: String, etiqueta: String, valor: String| CampoForm {
        param: param_vista(
            f,
            &Parametro {
                nombre,
                etiqueta,
                ..p.clone()
            },
        ),
        valor,
        sin_leer: false,
    };
    let mut out = Vec::new();
    for i in usadas(f, actual) {
        for (n, p) in ps.iter().enumerate() {
            let nombre = format!("{}_{i}", p.nombre);
            let valor = crate::changes::buscar(actual, &nombre)
                .unwrap_or_default()
                .to_owned();
            let quitar = if n == 0 { " (0.0.0.0 la quita)" } else { "" };
            let etiqueta = format!("{}, ranura {i}{quitar}", p.etiqueta);
            out.push(campo(p, nombre, etiqueta, valor));
        }
    }
    for (n, p) in ps.iter().enumerate() {
        let vacia = if n == 0 {
            " (vacía: ninguna)"
        } else {
            " (vacía: una sola dirección)"
        };
        let etiqueta = format!("{}: nueva{vacia}", p.etiqueta);
        out.push(campo(p, p.nombre.clone(), etiqueta, String::new()));
    }
    out
}
