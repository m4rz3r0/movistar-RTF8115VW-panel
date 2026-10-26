//! Pruebas de la identidad de una instancia al planificar.

use super::identidad_cambiada;
use std::collections::BTreeMap;

fn mapa(pares: &[(&str, &str)]) -> BTreeMap<String, String> {
    pares
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// La misma instancia (aunque cambie el formato de su MAC) pasa; otra
/// en su id, o sin el dato, no.
#[test]
fn identidad_de_la_instancia() {
    let id = mapa(&[("mac_addr", "02:00:00:00:00:0A")]);
    assert_eq!(
        identidad_cambiada(&id, &mapa(&[("mac_addr", " 02:00:00:00:00:0a")])),
        None
    );
    assert_eq!(
        identidad_cambiada(&id, &mapa(&[("mac_addr", "02:00:00:00:00:0b")])),
        Some("mac_addr")
    );
    assert_eq!(identidad_cambiada(&id, &BTreeMap::new()), Some("mac_addr"));
    assert_eq!(identidad_cambiada(&BTreeMap::new(), &BTreeMap::new()), None);
}
