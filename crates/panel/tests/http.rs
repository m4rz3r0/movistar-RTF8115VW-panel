//! Pruebas HTTP del panel en proceso, contra `router-sim`.

mod common;

use axum::{
    body::Body,
    http::{Response, StatusCode},
};
use common::{Panel, body_text, get, get_con_cookie, panel, sesion};
use panel::poller::{Poller, PollerConfig};
use std::time::Duration;

const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'";

async fn json(resp: Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Arranca el sondeo con intervalos cortos y un cliente del router de tiempo
/// de espera corto, y espera a que `/salud` lo vea sano.
async fn con_sondeo(p: &mut Panel) {
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, common::USER, common::PASS);
    cfg.timeout = Duration::from_millis(500);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    let h = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(100),
            ..PollerConfig::default()
        },
    );
    let start = std::time::Instant::now();
    while !h.healthy() {
        assert!(start.elapsed() < Duration::from_secs(5), "sondeo sin datos");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn salud_sin_sondeo_es_error() {
    let p = panel().await;
    let body = json(get(&p.app, "/salud").await).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["router"], "error");
}

#[tokio::test]
async fn salud_ok() {
    let mut p = panel().await;
    con_sondeo(&mut p).await;
    let resp = get(&p.app, "/salud").await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        json(resp).await,
        serde_json::json!({"ok": true, "version": env!("CARGO_PKG_VERSION"), "router": "ok"})
    );
}

#[tokio::test]
async fn salud_responde_aunque_el_router_no() {
    let mut p = panel().await;
    con_sondeo(&mut p).await;
    p.sim.control.set_unresponsive(true);
    // Sin datos nuevos en 3 intervalos (300 ms) deja de estar sano.
    tokio::time::sleep(Duration::from_millis(400)).await;
    let start = std::time::Instant::now();
    let resp = get(&p.app, "/salud").await;
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "{:?}",
        start.elapsed()
    );
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json(resp).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["router"], "error");
}

fn assert_cabeceras_fijas(resp: &Response<Body>, uri: &str) {
    let h = resp.headers();
    assert_eq!(h["referrer-policy"], "same-origin", "{uri}");
    assert_eq!(h["x-content-type-options"], "nosniff", "{uri}");
    assert_eq!(h["x-frame-options"], "DENY", "{uri}");
}

#[tokio::test]
async fn cabeceras_de_seguridad_presentes() {
    let p = panel().await;
    let resp = get(&p.app, "/salud").await;
    assert_eq!(resp.headers()["content-security-policy"], CSP);
    assert_cabeceras_fijas(&resp, "/salud");
}

/// Comprueba que una página HTML de Leptos lleva la CSP global con el nonce
/// de sus scripts en línea añadido a `script-src`.
async fn assert_csp_con_nonce(resp: Response<Body>, uri: &str) {
    assert_cabeceras_fijas(&resp, uri);
    let csp = resp.headers()["content-security-policy"]
        .to_str()
        .unwrap()
        .to_owned();
    let html = body_text(resp).await;
    let nonce = html
        .split("nonce=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("la página lleva scripts con nonce");
    assert!(!nonce.is_empty(), "{uri}");
    let esperada = CSP.replace(
        "script-src 'self' 'wasm-unsafe-eval'",
        &format!("script-src 'self' 'wasm-unsafe-eval' 'nonce-{nonce}'"),
    );
    assert_eq!(csp, esperada, "{uri}");
}

#[tokio::test]
async fn pagina_leptos_lleva_csp_con_su_nonce() {
    let p = panel().await;
    let resp = get(&p.app, "/login").await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_csp_con_nonce(resp, "/login").await;
}

/// La 404 la genera Leptos (página HTML con sus scripts): también lleva nonce.
#[tokio::test]
async fn pagina_no_encontrada_lleva_csp_con_su_nonce() {
    let p = panel().await;
    let resp = get(&p.app, "/no-existe").await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_csp_con_nonce(resp, "/no-existe").await;
}

#[tokio::test]
async fn fichero_estatico_con_cabeceras() {
    let p = panel().await;
    let resp = get(&p.app, "/pkg/panel.css").await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers()["content-security-policy"], CSP);
    assert_cabeceras_fijas(&resp, "/pkg/panel.css");
    assert_eq!(body_text(resp).await, "main { padding: 0 }\n");
}

#[tokio::test]
async fn portada_renderiza_en_servidor() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let resp = get_con_cookie(&p.app, "/", &cookie).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let html = body_text(resp).await;
    assert!(html.contains("Router · RTF8115VW"), "{html}");
    assert!(html.contains("<h1>Estado</h1>"), "{html}");
    assert!(
        html.contains("Esperando el primer dato del router"),
        "{html}"
    );
}

#[tokio::test]
async fn migraciones_crean_tablas() {
    let p = panel().await;
    let tablas: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .fetch_all(&p.state.db)
            .await
            .unwrap();
    for t in [
        "users",
        "sessions",
        "login_attempts",
        "changes",
        "reveals",
        "samples_raw",
        "samples_1m",
        "samples_15m",
        "wan_events",
    ] {
        assert!(tablas.iter().any(|n| n == t), "falta {t}: {tablas:?}");
    }
}
