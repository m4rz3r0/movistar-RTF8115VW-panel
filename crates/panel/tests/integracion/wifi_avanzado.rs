//! WiFi avanzado: renovación de la clave de grupo, PMF y separación leídos en cada red, y las otras redes de invitados.

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

/// La lectura del punto de acceso 0 en el simulador, campo a campo.
async fn punto_0_en_el_sim(p: &Panel) -> Vec<(String, String)> {
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "0")])
        .await
        .unwrap();
    let ap = n.get("dpm_cfg_wifi_acc_pt").unwrap();
    ap.children
        .iter()
        .map(|c| (c.tag.clone(), c.text.clone()))
        .collect()
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
    let antes = punto_0_en_el_sim(&p).await;
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
    // La clave y el nombre de la red viajan con el valor que tenían, y el
    // resto de lo que no se ha tocado, también.
    for k in ["key_phrase", "ssid"] {
        let enviado = w[0].iter().find(|(a, _)| a == k).map(|(_, v)| v);
        let previo = antes.iter().find(|(a, _)| a == k).map(|(_, v)| v);
        assert!(
            previo.is_some_and(|v| !v.is_empty()),
            "sin {k} en la lectura"
        );
        assert_eq!(enviado, previo, "{k} cambia en la escritura");
    }
    for (k, v) in &w[0] {
        if k == "regen_key_interval" {
            continue;
        }
        if let Some((_, previo)) = antes.iter().find(|(a, _)| a == k) {
            assert_eq!(v, previo, "{k} cambia en la escritura");
        }
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
    for (ajuste, valores) in [
        ("wifi.red_24_avanzado", json!({ "pmf": "desactivada" })),
        ("wifi.red_5_avanzado", json!({ "wps_bloqueo": "true" })),
        ("wifi.red_24_avanzado", json!({ "separada": "true" })),
        ("wifi.invitados_avanzado", json!({ "separada": "false" })),
    ] {
        let (status, body) = preparar(&p, &cookie, ajuste, valores.clone()).await;
        assert_eq!(status, 400, "{ajuste} {valores}: {body}");
        assert!(
            body.contains("solo se puede consultar"),
            "{ajuste} {valores}: {body}"
        );
    }
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
    assert!(problema.contains("(red 3)"), "{problema}");
    assert!(
        r["texto"]
            .as_str()
            .is_some_and(|t| t.starts_with("Recomendado si")
                && t.contains("si la web del router lo permite")),
        "{r}"
    );
}

/// El dato «Sin leer»: no se sabe la cifra entera.
const SIN_LEER: &str = "Sin leer";

/// Si falla la lista de puntos de acceso de una radio, faltan sus redes de
/// invitados: el dato sale «Sin leer», no con una cifra parcial, y la
/// recomendación solo nombra las redes que se ven.
#[tokio::test]
async fn otras_invitadas_con_una_lista_sin_leer() {
    // Falla la de 5 GHz, la de la red 3 sin separar: ni cifra ni aviso.
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("invitada-sin-separar"))
        .arrancar_con_gestor()
        .await;
    p.sim
        .control
        .fail_reads_con("cbListWifiAccPt", "dpm_cfg_wifi_id", "1", true);
    let e = elemento(&p, &cookie, "invitados").await;
    assert_eq!(otras_invitadas(&e), json!({ "Texto": SIN_LEER }), "{e}");
    assert_ne!(e["recomendacion"]["informativa"], true, "{e}");

    // Falla la de 2,4 GHz: la red 3 se ve y se avisa de ella, pero sin cifra.
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("invitada-sin-separar"))
        .arrancar_con_gestor()
        .await;
    p.sim
        .control
        .fail_reads_con("cbListWifiAccPt", "dpm_cfg_wifi_id", "0", true);
    let e = elemento(&p, &cookie, "invitados").await;
    assert_eq!(otras_invitadas(&e), json!({ "Texto": SIN_LEER }), "{e}");
    let r = &e["recomendacion"];
    assert_eq!(r["informativa"], true, "{e}");
    assert!(r["problema"].as_str().unwrap().contains("(red 3)"), "{r}");
}

/// Una red de la que no se sabe si está separada no cuenta como «no
/// separada»: el dato sale «Sin leer».
#[tokio::test]
async fn otras_invitadas_con_separacion_ilegible() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("invitada-separacion-ilegible"))
        .arrancar_con_gestor()
        .await;
    let e = elemento(&p, &cookie, "invitados").await;
    assert_eq!(otras_invitadas(&e), json!({ "Texto": SIN_LEER }), "{e}");
    assert_ne!(e["recomendacion"]["informativa"], true, "{e}");
}
