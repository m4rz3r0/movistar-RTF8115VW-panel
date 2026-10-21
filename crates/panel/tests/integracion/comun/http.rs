//! Peticiones al panel en proceso y lectura de sus respuestas.

use super::{ORIGIN, Panel};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response},
};
use serde_json::Value;
use std::net::SocketAddr;
use tower::ServiceExt;

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

/// Página `ruta` con la sesión `cookie`, exigiendo 200; devuelve el HTML.
pub async fn pagina(p: &Panel, cookie: &str, ruta: &str) -> String {
    let resp = get_con_cookie(&p.app, ruta, cookie).await;
    assert_eq!(resp.status(), 200, "{ruta}");
    body_text(resp).await
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

pub fn set_cookie(resp: &Response<Body>) -> String {
    resp.headers()["set-cookie"].to_str().unwrap().to_owned()
}

pub async fn body_text(resp: Response<Body>) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
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

/// Parte `html` en lo que queda fuera de los bloques `abre`…`cierra` y los
/// bloques mismos.
pub fn partir<'a>(html: &'a str, abre: &str, cierra: &str) -> (String, Vec<&'a str>) {
    let mut fuera = String::new();
    let mut dentro = Vec::new();
    let mut resto = html;
    while let Some(i) = resto.find(abre) {
        fuera.push_str(&resto[..i]);
        let fin = i + resto[i..].find(cierra).expect("bloque sin cerrar") + cierra.len();
        dentro.push(&resto[i..fin]);
        resto = &resto[fin..];
    }
    fuera.push_str(resto);
    (fuera, dentro)
}
