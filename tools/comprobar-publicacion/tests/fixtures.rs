//! Las fixtures del router no revelan la red del dueño: con las excepciones
//! revisadas de `tools/publicacion-permitidos.txt`, ninguna dirección IPv4,
//! MAC ni dominio sale como hallazgo.

use comprobar_publicacion::{ListasLocales, Permitidos, Regla, revisar_texto};
use std::{fs, path::Path};

#[test]
fn fixtures_sin_topologia() {
    let raiz = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let permitidos = Permitidos::leer(
        &fs::read_to_string(raiz.join("tools/publicacion-permitidos.txt")).unwrap(),
    )
    .unwrap();
    let mut nombres: Vec<String> = fs::read_dir(raiz.join("crates/hgu/fixtures"))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".xml"))
        .collect();
    nombres.sort();
    assert!(!nombres.is_empty(), "no hay fixtures");

    let mut hallazgos = Vec::new();
    for nombre in &nombres {
        let ruta = format!("crates/hgu/fixtures/{nombre}");
        let texto = fs::read_to_string(raiz.join(&ruta)).unwrap();
        hallazgos.extend(
            revisar_texto(&ruta, &texto, &permitidos, &ListasLocales::vacia())
                .into_iter()
                .filter(|h| matches!(h.regla, Regla::Ipv4 | Regla::Mac | Regla::Dominio)),
        );
    }
    let lista: Vec<String> = hallazgos.iter().map(ToString::to_string).collect();
    assert!(
        lista.is_empty(),
        "fixtures con topología:\n{}",
        lista.join("\n")
    );
}
