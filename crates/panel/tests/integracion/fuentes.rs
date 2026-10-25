//! De dónde sale cada recomendación (Tarea 8 de la fase 3): las fuentes, o
//! «criterio del panel», viajan con el campo, el paso de Mejoras y la
//! recomendación de la tarjeta.

use crate::comun::{PanelPrueba, leer, ok, variante};
use serde_json::{Value, json};

/// El campo `campo` del ajuste `ajuste` en el elemento `elemento` de la
/// página `tema`.
fn campo<'a>(tema: &'a Value, elemento: &str, ajuste: &str, campo: &str) -> &'a Value {
    tema["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == elemento)
        .and_then(|e| e["ajustes"].as_array())
        .and_then(|a| a.iter().find(|a| a["id"] == ajuste))
        .and_then(|a| a["campos"].as_array())
        .and_then(|c| c.iter().find(|c| c["id"] == campo))
        .unwrap_or_else(|| panic!("falta {elemento}/{ajuste}.{campo}: {tema}"))
}

/// La recomendación de la tarjeta del elemento `id`.
fn recomendacion<'a>(tema: &'a Value, id: &str) -> &'a Value {
    let e = tema["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .unwrap_or_else(|| panic!("falta {id}: {tema}"));
    &e["recomendacion"]
}

#[tokio::test]
async fn recomendacion_con_sus_fuentes() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    // El WPS de la red de 2,4 GHz, con el aviso del CERT/CC.
    let wifi = leer(&p, &cookie, "wifi").await;
    let wps = campo(&wifi, "red-24", "wifi.red_24", "wps");
    let fuentes = wps["fuentes"].as_array().expect("fuentes");
    let cert = fuentes
        .iter()
        .find(|f| {
            f["url"]
                .as_str()
                .is_some_and(|u| u.starts_with("https://kb.cert.org/"))
        })
        .unwrap_or_else(|| panic!("sin la fuente del CERT/CC: {wps}"));
    assert_eq!(cert["clase"], "fuentes-clase-guia", "{cert}");
    assert_eq!(cert["consultada"], "2026-10-22", "{cert}");
    assert!(
        cert["titulo"].as_str().is_some_and(|t| !t.is_empty()),
        "{cert}"
    );
    assert!(
        cert["organismo"].as_str().is_some_and(|t| !t.is_empty()),
        "{cert}"
    );
    // La recomendación de la tarjeta (quitar el WPS) lleva las mismas.
    let r = recomendacion(&wifi, "red-24");
    assert_eq!(r["campo"], "wps", "{r}");
    assert_eq!(r["fuentes"], wps["fuentes"], "{r}");
    // El espejo de Sistema es criterio del panel.
    let sistema = leer(&p, &cookie, "sistema").await;
    let espejo = sistema["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|e| e["ajustes"].as_array().unwrap())
        .filter(|a| a["id"] == "sistema.diagnostico")
        .flat_map(|a| a["campos"].as_array().unwrap())
        .find(|c| c["id"] == "espejo")
        .unwrap_or_else(|| panic!("sin el espejo: {sistema}"));
    assert_eq!(espejo["respaldo"], "fuentes-respaldo-criterio", "{espejo}");
    // En Mejoras, cada paso de quitar el WPS trae sus fuentes.
    let m = ok(&p, "leer_mejoras", json!({ "fresca": true }), &cookie).await;
    let paso = m["lotes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|l| l["pasos"].as_array().unwrap())
        .find(|x| x["clave"] == "wps@red-24")
        .unwrap_or_else(|| panic!("sin wps@red-24: {m}"));
    assert!(
        paso["fuentes"].as_array().is_some_and(|f| !f.is_empty()),
        "{paso}"
    );
    // El de los DNS nombra la página de cada proveedor, el familiar también.
    let dns = m["lotes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|l| l["pasos"].as_array().unwrap())
        .find(|x| x["clave"] == "dns_casa")
        .unwrap_or_else(|| panic!("sin dns_casa: {m}"));
    let urls: Vec<&str> = dns["fuentes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["url"].as_str())
        .collect();
    assert!(
        urls.contains(&"https://developers.cloudflare.com/1.1.1.1/setup/#1111-for-families"),
        "{urls:?}"
    );
}

#[tokio::test]
async fn recomendaciones_que_informan_con_sus_fuentes() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("dmz-activa"))
        .arrancar_con_gestor()
        .await;
    let puertos = leer(&p, &cookie, "puertos").await;
    let r = recomendacion(&puertos, "dmz");
    assert_eq!(r["informativa"], true, "{r}");
    assert!(
        r["fuentes"].as_array().is_some_and(|f| !f.is_empty()),
        "{r}"
    );
    // El cortafuegos (sin campo propio): las del elemento.
    let seguridad = leer(&p, &cookie, "seguridad").await;
    let r = recomendacion(&seguridad, "cortafuegos");
    assert_eq!(r["informativa"], true, "{r}");
    assert!(
        r["fuentes"].as_array().is_some_and(|f| !f.is_empty()),
        "{r}"
    );
}
