//! La página de un tema de una sola lectura (`leer_tema`): elementos con su
//! estado, resumen, recomendación principal y lecturas que fallan solas.

use crate::comun::{Panel, PanelPrueba, aplicar_ajuste, leer, ok, sesion};
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
    // Qué pasa ahora, además de para quién es lo recomendado.
    assert!(
        r["problema"]
            .as_str()
            .unwrap()
            .starts_with("El WPS está activado"),
        "{r}"
    );
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

/// Un ajuste que no está en ningún elemento declarado (la lista de reglas
/// de Puertos) es un elemento por sí solo, con las acciones de su lista.
#[tokio::test]
async fn ajustes_sueltos_son_elementos() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let pagina = leer(&p, &cookie, "puertos").await;
    let reglas = elemento(&pagina, "reglas");
    let ids: Vec<&str> = reglas["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["puertos.reglas"]);
    assert_eq!(reglas["clave"], "ajuste-puertos-reglas");
    assert_eq!(
        reglas["acciones"],
        json!(["activar_regla", "desactivar_regla", "borrar_regla"])
    );
    assert_eq!(pagina["resumen"], json!([]));
    // Las tarjetas de encima no tienen acciones de la lista.
    assert_eq!(elemento(&pagina, "upnp")["acciones"], json!([]));
}

/// «Volver a leer» (`fresca`) no sale de la caché corta; sin ella, sí. La
/// página dice cuánto hace que se leyó lo que enseña.
#[tokio::test]
async fn volver_a_leer_sin_la_cache() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let ssid = |pagina: &Value| elemento(pagina, "red-5")["ssid"].clone();
    let primera = leer(&p, &cookie, "wifi").await;
    assert!(
        primera["leido_hace_s"].as_u64().unwrap() < 5,
        "{}",
        primera["leido_hace_s"]
    );
    // Otro cambia el router (la web del operador): sin escribir desde el
    // panel, la caché sigue valiendo.
    let antes = ssid(&primera);
    let otro = hgu::Client::new(hgu::ClientConfig::new(
        &p.sim.base_url,
        crate::comun::USER,
        crate::comun::PASS,
    ));
    otro.write(
        "wifi.asp",
        "cbSetWifiAccPt",
        &[("dpm_cfg_wifi_acc_pt_id", "1"), ("ssid", "RED-8")],
    )
    .await
    .unwrap();
    let cacheada = ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    assert_eq!(ssid(&cacheada), antes);
    let fresca = ok(
        &p,
        "leer_tema",
        json!({ "tema": "wifi", "fresca": true }),
        &cookie,
    )
    .await;
    assert_eq!(ssid(&fresca), json!("RED-8"));
}

/// «Apagada desde»: si lo último que hizo el panel con la red fue apagarla
/// y sigue apagada, cuándo; de una red que el panel no ha tocado, nada.
#[tokio::test]
async fn apagada_desde_segun_el_historial() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let preparar = |valor: &str| {
        json!({ "p": { "ajuste": "wifi.red_24", "instancia": null, "borrar": false,
                       "valores": { "activa": valor } } })
    };
    let g = ok(&p, "preparar_ajuste", preparar("true"), &cookie).await;
    aplicar_ajuste(&p, &cookie, &g).await;
    let encendida = leer(&p, &cookie, "wifi").await;
    assert!(elemento(&encendida, "red-24")["pie"]["apagada_desde"].is_null());
    let antes = panel::series::now();
    let g = ok(&p, "preparar_ajuste", preparar("false"), &cookie).await;
    aplicar_ajuste(&p, &cookie, &g).await;
    let pagina = leer(&p, &cookie, "wifi").await;
    let desde = elemento(&pagina, "red-24")["pie"]["apagada_desde"]
        .as_i64()
        .unwrap_or_else(|| panic!("{pagina}"));
    assert!(
        desde >= antes - 1 && desde <= panel::series::now() + 1,
        "{desde}"
    );
    assert!(elemento(&pagina, "red-5")["pie"]["apagada_desde"].is_null());
}
