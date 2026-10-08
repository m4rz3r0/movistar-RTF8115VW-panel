//! Pruebas HTTP del panel en proceso, contra `router-sim`.

use axum::{
    Router,
    body::Body,
    http::{Request, Response, StatusCode},
};
use leptos::prelude::LeptosOptions;
use panel::{config::Config, state::AppState};
use router_sim::{Sim, SimConfig, SimHandle};
use std::{path::PathBuf, time::Duration};
use tempfile::TempDir;
use tower::ServiceExt;

const USER: &str = "1234";
const PASS: &str = "clave-de-prueba";
const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'";

struct Panel {
    app: Router,
    sim: SimHandle,
    state: AppState,
    _dir: TempDir,
}

async fn panel() -> Panel {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../hgu/fixtures");
    let sim = Sim::start(SimConfig::new(USER, PASS, fixtures)).await;
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("router.secret");
    std::fs::write(&secret, format!("{PASS}\n")).unwrap();
    let cfg = Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        db_path: dir.path().join("panel.db"),
        router_url: sim.base_url.clone(),
        router_user: USER.into(),
        router_secret_file: secret,
        history_key_file: dir.path().join("history.key"),
        trusted_proxy: "127.0.0.1".parse().unwrap(),
        public_origin: "http://localhost:3000".into(),
    };
    let site = dir.path().join("site");
    std::fs::create_dir_all(site.join("pkg")).unwrap();
    std::fs::write(site.join("pkg/panel.css"), "main { padding: 0 }\n").unwrap();
    let leptos = LeptosOptions::builder()
        .output_name("panel")
        .site_root(site.to_str().unwrap())
        .build();
    let state = AppState::new(cfg, leptos).await.unwrap();
    Panel {
        app: panel::build_router(state.clone()),
        sim,
        state,
        _dir: dir,
    }
}

async fn get(app: &Router, uri: &str) -> Response<Body> {
    app.clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn json(resp: Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn salud_ok() {
    let p = panel().await;
    let resp = get(&p.app, "/salud").await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        json(resp).await,
        serde_json::json!({"ok": true, "version": env!("CARGO_PKG_VERSION"), "router": "ok"})
    );
}

#[tokio::test]
async fn salud_responde_aunque_el_router_no() {
    let p = panel().await;
    p.sim.control.set_unresponsive(true);
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
    assert_eq!(h["referrer-policy"], "no-referrer", "{uri}");
    assert_eq!(h["x-content-type-options"], "nosniff", "{uri}");
    assert_eq!(h["x-frame-options"], "DENY", "{uri}");
}

async fn body_text(resp: Response<Body>) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
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
    let resp = get(&p.app, "/").await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_csp_con_nonce(resp, "/").await;
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
    let resp = get(&p.app, "/").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let html = body_text(resp).await;
    assert!(html.contains("Router · RTF8115VW"), "{html}");
    assert!(html.contains("Panel en construcción"), "{html}");
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
