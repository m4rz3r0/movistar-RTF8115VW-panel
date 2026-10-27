//! Pruebas HTTP del panel en proceso, contra `router-sim`.

use crate::comun::{
    ORIGIN, PANEL_PASS, PANEL_USER, Panel, PanelPrueba, body_text, con_usuario, esperar_hasta, get,
    get_con_cookie, login_body, post_form, send, sesion,
};
use axum::{
    body::Body,
    http::{Response, StatusCode},
};
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
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, crate::comun::USER, crate::comun::PASS);
    cfg.timeout = Duration::from_millis(500);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    p.app = panel::build_router(p.state.clone());
    let h = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(100),
            ..PollerConfig::default()
        },
    );
    esperar_hasta(Duration::from_secs(5), "sondeo sin datos", async || {
        h.healthy()
    })
    .await;
}

/// Un cliente del router nuevo (sin ninguna comunicación aún) en el panel.
fn cliente_nuevo(p: &mut Panel) {
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, crate::comun::USER, crate::comun::PASS);
    cfg.timeout = Duration::from_millis(500);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    p.app = panel::build_router(p.state.clone());
}

/// Sin ninguna comunicación con el router todavía (justo al arrancar), no
/// se dice que falle: «pendiente».
#[tokio::test]
async fn salud_sin_comunicacion_es_pendiente() {
    let mut p = PanelPrueba::nuevo().arrancar().await;
    cliente_nuevo(&mut p);
    let body = json(get(&p.app, "/salud").await).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["router"], "pendiente");
}

/// `/salud` dice cómo acabó la última comunicación real con el router,
/// aunque el sondeo no tenga datos (justo después de desplegar, las
/// lecturas de las páginas iban bien y decía «error»).
#[tokio::test]
async fn salud_ok_si_el_panel_lee_el_router_aunque_no_haya_sondeo() {
    let mut p = PanelPrueba::nuevo().arrancar().await;
    cliente_nuevo(&mut p);
    p.state.hgu.call("cbGetSummary", &[]).await.unwrap();
    let body = json(get(&p.app, "/salud").await).await;
    assert_eq!(body["router"], "ok", "{body}");
}

#[tokio::test]
async fn salud_ok() {
    let mut p = PanelPrueba::nuevo().arrancar().await;
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
    let mut p = PanelPrueba::nuevo().arrancar().await;
    con_sondeo(&mut p).await;
    p.sim.control.set_unresponsive(true);
    // En cuanto una petición del sondeo agota su tiempo de espera (500 ms).
    esperar_hasta(
        Duration::from_secs(5),
        "sin el error del router",
        async || {
            p.state.hgu.contacto().map(|c| c.estado)
                == Some(hgu::client::EstadoContacto::SinRespuesta)
        },
    )
    .await;
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
    let p = PanelPrueba::nuevo().arrancar().await;
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
    let p = PanelPrueba::nuevo().arrancar().await;
    crate::comun::con_usuario(&p).await;
    let resp = get(&p.app, "/login").await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_csp_con_nonce(resp, "/login").await;
}

/// La 404 la genera Leptos (página HTML con sus scripts): también lleva nonce.
#[tokio::test]
async fn pagina_no_encontrada_lleva_csp_con_su_nonce() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let resp = get(&p.app, "/no-existe").await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_csp_con_nonce(resp, "/no-existe").await;
}

#[tokio::test]
async fn fichero_estatico_con_cabeceras() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let resp = get(&p.app, "/pkg/panel.css").await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers()["content-security-policy"], CSP);
    assert_cabeceras_fijas(&resp, "/pkg/panel.css");
    assert_eq!(body_text(resp).await, "main { padding: 0 }\n");
}

#[tokio::test]
async fn portada_renderiza_en_servidor() {
    let p = PanelPrueba::nuevo().arrancar().await;
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
    let p = PanelPrueba::nuevo().arrancar().await;
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

/// Con varios orígenes en `RP_ORIGIN`, cualquiera de ellos vale en `Origin`;
/// uno ajeno, no.
#[tokio::test]
async fn segundo_origen_aceptado_y_ajeno_rechazado() {
    const SEGUNDO: &str = "https://panel.example";
    let p = PanelPrueba::nuevo()
        .config(|c| c.public_origins = vec![ORIGIN.into(), SEGUNDO.into()])
        .arrancar()
        .await;
    con_usuario(&p).await;
    // Sin `Sec-Fetch-Site`, manda `Origin`.
    let desde = |origen: &'static str| {
        let mut req = post_form(
            "/api/login",
            &login_body(PANEL_USER, PANEL_PASS),
            "127.0.0.1",
        );
        req.headers_mut().remove("sec-fetch-site");
        req.headers_mut().insert("origin", origen.parse().unwrap());
        req
    };
    assert_eq!(send(&p.app, desde(ORIGIN)).await.status(), StatusCode::OK);
    assert_eq!(send(&p.app, desde(SEGUNDO)).await.status(), StatusCode::OK);
    let resp = send(&p.app, desde("https://otro.example")).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert!(resp.headers().get("set-cookie").is_none());
}
