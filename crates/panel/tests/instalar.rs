//! Pruebas del asistente de primera instalación: token de un solo uso,
//! bloqueo de todo lo demás sin usuarios, foto de partida cifrada y modo solo
//! lectura con un firmware no probado.

mod common;

use axum::{body::Body, http::Request};
use common::{
    PANEL_PASS, PANEL_USER, PASS, Panel, USER, body_text, get, get_con_cookie, panel_sin_router,
    post_json, send, set_cookie,
};
use panel::{
    changes::{CambioError, ChangeManager, ChangeRequest, Tiempos},
    fotos::{self, TipoFoto},
    instalar,
    instancia::{self, claves},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};

/// SSID de una de las redes de las fixtures.
const SSID: &str = "RED-1";

/// Fichero del token: junto a la base de datos.
fn fichero_token(p: &Panel) -> PathBuf {
    p.state.cfg.db_path.parent().unwrap().join("instalar.token")
}

/// Panel recién instalado (sin usuarios ni router) con el token generado al
/// arrancar.
async fn recien_instalado() -> Panel {
    let p = panel_sin_router("127.0.0.1".parse().unwrap()).await;
    assert!(instalar::token_inicial(&p.state).await.is_some());
    p
}

/// Llama a una server function desde `peer`, con cookie opcional.
async fn llamar_desde(
    p: &Panel,
    nombre: &str,
    cuerpo: Value,
    cookie: Option<&str>,
    peer: &str,
) -> (u16, String, Option<String>) {
    let mut req = post_json(&format!("/api/{nombre}"), &cuerpo.to_string(), peer);
    if let Some(c) = cookie {
        req.headers_mut().insert("cookie", c.parse().unwrap());
    }
    let resp = send(&p.app, req).await;
    let status = resp.status().as_u16();
    let cookie = resp.headers().get("set-cookie").map(|_| {
        set_cookie(&resp)
            .split(';')
            .next()
            .unwrap()
            .trim()
            .to_owned()
    });
    (status, body_text(resp).await, cookie)
}

async fn llamar(p: &Panel, nombre: &str, cuerpo: Value, cookie: Option<&str>) -> (u16, String) {
    let (s, b, _) = llamar_desde(p, nombre, cuerpo, cookie, "127.0.0.1").await;
    (s, b)
}

async fn usuarios(p: &Panel) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&p.state.db)
        .await
        .unwrap()
}

/// `crear_admin` con el token del fichero; devuelve la cookie de sesión.
async fn crear_admin(p: &Panel) -> String {
    let token = std::fs::read_to_string(fichero_token(p)).unwrap();
    let (status, body, cookie) = llamar_desde(
        p,
        "crear_admin",
        json!({ "token": token.trim(), "usuario": PANEL_USER, "contrasena": PANEL_PASS }),
        None,
        "127.0.0.1",
    )
    .await;
    assert_eq!(status, 200, "{body}");
    cookie.expect("crear_admin abre la sesión")
}

fn router(p: &Panel) -> Value {
    json!({ "token": "", "url": p.sim.base_url, "usuario": USER, "contrasena": PASS })
}

fn json_de(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|e| panic!("{e}: {body}"))
}

#[tokio::test]
async fn sin_usuarios_redirige_a_instalar() {
    let p = recien_instalado().await;
    for ruta in ["/", "/login", "/historial", "/explorador"] {
        let resp = get(&p.app, ruta).await;
        assert_eq!(resp.status(), 303, "{ruta}");
        assert_eq!(resp.headers()["location"], "/instalar", "{ruta}");
    }
    assert_eq!(get(&p.app, "/instalar").await.status(), 200);
    assert_eq!(get(&p.app, "/salud").await.status(), 200);
    assert_eq!(get(&p.app, "/pkg/panel.css").await.status(), 200);
}

#[tokio::test]
async fn instalar_sin_token_403() {
    let p = recien_instalado().await;
    let admin =
        |token: Value| json!({ "token": token, "usuario": PANEL_USER, "contrasena": PANEL_PASS });
    for token in [json!("malo"), json!("")] {
        let (status, body) = llamar(&p, "crear_admin", admin(token.clone()), None).await;
        assert_eq!(status, 403, "{token}: {body}");
    }
    // Sin el campo: tampoco.
    let (status, _) = llamar(
        &p,
        "crear_admin",
        json!({ "usuario": PANEL_USER, "contrasena": PANEL_PASS }),
        None,
    )
    .await;
    assert_eq!(status, 403);
    // El resto de pasos, igual.
    let (status, _) = llamar(&p, "probar_router", router(&p), None).await;
    assert_eq!(status, 403);
    let (status, _) = llamar(&p, "guardar_router", router(&p), None).await;
    assert_eq!(status, 403);
    let (status, _) = llamar(&p, "lectura_inicial", json!({ "token": "malo" }), None).await;
    assert_eq!(status, 403);
    // Con el token bueno, pero desde fuera de la red local.
    let token = std::fs::read_to_string(fichero_token(&p)).unwrap();
    let (status, _, _) = llamar_desde(
        &p,
        "crear_admin",
        admin(json!(token.trim())),
        None,
        "203.0.113.9",
    )
    .await;
    assert_eq!(status, 403);
    assert_eq!(usuarios(&p).await, 0);
    assert!(!p.state.hgu.configurado());
}

#[tokio::test]
async fn con_usuarios_no_hay_asistente() {
    let p = panel_sin_router("127.0.0.1".parse().unwrap()).await;
    // Un token que quedó de otra vez.
    std::fs::write(fichero_token(&p), "viejo\n").unwrap();
    panel::auth::create_user(&p.state.db, PANEL_USER, PANEL_PASS)
        .await
        .unwrap();
    assert_eq!(instalar::token_inicial(&p.state).await, None);
    assert!(!fichero_token(&p).exists(), "el fichero del token sobra");
    assert_eq!(get(&p.app, "/instalar").await.status(), 404);
    let (status, _) = llamar(
        &p,
        "crear_admin",
        json!({ "token": "viejo", "usuario": "otro", "contrasena": PANEL_PASS }),
        None,
    )
    .await;
    assert_eq!(status, 404);
    // Con sesión tampoco: la instalación no está en curso.
    let cookie = common::sesion_de(&p).await;
    assert_eq!(
        get_con_cookie(&p.app, "/instalar", &cookie).await.status(),
        404
    );
    let (status, _) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 404);
    assert_eq!(usuarios(&p).await, 1);
    // Y las páginas piden sesión como siempre.
    let resp = get(&p.app, "/").await;
    assert_eq!(resp.status(), 303);
    assert_eq!(resp.headers()["location"], "/login?next=%2F");
}

#[tokio::test]
async fn flujo_completo_con_sim() {
    let p = recien_instalado().await;
    let cookie = crear_admin(&p).await;
    assert!(!fichero_token(&p).exists(), "el token es de un solo uso");
    assert_eq!(usuarios(&p).await, 1);
    // El token ya no vale para otro administrador.
    let (status, _) = llamar(
        &p,
        "crear_admin",
        json!({ "token": "", "usuario": "otro", "contrasena": PANEL_PASS }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 404);
    // Sin sesión, el resto del asistente no responde.
    let (status, _) = llamar(&p, "probar_router", router(&p), None).await;
    assert_eq!(status, 401);

    let (status, body) = llamar(&p, "probar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    let info = json_de(&body);
    assert_eq!(info["modelo"], "RTF8115VW");
    assert_eq!(info["firmware"], "ES_g14.17_RTF_TEF001_V9.17_V034");
    assert_eq!(info["probado"], true);
    assert!(!p.state.hgu.configurado(), "probar no guarda nada");

    let mut malo = router(&p);
    malo["contrasena"] = json!("otra");
    let (status, _) = llamar(&p, "guardar_router", malo, Some(&cookie)).await;
    assert_eq!(status, 502, "no guarda credenciales que no entran");
    assert!(!p.state.hgu.configurado());

    let (status, body) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert!(p.state.hgu.configurado());
    let guardadas = instancia::leer_credenciales(&p.state.db, &p.state.history_key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(guardadas.url, p.sim.base_url);
    assert_ne!(
        instancia::leer(&p.state.db, claves::SOLO_LECTURA).await,
        Some(json!(true))
    );

    let (status, body) = llamar(&p, "lectura_inicial", json!({ "token": "" }), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    let resumen = json_de(&body);
    assert!(
        resumen["funciones_leidas"].as_u64().unwrap() >= 40,
        "{resumen}"
    );
    assert_eq!(resumen["fallidas"], json!([]), "{resumen}");
    assert!(
        fotos::ultima(&p.state.db, &p.state.history_key, TipoFoto::Partida)
            .await
            .is_some()
    );

    // Terminado: sesión abierta, el asistente ya no está.
    assert!(!fichero_token(&p).exists());
    assert_eq!(get_con_cookie(&p.app, "/", &cookie).await.status(), 200);
    assert_eq!(
        get_con_cookie(&p.app, "/instalar", &cookie).await.status(),
        404
    );
    let (status, _) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 404);
}

/// Petición de escritura cualquiera.
fn escritura() -> ChangeRequest {
    ChangeRequest {
        funcion: "cbSetUpnpCfg".into(),
        instancia: None,
        valores: BTreeMap::from([("adm_state".to_owned(), "1".to_owned())]),
        borrar: false,
    }
}

#[tokio::test]
async fn firmware_no_probado_queda_en_solo_lectura() {
    let p = recien_instalado().await;
    p.sim
        .control
        .set_firmware("ES_g99.00_RTF_TEF001_V99.00_V001");
    let cookie = crear_admin(&p).await;
    let (status, body) = llamar(&p, "probar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(json_de(&body)["probado"], false);
    let (status, body) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        instancia::leer(&p.state.db, claves::SOLO_LECTURA).await,
        Some(json!(true))
    );

    let gestor = ChangeManager::instalar(&p.state, Tiempos::default());
    match gestor.preparar(escritura()).await {
        Err(CambioError::Bloqueada(m)) => assert_eq!(m, "panel-solo-lectura"),
        r => panic!("{r:?}"),
    }
    // Desde la interfaz, con el motivo traducido.
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetUpnpCfg", "instancia": null,
            "valores": [["adm_state", "1"]], "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("solo lectura"), "{body}");

    // Confirmado el firmware (lo hará «Ajustes del panel»), se escribe.
    instancia::escribir(&p.state.db, claves::SOLO_LECTURA, &json!(false))
        .await
        .unwrap();
    assert!(gestor.preparar(escritura()).await.is_ok());
}

#[tokio::test]
async fn firmware_confirmado_no_bloquea() {
    let p = recien_instalado().await;
    p.sim
        .control
        .set_firmware("ES_g99.00_RTF_TEF001_V99.00_V001");
    instancia::escribir(&p.state.db, claves::FIRMWARE_CONFIRMADO, &json!(true))
        .await
        .unwrap();
    let cookie = crear_admin(&p).await;
    let (status, body) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert_ne!(
        instancia::leer(&p.state.db, claves::SOLO_LECTURA).await,
        Some(json!(true))
    );
}

#[tokio::test]
async fn foto_de_partida_cifrada() {
    let p = recien_instalado().await;
    let cookie = crear_admin(&p).await;
    let (status, _) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200);
    let (status, body) = llamar(&p, "lectura_inicial", json!({ "token": "" }), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");

    let (tipo, blob): (String, Vec<u8>) = sqlx::query_as("SELECT tipo, contenido FROM fotos")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert_eq!(tipo, "partida");
    assert!(
        !blob.windows(SSID.len()).any(|w| w == SSID.as_bytes()),
        "la foto está en claro"
    );
    let (id, foto) = fotos::ultima(&p.state.db, &p.state.history_key, TipoFoto::Partida)
        .await
        .unwrap();
    assert!(id > 0);
    assert!(
        foto.get("cbListWifiAccPt")
            .is_some_and(|xml| xml.contains(&format!("<ssid>{SSID}</ssid>"))),
        "{:?}",
        foto.keys().collect::<Vec<_>>()
    );
    assert!(foto.contains_key("cbGetSummary"));
    // La foto manual es otra serie.
    assert!(
        fotos::ultima(&p.state.db, &p.state.history_key, TipoFoto::Manual)
            .await
            .is_none()
    );
    let manual = BTreeMap::from([("a".to_owned(), "<a>1</a>".to_owned())]);
    let id2 = fotos::guardar(&p.state.db, &p.state.history_key, TipoFoto::Manual, &manual)
        .await
        .unwrap();
    assert_eq!(
        fotos::ultima(&p.state.db, &p.state.history_key, TipoFoto::Manual).await,
        Some((id2, manual))
    );
}

#[tokio::test]
async fn pagina_del_asistente_traducida_y_sin_cache() {
    let p = recien_instalado().await;
    let resp = send(
        &p.app,
        Request::get("/instalar")
            .header("accept-language", "en")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(resp.status(), 200);
    let html = body_text(resp).await;
    assert!(html.contains("idioma-selector"), "selector de idioma");
    assert!(html.contains("Set up the panel"), "{html}");
    // Las respuestas del asistente no se guardan en caché.
    let req = post_json("/api/probar_router", &router(&p).to_string(), "127.0.0.1");
    let resp = send(&p.app, req).await;
    assert_eq!(resp.headers()["cache-control"], "no-store");
}
