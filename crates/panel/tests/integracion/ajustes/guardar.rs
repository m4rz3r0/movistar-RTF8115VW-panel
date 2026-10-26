//! Preparar y confirmar ajustes: pasos, cambio activo, sesión y errores.

use super::*;

#[tokio::test]
async fn guardar_nombre_no_toca_la_contrasena() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let clave = del_punto_de_acceso(&p, "0", "key_phrase").await;
    let seguridad = del_punto_de_acceso(&p, "0", "authentication").await;
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    let campos = cambios_del_grupo(&g);
    assert_eq!(campos.len(), 1, "{g}");
    assert_eq!(campos[0]["campo"], "ajuste-wifi-red_24-nombre");
    assert_eq!(campos[0]["despues"], "casa-nueva");
    assert_eq!(campos[0]["secreto"], false);
    assert_eq!(g["titulo"], "ajuste-wifi-red_24-titulo");
    let id = confirmar(&p, &cookie, &g, &[]).await;
    let c = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
    assert_eq!(del_punto_de_acceso(&p, "0", "ssid").await, "casa-nueva");
    assert_eq!(del_punto_de_acceso(&p, "0", "key_phrase").await, clave);
    assert_eq!(
        del_punto_de_acceso(&p, "0", "authentication").await,
        seguridad
    );
}

#[tokio::test]
async fn guardar_otro_campo_conserva_la_opcion_desconocida() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    escribir_red_24(&p, &[("authentication", "99")]).await;
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    assert_eq!(cambios_del_grupo(&g).len(), 1, "{g}");
    let id = confirmar(&p, &cookie, &g, &[]).await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    assert_eq!(del_punto_de_acceso(&p, "0", "authentication").await, "99");
    assert_eq!(del_punto_de_acceso(&p, "0", "ssid").await, "casa-nueva");
}

#[tokio::test]
async fn antes_es_el_valor_del_router_no_el_de_la_pagina() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let tema = leer(&p, &cookie, "wifi").await;
    let en_la_pagina = campo(ajuste(&tema, "wifi.red_24"), "nombre")["valor"]["Texto"].clone();
    escribir_red_24(&p, &[("ssid", "cambiado-fuera")]).await;
    assert_ne!(en_la_pagina, "cambiado-fuera");
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    let campos = cambios_del_grupo(&g);
    assert_eq!(campos[0]["antes"], "cambiado-fuera", "{g}");
    assert_eq!(campos[0]["despues"], "casa-nueva");
}

/// Ajuste de prueba con dos funciones de escritura: UPnP (`adm_state` 0 en
/// las fixtures) y el primer servidor de hora.
const DOS_FUNCIONES: &str = r#"
[[ajuste]]
id = "sistema.prueba"
tema = "sistema"
nivel = "basico"
leer = { funcion = "cbGetUpnpCfg" }

[[ajuste.campo]]
id = "upnp"
escribe = { funcion = "cbSetUpnpCfg", parametro = "adm_state" }
lee = "adm_state"
tipo = "booleano"

[[ajuste.campo]]
id = "hora"
escribe = { funcion = "cbSetTimeNtp", parametro = "ntp_svr_1" }
lee = "ntp_svr_1"
tipo = "texto"
"#;

#[tokio::test]
async fn desmarcar_un_paso_no_lo_escribe() {
    let (p, cookie) = PanelPrueba::nuevo()
        .ajustes(DOS_FUNCIONES)
        .arrancar_con_gestor()
        .await;
    let g = resumen(
        &p,
        &cookie,
        "sistema.prueba",
        json!({ "upnp": "true", "hora": "hora.example" }),
    )
    .await;
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(pasos.len(), 2, "{g}");
    assert_eq!(pasos[0]["indice"], 0);
    assert_eq!(pasos[0]["campos"][0]["campo"], "ajuste-sistema-prueba-upnp");
    assert_eq!(pasos[0]["campos"][0]["antes"], "false");
    assert_eq!(pasos[0]["campos"][0]["despues"], "true");
    assert_eq!(pasos[1]["campos"][0]["campo"], "ajuste-sistema-prueba-hora");
    let id = confirmar(&p, &cookie, &g, &[1]).await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    let escritas: Vec<String> = p.sim.control.writes().into_iter().map(|(f, _)| f).collect();
    assert_eq!(escritas, ["cbSetUpnpCfg"]);
}

/// Aplica `primero` y, mientras se vigila, prepara y confirma `segundo`:
/// espera en la cola y se aplica al terminar la vigilancia del primero.
/// Devuelve el panel para comprobar lo escrito.
async fn segundo_durante_la_vigilancia(primero: (&str, Value), segundo: (&str, Value)) -> Panel {
    let (p, cookie) = panel_vigilando().await;
    let g = resumen(&p, &cookie, primero.0, primero.1).await;
    let id = confirmar(&p, &cookie, &g, &[]).await;
    let c = ok(&p, "estado_cambio", json!({ "id": id }), &cookie).await;
    assert_eq!(c["estado"], "Vigilando", "{c}");
    let g2 = resumen(&p, &cookie, segundo.0, segundo.1).await;
    let id2 = confirmar(&p, &cookie, &g2, &[]).await;
    let c2 = ok(&p, "estado_cambio", json!({ "id": id2 }), &cookie).await;
    assert_eq!(c2["estado"], "EnCola", "{c2}");
    crate::comun::esperar_hasta(Duration::from_secs(5), "no se vigila", async || {
        ok(&p, "estado_cambio", json!({ "id": id }), &cookie).await["terminable"] == true
    })
    .await;
    let (status, body) = llamar(
        &p,
        "terminar_vigilancia",
        json!({ "id": id }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let c2 = crate::comun::esperar_hasta_con(Duration::from_secs(5), async || {
        let c = ok(&p, "estado_cambio", json!({ "id": id2 }), &cookie).await;
        if c["estado"] == "Vigilando" {
            Ok(c)
        } else {
            Err(format!("{c}"))
        }
    })
    .await;
    assert_eq!(c2["estado"], "Vigilando", "{c2}");
    p.state.cambios().unwrap().detener();
    p
}

#[tokio::test]
async fn segundo_ajuste_con_cambio_activo_se_encola() {
    let p = segundo_durante_la_vigilancia(
        ("wifi.red_24", json!({ "nombre": "casa-nueva" })),
        ("wifi.red_5", json!({ "nombre": "casa-nueva-5" })),
    )
    .await;
    assert_eq!(del_punto_de_acceso(&p, "0", "ssid").await, "casa-nueva");
}

/// Dos ajustes de la misma red: el segundo se prepara sobre lo que dejó el
/// primero y no lo pisa.
#[tokio::test]
async fn par_basico_y_avanzado_con_cambio_activo_se_encola() {
    let p = segundo_durante_la_vigilancia(
        ("wifi.red_24", json!({ "nombre": "casa-nueva" })),
        ("wifi.red_24_avanzado", json!({ "max_clientes": "7" })),
    )
    .await;
    assert_eq!(del_punto_de_acceso(&p, "0", "ssid").await, "casa-nueva");
    assert_eq!(del_punto_de_acceso(&p, "0", "max_clt_count").await, "7");
}

#[tokio::test]
async fn guardar_lo_mismo_da_409_ya_esta_asi() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let ssid = del_punto_de_acceso(&p, "0", "ssid").await;
    let (status, body) =
        preparar_ajuste(&p, &cookie, "wifi.red_24", json!({ "nombre": ssid })).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("Ya está así"), "{body}");
}

#[tokio::test]
async fn solo_lectura_da_403() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    instancia::escribir(&p.state.db, claves::SOLO_LECTURA, &json!(true))
        .await
        .unwrap();
    let (status, body) = preparar_ajuste(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("solo se pueden consultar"), "{body}");
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn valor_invalido_da_400_en_el_idioma() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let largo = "x".repeat(33);
    let (status, body) =
        preparar_ajuste(&p, &cookie, "wifi.red_24", json!({ "nombre": largo })).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("Demasiado largo"), "{body}");
    let en = format!("{cookie}; rp_lang=en");
    let (status, body) = preparar_ajuste(&p, &en, "wifi.red_24", json!({ "nombre": "" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("cannot be left empty"), "{body}");
    // Ajuste o campo que no existen.
    let (status, body) =
        preparar_ajuste(&p, &cookie, "wifi.no_existe", json!({ "nombre": "x" })).await;
    assert_eq!(status, 404, "{body}");
    let (status, body) = preparar_ajuste(&p, &cookie, "wifi.red_24", json!({ "ssid": "x" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(p.sim.control.writes().is_empty());
}

#[test]
fn detecta_funciones_de_la_api() {
    assert!(!sin_funciones_de_la_api("{\"f\":\"cbSetWifiAccPt\"}"));
    assert!(!sin_funciones_de_la_api("cbGetX"));
    assert!(sin_funciones_de_la_api("abcbSet cb cbx"));
}

#[tokio::test]
async fn resumen_sin_nombres_de_api() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva", "seguridad": "abierta", "contrasena": "clave-secreta-1" }),
    )
    .await;
    let texto = g.to_string();
    assert!(sin_funciones_de_la_api(&texto), "{texto}");
    for api in ["ssid", "authentication", "encryption", "key_phrase"] {
        assert!(!texto.contains(api), "{api}: {texto}");
    }
    assert!(!texto.contains("clave-secreta-1"), "{texto}");
    let clave = cambios_del_grupo(&g)
        .into_iter()
        .find(|c| c["campo"] == "ajuste-wifi-red_24-contrasena")
        .unwrap()
        .clone();
    assert_eq!(clave["secreto"], true);
    assert_eq!(clave["antes"], clave["despues"]);
    assert_eq!(clave["antes"], "••••••");
    let seguridad = cambios_del_grupo(&g)
        .into_iter()
        .find(|c| c["campo"] == "ajuste-wifi-red_24-seguridad")
        .unwrap()
        .clone();
    assert_eq!(seguridad["antes"], "wpa2");
    assert_eq!(seguridad["despues"], "abierta");
}

#[tokio::test]
async fn guardar_exige_sesion() {
    let (p, _) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = llamar(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "wifi.red_24", "instancia": null, "borrar": false, "valores": { "nombre": "x" } } }),
        None,
    )
    .await;
    assert_eq!(status, 401, "{body}");
}

#[tokio::test]
async fn borrar_en_un_ajuste_que_no_es_lista_da_400() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = llamar(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "wifi.red_24", "instancia": null, "borrar": true, "valores": {} } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("no ha podido preparar"), "{body}");
    assert!(p.sim.control.writes().is_empty());
}
