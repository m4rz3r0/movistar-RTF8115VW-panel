//! La cola por la API: confirmar durante una vigilancia deja el cambio
//! «EnCola» y solo quien lo encoló lo quita (con sesión y desde el mismo
//! origen).

use super::terminar::{aplicar_reserva, estado, sesion_de_otro, tiempos};
use super::*;
use crate::comun::{PanelPrueba, body_text, llamar, ok, post_json, send};
use serde_json::{Value, json};

async fn encolar_upnp(p: &Panel, cookie: &str) -> i64 {
    let peticion = json!({ "funcion": "cbSetUpnpCfg", "instancia": null,
                           "valores": [["adm_state", "1"]], "borrar": false });
    let prep = ok(
        p,
        "preparar_cambio",
        json!({ "peticion": peticion }),
        cookie,
    )
    .await;
    ok(
        p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap()
}

async fn quitar(p: &Panel, cookie: Option<&str>, id: i64) -> (u16, String) {
    llamar(p, "quitar_de_la_cola", json!({ "id": id }), cookie).await
}

#[tokio::test]
async fn en_cola_y_quitar_por_la_api() {
    let (p, cookie) = PanelPrueba::nuevo()
        .tiempos(tiempos(Duration::from_millis(30), 3))
        .arrancar_con_gestor()
        .await;
    let a = aplicar_reserva(&p, &cookie).await;
    let b = encolar_upnp(&p, &cookie).await;
    let c = estado(&p, &cookie, b).await;
    assert_eq!(c["estado"], "EnCola", "{c}");
    assert_eq!(c["puesto"], 1, "{c}");
    assert_eq!(c["quitable"], true, "{c}");

    // Otro usuario lo ve, pero no lo quita.
    let otra = sesion_de_otro(&p).await;
    assert_eq!(estado(&p, &otra, b).await["quitable"], false);
    let (status, body) = quitar(&p, Some(&otra), b).await;
    assert_eq!(status, 403, "{body}");
    // Sin sesión, 401; desde otro origen, 403.
    assert_eq!(quitar(&p, None, b).await.0, 401);
    let mut req = post_json(
        "/api/quitar_de_la_cola",
        &json!({ "id": b }).to_string(),
        "127.0.0.1",
    );
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    req.headers_mut().remove("sec-fetch-site");
    req.headers_mut()
        .insert("origin", "https://otro.example".parse().unwrap());
    let resp = send(&p.app, req).await;
    let status = resp.status().as_u16();
    assert_eq!(status, 403, "{}", body_text(resp).await);
    assert_eq!(estado(&p, &cookie, b).await["estado"], "EnCola");

    let (status, body) = quitar(&p, Some(&cookie), b).await;
    assert_eq!(status, 200, "{body}");
    let c: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(c["estado"], "Cancelado", "{c}");
    let error = c["error"].as_str().unwrap_or_default();
    assert!(error.contains("admin lo quitó de la cola"), "{error}");
    let (status, body) = quitar(&p, Some(&cookie), b).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("ya no está esperando"), "{body}");
    // El vigilado sigue a lo suyo.
    assert_eq!(estado(&p, &cookie, a).await["estado"], "Vigilando");
    p.state.cambios().unwrap().detener();
}

/// Con la cola llena, preparar ya avisa (409) con el tope.
#[tokio::test]
async fn cola_llena_por_la_api() {
    let (p, cookie) = PanelPrueba::nuevo()
        .tiempos(tiempos(Duration::from_millis(30), 3))
        .arrancar_con_gestor()
        .await;
    aplicar_reserva(&p, &cookie).await;
    for _ in 0..panel::changes::MAX_COLA {
        encolar_upnp(&p, &cookie).await;
    }
    let peticion = json!({ "funcion": "cbSetUpnpCfg", "instancia": null,
                           "valores": [["adm_state", "1"]], "borrar": false });
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": peticion }),
        &cookie,
    )
    .await;
    let (status, body) = llamar(
        &p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("Ya hay 5 cambios en cola"), "{body}");
    let (status, body) = llamar(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "wifi.red_24", "instancia": null, "borrar": false,
                       "valores": { "nombre": "casa-nueva" } } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("Ya hay 5 cambios en cola"), "{body}");
    p.state.cambios().unwrap().detener();
}
