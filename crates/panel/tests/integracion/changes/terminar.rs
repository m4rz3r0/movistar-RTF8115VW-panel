//! «Todo bien, terminar»: dar por buena la vigilancia antes de tiempo.

use super::*;
use crate::comun::{
    ORIGIN, PanelPrueba, body_text, esperar_hasta, esperar_libre, llamar, login_body, ok,
    post_form, post_json, send, set_cookie,
};
use serde_json::{Value, json};

/// Vigilancia larga (60 s), mirando cada `intervalo`, y que revierte tras
/// `fallos` comprobaciones fallidas seguidas.
fn tiempos(intervalo: Duration, fallos: u32) -> Tiempos {
    Tiempos {
        bajo: Duration::from_secs(60),
        medio: Duration::from_secs(60),
        alto: Duration::from_secs(60),
        intervalo,
        fallos_para_revertir: fallos,
        pausa_max: Duration::from_secs(2),
    }
}

/// Cambia la IP de la reserva 0 por la Biblioteca (se vigila: toca el
/// DHCP) y devuelve su id, ya vigilándose.
async fn aplicar_reserva(p: &Panel, cookie: &str) -> i64 {
    let peticion = json!({ "funcion": LEASES, "instancia": "0",
        "valores": [[POOL, "0"], ["ipv4_addr", "192.168.1.80"]], "borrar": false });
    let prep = ok(
        p,
        "preparar_cambio",
        json!({ "peticion": peticion }),
        cookie,
    )
    .await;
    assert!(prep["vigilancia_s"].as_u64().unwrap() > 0, "{prep}");
    let id = ok(
        p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap();
    let c = estado(p, cookie, id).await;
    assert_eq!(c["estado"], "Vigilando", "{c}");
    id
}

async fn estado(p: &Panel, cookie: &str, id: i64) -> Value {
    ok(p, "estado_cambio", json!({ "id": id }), cookie).await
}

async fn terminar(p: &Panel, cookie: &str, id: i64) -> (u16, String) {
    llamar(p, "terminar_vigilancia", json!({ "id": id }), Some(cookie)).await
}

/// Otro usuario del panel con su sesión.
async fn sesion_de_otro(p: &Panel) -> String {
    panel::auth::create_user(&p.state.db, "otra", "otra contraseña larga")
        .await
        .unwrap();
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body("otra", "otra contraseña larga"),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 200);
    set_cookie(&resp)
        .split(';')
        .next()
        .unwrap()
        .trim()
        .to_owned()
}

/// Quien hizo el cambio lo da por bueno: queda aplicado al momento, el
/// Historial dice quién terminó la vigilancia, el gestor queda libre y se
/// puede deshacer.
#[tokio::test]
async fn terminar_deja_aplicado_y_lo_anota() {
    let (p, cookie) = PanelPrueba::nuevo()
        .tiempos(tiempos(Duration::from_millis(30), 3))
        .arrancar_con_gestor()
        .await;
    let id = aplicar_reserva(&p, &cookie).await;
    assert_eq!(estado(&p, &cookie, id).await["terminable"], true);
    let (status, body) = terminar(&p, &cookie, id).await;
    assert_eq!(status, 200, "{body}");
    let c: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(c["estado"], "Aplicado", "{c}");
    assert_eq!(c["terminable"], false);
    let nota = c["nota"].as_str().unwrap_or_default();
    assert!(
        nota.contains("admin dio la vigilancia por terminada"),
        "{nota}"
    );
    esperar_libre(&p).await;
    let m = p.state.cambios().unwrap();
    assert!(m.se_puede_deshacer(id).await);
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
    // Ya no se vigila: otra vez, 409.
    let (status, body) = terminar(&p, &cookie, id).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("ya no se está vigilando"), "{body}");
}

/// Solo quien hizo el cambio: otro usuario del panel recibe 403 (y no ve
/// el botón), y la vigilancia sigue.
#[tokio::test]
async fn otro_usuario_no_lo_termina() {
    let (p, cookie) = PanelPrueba::nuevo()
        .tiempos(tiempos(Duration::from_millis(30), 3))
        .arrancar_con_gestor()
        .await;
    let id = aplicar_reserva(&p, &cookie).await;
    let otra = sesion_de_otro(&p).await;
    assert_eq!(estado(&p, &otra, id).await["terminable"], false);
    let (status, body) = terminar(&p, &otra, id).await;
    assert_eq!(status, 403, "{body}");
    assert_eq!(estado(&p, &cookie, id).await["estado"], "Vigilando");
    p.state.cambios().unwrap().detener();
}

/// Sin sesión, 401; desde otro origen, el freno de CSRF (403). Nada cambia.
#[tokio::test]
async fn exige_sesion_y_mismo_origen() {
    let (p, cookie) = PanelPrueba::nuevo()
        .tiempos(tiempos(Duration::from_millis(30), 3))
        .arrancar_con_gestor()
        .await;
    let id = aplicar_reserva(&p, &cookie).await;
    let (status, _) = llamar(&p, "terminar_vigilancia", json!({ "id": id }), None).await;
    assert_eq!(status, 401);
    let mut req = post_json(
        "/api/terminar_vigilancia",
        &json!({ "id": id }).to_string(),
        "127.0.0.1",
    );
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    req.headers_mut().remove("sec-fetch-site");
    req.headers_mut()
        .insert("origin", "https://otro.example".parse().unwrap());
    let resp = send(&p.app, req).await;
    let status = resp.status().as_u16();
    assert_eq!(status, 403, "{}", body_text(resp).await);
    assert_ne!(ORIGIN, "https://otro.example");
    assert_eq!(estado(&p, &cookie, id).await["estado"], "Vigilando");
    p.state.cambios().unwrap().detener();
}

/// Lo que no se vigila (ya aplicado, o la vuelta de un reinicio) no se
/// termina: 409.
#[tokio::test]
async fn no_se_termina_lo_que_no_se_vigila() {
    let (p, cookie) = PanelPrueba::nuevo()
        .tiempos(tiempos(Duration::from_millis(30), 3))
        .arrancar_con_gestor()
        .await;
    let peticion = json!({ "funcion": "cbSetUpnpCfg", "instancia": null,
                           "valores": [["adm_state", "1"]], "borrar": false });
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": peticion }),
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
    assert_eq!(estado(&p, &cookie, id).await["terminable"], false);
    let (status, body) = terminar(&p, &cookie, id).await;
    assert_eq!(status, 409, "{body}");
    let (status, _) = terminar(&p, &cookie, 9999).await;
    assert_eq!(status, 409);
}

/// Con el router fallando (internet caída en la última comprobación), no
/// se termina: podría quedarse puesto justo lo que la tumbó. Cuando vuelve,
/// sí.
#[tokio::test]
async fn no_se_termina_mientras_falla() {
    let (p, cookie) = PanelPrueba::nuevo()
        .tiempos(tiempos(Duration::from_millis(100), 1000))
        .arrancar_con_gestor()
        .await;
    let id = aplicar_reserva(&p, &cookie).await;
    p.sim.control.set_wan_up(false);
    // Unas cuantas comprobaciones (cada 100 ms) con internet caída.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (status, body) = terminar(&p, &cookie, id).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("última comprobación"), "{body}");
    assert_eq!(estado(&p, &cookie, id).await["estado"], "Vigilando");
    p.sim.control.set_wan_up(true);
    esperar_hasta(Duration::from_secs(5), "no se recupera", async || {
        terminar(&p, &cookie, id).await.0 == 200
    })
    .await;
    assert_eq!(estado(&p, &cookie, id).await["estado"], "Aplicado");
}
