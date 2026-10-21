//! Secretos en Ajustes: «Mostrar» (con reautenticación, sin caché, con
//! registro y `no-store`), cambiar una contraseña y deshacerlo, el valor
//! anterior de un paso en el Historial, y que ningún secreto acabe en el
//! HTML, en el registro ni reenviado vacío o tapado al router.

use crate::comun::{
    PANEL_PASS, Panel, PanelPrueba, aplicar_ajuste, body_text, cambios_del_grupo, elemento,
    esperar_final, esperar_hasta, llamar, ok, partir, post_json, registro_global, send,
};
use axum::http::header;
use serde_json::{Value, json};
use std::time::Duration;

mod deshacer;
mod guardar;
mod historial;
mod ver;

pub(crate) const CLAVE_1: &str = "clave-de-prueba-1";

pub(crate) const CLAVE_2: &str = "clave-de-prueba-2";

/// Escribe en el punto de acceso `id` del simulador, como lo haría el router.
pub(crate) async fn escribir_punto(p: &Panel, id: &str, pares: &[(&str, &str)]) {
    let mut params = vec![("dpm_cfg_wifi_acc_pt_id", id)];
    params.extend_from_slice(pares);
    p.state
        .hgu
        .write("wifi.asp", "cbSetWifiAccPt", &params)
        .await
        .unwrap();
}

/// Clave WiFi del punto de acceso `id` en el simulador.
async fn clave_del_punto(p: &Panel, id: &str) -> String {
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", id)])
        .await
        .unwrap();
    elemento(&n, "key_phrase").unwrap()
}

/// Llamada a una server function: estado, `Cache-Control` y cuerpo.
pub(crate) async fn llamada(
    p: &Panel,
    nombre: &str,
    cuerpo: Value,
    cookie: &str,
) -> (u16, String, String) {
    let mut req = post_json(&format!("/api/{nombre}"), &cuerpo.to_string(), "127.0.0.1");
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    let resp = send(&p.app, req).await;
    let status = resp.status().as_u16();
    let cache = resp
        .headers()
        .get(header::CACHE_CONTROL)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    (status, cache, body_text(resp).await)
}

/// Prepara el ajuste con `valores`: estado y cuerpo.
async fn preparar(p: &Panel, cookie: &str, ajuste: &str, valores: Value) -> (u16, String) {
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": ajuste, "instancia": null, "borrar": false, "valores": valores } }),
        Some(cookie),
    )
    .await
}

/// Cambia la clave de la red de 5 GHz a `nueva` desde Ajustes; el id.
async fn cambiar_clave_5(p: &Panel, cookie: &str, nueva: &str) -> i64 {
    let (status, body) = preparar(p, cookie, "wifi.red_5", json!({ "contrasena": nueva })).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    aplicar_ajuste(p, cookie, &g).await
}

/// La página `ruta` como la pide el navegador: su HTML.
pub(crate) async fn html(p: &Panel, cookie: &str, ruta: &str) -> String {
    let resp = send(
        &p.app,
        axum::http::Request::get(ruta)
            .header("cookie", cookie)
            .header("accept", "text/html")
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(resp.status(), 200, "{ruta}");
    body_text(resp).await
}

const CLAVE_3: &str = "clave-de-prueba-3";

const PIDE_LA_CLAVE: &str = "El router no deja leer esta clave: escríbela para guardar los cambios";

/// Lo que el simulador tiene en la línea de voz 0.
async fn de_la_voz(p: &Panel, nombre: &str) -> String {
    let n = p
        .state
        .hgu
        .call("cbGetVoiceLine", &[("interwork_id", "0")])
        .await
        .unwrap();
    elemento(&n, nombre).unwrap_or_default()
}

/// Campo `campo` del ajuste `ajuste` en la respuesta de `leer_tema`.
fn campo_de(tema: &Value, ajuste: &str, campo: &str) -> Value {
    tema["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == ajuste)
        .unwrap_or_else(|| panic!("falta {ajuste}"))["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == campo)
        .unwrap_or_else(|| panic!("falta {ajuste}.{campo}"))
        .clone()
}

/// Última escritura de `funcion` en el simulador.
fn ultima_escritura(p: &Panel, funcion: &str) -> Vec<(String, String)> {
    p.sim
        .control
        .writes()
        .into_iter()
        .rev()
        .find(|(f, _)| f == funcion)
        .unwrap_or_else(|| panic!("ninguna escritura de {funcion}"))
        .1
}

/// Texto que se lee en pantalla de un trozo de HTML: lo de fuera de las
/// etiquetas (sin los atributos).
pub(crate) fn texto_visible(html: &str) -> String {
    let mut out = String::new();
    let mut dentro = false;
    for c in html.chars() {
        match c {
            '<' => dentro = true,
            '>' => {
                dentro = false;
                out.push(' ');
            }
            _ if !dentro => out.push(c),
            _ => {}
        }
    }
    out
}
