//! Arnés común de las pruebas HTTP: panel en proceso contra `router-sim`, con
//! base de datos y secreto en un directorio temporal.

// Cada binario de pruebas usa solo parte del arnés.
#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response},
};
use leptos::prelude::LeptosOptions;
use panel::changes::{ChangeManager, Tiempos};
use panel::{config::Config, state::AppState};
use router_sim::{Sim, SimConfig, SimHandle};
use serde_json::{Value, json};
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    time::{Duration, Instant},
};
use tempfile::TempDir;
use tower::ServiceExt;

pub const USER: &str = "1234";
pub const PASS: &str = "clave-de-prueba";
/// Origen público configurado en el panel de pruebas.
pub const ORIGIN: &str = "http://localhost:3000";
/// Usuario y contraseña del panel que crean las pruebas.
pub const PANEL_USER: &str = "admin";
pub const PANEL_PASS: &str = "una contraseña larga";

pub struct Panel {
    pub app: Router,
    pub sim: SimHandle,
    pub state: AppState,
    _dir: TempDir,
}

/// Panel con `trusted_proxy = 127.0.0.1`.
pub async fn panel() -> Panel {
    panel_con_proxy("127.0.0.1".parse().unwrap()).await
}

pub async fn panel_con_proxy(trusted_proxy: IpAddr) -> Panel {
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
        trusted_proxy,
        public_origin: ORIGIN.into(),
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

pub async fn send(app: &Router, req: Request<Body>) -> Response<Body> {
    app.clone().oneshot(req).await.unwrap()
}

pub async fn get(app: &Router, uri: &str) -> Response<Body> {
    send(app, Request::get(uri).body(Body::empty()).unwrap()).await
}

/// `GET` con la cookie de sesión `cookie` (`nombre=valor`).
pub async fn get_con_cookie(app: &Router, uri: &str, cookie: &str) -> Response<Body> {
    send(
        app,
        Request::get(uri)
            .header("cookie", cookie)
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

/// `POST` de formulario a una server function desde `peer`, como lo haría el
/// navegador desde el propio panel.
pub fn post_form(uri: &str, body: &str, peer: &str) -> Request<Body> {
    let mut req = Request::post(uri)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("origin", ORIGIN)
        .header("sec-fetch-site", "same-origin")
        .body(Body::from(body.to_owned()))
        .unwrap();
    let peer: SocketAddr = format!("{peer}:50000").parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(peer));
    req
}

/// `POST` JSON a una server function desde `peer`, como lo haría el navegador
/// desde el propio panel.
pub fn post_json(uri: &str, body: &str, peer: &str) -> Request<Body> {
    let mut req = post_form(uri, body, peer);
    req.headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    req
}

/// Cuerpo del formulario de login.
pub fn login_body(usuario: &str, contrasena: &str) -> String {
    let enc = |s: &str| form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>();
    format!("usuario={}&contrasena={}", enc(usuario), enc(contrasena))
}

/// Crea el usuario del panel, inicia sesión desde 127.0.0.1 y devuelve la
/// cookie lista para la cabecera `Cookie`.
pub async fn sesion(p: &Panel) -> String {
    panel::auth::create_user(&p.state.db, PANEL_USER, PANEL_PASS)
        .await
        .unwrap();
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body(PANEL_USER, PANEL_PASS),
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

pub fn set_cookie(resp: &Response<Body>) -> String {
    resp.headers()["set-cookie"].to_str().unwrap().to_owned()
}

pub async fn body_text(resp: Response<Body>) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Panel con un cliente del router rápido y el gestor de cambios instalado
/// (vigilancia de 300 ms), más la cookie de una sesión.
pub async fn panel_con_gestor() -> (Panel, String) {
    let mut p = panel().await;
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, USER, PASS);
    cfg.timeout = Duration::from_millis(500);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    // Un solo cliente del router para las server functions y el gestor.
    p.app = panel::build_router(p.state.clone());
    let m = ChangeManager::instalar(
        &p.state,
        Tiempos {
            bajo: Duration::from_millis(300),
            medio: Duration::from_millis(300),
            alto: Duration::from_millis(600),
            intervalo: Duration::from_millis(30),
            fallos_para_revertir: 3,
            pausa_max: Duration::from_secs(2),
        },
    );
    m.reanudar_pendientes().await;
    let cookie = sesion(&p).await;
    (p, cookie)
}

/// Llama a la server function `nombre` con un cuerpo JSON.
pub async fn llamar(p: &Panel, nombre: &str, cuerpo: Value, cookie: Option<&str>) -> (u16, String) {
    let mut req = post_json(&format!("/api/{nombre}"), &cuerpo.to_string(), "127.0.0.1");
    if let Some(c) = cookie {
        req.headers_mut().insert("cookie", c.parse().unwrap());
    }
    let resp = send(&p.app, req).await;
    let status = resp.status().as_u16();
    (status, body_text(resp).await)
}

/// Como [`llamar`], exigiendo 200; devuelve el JSON.
pub async fn ok(p: &Panel, nombre: &str, cuerpo: Value, cookie: &str) -> Value {
    let (status, body) = llamar(p, nombre, cuerpo, Some(cookie)).await;
    assert_eq!(status, 200, "{nombre}: {body}");
    serde_json::from_str(&body).unwrap_or_else(|e| panic!("{nombre}: {e}: {body}"))
}

/// Valores de un formulario tal como llegan (más sus claves), para enviarlos
/// sin tocar.
pub fn valores_de(form: &Value) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = form["claves"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c[0].as_str().unwrap().into(), c[1].as_str().unwrap().into()))
        .collect();
    for c in form["campos"].as_array().unwrap() {
        v.push((
            c["param"]["nombre"].as_str().unwrap().into(),
            c["valor"].as_str().unwrap().into(),
        ));
    }
    v
}

pub fn fijar(v: &mut [(String, String)], k: &str, valor: &str) {
    v.iter_mut()
        .find(|(n, _)| n == k)
        .unwrap_or_else(|| panic!("falta {k}"))
        .1 = valor.into();
}

pub fn campo<'a>(form: &'a Value, nombre: &str) -> Option<&'a Value> {
    form["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["param"]["nombre"] == nombre)
}

/// Prepara y confirma; devuelve el id del cambio.
pub async fn aplicar(p: &Panel, cookie: &str, peticion: Value) -> i64 {
    let prep = ok(
        p,
        "preparar_cambio",
        json!({ "peticion": peticion }),
        cookie,
    )
    .await;
    let id = ok(
        p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        cookie,
    )
    .await;
    id.as_i64().unwrap()
}

/// Sondea `estado_cambio` como la interfaz hasta un estado final.
pub async fn esperar_final(p: &Panel, cookie: &str, id: i64) -> Value {
    let start = Instant::now();
    loop {
        let c = ok(p, "estado_cambio", json!({ "id": id }), cookie).await;
        if ["Aplicado", "Revertido", "Fallido"].contains(&c["estado"].as_str().unwrap()) {
            return c;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "sin final: {c}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
