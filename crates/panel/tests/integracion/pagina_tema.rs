//! La página de un tema de una sola lectura (`leer_tema`): elementos con su
//! estado, resumen, recomendación principal y lecturas que fallan solas.

use crate::comun::{Panel, PanelPrueba, leer, ok, sesion};
use regex::Regex;
use serde_json::{Value, json};

/// Elemento `id` de la página.
fn elemento<'a>(pagina: &'a Value, id: &str) -> &'a Value {
    pagina["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .unwrap_or_else(|| panic!("falta el elemento {id}: {pagina}"))
}

/// Dato de la franja de resumen con la clave `clave`.
fn resumen<'a>(pagina: &'a Value, clave: &str) -> &'a Value {
    pagina["resumen"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["clave"] == clave)
        .unwrap_or_else(|| panic!("falta {clave} en el resumen: {pagina}"))
}

/// Escribe en el punto de acceso `id` del simulador, como lo haría el router.
async fn escribir_punto(p: &Panel, id: &str, pares: &[(&str, &str)]) {
    let mut params = vec![("dpm_cfg_wifi_acc_pt_id", id)];
    params.extend_from_slice(pares);
    p.state
        .hgu
        .write("wifi.asp", "cbSetWifiAccPt", &params)
        .await
        .unwrap();
}

#[tokio::test]
async fn pagina_wifi_con_las_redes_apagadas() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let pagina = leer(&p, &cookie, "wifi").await;
    let ids: Vec<&str> = pagina["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["red-24", "red-5", "invitados", "radio-24", "radio-5"]);
    for red in ["red-24", "red-5", "invitados"] {
        let e = elemento(&pagina, red);
        assert_eq!(e["estado"], "apagada", "{e}");
        assert_eq!(e["texto_estado"], "Apagada", "{e}");
        assert_eq!(e["lectura"], "Leido", "{e}");
        assert_eq!(e["interruptor"]["encendido"], false, "{e}");
        assert_eq!(e["interruptor"]["campo"], "activa", "{e}");
        // Encenderla no corta nada: el interruptor no pregunta antes.
        assert!(e["interruptor"]["efecto"].is_null(), "{e}");
        // Seguridad, contraseña, visible, canal y ancho, aislamiento y
        // conectados (ninguno en las fixtures).
        assert_eq!(e["datos"].as_array().unwrap().len(), 6, "{e}");
        let conectados = e["datos"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["clave"] == "ajustes-dato-conectados")
            .unwrap_or_else(|| panic!("{e}"));
        assert_eq!(conectados["valor"], json!({ "Texto": "0" }));
        assert_eq!(e["ajustes"].as_array().unwrap().len(), 2, "{e}");
        assert!(
            e["ssid"].as_str().is_some_and(|s| s.starts_with("RED-")),
            "{e}"
        );
        assert!(
            e["acciones"]
                .as_array()
                .unwrap()
                .contains(&json!("quitar_wps")),
            "{e}"
        );
    }
    for radio in ["radio-24", "radio-5"] {
        let e = elemento(&pagina, radio);
        assert_eq!(e["estado"], "atencion", "{e}");
        assert_eq!(e["texto_estado"], "Encendida, sin redes", "{e}");
        assert_eq!(e["interruptor"]["encendido"], true, "{e}");
        // Apagarla corta la WiFi de su banda: pregunta antes.
        assert_eq!(e["interruptor"]["efecto"], "efecto-wifi_corte", "{e}");
        assert!(e["ssid"].is_null(), "{e}");
        assert!(e["pie"]["nota"].as_str().is_some(), "{e}");
        // Ninguna acción de las redes se aplica a una radio.
        assert_eq!(e["acciones"], json!([]), "{e}");
    }
    assert_eq!(
        resumen(&pagina, "ajustes-resumen-wifi-redes")["cifra"],
        "0 de 3"
    );
    assert_eq!(
        resumen(&pagina, "ajustes-resumen-wifi-radios")["cifra"],
        "2 de 2"
    );
    assert_eq!(
        resumen(&pagina, "ajustes-resumen-wifi-conectados")["cifra"],
        "0"
    );
    let acciones: Vec<&str> = pagina["acciones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        acciones,
        [
            "encender",
            "apagar",
            "quitar_wps",
            "ocultar_nombre",
            "mostrar_nombre",
            "aislar"
        ]
    );
}

#[tokio::test]
async fn recomendacion_principal_prefiere_el_riesgo() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    escribir_punto(&p, "0", &[("wps_adm_state", "1")]).await;
    escribir_punto(&p, "2", &[("isolation", "0")]).await;
    let pagina = leer(&p, &cookie, "wifi").await;
    // La red está apagada (encenderla también es una recomendación, y va
    // antes en el catálogo), pero el WPS es un riesgo.
    let r = &elemento(&pagina, "red-24")["recomendacion"];
    assert_eq!(r["boton"], "Quitar WPS", "{r}");
    assert_eq!(r["riesgo"], true, "{r}");
    assert_eq!(
        (&r["ajuste"], &r["campo"], &r["valor"]),
        (&json!("wifi.red_24"), &json!("wps"), &json!("false"))
    );
    assert!(
        r["texto"].as_str().unwrap().starts_with("Recomendado si"),
        "{r}"
    );
    // Sin el WPS, la de la red: encenderla, que no es un riesgo.
    escribir_punto(&p, "0", &[("wps_adm_state", "0")]).await;
    p.state.lecturas.invalidar();
    let pagina = leer(&p, &cookie, "wifi").await;
    let r = &elemento(&pagina, "red-24")["recomendacion"];
    assert_eq!(
        (&r["campo"], &r["riesgo"]),
        (&json!("activa"), &json!(false)),
        "{r}"
    );
    assert_eq!(r["boton"], "Encender", "{r}");
    let pendientes = resumen(&pagina, "ajustes-resumen-recomendaciones");
    assert_eq!(pendientes["atencion"], true, "{pendientes}");
}

#[tokio::test]
async fn pagina_con_lectura_fallida() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    p.sim
        .control
        .fail_reads_con("cbGetWifiAccPt", "dpm_cfg_wifi_acc_pt_id", "1", true);
    let pagina = leer(&p, &cookie, "wifi").await;
    let red5 = elemento(&pagina, "red-5");
    let motivo = red5["lectura"]["SinLeer"]
        .as_str()
        .unwrap_or_else(|| panic!("{red5}"));
    assert!(!motivo.is_empty() && !motivo.contains("cbGet"), "{motivo}");
    assert_eq!(red5["estado"], "peligro", "{red5}");
    assert_eq!(red5["texto_estado"], "Sin leer", "{red5}");
    assert!(red5["interruptor"].is_null(), "{red5}");
    assert_eq!(red5["datos"], json!([]), "{red5}");
    assert!(red5["recomendacion"].is_null(), "{red5}");
    assert_eq!(red5["acciones"], json!([]), "{red5}");
    for id in ["red-24", "invitados", "radio-24", "radio-5"] {
        let e = elemento(&pagina, id);
        assert_eq!(e["lectura"], "Leido", "{e}");
        assert_ne!(e["estado"], "peligro", "{e}");
    }
    assert!(
        !elemento(&pagina, "red-24")["acciones"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // El resumen sigue: dos redes que se pueden leer, ninguna encendida.
    assert_eq!(
        resumen(&pagina, "ajustes-resumen-wifi-redes")["cifra"],
        "0 de 3"
    );
}

/// Nombres de la API (`cbXxx`) en el JSON de la página fuera de los
/// detalles técnicos de cada ajuste.
fn nombres_de_la_api(pagina: &Value) -> Vec<String> {
    let mut sin_tecnico = pagina.clone();
    for e in sin_tecnico["elementos"].as_array_mut().unwrap() {
        for a in e["ajustes"].as_array_mut().unwrap() {
            a.as_object_mut().unwrap().remove("tecnico");
        }
    }
    let texto = sin_tecnico.to_string();
    Regex::new(r"cb[A-Z]\w*")
        .unwrap()
        .find_iter(&texto)
        .map(|m| m.as_str().to_owned())
        .collect()
}

#[tokio::test]
async fn pagina_sin_nombres_de_la_api() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let en = format!("{cookie}; rp_lang=en");
    for (idioma, c) in [("es", &cookie), ("en", &en)] {
        let pagina = ok(&p, "leer_tema", json!({ "tema": "wifi" }), c).await;
        assert_eq!(nombres_de_la_api(&pagina), Vec::<String>::new(), "{idioma}");
        // Los detalles técnicos sí los llevan.
        assert!(pagina.to_string().contains("cbSetWifiAccPt"), "{idioma}");
        let redes = resumen(&pagina, "ajustes-resumen-wifi-redes")["cifra"].as_str();
        assert_eq!(
            redes,
            Some(if idioma == "es" { "0 de 3" } else { "0 of 3" })
        );
    }
}

/// Las pestañas sin elementos declarados: cada ajuste es un elemento.
#[tokio::test]
async fn ajustes_sueltos_son_elementos() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let pagina = leer(&p, &cookie, "puertos").await;
    let upnp = elemento(&pagina, "upnp");
    let ids: Vec<&str> = upnp["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["puertos.upnp"]);
    assert_eq!(upnp["clave"], "ajuste-puertos-upnp");
    assert_eq!(pagina["resumen"], json!([]));
    assert_eq!(pagina["acciones"], json!([]));
}
