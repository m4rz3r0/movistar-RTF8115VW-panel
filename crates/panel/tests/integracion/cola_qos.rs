//! Las colas de QoS en la Biblioteca: `cbGetQueue` es una lista de 8 colas
//! (ids 0 a 7) y el formulario y la escritura van siempre por el id de la
//! cola, nunca por la primera que se lea. La web usa `cbSetQueue` de dos
//! formas, que son dos funciones del catálogo: el estado de cada cola
//! (`cbSetQueue`) y la cola por defecto (`cbSetQueue.defecto`).

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
}

/// Aplica en la Biblioteca `funcion` con `valores` y espera a que termine.
async fn aplicar(p: &Panel, cookie: &str, funcion: &str, valores: Value, palabra: Value) {
    let prep = ok(
        p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": funcion, "instancia": null,
                              "valores": valores, "borrar": false } }),
        cookie,
    )
    .await;
    let id = ok(
        p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": palabra }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap();
    let c = esperar_final(p, cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
}

/// Activar o desactivar una cola manda, como la web, solo su id y su estado.
#[tokio::test]
async fn activar_una_cola_manda_solo_su_id_y_su_estado() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let mut valores = valores_de(&formulario(&p, &cookie, "5").await);
    fijar(&mut valores, "adm_state", "0");
    aplicar(&p, &cookie, "cbSetQueue", json!(valores), Value::Null).await;

    let writes = p.sim.control.writes();
    assert_eq!(writes.len(), 1, "{writes:?}");
    assert_eq!(writes[0].0, "cbSetQueue");
    let mut enviado = writes[0].1.clone();
    enviado.sort();
    assert_eq!(
        enviado,
        [
            ("adm_state".to_owned(), "0".to_owned()),
            (COLA.to_owned(), "5".to_owned())
        ]
    );
    let estado = |f: &Value| campo(f, "adm_state").unwrap()["valor"].clone();
    assert_eq!(estado(&formulario(&p, &cookie, "5").await), "0");
    assert_eq!(estado(&formulario(&p, &cookie, "4").await), "1");
}

/// Elegir la cola por defecto es otro formulario de la web: manda el id de
/// la cola y `default_traffic_class=-1` a la misma CGI, y el router toma la
/// clase de tráfico de esa cola como la de por defecto de todas.
#[tokio::test]
async fn la_cola_por_defecto_como_la_web() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    aplicar(
        &p,
        &cookie,
        "cbSetQueue.defecto",
        json!([[COLA, "2"]]),
        json!("CONFIRMAR"),
    )
    .await;

    let writes = p.sim.control.writes();
    assert_eq!(writes.len(), 1, "{writes:?}");
    assert_eq!(writes[0].0, "cbSetQueue");
    let mut enviado = writes[0].1.clone();
    enviado.sort();
    assert_eq!(
        enviado,
        [
            ("default_traffic_class".to_owned(), "-1".to_owned()),
            (COLA.to_owned(), "2".to_owned())
        ]
    );
    let clase = |f: &Value| campo(f, "default_traffic_class").map(|c| c["valor"].clone());
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": "cbSetQueue.defecto", "instancia": null, "claves": [] }),
        &cookie,
    )
    .await;
    assert_eq!(clase(&f), None, "el valor fijo no se enseña: {f}");
    for id in ["0", "2", "7"] {
        let n = p
            .state
            .hgu
            .call_por_id("cbGetQueue", &[], "dpm_cfg_queue_0i", &[(COLA, id)])
            .await
            .unwrap();
        let cola = &n.all("dpm_cfg_queue")[0];
        assert_eq!(
            cola.text_of("default_traffic_class"),
            Some("2"),
            "cola {id}"
        );
    }
}
