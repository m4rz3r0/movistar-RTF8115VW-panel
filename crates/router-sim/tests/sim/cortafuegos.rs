//! Cortafuegos: la página de la lista, el alta y la baja de una regla y la baja de un filtro entero.

use super::*;
use hgu::cortafuegos::{
    ALTA, BAJA, PAGINA_EDITOR, PAGINA_LISTA, carga_sin_ping, es_sin_ping, item, leer,
};

#[tokio::test]
async fn alta_y_baja_de_una_regla() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let filtros = leer(&c).await.unwrap();
    assert_eq!(filtros, sim.control.cortafuegos());
    assert_eq!(filtros[0].reglas.len(), 10);
    let carga = carga_sin_ping(&filtros[0]);
    let p: Vec<(&str, &str)> = carga.iter().map(|(k, v)| (*k, v.as_str())).collect();
    c.basic_write(PAGINA_EDITOR, ALTA, &p).await.unwrap();
    let f = leer(&c).await.unwrap();
    assert_eq!(f[0].reglas.len(), 11);
    let nueva = f[0].reglas.last().unwrap();
    assert_eq!(nueva.rid, "16");
    assert!(es_sin_ping(nueva), "{nueva:?}");
    let it = item("0", "16");
    c.basic_write(PAGINA_LISTA, BAJA, &[("item", &it), ("escape", "true")])
        .await
        .unwrap();
    assert_eq!(leer(&c).await.unwrap()[0].reglas.len(), 10);
    // Las dos escrituras, por la UI básica con su acción.
    let w = sim.control.writes();
    assert_eq!(w.len(), 2, "{w:?}");
    assert!(w.iter().all(|(f, _)| f == "scbidirectionalflt.cmd"));
    assert!(
        w[1].1.contains(&("item".to_owned(), "0|16".to_owned())),
        "{w:?}"
    );
}

#[tokio::test]
async fn baja_sin_regla_borra_el_filtro_como_el_router() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.basic_write(PAGINA_LISTA, BAJA, &[("item", "0"), ("escape", "true")])
        .await
        .unwrap();
    let f = sim.control.cortafuegos();
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].fid, "1");
}

#[tokio::test]
async fn cortafuegos_ilegible_sin_var_filter() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    sim.control.cortafuegos_ilegible(true);
    assert!(leer(&c).await.is_err());
    sim.control.cortafuegos_ilegible(false);
    assert_eq!(leer(&c).await.unwrap().len(), 2);
}
