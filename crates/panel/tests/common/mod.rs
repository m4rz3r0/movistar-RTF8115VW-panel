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
use panel::{config::Config, state::AppState};
use router_sim::{Sim, SimConfig, SimHandle};
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
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
