//! Cabeceras de seguridad comunes a todas las respuestas.

use axum::{
    http::{HeaderName, HeaderValue, header},
    response::Response,
};
use tower::util::MapResponseLayer;

/// Política de seguridad de contenidos del panel.
pub const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'";

const HEADERS: [(HeaderName, &str); 4] = [
    (header::CONTENT_SECURITY_POLICY, CSP),
    (header::REFERRER_POLICY, "no-referrer"),
    (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    (header::X_FRAME_OPTIONS, "DENY"),
];

/// Capa que añade (o sustituye) las cabeceras de seguridad en cada respuesta.
pub fn layer() -> MapResponseLayer<fn(Response) -> Response> {
    MapResponseLayer::new(add_headers as fn(Response) -> Response)
}

fn add_headers(mut resp: Response) -> Response {
    let headers = resp.headers_mut();
    for (name, value) in HEADERS {
        headers.insert(name, HeaderValue::from_static(value));
    }
    resp
}
