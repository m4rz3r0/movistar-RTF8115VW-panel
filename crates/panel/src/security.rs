//! Cabeceras de seguridad comunes a todas las respuestas y freno de CSRF en
//! las server functions.

use crate::state::AppState;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
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
    // `same-origin` y no `no-referrer`: con `no-referrer` los navegadores mandan
    // `Origin: null` en los POST del propio panel.
    (header::REFERRER_POLICY, "same-origin"),
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

/// Middleware CSRF: rechaza con 403 las peticiones que cambian estado
/// (cualquier método salvo `GET`, `HEAD` y `OPTIONS`) a `/api/*` que no vengan
/// del propio panel según [`same_origin`]. Si faltan `Sec-Fetch-Site` y
/// `Origin` se deja pasar: la server function sigue exigiendo la cookie de
/// sesión, que es `SameSite=Strict`.
pub async fn csrf(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let safe_method = matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    if !safe_method
        && req.uri().path().starts_with("/api/")
        && !same_origin(req.headers(), &state.cfg.public_origin)
    {
        return (StatusCode::FORBIDDEN, "Origen no permitido").into_response();
    }
    next.run(req).await
}

/// ¿Viene la petición del propio panel? `Sec-Fetch-Site`, si está, manda
/// (todas sus copias deben ser `same-origin`). Si no está, `Origin` (todas sus
/// copias) debe ser exactamente `public_origin`; `Origin: null` no vale. Sin
/// ninguna de las dos, sí.
fn same_origin(headers: &HeaderMap, public_origin: &str) -> bool {
    let sites = headers.get_all("sec-fetch-site");
    if sites.iter().next().is_some() {
        return sites.iter().all(|v| v.as_bytes() == b"same-origin");
    }
    headers
        .get_all(header::ORIGIN)
        .iter()
        .all(|v| v.as_bytes() == public_origin.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::{csp, same_origin};
    use axum::http::HeaderMap;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.append(*k, v.parse().unwrap());
        }
        h
    }

    #[test]
    fn mismo_origen_segun_cabeceras() {
        let o = "https://router.example";
        let ok = |pairs: &[(&'static str, &'static str)]| same_origin(&headers(pairs), o);
        // Sin ninguna de las dos cabeceras se deja pasar (la sesión sigue
        // siendo obligatoria).
        assert!(ok(&[]));
        assert!(ok(&[("origin", "https://router.example")]));
        assert!(ok(&[("sec-fetch-site", "same-origin")]));
        // Sec-Fetch-Site manda cuando está: el navegador manda `Origin: null`
        // en algunos POST del mismo origen.
        assert!(ok(&[("origin", "null"), ("sec-fetch-site", "same-origin")]));
        assert!(!ok(&[
            ("origin", "https://router.example"),
            ("sec-fetch-site", "cross-site")
        ]));
        assert!(!ok(&[("sec-fetch-site", "same-site")]));
        assert!(!ok(&[("sec-fetch-site", "none")]));
        assert!(!ok(&[
            ("sec-fetch-site", "same-origin"),
            ("sec-fetch-site", "cross-site")
        ]));
        // Sin Sec-Fetch-Site, Origin debe ser exactamente el público.
        assert!(!ok(&[("origin", "https://otro.example")]));
        assert!(!ok(&[("origin", "null")]));
        assert!(!ok(&[
            ("origin", "https://router.example"),
            ("origin", "https://otro.example")
        ]));
    }

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
