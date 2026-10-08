//! Cabeceras de seguridad comunes a todas las respuestas.

use axum::{
    http::{HeaderName, HeaderValue, header},
    response::Response,
};
use std::sync::LazyLock;
use tower::util::MapResponseLayer;

/// Fuentes permitidas para scripts, sin nonce.
const SCRIPT_SRC: &str = "'self' 'wasm-unsafe-eval'";

/// Política de seguridad de contenidos del panel. Con `nonce`, `script-src`
/// admite además los scripts en línea que Leptos marca con ese nonce.
pub fn csp(nonce: Option<&str>) -> String {
    let script_src = match nonce {
        Some(n) => format!("{SCRIPT_SRC} 'nonce-{n}'"),
        None => SCRIPT_SRC.to_owned(),
    };
    format!(
        "default-src 'self'; script-src {script_src}; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'"
    )
}

/// CSP sin nonce, para las respuestas que no son páginas de Leptos.
static CSP_SIN_NONCE: LazyLock<HeaderValue> =
    LazyLock::new(|| HeaderValue::from_str(&csp(None)).expect("CSP válida como cabecera"));

const OTHER_HEADERS: [(HeaderName, &str); 3] = [
    (header::REFERRER_POLICY, "no-referrer"),
    (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    (header::X_FRAME_OPTIONS, "DENY"),
];

/// Capa que añade las cabeceras de seguridad a cada respuesta. Respeta la CSP
/// que ya traiga la respuesta (las páginas de Leptos la ponen con su nonce).
pub fn layer() -> MapResponseLayer<fn(Response) -> Response> {
    MapResponseLayer::new(add_headers as fn(Response) -> Response)
}

fn add_headers(mut resp: Response) -> Response {
    let headers = resp.headers_mut();
    headers
        .entry(header::CONTENT_SECURITY_POLICY)
        .or_insert_with(|| CSP_SIN_NONCE.clone());
    for (name, value) in OTHER_HEADERS {
        headers.insert(name, HeaderValue::from_static(value));
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::csp;

    #[test]
    fn csp_sin_nonce_es_la_politica_global() {
        assert_eq!(
            csp(None),
            "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'"
        );
    }

    #[test]
    fn csp_con_nonce_solo_cambia_script_src() {
        assert_eq!(
            csp(Some("abc")),
            "default-src 'self'; script-src 'self' 'wasm-unsafe-eval' 'nonce-abc'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'"
        );
    }
}
