//! WiFi avanzado (Tarea 5 de la fase 3): renovación de la clave de grupo, PMF y separación leídos en cada red, y las otras redes de invitados.

use crate::comun::{Panel, PanelPrueba, ajustes_de, aplicar_ajuste, leer, llamar, ok, variante};
use serde_json::{Value, json};

/// Campo `campo` del ajuste `ajuste` en la página de la WiFi.
fn campo(tema: &Value, ajuste: &str, campo: &str) -> Value {
    let a = ajustes_de(tema)
        .into_iter()
        .find(|a| a["id"] == ajuste)
        .unwrap_or_else(|| panic!("sin {ajuste}: {tema}"));
    a["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == campo)
        .cloned()
        .unwrap_or_else(|| panic!("sin {ajuste}.{campo}: {a}"))
}

/// El elemento `id` de la página de la WiFi.
async fn elemento(p: &Panel, cookie: &str, id: &str) -> Value {
    let tema = leer(p, cookie, "wifi").await;
    tema["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .cloned()
        .unwrap_or_else(|| panic!("sin {id}: {tema}"))
}

/// `regen_key_interval` del punto de acceso 0, tal cual en el simulador.
async fn renovacion_en_el_sim(p: &Panel) -> String {
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "0")])
        .await
        .unwrap();
    n.get("dpm_cfg_wifi_acc_pt")
        .and_then(|ap| ap.text_of("regen_key_interval"))
        .unwrap()
        .to_owned()
}

/// Escrituras de los puntos de acceso recibidas por el simulador.
fn escrituras_wifi(p: &Panel) -> Vec<Vec<(String, String)>> {
    p.sim
        .control
        .writes()
        .into_iter()
        .filter(|(f, _)| f == "cbSetWifiAccPt")
        .map(|(_, ps)| ps)
        .collect()
}

async fn preparar(p: &Panel, cookie: &str, ajuste: &str, valores: Value) -> (u16, String) {
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": ajuste, "instancia": null, "borrar": false, "valores": valores } }),
        Some(cookie),
    )
    .await
}

#[tokio::test]
async fn renovacion_de_clave_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    assert_eq!(renovacion_en_el_sim(&p).await, "0");
    let (status, body) = preparar(
        &p,
        &cookie,
        "wifi.red_24_avanzado",
        json!({ "renovacion_clave": "hora" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(renovacion_en_el_sim(&p).await, "3600");
    let w = escrituras_wifi(&p);
    assert_eq!(w.len(), 1, "{w:?}");
    let claves: Vec<&str> = w[0].iter().map(|(k, _)| k.as_str()).collect();
    assert!(claves.contains(&"regen_key_interval"), "{claves:?}");
    for no in ["wifi_pmf", "guest_networking"] {
        assert!(!claves.contains(&no), "la escritura lleva {no}: {claves:?}");
    }
    let tema = leer(&p, &cookie, "wifi").await;
    assert_eq!(
        campo(&tema, "wifi.red_24_avanzado", "renovacion_clave")["valor"],
        json!({ "Opcion": "hora" })
    );
    let d = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    aplicar_ajuste(&p, &cookie, &d).await;
    assert_eq!(renovacion_en_el_sim(&p).await, "0");
}

#[tokio::test]
async fn pmf_y_separada_solo_lectura() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let tema = leer(&p, &cookie, "wifi").await;
    let pmf = campo(&tema, "wifi.red_24_avanzado", "pmf");
    assert_eq!(pmf["valor"], json!({ "Opcion": "desactivada" }), "{pmf}");
    assert!(pmf["solo_lectura"].is_string(), "{pmf}");
    let separada = campo(&tema, "wifi.red_24_avanzado", "separada");
    assert_eq!(
        separada["valor"],
        json!({ "Booleano": false }),
        "{separada}"
    );
    let wps = campo(&tema, "wifi.red_5_avanzado", "wps_bloqueo");
    assert_eq!(wps["valor"], json!({ "Booleano": false }), "{wps}");
    assert!(wps["solo_lectura"].is_string(), "{wps}");
    let separada = campo(&tema, "wifi.invitados_avanzado", "separada");
    assert_eq!(separada["valor"], json!({ "Booleano": true }), "{separada}");
    let (status, body) = preparar(
        &p,
        &cookie,
        "wifi.red_24_avanzado",
        json!({ "pmf": "desactivada" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(escrituras_wifi(&p).is_empty());
}

/// El dato «Otras redes de invitados» de la tarjeta de invitados.
fn otras_invitadas(e: &Value) -> Value {
    e["datos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["clave"] == "ajustes-dato-otras_invitadas")
        .map(|d| d["valor"].clone())
        .unwrap_or_else(|| panic!("sin el dato: {e}"))
}

#[tokio::test]
async fn otras_invitadas_en_la_tarjeta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let e = elemento(&p, &cookie, "invitados").await;
    assert_eq!(
        otras_invitadas(&e),
        json!({ "Texto": "5 de 5 separadas" }),
        "{e}"
    );
    assert_ne!(e["recomendacion"]["informativa"], true, "{e}");

    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("invitada-sin-separar"))
        .arrancar_con_gestor()
        .await;
    let e = elemento(&p, &cookie, "invitados").await;
    assert_eq!(
        otras_invitadas(&e),
        json!({ "Texto": "4 de 5 separadas" }),
        "{e}"
    );
    let r = &e["recomendacion"];
    assert_eq!(r["informativa"], true, "{e}");
    let problema = r["problema"].as_str().unwrap();
    assert!(problema.contains("RED-2"), "{problema}");
    assert!(problema.contains('3'), "{problema}");
    assert!(
        r["texto"]
            .as_str()
            .is_some_and(|t| t.starts_with("Recomendado si")),
        "{r}"
    );
}
