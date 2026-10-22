//! Acciones sobre varios elementos (`preparar_accion`): un grupo con un paso
//! por elemento y un solo deshacer, «ya estaba así», el límite de 20, otro
//! cambio en curso y el panel en solo lectura.

use crate::comun::{
    Panel, PanelPrueba, aplicar_ajuste, elemento, llamar, ok, partir, tiempos_rapidos,
};
use panel::instancia::{self, claves};
use serde_json::{Value, json};
use std::time::Duration;

/// `preparar_accion` de `accion` en la WiFi sobre `elementos`: estado y cuerpo.
async fn accion(p: &Panel, cookie: &str, accion: &str, elementos: &[&str]) -> (u16, String) {
    llamar(
        p,
        "preparar_accion",
        json!({ "tema": "wifi", "accion": accion, "elementos": elementos }),
        Some(cookie),
    )
    .await
}

/// Como [`accion`], exigiendo 200; devuelve el resumen.
async fn grupo(p: &Panel, cookie: &str, nombre: &str, elementos: &[&str]) -> Value {
    let (status, body) = accion(p, cookie, nombre, elementos).await;
    assert_eq!(status, 200, "preparar_accion {nombre}: {body}");
    serde_json::from_str(&body).unwrap()
}

/// `preparar_ajuste` de `ajuste` con `valores`: estado y cuerpo.
async fn preparar_ajuste(p: &Panel, cookie: &str, ajuste: &str, valores: Value) -> (u16, String) {
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": ajuste, "instancia": null, "borrar": false, "valores": valores } }),
        Some(cookie),
    )
    .await
}

/// Elemento `nombre` del punto de acceso `id` en el simulador.
async fn del_punto(p: &Panel, id: &str, nombre: &str) -> String {
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", id)])
        .await
        .unwrap();
    elemento(&n, nombre).unwrap_or_else(|| panic!("falta {nombre}"))
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
async fn accion_quitar_wps_en_dos_redes() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    assert_eq!(del_punto(&p, "0", "wps_adm_state").await, "1");
    assert_eq!(del_punto(&p, "1", "wps_adm_state").await, "1");
    let g = grupo(&p, &cookie, "quitar_wps", &["red-24", "red-5"]).await;
    assert_eq!(g["titulo"], "ajuste-accion-quitar_wps-titulo", "{g}");
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(pasos.len(), 2, "un paso por red: {g}");
    let titulos: Vec<&str> = pasos
        .iter()
        .map(|p| p["titulo"].as_str().unwrap())
        .collect();
    assert_eq!(
        titulos,
        ["ajuste-wifi-red_24-titulo", "ajuste-wifi-red_5-titulo"]
    );
    for paso in pasos {
        assert_eq!(paso["sin_cambios"], false, "{paso}");
        let campos = paso["campos"].as_array().unwrap();
        assert_eq!(campos.len(), 1, "{paso}");
        assert!(campos[0]["campo"].as_str().unwrap().ends_with("-wps"));
        assert_eq!(
            (&campos[0]["antes"], &campos[0]["despues"]),
            (&json!("true"), &json!("false"))
        );
    }
    assert!(!g.to_string().contains("cbSet"), "{g}");
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(del_punto(&p, "0", "wps_adm_state").await, "0");
    assert_eq!(del_punto(&p, "1", "wps_adm_state").await, "0");

    // Un solo deshacer deja las dos como estaban.
    let inverso = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    assert_eq!(inverso["titulo"], "ajuste-accion-quitar_wps-titulo");
    let pasos = inverso["pasos"].as_array().unwrap();
    assert_eq!(pasos.len(), 2, "{inverso}");
    assert!(
        pasos.iter().all(|p| p["campos"][0]["despues"] == "true"),
        "{inverso}"
    );
    aplicar_ajuste(&p, &cookie, &inverso).await;
    assert_eq!(del_punto(&p, "0", "wps_adm_state").await, "1");
    assert_eq!(del_punto(&p, "1", "wps_adm_state").await, "1");
}

#[tokio::test]
async fn accion_ya_estaba_asi() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    escribir_punto(&p, "0", &[("wps_adm_state", "0")]).await;
    let g = grupo(&p, &cookie, "quitar_wps", &["red-24", "red-5"]).await;
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(pasos.len(), 2, "{g}");
    assert_eq!(pasos[0]["sin_cambios"], true, "la de 2,4 GHz ya estaba así");
    assert_eq!(pasos[1]["sin_cambios"], false);
    aplicar_ajuste(&p, &cookie, &g).await;
    // Solo se escribe la que cambia.
    let escritas = p.sim.control.writes();
    let escrituras_red_24 = escritas
        .iter()
        .filter(|(f, params)| {
            f == "cbSetWifiAccPt"
                && params
                    .iter()
                    .any(|(k, v)| k == "dpm_cfg_wifi_acc_pt_id" && v == "0")
        })
        .count();
    assert_eq!(
        escrituras_red_24, 1,
        "solo la escritura de la prueba: {escritas:?}"
    );

    // Las dos ya lo tienen: 409, sin ningún cambio nuevo en el Historial.
    let cambios = || async {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM changes")
            .fetch_one(&p.state.db)
            .await
            .unwrap()
    };
    let antes = cambios().await;
    let (status, body) = accion(&p, &cookie, "quitar_wps", &["red-24", "red-5"]).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("ya estaban así"), "{body}");
    let en = format!("{cookie}; rp_lang=en");
    let (status, body) = accion(&p, &en, "quitar_wps", &["red-24", "red-5"]).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("already"), "{body}");
    assert_eq!(
        cambios().await,
        antes,
        "el 409 no deja nada en el Historial"
    );
}

#[tokio::test]
async fn accion_mas_de_veinte() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let muchos = vec!["red-24"; 21];
    let (status, body) = accion(&p, &cookie, "quitar_wps", &muchos).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("20"), "{body}");
    // Una acción que no se aplica a uno de los elementos, o que no existe.
    let (status, body) = accion(&p, &cookie, "quitar_wps", &["red-24", "radio-24"]).await;
    assert_eq!(status, 400, "{body}");
    assert!(!body.contains("cbSet"), "{body}");
    let (status, _) = accion(&p, &cookie, "nada", &["red-24"]).await;
    assert_eq!(status, 400);
    let (status, _) = accion(&p, &cookie, "quitar_wps", &[]).await;
    assert_eq!(status, 400);
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn interruptor_con_cambio_activo() {
    let mut t = tiempos_rapidos();
    t.bajo = Duration::from_secs(30);
    t.medio = Duration::from_secs(30);
    t.alto = Duration::from_secs(30);
    let (p, cookie) = PanelPrueba::nuevo().tiempos(t).arrancar_con_gestor().await;
    // El interruptor de una red, aplicado y vigilándose.
    let (status, body) =
        preparar_ajuste(&p, &cookie, "wifi.red_24", json!({ "activa": "true" })).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let id = ok(
        &p,
        "confirmar_ajuste",
        json!({ "token": g["token"], "desmarcados": [] }),
        &cookie,
    )
    .await;
    let c = ok(&p, "estado_cambio", json!({ "id": id }), &cookie).await;
    assert_eq!(c["estado"], "Vigilando", "{c}");
    // Otro interruptor o una acción en lote: «hay otro cambio en curso».
    let (status, body) =
        preparar_ajuste(&p, &cookie, "wifi.invitados", json!({ "activa": "true" })).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("otro cambio en curso"), "{body}");
    let (status, body) = accion(&p, &cookie, "quitar_wps", &["red-5", "invitados"]).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("otro cambio en curso"), "{body}");
}

#[tokio::test]
async fn solo_lectura_desactiva_todo() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    instancia::escribir(&p.state.db, claves::SOLO_LECTURA, &json!(true))
        .await
        .unwrap();
    let (status, body) = accion(&p, &cookie, "quitar_wps", &["red-24", "red-5"]).await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("solo se pueden consultar"), "{body}");
    let (status, body) =
        preparar_ajuste(&p, &cookie, "wifi.red_24", json!({ "activa": "true" })).await;
    assert_eq!(status, 403, "{body}");
    let peticion = |ajuste: &str, campo: &str, valor: &str| json!({ "ajuste": ajuste, "instancia": null, "borrar": false, "valores": { campo: valor } });
    let (status, body) = llamar(
        &p,
        "preparar_elemento",
        json!({ "peticiones": [
            peticion("wifi.red_5", "nombre", "RED-9"),
            peticion("wifi.red_5_avanzado", "max_clientes", "9"),
        ] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 403, "{body}");
    assert!(p.sim.control.writes().is_empty());

    let html = crate::comun::pagina(&p, &cookie, "/ajustes/wifi").await;
    let (visible, _) = partir(&html, "<script", "</script>");
    let interruptor = regex::Regex::new(r#"<button[^>]*role="switch"[^>]*>"#).unwrap();
    let todos: Vec<&str> = interruptor
        .find_iter(&visible)
        .map(|m| m.as_str())
        .collect();
    assert_eq!(todos.len(), 5, "uno por red y por radio: {visible}");
    for i in &todos {
        assert!(i.contains(r#"aria-disabled="true""#), "{i}");
    }
    // La barra y los botones de las recomendaciones, también, y se explica.
    let barra = regex::Regex::new(r#"<div[^>]*class="barra-acciones[^"]*"[^>]*>"#).unwrap();
    let b = barra.find(&visible).expect("la barra").as_str();
    assert!(b.contains(r#"aria-disabled="true""#), "{b}");
    let rec = regex::Regex::new(r#"<button[^>]*class="b-rec"[^>]*>"#).unwrap();
    let recs: Vec<&str> = rec.find_iter(&visible).map(|m| m.as_str()).collect();
    assert!(!recs.is_empty(), "{visible}");
    for r in recs {
        assert!(r.contains(r#"aria-disabled="true""#), "{r}");
    }
    assert!(visible.contains("solo se pueden consultar"), "{visible}");
}

/// El detalle de una red guarda lo esencial y lo avanzado en un solo cambio
/// (un paso por ajuste, con su título), que se deshace entero.
#[tokio::test]
async fn detalle_guarda_dos_ajustes_en_un_cambio() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let nombre = del_punto(&p, "1", "ssid").await;
    let maximo = del_punto(&p, "1", "max_clt_count").await;
    let peticion = |ajuste: &str, campo: &str, valor: &str| json!({ "ajuste": ajuste, "instancia": null, "borrar": false, "valores": { campo: valor } });
    let g = ok(
        &p,
        "preparar_elemento",
        json!({ "peticiones": [
            peticion("wifi.red_5", "nombre", "RED-9"),
            peticion("wifi.red_5_avanzado", "max_clientes", "9"),
        ] }),
        &cookie,
    )
    .await;
    assert_eq!(g["titulo"], "ajuste-wifi-red_5-titulo", "{g}");
    let titulos: Vec<&str> = g["pasos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["titulo"].as_str().unwrap())
        .collect();
    assert_eq!(
        titulos,
        [
            "ajuste-wifi-red_5-titulo",
            "ajuste-wifi-red_5_avanzado-titulo"
        ]
    );
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(del_punto(&p, "1", "ssid").await, "RED-9");
    assert_eq!(del_punto(&p, "1", "max_clt_count").await, "9");
    let inverso = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    let campos: Vec<&str> = inverso["pasos"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["campos"].as_array().unwrap())
        .map(|c| c["campo"].as_str().unwrap())
        .collect();
    assert_eq!(
        campos.len(),
        2,
        "el deshacer enseña los dos campos: {inverso}"
    );
    assert!(campos.contains(&"ajuste-wifi-red_5_avanzado-max_clientes"));
    aplicar_ajuste(&p, &cookie, &inverso).await;
    assert_eq!(del_punto(&p, "1", "ssid").await, nombre);
    assert_eq!(del_punto(&p, "1", "max_clt_count").await, maximo);
    // Una lista o un componente propio no van por aquí.
    let (status, _) = llamar(
        &p,
        "preparar_elemento",
        json!({ "peticiones": [
            peticion("wifi.red_5", "nombre", "RED-8"),
            peticion("red_local.reservas", "ip", "192.0.2.9"),
        ] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400);
}

/// Una acción sobre un ajuste que reenvía una clave que el router no deja
/// leer (la de una línea de teléfono) la borraría: 400, sin escribir nada.
#[tokio::test]
async fn accion_sobre_una_clave_tapada_da_400() {
    let toml = r#"
[[ajuste]]
id = "telefono.linea"
tema = "telefono"
nivel = "basico"
leer = { funcion = "cbGetVoiceLine", params = { interwork_id = "0" } }
fijos = { interwork_id = "0" }

[[ajuste.campo]]
id = "activa"
escribe = { funcion = "cbSetVoiceLine", parametro = "adm_state" }
lee = "adm_state"
tipo = "booleano"

[[ajuste.campo]]
id = "contrasena"
escribe = { funcion = "cbSetVoiceLine", parametro = "auth_password" }
lee = "auth_password"
tipo = "secreto"
validar = { min = 1 }

[[elemento]]
id = "linea"
tema = "telefono"
ajustes = ["telefono.linea"]
interruptor = "telefono.linea.activa"

[[accion]]
id = "apagar"
tema = "telefono"
campo = "activa"
valor = "false"
"#;
    let (p, cookie) = PanelPrueba::nuevo()
        .ajustes(toml)
        .arrancar_con_gestor()
        .await;
    let (status, body) = llamar(
        &p,
        "preparar_accion",
        json!({ "tema": "telefono", "accion": "apagar", "elementos": ["linea"] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("no deja leer"), "{body}");
    assert!(p.sim.control.writes().is_empty());
}
