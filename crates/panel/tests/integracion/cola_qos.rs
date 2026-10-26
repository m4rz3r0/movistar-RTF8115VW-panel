//! `cbSetQueue` en la Biblioteca: `cbGetQueue` es una lista de 8 colas
//! (ids 0 a 7) y el formulario y la escritura van siempre por el id de la
//! cola, nunca por la primera que se lea.

use crate::comun::{Panel, PanelPrueba, campo, esperar_final, fijar, llamar, ok, valores_de};
use serde_json::{Value, json};

const COLA: &str = "dpm_cfg_queue_id";

async fn formulario(p: &Panel, cookie: &str, id: &str) -> Value {
    ok(
        p,
        "formulario",
        json!({ "funcion": "cbSetQueue", "instancia": null, "claves": [[COLA, id]] }),
        cookie,
    )
    .await
}

#[tokio::test]
async fn cada_cola_por_su_id() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    // Sin el id no hay formulario: no se sabe qué cola es.
    let (status, body) = llamar(
        &p,
        "formulario",
        json!({ "funcion": "cbSetQueue", "instancia": null, "claves": [] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");

    let f = formulario(&p, &cookie, "5").await;
    assert_eq!(f["operacion"], "Editar");
    assert_eq!(campo(&f, "default_traffic_class").unwrap()["valor"], "4");
    let mut valores = valores_de(&f);
    fijar(&mut valores, "default_traffic_class", "2");
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetQueue", "instancia": null,
                              "valores": valores, "borrar": false } }),
        &cookie,
    )
    .await;
    let id = ok(
        &p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    let c = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");

    // Se envió con el id de la cola y solo cambió esa.
    let writes = p.sim.control.writes();
    assert_eq!(writes.len(), 1, "{writes:?}");
    assert!(
        writes[0].1.iter().any(|(k, v)| k == COLA && v == "5"),
        "{writes:?}"
    );
    let clase = |f: &Value| campo(f, "default_traffic_class").unwrap()["valor"].clone();
    assert_eq!(clase(&formulario(&p, &cookie, "5").await), "2");
    assert_eq!(clase(&formulario(&p, &cookie, "0").await), "4");
    assert_eq!(clase(&formulario(&p, &cookie, "7").await), "4");
}
