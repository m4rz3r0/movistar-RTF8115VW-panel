//! Conexión a internet (Tarea 4 de la fase 3): la tarjeta de solo lectura de Internet con las conexiones, sus secretos y la fibra.

use crate::ajustes_secretos::llamada;
use crate::comun::{Panel, ajustes_de};
use crate::comun::{PanelPrueba, aplicar_ajuste, leer, ok, pagina, registro_global, variante};
use serde_json::{Value, json};

/// Valores de la capa `conexion-secretos`.
const CLAVE_PPP: &str = "clave-ppp-de-prueba";
const SERIE: &str = "SERIE-DE-PRUEBA";

/// El ajuste `id` de la página `tema`.
fn ajuste<'a>(tema: &'a Value, id: &str) -> &'a Value {
    ajustes_de(tema)
        .into_iter()
        .find(|a| a["id"] == id)
        .unwrap_or_else(|| panic!("sin {id}: {tema}"))
}

/// El campo `campo` de una instancia o de un ajuste suelto.
fn campo<'a>(campos: &'a Value, id: &str) -> &'a Value {
    campos["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("sin {id}: {campos}"))
}

/// El elemento `id` de la página `tema`.
fn elemento<'a>(tema: &'a Value, id: &str) -> &'a Value {
    tema["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .unwrap_or_else(|| panic!("sin {id}: {tema}"))
}

/// Todo lo guardado en el Historial, como texto.
async fn historial_en_texto(p: &Panel) -> String {
    let filas: Vec<String> = sqlx::query_scalar(
        "SELECT COALESCE(diff, '') || COALESCE(anteriores, '') || COALESCE(error, '') || \
         COALESCE(nota, '') || COALESCE(solicitud, '') || COALESCE(nuevos, '') FROM changes",
    )
    .fetch_all(&p.state.db)
    .await
    .unwrap();
    filas.join("\n")
}

#[tokio::test]
async fn conexion_a_internet_con_sus_secretos() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let tema = leer(&p, &cookie, "internet").await;
    let a = ajuste(&tema, "internet.conexion");
    assert_eq!(a["componente"], "conexion", "{a}");
    let instancias = a["instancias"].as_array().unwrap();
    assert_eq!(instancias.len(), 3, "{a}");
    let i = &instancias[0];
    assert_eq!(i["id"], "2", "{i}");
    assert_eq!(campo(i, "tipo")["valor"], json!({ "Opcion": "pppoe" }));
    assert_eq!(campo(i, "ip")["valor"], json!({ "Texto": "203.0.113.6" }));
    assert_eq!(campo(i, "vlan")["valor"], json!({ "Texto": "6" }));
    assert_eq!(campo(i, "mtu")["valor"], json!({ "Texto": "1492" }));
    assert_eq!(campo(i, "ipv6")["valor"], json!({ "Booleano": true }));
    assert_eq!(
        campo(i, "usuario")["valor"],
        json!({ "Texto": "X-REDACTED-X" })
    );
    let clave = campo(i, "contrasena");
    assert_eq!(clave["valor"], "Secreto", "{clave}");
    assert_eq!(clave["valor_secreto"], "X-REDACTED-X", "{clave}");
    // Todo de solo lectura.
    for c in i["campos"].as_array().unwrap() {
        assert!(c["solo_lectura"].is_string(), "{c}");
    }
    let notas: Vec<&Value> = instancias.iter().map(|i| &i["nota"]).collect();
    let del_operador = json!("ajuste-internet-conexion-del-operador");
    assert_eq!(notas, [&Value::Null, &del_operador, &del_operador]);
    let fibra = ajuste(&tema, "internet.fibra");
    assert_eq!(campo(fibra, "estado")["valor"], json!({ "Opcion": "o5" }));
    assert_eq!(campo(fibra, "serie")["valor"], "Secreto", "{fibra}");
    let e = elemento(&tema, "conexion");
    assert_eq!(e["lectura"], "Leido", "{e}");
    assert_eq!(e["estado"], "emite", "{e}");
    assert_eq!(e["texto_estado"], "Conectada (PPPoE)", "{e}");
}

#[tokio::test]
async fn conexion_secretos_no_salen_del_sitio() {
    let registro = registro_global();
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("conexion-secretos"))
        .arrancar_con_gestor()
        .await;
    let (status, cache, body) =
        llamada(&p, "leer_tema", json!({ "tema": "internet" }), &cookie).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
    // Llegan solo para el campo con el ojo.
    let tema: Value = serde_json::from_str(&body).unwrap();
    let i = &ajuste(&tema, "internet.conexion")["instancias"][0];
    assert_eq!(campo(i, "contrasena")["valor_secreto"], CLAVE_PPP, "{i}");
    let fibra = ajuste(&tema, "internet.fibra");
    assert_eq!(campo(fibra, "serie")["valor_secreto"], SERIE, "{fibra}");
    // Otra vez (de la caché), la página y un cambio en el mismo tema.
    leer(&p, &cookie, "internet").await;
    pagina(&p, &cookie, "/ajustes/internet").await;
    let g = ok(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "internet.dns", "instancia": null, "borrar": false,
                       "valores": { "rele": "true" } } }),
        &cookie,
    )
    .await;
    aplicar_ajuste(&p, &cookie, &g).await;
    let historial = historial_en_texto(&p).await;
    assert!(!historial.is_empty(), "el cambio está en el Historial");
    for x in [CLAVE_PPP, SERIE] {
        assert!(!registro.texto().contains(x), "{x} en el registro");
        assert!(!historial.contains(x), "{x} en el Historial");
        let inicio = pagina(&p, &cookie, "/").await;
        assert!(!inicio.contains(x), "{x} en Inicio");
    }
}

#[tokio::test]
async fn conexion_sin_leer_con_motivo() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.sim.control.fail_reads("cbGetWanIPIntf", true);
    let tema = leer(&p, &cookie, "internet").await;
    let e = elemento(&tema, "conexion");
    let motivo = e["lectura"]["SinLeer"]
        .as_str()
        .unwrap_or_else(|| panic!("leído: {e}"));
    assert!(!motivo.is_empty());
    assert_eq!(e["texto_estado"], "Sin leer", "{e}");
    let api = regex::Regex::new(r"cb[A-Z]").unwrap();
    // Ni en el motivo ni en lo que se ve de la tarjeta (los detalles
    // técnicos plegados aparte).
    let visible = json!({
        "lectura": e["lectura"],
        "texto_estado": e["texto_estado"],
        "datos": e["datos"],
        "pie": e["pie"],
        "estados": e["ajustes"].as_array().unwrap().iter().map(|a| &a["estado"]).collect::<Vec<_>>(),
    })
    .to_string();
    assert!(!api.is_match(&visible), "{visible}");
    // La fibra y el resto de Internet se leen.
    assert_eq!(ajuste(&tema, "internet.fibra")["estado"], "Leido");
    assert_eq!(elemento(&tema, "dns")["lectura"], "Leido");
}

/// Fixtures de serie del simulador.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../hgu/fixtures");

/// Capa con `cbGetWanIPIntf` de serie cambiado por `cambiar`.
fn capa_wan(dir: &std::path::Path, cambiar: impl Fn(String) -> String) {
    let wan = std::fs::read_to_string(format!("{FIXTURES}/cbGetWanIPIntf.xml")).unwrap();
    std::fs::write(dir.join("cbGetWanIPIntf.xml"), cambiar(wan)).unwrap();
}

/// Las notas de las instancias de las conexiones, en su orden.
fn notas(tema: &Value) -> Vec<Value> {
    ajuste(tema, "internet.conexion")["instancias"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["nota"].clone())
        .collect()
}

/// La PPP caída pierde su dirección: la conexión de internet sigue siendo
/// la del panel (la detectada al instalar), que sale «Sin conexión», y las
/// demás siguen siendo del operador.
#[tokio::test]
async fn conexion_ppp_caida_sin_conexion() {
    let capa = tempfile::tempdir().unwrap();
    capa_wan(capa.path(), |wan| {
        wan.replacen(
            "<opr_status>1</opr_status>",
            "<opr_status>0</opr_status>",
            1,
        )
        .replacen(
            "<dis_ipv4_addr>203.0.113.6</dis_ipv4_addr>",
            "<dis_ipv4_addr></dis_ipv4_addr>",
            1,
        )
    });
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(capa.path())
        .arrancar_con_gestor()
        .await;
    assert_eq!(
        p.state.wan().as_deref(),
        Some("ip2"),
        "detectada al instalar"
    );
    let tema = leer(&p, &cookie, "internet").await;
    let del_operador = json!("ajuste-internet-conexion-del-operador");
    assert_eq!(
        notas(&tema),
        [Value::Null, del_operador.clone(), del_operador]
    );
    assert_eq!(
        ajuste(&tema, "internet.conexion")["instancias"][0]["id"],
        "2"
    );
    let e = elemento(&tema, "conexion");
    assert_eq!(e["estado"], "atencion", "{e}");
    assert_eq!(e["texto_estado"], "Sin conexión", "{e}");
}

/// La conexión elegida a mano en el panel manda, como en el gestor.
#[tokio::test]
async fn conexion_sigue_la_elegida_a_mano() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.state.fijar_red(p.state.red(), Some("ip4".into()));
    let tema = leer(&p, &cookie, "internet").await;
    let i = &ajuste(&tema, "internet.conexion")["instancias"];
    assert_eq!(i[0]["id"], "4", "{i}");
    assert_eq!(i[0]["nota"], Value::Null, "{i}");
    assert_eq!(i[1]["nota"], "ajuste-internet-conexion-del-operador", "{i}");
}

/// Sin conexión del panel y con dos conexiones de datos con dirección de
/// internet, no se sabe cuál es: ninguna se titula «Tu conexión a
/// internet» y la tarjeta dice «Sin datos» (la lectura sí funcionó).
#[tokio::test]
async fn conexion_sin_identificar_sin_datos() {
    let capa = tempfile::tempdir().unwrap();
    capa_wan(capa.path(), |wan| {
        wan.replace(
            "<opr_app_mask>4</opr_app_mask>",
            "<opr_app_mask>1</opr_app_mask>",
        )
    });
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(capa.path())
        .arrancar_con_gestor()
        .await;
    p.state.fijar_red(None, None);
    let tema = leer(&p, &cookie, "internet").await;
    let sin = json!("ajuste-internet-conexion-sin-identificar");
    assert_eq!(notas(&tema), [sin.clone(), sin.clone(), sin]);
    let e = elemento(&tema, "conexion");
    assert_eq!(e["lectura"], "Leido", "{e}");
    assert_eq!(e["estado"], "atencion", "{e}");
    assert_eq!(e["texto_estado"], "Sin datos", "{e}");
}
