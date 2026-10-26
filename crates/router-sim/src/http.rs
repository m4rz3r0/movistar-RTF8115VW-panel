//! Rutas HTTP: login, páginas, CGI y middlewares.

use crate::LOGGED_OUT;
use crate::cortafuegos;
use crate::escrituras::write;
use crate::estado::{HangAfterWrite, Inner, Params, SimState};
use crate::lecturas::read;
use crate::usuarios;
use crate::xml::{failure, leaf, response, result_node};
use axum::{
    body::Bytes,
    extract::{Path, RawQuery, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use hgu::fmt::mess_userpass;
use std::{
    collections::HashMap,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

fn cookies(headers: &HeaderMap) -> HashMap<String, String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect()
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = b
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (b[i], hex) {
            (b'%', Some(v)) => {
                out.push(v);
                i += 3;
            }
            (b'+', _) => {
                out.push(b' ');
                i += 1;
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub(crate) fn form(body: &[u8]) -> Params {
    String::from_utf8_lossy(body)
        .split('&')
        .filter(|kv| !kv.is_empty())
        .map(|kv| {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            (pct_decode(k), pct_decode(v))
        })
        .collect()
}

pub(crate) fn param<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn xml(body: String) -> Response {
    ([(header::CONTENT_TYPE, "text/xml")], body).into_response()
}

fn has_basic(st: &SimState, c: &HashMap<String, String>) -> bool {
    c.get("_httpdSessionId_")
        .is_some_and(|id| st.basic.contains(id))
}

/// Cuelga las peticiones mientras el router «no responde» y retarda cada
/// respuesta lo fijado con [`SimControl::set_retardo`] (salvo la del propio
/// ajuste del retardo).
pub(crate) async fn hang_if_unresponsive(
    State(s): State<Arc<Inner>>,
    req: Request,
    next: Next,
) -> Response {
    while s.unresponsive.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let ms = s.retardo_ms.load(Ordering::SeqCst);
    if ms > 0 && req.uri().path() != RUTA_RETARDO {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }
    next.run(req).await
}

/// Ruta de control del retardo (no existe en el router real).
pub(crate) const RUTA_RETARDO: &str = "/__sim/retardo";

/// `POST /__sim/retardo` con los milisegundos en el cuerpo: fija el retardo
/// de cada respuesta ([`SimControl::set_retardo`]).
pub(crate) async fn fijar_retardo(State(s): State<Arc<Inner>>, body: Bytes) -> Response {
    match std::str::from_utf8(&body)
        .ok()
        .and_then(|t| t.trim().parse::<u64>().ok())
    {
        Some(ms) => {
            s.retardo_ms.store(ms, Ordering::SeqCst);
            StatusCode::NO_CONTENT.into_response()
        }
        None => (StatusCode::BAD_REQUEST, "milisegundos").into_response(),
    }
}

/// Como el micro_httpd del router real: un POST sin `Content-Length` (lo que
/// hace hyper con un cuerpo vacío) recibe 400 Bad Request.
pub(crate) async fn post_needs_content_length(req: Request, next: Next) -> Response {
    if req.method() == axum::http::Method::POST
        && !req.headers().contains_key(header::CONTENT_LENGTH)
    {
        return (
            StatusCode::BAD_REQUEST,
            "<html><body>400 Bad Request</body></html>",
        )
            .into_response();
    }
    next.run(req).await
}

pub(crate) async fn root(State(s): State<Arc<Inner>>) -> Response {
    let id = s.state().next_token();
    (
        [(header::SET_COOKIE, format!("_httpdSessionId_={id}; path=/"))],
        "<html><body>router-sim</body></html>",
    )
        .into_response()
}

pub(crate) async fn basic_login(
    State(s): State<Arc<Inner>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let f = form(&body);
    let rechazada = {
        let mut st = s.state();
        s.aplicar_contrasena_pendiente(&mut st, true);
        st.login_falla_con.clone()
    };
    let good = credenciales_validas(&s, &f, ("loginUsername", "loginPassword"))
        && rechazada.is_none_or(|r| param(&f, "loginPassword") != Some(mess_userpass(&r).as_str()));
    if !good {
        s.state().inicios_fallidos += 1;
        return "<script>location.href = \"/te_acceso_router.asp\"</script>".into_response();
    }
    let mut st = s.state();
    let (id, new) = match cookies(&headers).remove("_httpdSessionId_") {
        Some(id) => (id, false),
        None => (st.next_token(), true),
    };
    st.basic.insert(id.clone());
    let body = "<script>location.href = \"/te_wifi.asp\"</script>";
    if new {
        (
            [(header::SET_COOKIE, format!("_httpdSessionId_={id}; path=/"))],
            body,
        )
            .into_response()
    } else {
        body.into_response()
    }
}

pub(crate) async fn logout(State(s): State<Arc<Inner>>, headers: HeaderMap) -> Response {
    let c = cookies(&headers);
    let mut st = s.state();
    if let Some(id) = c.get("_httpdSessionId_") {
        st.basic.remove(id);
    }
    if let Some(sid) = c.get("SessionID") {
        st.advanced.remove(sid);
    }
    LOGGED_OUT.into_response()
}

pub(crate) async fn page(
    State(s): State<Arc<Inner>>,
    Path(page): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !page.ends_with(".asp") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut st = s.state();
    if !has_basic(&st, &cookies(&headers)) {
        return LOGGED_OUT.into_response();
    }
    let key = st.next_token();
    st.session_key = Some(key.clone());
    if cortafuegos::es_suya(&page) {
        return cortafuegos::pagina(&st, &page, &key).into_response();
    }
    let vars: String = PAGE_VARS
        .iter()
        .filter(|(p, _)| *p == page)
        .flat_map(|(_, vars)| vars.iter())
        .map(|(k, v)| format!("var {k}='{v}';\r\n"))
        .collect();
    format!(
        "<html><head><script>\r\nvar sessionKey='{key}';\r\n{vars}</script></head><body>{page}</body></html>"
    )
    .into_response()
}

/// Variables `var x='…';` sintéticas de algunas páginas básicas, con nombres
/// de parámetros del catálogo (incluidos secretos inventados para probar el
/// enmascarado).
pub const PAGE_VARS: &[(&str, &[(&str, &str)])] = &[(
    "te_wifi.asp",
    &[
        ("wlSsid", "RED-SIM"),
        ("wlHide", "0"),
        ("wlChannel", "6"),
        ("unitNum", "0"),
        ("wlMACaddr", "02:00:00:00:00:0d"),
        ("wlKey1", "wep-sim-secreta"),
        ("wlWpaPsk", "psk-sim-secreta"),
    ],
)];

/// Escritura de la UI básica: `GET /cgi-bin/<x>.cmd|.cgi|.wl?…&sessionKey=…`.
/// Se registra en `writes` y responde OK; solo cambia el estado simulado la
/// del cortafuegos (`scbidirectionalflt.cmd`).
pub(crate) async fn basic_cgi(
    State(s): State<Arc<Inner>>,
    Path(file): Path<String>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> Response {
    if ![".cmd", ".cgi", ".wl"].iter().any(|e| file.ends_with(e)) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut st = s.state();
    if !has_basic(&st, &cookies(&headers)) {
        return LOGGED_OUT.into_response();
    }
    let params = form(query.unwrap_or_default().as_bytes());
    let key_ok = st
        .session_key
        .as_deref()
        .is_some_and(|k| param(&params, "sessionKey") == Some(k));
    if !key_ok {
        return "<html><body>sessionKey no válido</body></html>".into_response();
    }
    let clean: Params = params
        .into_iter()
        .filter(|(k, _)| k != "sessionKey")
        .collect();
    if file == cortafuegos::ORDEN {
        cortafuegos::escribir(&mut st, &clean);
    }
    st.writes.push((file, clean));
    "<html><body>ok</body></html>".into_response()
}

pub(crate) async fn cgi(
    State(s): State<Arc<Inner>>,
    Path(file): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(name) = file.strip_suffix(".xml") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let (respuesta, sin_respuesta) = cgi_sync(&s, name, &headers, &body);
    if sin_respuesta {
        std::future::pending::<()>().await;
    }
    respuesta
}

/// Atiende `/cgi-bin/<name>.xml`; el segundo valor dice si hay que dejar la
/// petición sin respuesta (`no_answer_next_write`).
fn cgi_sync(s: &Inner, name: &str, headers: &HeaderMap, body: &Bytes) -> (Response, bool) {
    let c = cookies(headers);
    let params = form(body);
    let mut st = s.state();
    if name == "cbCheckLogin" {
        return (xml(advanced_login(s, &mut st, &c, &params)), false);
    }
    let valid = c.get("SessionStatus").is_some_and(|v| v == "Valid")
        && c.get("SessionID")
            .is_some_and(|sid| st.advanced.contains(sid));
    if !valid {
        return (LOGGED_OUT.into_response(), false);
    }
    if is_read(name) {
        st.reads.push((name.to_owned(), params.clone()));
    }
    let falla_con = st
        .failing_reads_con
        .iter()
        .any(|(n, k, v)| n == name && param(&params, k) == Some(v.as_str()));
    if is_read(name) && (st.failing_reads.contains(name) || falla_con) {
        (xml(failure(name, "lectura fallida (simulado)")), false)
    } else if is_read(name) {
        (xml(read(s, &st, name, &params)), false)
    } else {
        let real = param(&params, "validation_only") != Some("1");
        let body = write(s, &mut st, name, &params);
        if st.hang_after_write == HangAfterWrite::Now {
            st.hang_after_write = HangAfterWrite::Off;
            s.unresponsive.store(true, Ordering::SeqCst);
        }
        let sin_respuesta = real && std::mem::take(&mut st.no_answer_next);
        (xml(body), sin_respuesta)
    }
}

fn advanced_login(
    s: &Inner,
    st: &mut SimState,
    c: &HashMap<String, String>,
    params: &[(String, String)],
) -> String {
    if !has_basic(st, c) {
        return LOGGED_OUT.into();
    }
    s.aplicar_contrasena_pendiente(st, false);
    let good = credenciales_validas(s, params, ("rqUsername", "rqPasswd"))
        && st
            .login_falla_con
            .as_deref()
            .is_none_or(|r| param(params, "rqPasswd") != Some(mess_userpass(r).as_str()));
    if !good {
        return failure("cbCheckLogin", "login");
    }
    let sid = st.next_token();
    st.advanced.insert(sid.clone());
    response(&result_node(
        "cbCheckLogin",
        vec![leaf("SessionID", &sid)],
        "0",
        "",
    ))
}

/// ¿Vienen en `params` (con XOR 0x1f, en los campos `campos`: usuario y
/// contraseña) las credenciales de un usuario de la tabla?
fn credenciales_validas(s: &Inner, params: &[(String, String)], campos: (&str, &str)) -> bool {
    match (param(params, campos.0), param(params, campos.1)) {
        (Some(u), Some(c)) => usuarios::entra(&s.usuarios(), &mess_userpass(u), &mess_userpass(c)),
        _ => false,
    }
}

fn is_read(name: &str) -> bool {
    name.contains("Get") || name.starts_with("cbList") || name.starts_with("cbCheck")
}
