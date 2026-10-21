//! Lectura de los ajustes del router por tema (`leer_tema`): valores
//! legibles, secretos que no salen del servidor, lecturas que fallan solas y
//! el catálogo comprobado al arrancar.

use crate::comun::{
    Panel, PanelPrueba, aplicar, body_text, cambios_del_grupo, elemento, esperar_final,
    esperar_libre, leer, llamar, ok, partir, post_json, send, sesion, tiempos_rapidos, variante,
};
use axum::http::header;
use panel::instancia::{self, claves};
use serde_json::{Value, json};
use std::time::Duration;

mod catalogo_real;
mod deshacer;
mod guardar;
mod lectura;
mod paginas;

/// Ajuste `id` de la respuesta de `leer_tema`.
fn ajuste<'a>(tema: &'a Value, id: &str) -> &'a Value {
    tema["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id)
        .unwrap_or_else(|| panic!("falta el ajuste {id}: {tema}"))
}

/// Campo `id` de un ajuste (o de una instancia).
fn campo<'a>(ajuste: &'a Value, id: &str) -> &'a Value {
    ajuste["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("falta el campo {id}: {ajuste}"))
}

/// Escribe en el punto de acceso 0 del simulador, como lo haría el router.
async fn escribir_red_24(p: &Panel, pares: &[(&str, &str)]) {
    let mut params = vec![("dpm_cfg_wifi_acc_pt_id", "0")];
    params.extend_from_slice(pares);
    p.state
        .hgu
        .write("wifi.asp", "cbSetWifiAccPt", &params)
        .await
        .unwrap();
}

/// `leer_tema` con la sesión `cookie` (o sin ella): estado, `Cache-Control`
/// y cuerpo.
async fn peticion(p: &Panel, tema: &str, cookie: Option<&str>) -> (u16, String, String) {
    let mut req = post_json(
        "/api/leer_tema",
        &json!({ "tema": tema }).to_string(),
        "127.0.0.1",
    );
    if let Some(c) = cookie {
        req.headers_mut().insert("cookie", c.parse().unwrap());
    }
    let resp = send(&p.app, req).await;
    let status = resp.status().as_u16();
    let cache = resp
        .headers()
        .get(header::CACHE_CONTROL)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    (status, cache, body_text(resp).await)
}

// ---------------------------------------------------------------- guardar

/// `preparar_ajuste` del ajuste `ajuste` con los valores legibles `valores`
/// (`{campo: valor}`): estado y cuerpo.
async fn preparar_ajuste(p: &Panel, cookie: &str, ajuste: &str, valores: Value) -> (u16, String) {
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": ajuste, "instancia": null, "borrar": false, "valores": valores } }),
        Some(cookie),
    )
    .await
}

/// Como [`preparar_ajuste`], exigiendo 200: el resumen (`GrupoVista`).
async fn resumen(p: &Panel, cookie: &str, ajuste: &str, valores: Value) -> Value {
    let (status, body) = preparar_ajuste(p, cookie, ajuste, valores).await;
    assert_eq!(status, 200, "preparar_ajuste {ajuste}: {body}");
    serde_json::from_str(&body).unwrap()
}

/// Confirma el resumen `grupo` sin los pasos `desmarcados`; devuelve el id.
async fn confirmar(p: &Panel, cookie: &str, grupo: &Value, desmarcados: &[usize]) -> i64 {
    ok(
        p,
        "confirmar_ajuste",
        json!({ "token": grupo["token"], "desmarcados": desmarcados }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap()
}

/// Elemento `nombre` del punto de acceso `id` en el simulador.
async fn del_punto_de_acceso(p: &Panel, id: &str, nombre: &str) -> String {
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", id)])
        .await
        .unwrap();
    elemento(&n, nombre).unwrap_or_else(|| panic!("falta {nombre}"))
}

/// Panel con una vigilancia larga: el primer cambio sigue vigilándose
/// mientras se prepara el segundo.
async fn panel_vigilando() -> (Panel, String) {
    let mut t = tiempos_rapidos();
    t.bajo = Duration::from_secs(30);
    t.medio = Duration::from_secs(30);
    t.alto = Duration::from_secs(30);
    PanelPrueba::nuevo().tiempos(t).arrancar_con_gestor().await
}

/// `texto` no nombra funciones de la API (`cbGet…`, `cbSet…`).
fn sin_funciones_de_la_api(texto: &str) -> bool {
    let b = texto.as_bytes();
    !(0..b.len().saturating_sub(2)).any(|i| {
        let limite = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        limite && &b[i..i + 2] == b"cb" && b[i + 2].is_ascii_uppercase()
    })
}
