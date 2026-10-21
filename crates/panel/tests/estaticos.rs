//! Ficheros de la web: comprimidos si el navegador los acepta, con caché
//! para siempre si llevan hash en el nombre; las páginas, sin caché.

mod comun;

use axum::{
    body::Body,
    http::{Request, Response, header},
};
use comun::{Panel, PanelPrueba, body_text, get, send, sesion};
use std::path::PathBuf;

const HASH: &str = "Q1w2E3r4T5y6U7i8O9p0Aa";

fn pkg(p: &Panel) -> PathBuf {
    PathBuf::from(p.state.leptos.site_root.as_ref()).join("pkg")
}

fn cabecera(resp: &Response<Body>, nombre: header::HeaderName) -> String {
    resp.headers()
        .get(nombre)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default()
}

async fn get_con(p: &Panel, uri: &str, aceptar: &str) -> Response<Body> {
    send(
        &p.app,
        Request::get(uri)
            .header(header::ACCEPT_ENCODING, aceptar)
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

/// Web compilada con hash y precomprimida, como la deja `cargo leptos build
/// --release --precompress`.
fn web_con_hash(p: &Panel) {
    let dir = pkg(p);
    std::fs::write(
        dir.join(format!("panel.{HASH}.wasm")),
        b"\0asm-sin-comprimir",
    )
    .unwrap();
    std::fs::write(dir.join(format!("panel.{HASH}.wasm.br")), b"brotli").unwrap();
    std::fs::write(dir.join(format!("panel.{HASH}.wasm.gz")), b"gzip").unwrap();
    std::fs::write(dir.join(format!("panel.{HASH}.css")), b"main{}").unwrap();
    std::fs::write(
        dir.parent().unwrap().join("hash.txt"),
        format!("js: {HASH}\nwasm: {HASH}\ncss: {HASH}\n"),
    )
    .unwrap();
}

#[tokio::test]
async fn wasm_comprimido_y_para_siempre() {
    let p = PanelPrueba::nuevo().arrancar().await;
    web_con_hash(&p);
    let uri = format!("/pkg/panel.{HASH}.wasm");
    let resp = get_con(&p, &uri, "gzip, deflate, br").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(cabecera(&resp, header::CONTENT_ENCODING), "br");
    assert_eq!(
        cabecera(&resp, header::CACHE_CONTROL),
        "public, max-age=31536000, immutable"
    );
    assert!(
        cabecera(&resp, header::VARY)
            .to_lowercase()
            .contains("accept-encoding")
    );
    assert_eq!(body_text(resp).await, "brotli");
    let resp = get_con(&p, &uri, "gzip").await;
    assert_eq!(cabecera(&resp, header::CONTENT_ENCODING), "gzip");
    // Sin compresión aceptada, el original (con la misma caché).
    let resp = get(&p.app, &uri).await;
    assert_eq!(cabecera(&resp, header::CONTENT_ENCODING), "");
    assert_eq!(
        cabecera(&resp, header::CACHE_CONTROL),
        "public, max-age=31536000, immutable"
    );
}

#[tokio::test]
async fn sin_hash_se_revalida() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let resp = get(&p.app, "/pkg/panel.css").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(cabecera(&resp, header::CACHE_CONTROL), "no-cache");
    // Lo que no existe, sin caché propia.
    let resp = get(&p.app, &format!("/pkg/panel.{HASH}.js")).await;
    assert_ne!(
        cabecera(&resp, header::CACHE_CONTROL),
        "public, max-age=31536000, immutable"
    );
}

#[tokio::test]
async fn paginas_sin_cache() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let resp = get(&p.app, "/login").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(cabecera(&resp, header::CACHE_CONTROL), "no-store");
    let resp = comun::get_con_cookie(&p.app, "/ajustes/internet", &cookie).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(cabecera(&resp, header::CACHE_CONTROL), "no-store");
}

/// Con `hash.txt` en la web, la página pide los ficheros con hash; sin él,
/// los de siempre.
#[tokio::test]
async fn la_pagina_pide_los_ficheros_con_hash() {
    let mut p = PanelPrueba::nuevo().arrancar().await;
    // Con un usuario: si no, `/login` lleva al asistente.
    sesion(&p).await;
    let html = body_text(get(&p.app, "/login").await).await;
    assert!(html.contains("/pkg/panel.css"), "{html}");
    web_con_hash(&p);
    panel::estaticos::configurar_hash(&mut p.state.leptos);
    assert!(p.state.leptos.hash_files);
    p.app = panel::build_router(p.state.clone());
    let html = body_text(get(&p.app, "/login").await).await;
    for f in ["css", "wasm", "js"] {
        assert!(html.contains(&format!("panel.{HASH}.{f}")), "{f}: {html}");
    }
}

/// Un 304 (el navegador ya lo tiene) repite la caché y el `Vary` del 200.
/// El manejador de ficheros de Leptos no contesta 304 (no le pasa las
/// cabeceras condicionales a `ServeDir`), así que se prueba la capa sola,
/// delante de una ruta que contesta 304.
#[tokio::test]
async fn el_304_repite_la_cache() {
    use axum::{Router, http::StatusCode, middleware, routing::get as ruta};
    let app = Router::new()
        .route("/pkg/{f}", ruta(|| async { StatusCode::NOT_MODIFIED }))
        .layer(middleware::from_fn(panel::estaticos::cabeceras));
    let resp = get(&app, &format!("/pkg/panel.{HASH}.css")).await;
    assert_eq!(resp.status(), 304);
    assert_eq!(
        cabecera(&resp, header::CACHE_CONTROL),
        "public, max-age=31536000, immutable"
    );
    assert!(
        cabecera(&resp, header::VARY)
            .to_lowercase()
            .contains("accept-encoding")
    );
    let resp = get(&app, "/pkg/panel.css").await;
    assert_eq!(cabecera(&resp, header::CACHE_CONTROL), "no-cache");
}
