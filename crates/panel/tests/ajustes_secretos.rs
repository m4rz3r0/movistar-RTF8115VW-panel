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

fn mostrar(ajuste: &str, campo: &str, contrasena: &str) -> Value {
    json!({ "ajuste": ajuste, "campo": campo, "instancia": null, "contrasena": contrasena })
}

/// Filas de `reveals`: (ajuste, campo, cambio_id, funcion).
async fn revelados(p: &Panel) -> Vec<(Option<String>, String, Option<i64>, String)> {
    sqlx::query_as("SELECT ajuste, campo, cambio_id, funcion FROM reveals ORDER BY id")
        .fetch_all(&p.state.db)
        .await
        .unwrap()
}

/// Todo lo que guarda `reveals`, junto.
async fn todo_reveals(p: &Panel) -> String {
    let filas: Vec<String> = sqlx::query_scalar(
        "SELECT COALESCE(funcion,'') || '|' || COALESCE(campo,'') || '|' || \
         COALESCE(instancia,'') || '|' || COALESCE(ajuste,'') || '|' || \
         COALESCE(CAST(cambio_id AS TEXT),'') FROM reveals",
    )
    .fetch_all(&p.state.db)
    .await
    .unwrap();
    filas.join("\n")
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
async fn mostrar_sin_reautenticacion_401() {
    let (p, cookie) = panel_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    let (status, cache, body) = llamada(
        &p,
        "revelar_ajuste",
        mostrar("wifi.red_5", "contrasena", ""),
        &cookie,
    )
    .await;
    assert_eq!(status, 401, "{body}");
    assert_eq!(cache, "no-store");
    assert!(body.contains(panel::auth::ERROR_REAUTENTICACION), "{body}");
    assert!(!body.contains(CLAVE_1));
    assert!(revelados(&p).await.is_empty());
    // Sin sesión, tampoco.
    let (status, _) = llamar(
        &p,
        "revelar_ajuste",
        mostrar("wifi.red_5", "contrasena", PANEL_PASS),
        None,
    )
    .await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn mostrar_con_reautenticacion_no_store_y_registro() {
    let (p, cookie) = panel_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    let registro = registro_global();
    let (status, cache, body) = llamada(
        &p,
        "revelar_ajuste",
        mostrar("wifi.red_5", "contrasena", PANEL_PASS),
        &cookie,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
    assert_eq!(serde_json::from_str::<String>(&body).unwrap(), CLAVE_1);
    let filas = revelados(&p).await;
    assert_eq!(filas.len(), 1, "{filas:?}");
    assert_eq!(filas[0].0.as_deref(), Some("wifi.red_5"));
    assert_eq!(filas[0].1, "contrasena");
    assert_eq!(filas[0].2, None);
    assert!(!todo_reveals(&p).await.contains(CLAVE_1));

    // Con la ventana abierta no vuelve a pedirla, y no sale de la caché de
    // lecturas: tras leer el tema y cambiar la clave en el router, enseña la
    // nueva.
    ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_2)]).await;
    let (status, cache, body) = llamada(
        &p,
        "revelar_ajuste",
        mostrar("wifi.red_5", "contrasena", ""),
        &cookie,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
    assert_eq!(serde_json::from_str::<String>(&body).unwrap(), CLAVE_2);
    assert_eq!(revelados(&p).await.len(), 2, "cada «Mostrar» se anota");

    // Ni el registro ni `reveals` guardan el valor.
    let texto = registro.texto();
    assert!(texto.contains("wifi.red_5"), "{texto}");
    assert!(
        !texto.contains(CLAVE_1) && !texto.contains(CLAVE_2),
        "{texto}"
    );
    assert!(!todo_reveals(&p).await.contains(CLAVE_2));

    // Un campo que no es secreto, o que no existe, no se «muestra».
    for (ajuste, campo) in [("wifi.red_5", "nombre"), ("wifi.red_5", "no_existe")] {
        let (status, _, body) =
            llamada(&p, "revelar_ajuste", mostrar(ajuste, campo, ""), &cookie).await;
        assert!(status == 400 || status == 404, "{campo}: {status} {body}");
    }
    let (status, _, _) = llamada(
        &p,
        "revelar_ajuste",
        mostrar("wifi.no_existe", "contrasena", ""),
        &cookie,
    )
    .await;
    assert_eq!(status, 404);
    assert_eq!(revelados(&p).await.len(), 2);
}

#[tokio::test]
async fn mostrar_con_contrasena_mala_frena_como_el_login() {
    let (p, cookie) = panel_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    let (status, _, body) = llamada(
        &p,
        "revelar_ajuste",
        mostrar("wifi.red_5", "contrasena", "mala"),
        &cookie,
    )
    .await;
    assert_eq!(status, 403, "{body}");
    assert!(!body.contains(CLAVE_1));
    // El freno del login: justo después, ni con la buena.
    let (status, _, body) = llamada(
        &p,
        "revelar_ajuste",
        mostrar("wifi.red_5", "contrasena", PANEL_PASS),
        &cookie,
    )
    .await;
    assert_eq!(status, 429, "{body}");
    assert!(!body.contains(CLAVE_1));
    assert!(revelados(&p).await.is_empty());
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let v = ok(
        &p,
        "revelar_ajuste",
        mostrar("wifi.red_5", "contrasena", PANEL_PASS),
        &cookie,
    )
    .await;
    assert_eq!(v, CLAVE_1);
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
solo_lectura = "ajustes-secreto-basica-motivo"
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
    // Lo que la página necesita: el motivo, y «Mostrar» sí funciona.
    let tema = ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    let campo = tema["ajustes"][0]["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "contrasena")
        .unwrap()
        .clone();
    assert_eq!(campo["solo_lectura"], "ajustes-secreto-basica-motivo");
    let v = ok(
        &p,
        "revelar_ajuste",
        mostrar("wifi.red_basica", "contrasena", PANEL_PASS),
        &cookie,
    )
    .await;
    assert_eq!(v, CLAVE_1);
}

#[tokio::test]
async fn revelar_anterior_de_un_paso() {
    let (p, cookie) = panel_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    let id = cambiar_clave_5(&p, &cookie, CLAVE_2).await;
    let anterior = |contrasena: &str| json!({ "cambio": id, "orden": 0, "campo": "key_phrase", "contrasena": contrasena });
    // Se exige reautenticación.
    let (status, cache, body) = llamada(&p, "revelar_anterior", anterior(""), &cookie).await;
    assert_eq!(status, 401, "{body}");
    assert_eq!(cache, "no-store");
    assert!(!body.contains(CLAVE_1));
    let (status, cache, body) =
        llamada(&p, "revelar_anterior", anterior(PANEL_PASS), &cookie).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
    assert_eq!(serde_json::from_str::<String>(&body).unwrap(), CLAVE_1);
    let filas = revelados(&p).await;
    assert_eq!(filas.len(), 1, "{filas:?}");
    assert_eq!(filas[0].1, "key_phrase");
    assert_eq!(filas[0].2, Some(id));
    assert_eq!(filas[0].3, "cbSetWifiAccPt");
    assert!(!todo_reveals(&p).await.contains(CLAVE_1));

    // El Historial ofrece «Mostrar el valor anterior» en ese paso.
    let h = ok(&p, "historial", json!({ "pagina": 1 }), &cookie).await;
    let texto = h.to_string();
    assert!(
        !texto.contains(CLAVE_1) && !texto.contains(CLAVE_2),
        "{texto}"
    );
    let fila = h["entradas"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|e| e.get("Cambio").filter(|c| c["id"] == id))
        .unwrap_or_else(|| panic!("falta el cambio {id}: {h}"));
    assert_eq!(fila["anteriores"], json!([[0, "key_phrase"]]), "{fila}");
    // El revelado, con el cambio al que pertenece.
    let revelado = h["entradas"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|e| e.get("Revelado"))
        .unwrap();
    assert_eq!(revelado["cambio"], id, "{revelado}");

    // Un campo que no es secreto, otro paso o un cambio que no existe: nada.
    for cuerpo in [
        json!({ "cambio": id, "orden": 0, "campo": "ssid", "contrasena": "" }),
        json!({ "cambio": id, "orden": 1, "campo": "key_phrase", "contrasena": "" }),
        json!({ "cambio": id + 50, "orden": 0, "campo": "key_phrase", "contrasena": "" }),
    ] {
        let (status, _, body) = llamada(&p, "revelar_anterior", cuerpo.clone(), &cookie).await;
        assert!(status == 400 || status == 404, "{cuerpo}: {status} {body}");
    }
    assert_eq!(revelados(&p).await.len(), 1);
}

/// El Historial muestra un revelado de Ajustes con el título del ajuste y
/// el campo, sin la función.
#[tokio::test]
async fn historial_con_el_titulo_del_ajuste() {
    let (p, cookie) = panel_con_gestor().await;
    ok(
        &p,
        "revelar_ajuste",
        mostrar("wifi.red_5", "contrasena", PANEL_PASS),
        &cookie,
    )
    .await;
    let h = ok(&p, "historial", json!({ "pagina": 1 }), &cookie).await;
    let r = h["entradas"][0]["Revelado"].clone();
    assert_eq!(r["ajuste"], "Red WiFi 5 GHz", "{r}");
    assert_eq!(r["campo"], "Contraseña", "{r}");
    assert_eq!(r["ruta"], Value::Null, "sin enlace a la función: {r}");
    let pagina = html(&p, &cookie, "/historial").await;
    let fila = &pagina[pagina.find("hist-revelado").expect("la fila")..];
    let fila = &fila[..fila.find("</li>").unwrap()];
    assert!(
        fila.contains("Red WiFi 5 GHz") && fila.contains("Contraseña"),
        "{fila}"
    );
    assert!(
        !fila.contains("cbGetWifiAccPt") && !fila.contains("key_phrase"),
        "{fila}"
    );
}

#[tokio::test]
async fn html_nunca_contiene_secretos() {
    let (p, cookie) = panel_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    ok(
        &p,
        "revelar_ajuste",
        mostrar("wifi.red_5", "contrasena", PANEL_PASS),
        &cookie,
    )
    .await;
    let id = cambiar_clave_5(&p, &cookie, CLAVE_2).await;
    ok(
        &p,
        "revelar_anterior",
        json!({ "cambio": id, "orden": 0, "campo": "key_phrase", "contrasena": "" }),
        &cookie,
    )
    .await;
    for ruta in ["/ajustes/wifi", "/historial"] {
        let pagina = html(&p, &cookie, ruta).await;
        assert!(!pagina.contains(CLAVE_1), "{ruta}: la clave vieja");
        assert!(!pagina.contains(CLAVE_2), "{ruta}: la nueva");
    }
    // Tampoco lo que lee la página después.
    let tema = ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie)
        .await
        .to_string();
    assert!(!tema.contains(CLAVE_1) && !tema.contains(CLAVE_2));
    // Ni lo que se guarda en claro del cambio.
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
}

/// Cambiar otro campo de la línea de voz no reenvía su contraseña: el
/// router la da vacía, y reenviarla podría borrarla. Mientras no se valide
/// con un router real, la escritura se bloquea con su motivo, también desde
/// la Biblioteca.
#[tokio::test]
async fn voz_no_reenvia_su_contrasena_vacia() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, body) =
        preparar(&p, &cookie, "telefono.linea", json!({ "activa": "false" })).await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("contraseña"), "{body}");
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetVoiceLine", "instancia": null,
                "valores": [["interwork_id", "0"], ["adm_state", "0"]], "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 403, "{body}");
    assert!(
        p.sim.control.writes().is_empty(),
        "{:?}",
        p.sim.control.writes()
    );
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
