//! Arnés común de las pruebas HTTP: panel en proceso contra `router-sim`, con
//! base de datos y clave en un directorio temporal.

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
use panel::{
    config::Config,
    instancia::{self, CredencialesRouter},
    redes,
    state::AppState,
};
use router_sim::{Sim, SimConfig, SimHandle};
use serde_json::{Value, json};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
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

/// Panel con el proxy de confianza `127.0.0.1`.
pub async fn panel() -> Panel {
    panel_con_proxy("127.0.0.1").await
}

/// Panel con los proxies de confianza `proxies` (como `RP_TRUSTED_PROXY`).
pub async fn panel_con_proxy(proxies: &str) -> Panel {
    panel_ajustado(|c| c.trusted_proxies = redes::lista(proxies).unwrap()).await
}

/// Panel con la configuración de serie de las pruebas cambiada por `ajustar`.
pub async fn panel_ajustado(ajustar: impl FnOnce(&mut Config)) -> Panel {
    panel_con_sim(ajustar, sim_config()).await
}

/// Panel contra un simulador con la capa de fixtures `capa` (una variante de
/// línea, ver [`variante`]).
pub async fn panel_con_capa(capa: &Path) -> Panel {
    panel_con_sim(|_| {}, sim_config().con_capa(capa)).await
}

/// Directorio de la variante de fixtures `nombre` (`fixtures-variantes/`).
pub fn variante(nombre: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../hgu/fixtures-variantes")
        .join(nombre)
}

/// Configuración del simulador con las fixtures de serie.
pub fn sim_config() -> SimConfig {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../hgu/fixtures");
    SimConfig::new(USER, PASS, fixtures)
}

async fn panel_con_sim(ajustar: impl FnOnce(&mut Config), sim: SimConfig) -> Panel {
    let p = panel_sin_router_con(ajustar, sim).await;
    let (sim, state, dir) = (p.sim, p.state, p._dir);
    // Las credenciales del simulador, como las dejaría el asistente.
    instancia::guardar_credenciales(
        &state.db,
        &state.history_key,
        &CredencialesRouter {
            url: sim.base_url.clone(),
            usuario: USER.into(),
            contrasena: PASS.into(),
        },
    )
    .await
    .unwrap();
    instancia::arrancar_router(&state).await.unwrap();
    Panel {
        app: panel::build_router(state.clone()),
        sim,
        state,
        _dir: dir,
    }
}

/// Panel recién instalado: sin variables del router y con la base de datos
/// vacía (el simulador arranca, pero el panel no lo conoce).
pub async fn panel_sin_router() -> Panel {
    panel_sin_router_ajustado(|_| {}).await
}

/// Como [`panel_sin_router`], con la configuración cambiada por `ajustar`.
pub async fn panel_sin_router_ajustado(ajustar: impl FnOnce(&mut Config)) -> Panel {
    panel_sin_router_con(ajustar, sim_config()).await
}

async fn panel_sin_router_con(ajustar: impl FnOnce(&mut Config), sim: SimConfig) -> Panel {
    let sim = Sim::start(sim).await;
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = config_en(dir.path());
    ajustar(&mut cfg);
    let state = estado(cfg, dir.path()).await;
    instancia::arrancar_router(&state).await.unwrap();
    Panel {
        app: panel::build_router(state.clone()),
        sim,
        state,
        _dir: dir,
    }
}

/// Configuración del panel con la base de datos y la clave en `dir`, sin
/// variables del router, con el proxy de confianza `127.0.0.1` y 1 ms entre
/// peticiones al simulador (en vez de 150 ms).
pub fn config_en(dir: &Path) -> Config {
    Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        db_path: dir.join("panel.db"),
        router_url: None,
        router_user: None,
        router_secret_file: None,
        history_key_file: dir.join("history.key"),
        trusted_proxies: redes::lista("127.0.0.1").unwrap(),
        public_origins: vec![ORIGIN.into()],
        install_nets: Vec::new(),
        router_min_gap: Duration::from_millis(1),
    }
}

/// Estado del servidor con `cfg` y los ficheros estáticos en `dir/site`
/// (sin arrancar el router).
pub async fn estado(cfg: Config, dir: &Path) -> AppState {
    let site = dir.join("site");
    std::fs::create_dir_all(site.join("pkg")).unwrap();
    std::fs::write(site.join("pkg/panel.css"), "main { padding: 0 }\n").unwrap();
    let leptos = LeptosOptions::builder()
        .output_name("panel")
        .site_root(site.to_str().unwrap())
        .build();
    AppState::new(cfg, leptos).await.unwrap()
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
    sesion_de(p).await
}

/// Panel ya instalado: con el usuario del panel (si no estaba). Sin
/// usuarios, toda página redirige al asistente de instalación.
pub async fn con_usuario(p: &Panel) {
    match panel::auth::create_user(&p.state.db, PANEL_USER, PANEL_PASS).await {
        Ok(()) | Err(panel::auth::UserError::Exists) => {}
        Err(e) => panic!("{e}"),
    }
}

/// Inicia sesión desde 127.0.0.1 con el usuario del panel (ya creado) y
/// devuelve la cookie lista para la cabecera `Cookie`.
pub async fn sesion_de(p: &Panel) -> String {
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
    gestor_y_sesion(panel().await).await
}

/// Como [`panel_con_gestor`], contra un simulador con la capa `capa`.
pub async fn panel_con_gestor_y_capa(capa: &Path) -> (Panel, String) {
    gestor_y_sesion(panel_con_capa(capa).await).await
}

/// Tiempos rápidos del gestor en las pruebas: vigilancia de 300 ms (600 ms
/// en riesgo alto).
pub fn tiempos_rapidos() -> Tiempos {
    Tiempos {
        bajo: Duration::from_millis(300),
        medio: Duration::from_millis(300),
        alto: Duration::from_millis(600),
        intervalo: Duration::from_millis(30),
        fallos_para_revertir: 3,
        pausa_max: Duration::from_secs(2),
    }
}

/// Como [`panel_con_gestor`], con los tiempos del gestor `t` (p. ej. una
/// vigilancia larga para probar lo que pasa mientras dura).
pub async fn panel_con_tiempos(t: Tiempos) -> (Panel, String) {
    gestor_y_sesion_con(panel().await, t).await
}

/// Como [`panel_con_gestor`], con el catálogo de ajustes sustituido por el
/// del TOML `toml` (un fichero de ajustes; sin excluidas ni comprobación
/// contra el catálogo de funciones, que pide cubrir todas las escrituras).
pub async fn panel_con_ajustes(toml: &str) -> (Panel, String) {
    let mut p = panel().await;
    let c = ajustes::CatalogoAjustes::from_toml(&[("prueba", toml)], "").unwrap();
    p.state.ajustes = std::sync::Arc::new(c);
    gestor_y_sesion(p).await
}

async fn gestor_y_sesion(p: Panel) -> (Panel, String) {
    gestor_y_sesion_con(p, tiempos_rapidos()).await
}

async fn gestor_y_sesion_con(mut p: Panel, t: Tiempos) -> (Panel, String) {
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, USER, PASS);
    cfg.timeout = Duration::from_millis(500);
    cfg.min_gap = p.state.cfg.router_min_gap;
    p.state.hgu = hgu::Client::new(cfg);
    // Un solo cliente del router para las server functions y el gestor.
    p.app = panel::build_router(p.state.clone());
    let m = ChangeManager::instalar(&p.state, t);
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

/// Espera a que el gestor suelte el cambio activo: lo hace justo después de
/// guardarlo en su estado final.
pub async fn esperar_libre(p: &Panel) {
    let g = p.state.cambios().unwrap();
    let inicio = Instant::now();
    while g.activo().is_some() {
        assert!(
            inicio.elapsed() < Duration::from_secs(5),
            "el gestor sigue ocupado"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
