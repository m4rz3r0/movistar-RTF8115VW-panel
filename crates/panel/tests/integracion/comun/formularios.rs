//! Formularios y temas de ajustes tal como los devuelven las server functions.

use super::{Panel, ok};
use serde_json::{Value, json};

/// Valores de un formulario tal como llegan (más sus claves), para enviarlos
/// sin tocar.
pub fn valores_de(form: &Value) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = form["claves"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c[0].as_str().unwrap().into(), c[1].as_str().unwrap().into()))
        .collect();
    for c in form["campos"].as_array().unwrap() {
        v.push((
            c["param"]["nombre"].as_str().unwrap().into(),
            c["valor"].as_str().unwrap().into(),
        ));
    }
    v
}

pub fn fijar(v: &mut [(String, String)], k: &str, valor: &str) {
    v.iter_mut()
        .find(|(n, _)| n == k)
        .unwrap_or_else(|| panic!("falta {k}"))
        .1 = valor.into();
}

pub fn campo<'a>(form: &'a Value, nombre: &str) -> Option<&'a Value> {
    form["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["param"]["nombre"] == nombre)
}

/// El tema de ajustes `tema` (`leer_tema`).
pub async fn leer(p: &Panel, cookie: &str, tema: &str) -> Value {
    ok(p, "leer_tema", json!({ "tema": tema }), cookie).await
}

/// Los campos que cambia un grupo preparado, de todos sus pasos.
pub fn cambios_del_grupo(grupo: &Value) -> Vec<&Value> {
    grupo["pasos"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["campos"].as_array().unwrap())
        .collect()
}

/// Texto del primer elemento `nombre` de `n` o de sus descendientes.
pub fn elemento(n: &hgu::xml::XmlNode, nombre: &str) -> Option<String> {
    n.text_of(nombre)
        .map(str::to_owned)
        .or_else(|| n.children.iter().find_map(|c| elemento(c, nombre)))
}
