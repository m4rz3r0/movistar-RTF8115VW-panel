//! Pruebas del editor completo en el gestor: qué peticiones se eximen del bloqueo y qué reglas se pueden volver a escribir.

use super::*;

fn req(
    funcion: &str,
    instancia: Option<&str>,
    valores: &[(&str, &str)],
    borrar: bool,
) -> ChangeRequest {
    ChangeRequest {
        funcion: funcion.into(),
        instancia: instancia.map(str::to_owned),
        valores: valores
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        borrar,
        identidad: BTreeMap::new(),
    }
}

const REGLA: [(&str, &str); 6] = [
    ("belong", "0"),
    ("version", "0"),
    ("enabled", "1"),
    ("protocol", "2"),
    ("specificAction", "2"),
    ("dstportstart", "8080"),
];

const FILTRO: [(&str, &str); 5] = [
    ("name", "PRUEBA"),
    ("interface", "0.2"),
    ("type", "0"),
    ("version", "0"),
    ("default", "0"),
];

/// Cada forma de la web se exime; con otra forma (sin instancia donde va,
/// con valores en una baja, un id que no es un número, un campo que no
/// vale o la función de otra cosa), no.
#[test]
fn exento_solo_con_la_forma_de_la_web() {
    assert!(exento(&req(ALTA_REGLA, None, &REGLA, false)));
    assert!(exento(&req(MODIFICAR, Some("3"), &REGLA, false)));
    assert!(exento(&req(ALTA_REGLA, Some("3"), &[], true)));
    assert!(exento(&req(ALTA_FILTRO, None, &FILTRO, false)));
    assert!(exento(&req(MODIFICAR_FILTRO, Some("2"), &FILTRO, false)));
    assert!(exento(&req(BAJA_FILTRO, Some("2"), &[], true)));
    assert!(exento(&req(ALTA_FILTRO, Some("2"), &[], true)));

    assert!(!exento(&req(MODIFICAR, None, &REGLA, false)));
    assert!(!exento(&req(ALTA_REGLA, Some("3"), &REGLA, true)));
    assert!(!exento(&req(ALTA_REGLA, Some("0|3"), &[], true)));
    assert!(!exento(&req(BAJA_FILTRO, Some("2"), &[], false)));
    let mut icmp_con_puertos = REGLA;
    icmp_con_puertos[3] = ("protocol", "1");
    assert!(!exento(&req(ALTA_REGLA, None, &icmp_con_puertos, false)));
    let mut con_target = REGLA.to_vec();
    con_target.push(("target", "rule"));
    assert!(!exento(&req(ALTA_REGLA, None, &con_target, false)));
    assert!(!exento(&req(ALTA_FILTRO, None, &REGLA, false)));
    assert!(!exento(&req(MODIFICAR_FILTRO, Some("2"), &REGLA, false)));
    assert!(!exento(&req(BAJA, Some("2"), &[], true)));
}
