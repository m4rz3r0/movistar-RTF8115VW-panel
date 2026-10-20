//! Secretos en Ajustes: «Mostrar» (con reautenticación, sin caché, con
//! registro y `no-store`), cambiar una contraseña y deshacerlo, el valor
//! anterior de un paso en el Historial, y que ningún secreto acabe en el
//! HTML, en el registro ni reenviado vacío o tapado al router.

mod common;

use axum::http::header;
use common::{
    PANEL_PASS, Panel, body_text, esperar_final, esperar_libre, llamar, ok, panel_con_ajustes,
    panel_con_gestor, post_json, send,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

const CLAVE_1: &str = "clave-de-prueba-1";
const CLAVE_2: &str = "clave-de-prueba-2";

/// Destino del registro en memoria, para comprobar lo que se anota.
#[derive(Clone, Default)]
struct Registro(Arc<std::sync::Mutex<Vec<u8>>>);

impl Registro {
    fn texto(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

impl std::io::Write for Registro {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Registro {
    type Writer = Registro;

    fn make_writer(&'a self) -> Registro {
        self.clone()
    }
}

/// El registro de todo este binario de pruebas (todas las pruebas, en
/// todos los hilos): uno de hilo pierde eventos cuando otras pruebas corren
/// a la vez (la caché de interés de `tracing`). Que tampoco las demás
/// pruebas anoten un secreto es más estricto, no menos.
fn registro_global() -> Registro {
    static REGISTRO: std::sync::OnceLock<Registro> = std::sync::OnceLock::new();
    REGISTRO
        .get_or_init(|| {
            let registro = Registro::default();
            tracing::subscriber::set_global_default(
                tracing_subscriber::fmt()
                    .with_writer(registro.clone())
                    .with_max_level(tracing::Level::DEBUG)
                    .with_ansi(false)
                    .finish(),
            )
            .expect("el único registro global de estas pruebas");
            registro
        })
        .clone()
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

/// Elemento `nombre` de una respuesta del router, a cualquier profundidad.
fn elemento(n: &hgu::xml::XmlNode, nombre: &str) -> Option<String> {
    n.text_of(nombre)
        .map(str::to_owned)
        .or_else(|| n.children.iter().find_map(|c| elemento(c, nombre)))
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
async fn llamada(p: &Panel, nombre: &str, cuerpo: Value, cookie: &str) -> (u16, String, String) {
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

/// Confirma el resumen `grupo` y espera a que quede aplicado; su id.
async fn aplicar(p: &Panel, cookie: &str, grupo: &Value) -> i64 {
    let id = ok(
        p,
        "confirmar_ajuste",
        json!({ "token": grupo["token"], "desmarcados": [] }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap();
    let c = esperar_final(p, cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
    esperar_libre(p).await;
    id
}

/// Campos cambiados de todos los pasos de un resumen.
fn cambios(grupo: &Value) -> Vec<&Value> {
    grupo["pasos"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["campos"].as_array().unwrap())
        .collect()
}

/// Cambia la clave de la red de 5 GHz a `nueva` desde Ajustes; el id.
async fn cambiar_clave_5(p: &Panel, cookie: &str, nueva: &str) -> i64 {
    let (status, body) = preparar(p, cookie, "wifi.red_5", json!({ "contrasena": nueva })).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    aplicar(p, cookie, &g).await
}

/// La página `ruta` como la pide el navegador: su HTML.
async fn html(p: &Panel, cookie: &str, ruta: &str) -> String {
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

#[tokio::test]
async fn cambiar_contrasena_wifi_y_deshacer() {
    let (p, cookie) = panel_con_gestor().await;
    let original = clave_del_punto(&p, "1").await;
    let (status, body) =
        preparar(&p, &cookie, "wifi.red_5", json!({ "contrasena": CLAVE_2 })).await;
    assert_eq!(status, 200, "{body}");
    assert!(
        !body.contains(CLAVE_2) && !body.contains(&original),
        "{body}"
    );
    let g: Value = serde_json::from_str(&body).unwrap();
    let campos = cambios(&g);
    assert_eq!(campos.len(), 1, "{g}");
    assert_eq!(campos[0]["campo"], "ajuste-wifi-red_5-contrasena");
    assert_eq!(campos[0]["secreto"], true);
    assert_eq!(campos[0]["antes"], panel::ui::explorador::MASCARA);
    assert_eq!(campos[0]["despues"], panel::ui::explorador::MASCARA);
    let id = aplicar(&p, &cookie, &g).await;
    assert_eq!(clave_del_punto(&p, "1").await, CLAVE_2);

    let (status, body) = llamar(
        &p,
        "deshacer_ajuste",
        json!({ "cambio": id }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert!(
        !body.contains(CLAVE_2) && !body.contains(&original),
        "{body}"
    );
    let inverso: Value = serde_json::from_str(&body).unwrap();
    let campos = cambios(&inverso);
    assert_eq!(campos.len(), 1, "{inverso}");
    assert_eq!(campos[0]["antes"], panel::ui::explorador::MASCARA);
    assert_eq!(campos[0]["despues"], panel::ui::explorador::MASCARA);
    aplicar(&p, &cookie, &inverso).await;
    assert_eq!(clave_del_punto(&p, "1").await, original);
}

/// «Vacío = sin tocar» nunca acaba en un "" enviado: una clave vacía en la
/// petición no vale.
#[tokio::test]
async fn contrasena_vacia_no_se_envia() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, body) = preparar(&p, &cookie, "wifi.red_5", json!({ "contrasena": "" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(p.sim.control.writes().is_empty());
}

/// Un secreto de solo lectura (el de la web básica, mientras no se valide
/// su formato) no se escribe aunque la petición lo traiga, ni con la
/// máscara; se puede «Mostrar».
#[tokio::test]
async fn campo_secreto_solo_lectura_rechaza_escritura() {
    let toml = r#"
[[ajuste]]
id = "wifi.red_basica"
tema = "wifi"
nivel = "basico"
leer = { funcion = "cbGetWifiAccPt", params = { dpm_cfg_wifi_acc_pt_id = "1" } }

[[ajuste.campo]]
id = "nombre"
escribe = { funcion = "cbSetWifiAccPt", parametro = "ssid" }
lee = "ssid"
tipo = "texto"

[[ajuste.campo]]
id = "contrasena"
escribe = { funcion = "te_wifi.wl", parametro = "wlWpaPsk" }
lee = "key_phrase"
tipo = "secreto"
solo_lectura = "ajuste-puertos-alg-motivo"
"#;
    let (p, cookie) = panel_con_ajustes(toml).await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    for v in [CLAVE_2, panel::ui::explorador::MASCARA] {
        let (status, body) =
            preparar(&p, &cookie, "wifi.red_basica", json!({ "contrasena": v })).await;
        assert_eq!(status, 400, "{v}: {body}");
        let (status, body) = preparar(
            &p,
            &cookie,
            "wifi.red_basica",
            json!({ "nombre": "otra", "contrasena": v }),
        )
        .await;
        assert_eq!(status, 400, "con otro campo, {v}: {body}");
    }
    assert!(
        p.sim
            .control
            .writes()
            .iter()
            .all(|(f, _)| f != "te_wifi.wl")
    );
    assert_eq!(
        p.sim.control.writes().len(),
        1,
        "solo la escritura de la prueba"
    );
    // Lo que la página necesita: el motivo, y el valor para el ojo.
    let tema = ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    let campo = tema["ajustes"][0]["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "contrasena")
        .unwrap()
        .clone();
    assert_eq!(campo["solo_lectura"], "ajuste-puertos-alg-motivo");
    assert_eq!(campo["valor_secreto"], CLAVE_1);
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

/// La línea de voz con su contraseña (que el router da vacía) escrita: se
/// guarda, y va la escrita.
#[tokio::test]
async fn voz_con_la_clave_escrita_se_guarda() {
    let (p, cookie) = panel_con_gestor().await;
    assert_eq!(de_la_voz(&p, "auth_password").await, "");
    let tema = ok(&p, "leer_tema", json!({ "tema": "telefono" }), &cookie).await;
    let campo = campo_de(&tema, "telefono.linea", "contrasena");
    assert_eq!(campo["tapado"], true, "{campo}");
    // El router no la deja leer: el campo sale vacío.
    assert_eq!(campo["valor_secreto"], Value::Null, "{campo}");
    let (status, body) = preparar(
        &p,
        &cookie,
        "telefono.linea",
        json!({ "activa": "false", "contrasena": CLAVE_3 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert!(!body.contains(CLAVE_3));
    let g: Value = serde_json::from_str(&body).unwrap();
    aplicar(&p, &cookie, &g).await;
    let enviado = ultima_escritura(&p, "cbSetVoiceLine");
    assert!(
        enviado.contains(&("auth_password".into(), CLAVE_3.into())),
        "va la escrita"
    );
    assert_eq!(de_la_voz(&p, "adm_state").await, "0");
    // Como el router, el simulador sigue sin dejarla leer.
    assert_eq!(de_la_voz(&p, "auth_password").await, "");
}

/// Sin escribirla, no se guarda: 400 junto al campo, con el motivo, y nada
/// se escribe. Desde la Biblioteca, el gestor tampoco la reenvía vacía.
#[tokio::test]
async fn voz_con_la_clave_vacia_se_bloquea_con_el_motivo() {
    let (p, cookie) = panel_con_gestor().await;
    for valores in [
        json!({ "activa": "false" }),
        json!({ "activa": "false", "contrasena": panel::ui::explorador::MASCARA }),
    ] {
        let (status, body) = preparar(&p, &cookie, "telefono.linea", valores.clone()).await;
        assert_eq!(status, 400, "{valores}: {body}");
        assert!(body.contains("Contraseña de la línea"), "{body}");
        assert!(body.contains(PIDE_LA_CLAVE), "{body}");
    }
    let (status, body) = preparar(&p, &cookie, "telefono.linea", json!({ "contrasena": "" })).await;
    assert_eq!(status, 400, "{body}");
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetVoiceLine", "instancia": null,
                "valores": [["interwork_id", "0"], ["adm_state", "0"]], "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("clave"), "{body}");
    assert!(
        p.sim.control.writes().is_empty(),
        "{:?}",
        p.sim.control.writes()
    );
}

/// Las claves WEP no van marcadas: vacías (sin usar), se reenvían tal cual
/// y la WiFi se guarda.
#[tokio::test]
async fn wifi_con_wep_vacias_se_guarda() {
    let (p, cookie) = panel_con_gestor().await;
    let vacias: Vec<(&str, &str)> = (1..=4)
        .map(|i| match i {
            1 => ("wep_key1_data", ""),
            2 => ("wep_key2_data", ""),
            3 => ("wep_key3_data", ""),
            _ => ("wep_key4_data", ""),
        })
        .collect();
    escribir_punto(&p, "1", &vacias).await;
    let (status, body) = preparar(&p, &cookie, "wifi.red_5", json!({ "nombre": "otra-red" })).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    aplicar(&p, &cookie, &g).await;
    let enviado = ultima_escritura(&p, "cbSetWifiAccPt");
    for (k, _) in &vacias {
        assert!(
            enviado.contains(&((*k).into(), String::new())),
            "{k}: {enviado:?}"
        );
    }
}

/// El deshacer de un cambio de la línea no se bloquea: omite la contraseña
/// (su valor anterior es el vacío que dio el router) y vuelve lo demás.
#[tokio::test]
async fn deshacer_la_voz_omite_la_clave_tapada() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "telefono.linea",
        json!({ "activa": "false", "contrasena": CLAVE_3 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let id = aplicar(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let (status, body) = llamar(
        &p,
        "deshacer_ajuste",
        json!({ "cambio": id }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    aplicar(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let enviado = ultima_escritura(&p, "cbSetVoiceLine");
    assert!(
        enviado.iter().all(|(k, _)| k != "auth_password"),
        "{enviado:?}"
    );
    assert_eq!(de_la_voz(&p, "adm_state").await, "1");
    // El simulador, como el router, la da vacía al leerla: lo que cuenta es
    // que el deshacer no ha escrito ninguna (ni vacía).
    assert_eq!(de_la_voz(&p, "auth_password").await, "");
}

/// La reversión automática (la WAN cae mientras se vigila) tampoco se
/// bloquea: omite la contraseña y deja la línea como estaba.
#[tokio::test]
async fn revertir_la_voz_omite_la_clave_tapada() {
    let mut t = common::tiempos_rapidos();
    t.bajo = Duration::from_secs(5);
    t.medio = Duration::from_secs(5);
    let (p, cookie) = common::panel_con_tiempos(t).await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "telefono.linea",
        json!({ "activa": "false", "contrasena": CLAVE_3 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let id = ok(
        &p,
        "confirmar_ajuste",
        json!({ "token": g["token"], "desmarcados": [] }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    let inicio = std::time::Instant::now();
    while ok(&p, "estado_cambio", json!({ "id": id }), &cookie).await["estado"] != "Vigilando" {
        assert!(
            inicio.elapsed() < Duration::from_secs(5),
            "no llega a vigilarse"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    p.sim.control.set_wan_up(false);
    let c = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Revertido", "{c}");
    let enviado = ultima_escritura(&p, "cbSetVoiceLine");
    assert!(
        enviado.iter().all(|(k, _)| k != "auth_password"),
        "{enviado:?}"
    );
    assert_eq!(de_la_voz(&p, "adm_state").await, "1");
}

/// Una clave que el router da tapada (asteriscos o la máscara) tampoco se
/// reenvía al cambiar otro campo.
#[tokio::test]
async fn clave_tapada_no_se_reenvia() {
    let (p, cookie) = panel_con_gestor().await;
    for tapada in ["********", panel::ui::explorador::MASCARA] {
        escribir_punto(&p, "1", &[("key_phrase", tapada)]).await;
        let antes = p.sim.control.writes().len();
        let (status, body) =
            preparar(&p, &cookie, "wifi.red_5", json!({ "nombre": "otra-red" })).await;
        assert_eq!(status, 403, "{tapada}: {body}");
        assert_eq!(p.sim.control.writes().len(), antes, "{tapada}");
    }
    // Con una clave nueva escrita, sí (no se reenvía la tapada: va la nueva).
    let (status, body) = preparar(
        &p,
        &cookie,
        "wifi.red_5",
        json!({ "nombre": "otra-red", "contrasena": CLAVE_1 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
}

/// El valor de un secreto llega con la página (`leer_tema`, con `no-store`)
/// para el campo censurado con el ojo, también después de cambiarlo en el
/// router (sin la caché vieja). Ver un secreto ya no pide la contraseña ni
/// anota nada; lo que el router no deja leer (enmascarado o tapado al leer)
/// llega vacío.
#[tokio::test]
async fn los_secretos_llegan_con_la_pagina() {
    let (p, cookie) = panel_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    let (status, cache, body) = llamada(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
    let tema: Value = serde_json::from_str(&body).unwrap();
    let c = campo_de(&tema, "wifi.red_5", "contrasena");
    assert_eq!(c["valor"], "Secreto", "{c}");
    assert_eq!(c["valor_secreto"], CLAVE_1, "{c}");
    // Lo que no es secreto no lleva valor aparte.
    assert_eq!(
        campo_de(&tema, "wifi.red_5", "nombre")["valor_secreto"],
        Value::Null
    );
    // Uno que el router da enmascarado no es su valor: vacío.
    escribir_punto(&p, "1", &[("key_phrase", "********")]).await;
    p.state.lecturas.invalidar();
    let tema = ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    assert_eq!(
        campo_de(&tema, "wifi.red_5", "contrasena")["valor_secreto"],
        Value::Null
    );
    let filas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reveals")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert_eq!(filas, 0, "ver un secreto ya no se anota");
    // Las server functions para revelar ya no existen.
    for f in ["revelar_ajuste", "revelar_anterior", "revelar"] {
        let (status, _) = llamar(&p, f, json!({}), Some(&cookie)).await;
        assert_eq!(status, 404, "{f}");
    }
}

/// Un secreto que llega igual que lo leído no es un cambio: el resumen solo
/// enseña lo que cambia y, solo, no hay nada que guardar.
#[tokio::test]
async fn secreto_sin_cambiar_no_se_envia() {
    let (p, cookie) = panel_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "wifi.red_5",
        json!({ "nombre": "otra-red", "contrasena": CLAVE_1 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let campos = cambios(&g);
    assert_eq!(campos.len(), 1, "{g}");
    assert_eq!(campos[0]["campo"], "ajuste-wifi-red_5-nombre");
    let (status, body) =
        preparar(&p, &cookie, "wifi.red_5", json!({ "contrasena": CLAVE_1 })).await;
    assert_eq!(status, 409, "{body}");
}

/// Solo las páginas que enseñan secretos los llevan, y solo en el `value`
/// del campo (y en los datos de la página): nunca como texto a la vista. El
/// resto (Inicio, el Historial plegado, la Biblioteca sin abrir) no los
/// lleva, y nunca se guardan en claro.
#[tokio::test]
async fn secretos_solo_en_el_campo_de_las_paginas_que_los_muestran() {
    // El registro, antes de hacer nada: así recoge todo lo de este test.
    let registro = registro_global();
    let (p, cookie) = panel_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    cambiar_clave_5(&p, &cookie, CLAVE_2).await;
    // Dos veces: la segunda sale de la caché de lecturas (sin enmascarar).
    ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    let wifi = html(&p, &cookie, "/ajustes/wifi").await;
    // El ojo dice de qué campo es, y los gestores de contraseñas lo
    // ignoran.
    assert!(
        wifi.contains("aria-label=\"Mostrar Contraseña\""),
        "el nombre del ojo"
    );
    for marca in [
        "data-1p-ignore",
        "data-lpignore=\"true\"",
        "data-bwignore",
        "data-form-type=\"other\"",
    ] {
        assert!(wifi.contains(marca), "{marca}");
    }
    assert!(
        wifi.contains(CLAVE_2),
        "la página de la WiFi lleva la clave"
    );
    let (fuera, _) = partir(&wifi, "<script", "</script>");
    let visible = texto_visible(&fuera);
    assert!(!visible.contains(CLAVE_2), "a la vista: {visible}");
    let en_value = format!("value=\"{CLAVE_2}\"");
    assert_eq!(
        fuera.matches(CLAVE_2).count(),
        fuera.matches(&en_value).count(),
        "solo en el value del campo"
    );
    for ruta in ["/", "/historial", "/explorador", "/ajustes/puertos"] {
        let pagina = html(&p, &cookie, ruta).await;
        assert!(!pagina.contains(CLAVE_1), "{ruta}: la clave vieja");
        assert!(!pagina.contains(CLAVE_2), "{ruta}: la nueva");
    }
    let claro: Vec<String> = sqlx::query_scalar(
        "SELECT CAST(valores_anteriores AS TEXT) || CAST(valores_nuevos AS TEXT) || solicitud || diff \
         FROM cambio_pasos",
    )
    .fetch_all(&p.state.db)
    .await
    .unwrap();
    assert!(
        claro
            .iter()
            .all(|c| !c.contains(CLAVE_1) && !c.contains(CLAVE_2)),
        "{claro:?}"
    );
    // Ni en el registro, que sí ha recogido lo que se ha hecho.
    let texto = registro.texto();
    assert!(
        texto.contains("cbSetWifiAccPt"),
        "el registro no recoge nada: {texto}"
    );
    assert!(
        !texto.contains(CLAVE_1) && !texto.contains(CLAVE_2),
        "{texto}"
    );
}

/// El detalle de un cambio del Historial trae, al desplegarlo, el valor
/// anterior y el nuevo de cada secreto (descifrados solo para ese cambio,
/// con `no-store`); la lista del Historial no los trae nunca.
#[tokio::test]
async fn detalle_de_un_cambio_con_sus_secretos() {
    let (p, cookie) = panel_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    let id = cambiar_clave_5(&p, &cookie, CLAVE_2).await;
    let h = ok(&p, "historial", json!({ "pagina": 1 }), &cookie)
        .await
        .to_string();
    assert!(!h.contains(CLAVE_1) && !h.contains(CLAVE_2), "{h}");
    let (status, cache, body) =
        llamada(&p, "secretos_del_cambio", json!({ "cambio": id }), &cookie).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        v,
        json!([{ "orden": 0, "nombre": "key_phrase", "antes": CLAVE_1, "despues": CLAVE_2,
                 "tapado": false }]),
        "{v}"
    );
    // Sin sesión, nada; un cambio que no existe, vacío.
    let (status, _) = llamar(&p, "secretos_del_cambio", json!({ "cambio": id }), None).await;
    assert_eq!(status, 401);
    assert_eq!(
        ok(
            &p,
            "secretos_del_cambio",
            json!({ "cambio": id + 50 }),
            &cookie
        )
        .await,
        json!([])
    );
    let filas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reveals")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert_eq!(filas, 0);
}

/// La tabla de diferencias del Historial no enseña parámetros de la API: de
/// un cambio hecho desde Ajustes, la etiqueta del campo (la de Ajustes) en
/// los dos idiomas; de uno de la Biblioteca, el nombre técnico solo dentro
/// de los detalles técnicos plegados.
#[tokio::test]
async fn historial_con_las_etiquetas_de_ajustes() {
    let (p, cookie) = panel_con_gestor().await;
    let id = cambiar_clave_5(&p, &cookie, CLAVE_2).await;
    for (idioma, ajuste, campo) in [
        ("es", "Red WiFi 5 GHz", "Contraseña"),
        ("en", "5 GHz WiFi network", "Password"),
    ] {
        let con_idioma = format!("{cookie}; rp_lang={idioma}");
        let h = ok(&p, "historial", json!({ "pagina": 1 }), &con_idioma).await;
        let c = h["entradas"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|e| e.get("Cambio").filter(|c| c["id"] == id))
            .unwrap();
        assert_eq!(c["campos"][0]["etiqueta"], campo, "{idioma}: {c}");
        assert_eq!(c["titulo"], ajuste, "{idioma}: {c}");
        let pagina = html(&p, &con_idioma, "/historial").await;
        let fila = &pagina[pagina
            .find(&format!("id=\"cambio-{id}\""))
            .expect("la fila")..];
        let fila = &fila[..fila
            .find("<li class=\"hist-entrada")
            .unwrap_or(fila.find("</ol>").unwrap())];
        let (fuera, _) = partir(fila, "<details class=\"detalles-tecnicos\"", "</details>");
        let visible = texto_visible(&fuera);
        assert!(
            visible.contains(campo) && visible.contains(ajuste),
            "{idioma}: {visible}"
        );
        assert!(
            !visible.contains("key_phrase") && !visible.contains("cbSet"),
            "{idioma}: {visible}"
        );
    }
    // Uno de la Biblioteca: el nombre técnico, solo en los detalles.
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetUpnpCfg", "instancia": null,
                "valores": [["adm_state", "1"]], "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let prep: Value = serde_json::from_str(&body).unwrap();
    let otro = ok(
        &p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    esperar_final(&p, &cookie, otro).await;
    let pagina = html(&p, &cookie, "/historial").await;
    let fila = &pagina[pagina
        .find(&format!("id=\"cambio-{otro}\""))
        .expect("la fila")..];
    let fila = &fila[..fila
        .find("<li class=\"hist-entrada")
        .unwrap_or(fila.find("</ol>").unwrap())];
    let (fuera, tecnicos) = partir(fila, "<details class=\"detalles-tecnicos\"", "</details>");
    assert!(!texto_visible(&fuera).contains("adm_state"), "{fuera}");
    assert!(
        tecnicos.iter().any(|t| t.contains("adm_state")),
        "{tecnicos:?}"
    );
}

/// Una contraseña mala en el diálogo de reautenticación (la que se sigue
/// pidiendo para la contraseña del router y para exportar) cuenta para el
/// mismo freno que el login.
#[tokio::test]
async fn el_freno_es_el_mismo_que_el_del_login() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, _) = llamar(
        &p,
        "reautenticar",
        json!({ "contrasena": "mala" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 403);
    let resp = send(
        &p.app,
        common::post_form(
            "/api/login",
            &common::login_body(common::PANEL_USER, PANEL_PASS),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 429);
}

/// `html` sin los bloques `<abre …>…</cierra>` (sin anidar), y esos bloques.
fn partir<'a>(html: &'a str, abre: &str, cierra: &str) -> (String, Vec<&'a str>) {
    let mut fuera = String::new();
    let mut dentro = Vec::new();
    let mut resto = html;
    while let Some(i) = resto.find(abre) {
        fuera.push_str(&resto[..i]);
        let fin = i + resto[i..].find(cierra).expect("bloque sin cerrar") + cierra.len();
        dentro.push(&resto[i..fin]);
        resto = &resto[fin..];
    }
    fuera.push_str(resto);
    (fuera, dentro)
}

/// Texto que se lee en pantalla de un trozo de HTML: lo de fuera de las
/// etiquetas (sin los atributos).
fn texto_visible(html: &str) -> String {
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

/// Un cambio que solo tocó la clave de la voz no se puede deshacer: su valor
/// anterior es el vacío que dio el router. No se ofrece «Deshacer», y la
/// fila dice por qué (en los dos idiomas); el deshacer, pedido igualmente,
/// da 400 con ese motivo. Su detalle no enseña un «antes».
#[tokio::test]
async fn deshacer_solo_la_clave_tapada_no_se_ofrece() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "telefono.linea",
        json!({ "contrasena": CLAVE_3 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let id = aplicar(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    for (idioma, motivo) in [
        ("es", "no se puede volver atrás"),
        ("en", "can't be put back"),
    ] {
        let con_idioma = format!("{cookie}; rp_lang={idioma}");
        let h = ok(&p, "historial", json!({ "pagina": 1 }), &con_idioma).await;
        let c = h["entradas"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|e| e.get("Cambio").filter(|c| c["id"] == id))
            .unwrap();
        assert_eq!(c["puede_deshacer"], false, "{c}");
        assert!(
            c["sin_deshacer"]
                .as_str()
                .unwrap_or_default()
                .contains(motivo),
            "{idioma}: {c}"
        );
    }
    let (status, body) = llamar(
        &p,
        "deshacer_ajuste",
        json!({ "cambio": id }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("no se puede volver atrás"), "{body}");
    let (status, _) = llamar(&p, "deshacer", json!({ "change_id": id }), Some(&cookie)).await;
    assert_eq!(status, 400);
    let v = ok(&p, "secretos_del_cambio", json!({ "cambio": id }), &cookie).await;
    assert_eq!(
        v,
        json!([{ "orden": 0, "nombre": "auth_password", "antes": null, "despues": CLAVE_3,
                 "tapado": true }])
    );
}

/// En un cambio mixto (la línea y su clave), deshacer vuelve la línea y
/// avisa de que la clave se queda como se escribió.
#[tokio::test]
async fn deshacer_mixto_avisa_de_la_clave_tapada() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "telefono.linea",
        json!({ "activa": "false", "contrasena": CLAVE_3 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let id = aplicar(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let h = ok(&p, "historial", json!({ "pagina": 1 }), &cookie).await;
    let c = h["entradas"][0]["Cambio"].clone();
    assert_eq!(c["puede_deshacer"], true, "{c}");
    assert_eq!(c["sin_deshacer"], Value::Null, "{c}");
    let g = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    let aviso = g["aviso"].as_str().unwrap_or_default();
    assert!(
        aviso.contains("Contraseña de la línea") && aviso.contains("se queda"),
        "{g}"
    );
    let prep = ok(&p, "deshacer", json!({ "change_id": id }), &cookie).await;
    let aviso = prep["aviso"].as_str().unwrap_or_default();
    assert!(
        aviso.contains("Contraseña de la línea") && aviso.contains("se queda"),
        "{prep}"
    );
}
