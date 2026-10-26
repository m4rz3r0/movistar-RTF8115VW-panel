//! El modelo del catálogo frente a las fixtures del router real: una
//! escritura cuya lectura pareada es una lista tiene que elegir la
//! instancia por su id.

use super::*;
use catalogo::{Catalogo, Tipo};

/// Instancias (hijos con hijos) de la fixture de `lectura` sin parámetros;
/// vacío si no hay fixture.
fn instancias(lectura: &str) -> Vec<XmlNode> {
    let ruta = fixtures_dir().join(format!("{lectura}.xml"));
    let Ok(raw) = std::fs::read_to_string(&ruta) else {
        return Vec::new();
    };
    let n = hgu::xml::parse_response(lectura, &raw)
        .unwrap_or_else(|e| panic!("{}: {e}", ruta.display()));
    n.children
        .into_iter()
        .filter(|c| !c.children.is_empty())
        .collect()
}

/// ¿Es `i` una instancia de una lista del router: trae su posición
/// (`X_0i`) y su id (`X_id`)?
fn de_lista(i: &XmlNode) -> bool {
    i.children.iter().any(|c| {
        c.tag
            .strip_suffix("_0i")
            .is_some_and(|base| i.text_of(&format!("{base}_id")).is_some())
    })
}

/// Una escritura sin `lista` ni `{id}` en su lectura pareada se compara con
/// la primera instancia que lea: si la lectura da varias (las 8 colas de
/// `cbGetQueue`), el formulario enseña los valores de otra y la escritura
/// no dice cuál cambia.
#[test]
fn las_escrituras_de_una_lista_eligen_la_instancia_por_su_id() {
    let cat = Catalogo::load().expect("el catálogo carga");
    let mut fallos = Vec::new();
    for f in cat.todas() {
        if f.tipo == Tipo::Lectura || f.lista.is_some() {
            continue;
        }
        let Some(lp) = &f.lectura_pareada else {
            continue;
        };
        if lp.params.iter().any(|(_, v)| v.starts_with('{')) {
            continue;
        }
        let inst = instancias(&lp.funcion);
        if inst.len() > 1 && inst.iter().all(de_lista) {
            fallos.push(format!(
                "{}: {} es una lista de {} y la lectura pareada no elige la instancia por su id",
                f.name,
                lp.funcion,
                inst.len()
            ));
        }
    }
    assert!(fallos.is_empty(), "{fallos:#?}");
}

/// Escribir una cola cambia esa y no las demás, y su lectura pareada (por
/// el id) la encuentra.
#[tokio::test]
async fn una_cola_por_su_id() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.write(
        "queue.asp",
        "cbSetQueue",
        &[("dpm_cfg_queue_id", "5"), ("default_traffic_class", "2")],
    )
    .await
    .unwrap();
    let clase = async |id: &str| {
        let n = c
            .call_por_id(
                "cbGetQueue",
                &[],
                "dpm_cfg_queue_0i",
                &[("dpm_cfg_queue_id", id)],
            )
            .await
            .unwrap();
        let colas = n.all("dpm_cfg_queue");
        assert_eq!(colas.len(), 1, "{n:?}");
        colas[0]
            .text_of("default_traffic_class")
            .unwrap()
            .to_owned()
    };
    assert_eq!(clase("5").await, "2");
    assert_eq!(clase("0").await, "4");
    assert_eq!(clase("7").await, "4");
}
