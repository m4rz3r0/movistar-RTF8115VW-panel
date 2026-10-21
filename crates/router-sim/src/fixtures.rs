//! Carga de las fixtures y sus claves de búsqueda.

use crate::estado::{FixtureKey, Params};
use crate::sanitize;
use std::{collections::HashMap, path::Path as FsPath};

/// Clave de búsqueda: nombre y parámetros ordenados, sin `sessionKey`.
pub(crate) fn fixture_key(name: &str, params: &[(String, String)]) -> FixtureKey {
    let mut p: Params = params
        .iter()
        .filter(|(k, _)| k != "sessionKey")
        .cloned()
        .collect();
    p.sort();
    (name.to_owned(), p)
}

pub(crate) fn load_fixtures(dir: &FsPath) -> std::io::Result<HashMap<FixtureKey, String>> {
    let mut map = HashMap::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(stem) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".xml"))
        else {
            continue;
        };
        let (name, params) = sanitize::split_fixture_name(stem);
        map.insert(fixture_key(name, &params), std::fs::read_to_string(&path)?);
    }
    Ok(map)
}

/// Conexión a internet de las fixtures: la de la ruta por defecto
/// (`cbGetRoutingV4Rule` con `all=ALL` y `desc=Default`) o, sin ella, la
/// única de datos con IPv4 de internet (la misma regla que el panel).
pub(crate) fn wan_de_las_fixtures(fixtures: &HashMap<FixtureKey, String>) -> Option<String> {
    let leer = |name: &str, params: &[(&str, &str)]| {
        let params: Params = params
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        let raw = fixtures.get(&fixture_key(name, &params))?;
        hgu::xml::parse_response(name, raw).ok()
    };
    let conns = hgu::models::WanConn::parse_all(&leer("cbGetWanIPIntf", &[])?);
    let ruta = leer("cbGetRoutingV4Rule", &[("all", "ALL"), ("desc", "Default")])
        .and_then(|n| hgu::read::regla_por_defecto(&n));
    hgu::deteccion::elegir_wan(&conns, ruta.as_deref()).map(|w| w.name.clone())
}
