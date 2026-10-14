//! Red local: los aparatos conectados (`leer_dispositivos`) y las IP fijas
//! (reservas DHCP) creadas, borradas y deshechas como grupos de cambios,
//! con la validación de una reserva (`validar_reserva`) igual en el
//! servidor y en la página.

mod common;

use common::{
    Panel, esperar_final, esperar_libre, llamar, ok, panel_con_gestor, panel_con_gestor_y_capa,
    sesion, variante,
};
use panel::ajustes_router::{ErrorReserva, validar_reserva};
use serde_json::{Value, json};
use std::net::Ipv4Addr;

// ------------------------------------------------------ validar_reserva

/// La red de casa de fábrica: router 192.168.1.1/24.
const LAN: (Ipv4Addr, Ipv4Addr) = (
    Ipv4Addr::new(192, 168, 1, 1),
    Ipv4Addr::new(255, 255, 255, 0),
);

fn existentes() -> Vec<(String, String, String)> {
    [
        ("0", "02:00:00:00:00:04", "192.168.1.68"),
        ("1", "02:00:00:00:00:01", "192.168.1.69"),
    ]
    .iter()
    .map(|(i, m, ip)| ((*i).to_owned(), (*m).to_owned(), (*ip).to_owned()))
    .collect()
}

fn ip(s: &str) -> Ipv4Addr {
    s.parse().unwrap()
}

#[test]
fn reserva_valida() {
    let r = validar_reserva(
        "02:00:00:00:00:03",
        ip("192.168.1.62"),
        LAN,
        &existentes(),
        None,
    );
    assert_eq!(r, Ok(()));
}

#[test]
fn fuera_de_la_red() {
    for otra in ["192.0.2.10", "198.51.100.62", "203.0.113.1"] {
        let r = validar_reserva("02:00:00:00:00:03", ip(otra), LAN, &existentes(), None);
        assert_eq!(r, Err(ErrorReserva::FueraDeLaRed), "{otra}");
    }
}

#[test]
fn ip_del_router() {
    let r = validar_reserva(
        "02:00:00:00:00:03",
        ip("192.168.1.1"),
        LAN,
        &existentes(),
        None,
    );
    assert_eq!(r, Err(ErrorReserva::IpDelRouter));
}

#[test]
fn ip_repetida() {
    let r = validar_reserva(
        "02:00:00:00:00:03",
        ip("192.168.1.69"),
        LAN,
        &existentes(),
        None,
    );
    assert_eq!(r, Err(ErrorReserva::IpRepetida));
}

/// La MAC se compara sin mirar mayúsculas ni el separador.
#[test]
fn mac_repetida() {
    for mac in [
        "02:00:00:00:00:04",
        "02-00-00-00-00-04",
        "02:00:00:00:00:04",
    ] {
        let r = validar_reserva(mac, ip("192.168.1.90"), LAN, &existentes(), None);
        assert_eq!(r, Err(ErrorReserva::MacRepetida), "{mac}");
    }
    let mut mayusculas = existentes();
    mayusculas[0].1 = "02:00:00:00:00:0A".into();
    let r = validar_reserva(
        "02:00:00:00:00:0a",
        ip("192.168.1.90"),
        LAN,
        &mayusculas,
        None,
    );
    assert_eq!(r, Err(ErrorReserva::MacRepetida));
}

/// La primera y la última dirección de la red son de toda la red (en una
/// red de documentación, 192.0.2.0/24).
#[test]
fn difusion() {
    let lan = (Ipv4Addr::new(192, 0, 2, 1), Ipv4Addr::new(255, 255, 255, 0));
    for especial in ["192.0.2.255", "192.0.2.0"] {
        let r = validar_reserva("02:00:00:00:00:03", ip(especial), lan, &[], None);
        assert_eq!(r, Err(ErrorReserva::Difusion), "{especial}");
    }
    let r = validar_reserva("02:00:00:00:00:03", ip("192.0.2.254"), lan, &[], None);
    assert_eq!(r, Ok(()));
}

/// Al editar una reserva, su propia MAC e IP no cuentan como repetidas;
/// las de las demás, sí.
#[test]
fn editar_la_misma_no_es_repetida() {
    let e = existentes();
    let r = validar_reserva("02:00:00:00:00:04", ip("192.168.1.68"), LAN, &e, Some("0"));
    assert_eq!(r, Ok(()));
    let r = validar_reserva("02:00:00:00:00:04", ip("192.168.1.70"), LAN, &e, Some("0"));
    assert_eq!(r, Ok(()));
    let r = validar_reserva("02:00:00:00:00:04", ip("192.168.1.69"), LAN, &e, Some("0"));
    assert_eq!(r, Err(ErrorReserva::IpRepetida));
    let r = validar_reserva("02:00:00:00:00:01", ip("192.168.1.68"), LAN, &e, Some("0"));
    assert_eq!(r, Err(ErrorReserva::MacRepetida));
}

// ------------------------------------------------------- en el simulador

async fn dispositivos(p: &Panel, cookie: &str) -> Vec<Value> {
    ok(p, "leer_dispositivos", json!({}), cookie)
        .await
        .as_array()
        .unwrap()
        .clone()
}

fn por_mac<'a>(lista: &'a [Value], mac: &str) -> &'a Value {
    lista
        .iter()
        .find(|d| d["mac"] == mac)
        .unwrap_or_else(|| panic!("falta {mac}: {lista:?}"))
}

/// Reservas de `leer_tema` como triples `(id, mac, ip)`.
async fn reservas(p: &Panel, cookie: &str) -> Vec<(String, String, String)> {
    let tema = ok(p, "leer_tema", json!({ "tema": "red_local" }), cookie).await;
    let a = tema["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "red_local.reservas")
        .unwrap()
        .clone();
    assert_eq!(a["estado"], "Leido", "{a}");
    let valor = |i: &Value, campo: &str| {
        let c = i["campos"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == campo)
            .unwrap();
        c["valor"]["Texto"].as_str().unwrap_or_default().to_owned()
    };
    a["instancias"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["id"].as_str().unwrap().to_owned(),
                valor(i, "mac"),
                valor(i, "ip"),
            )
        })
        .collect()
}

/// `preparar_ajuste` de las reservas: estado y cuerpo.
async fn preparar(
    p: &Panel,
    cookie: &str,
    instancia: Option<&str>,
    borrar: bool,
    valores: Value,
) -> (u16, String) {
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": {
            "ajuste": "red_local.reservas",
            "instancia": instancia,
            "borrar": borrar,
            "valores": valores,
        } }),
        Some(cookie),
    )
    .await
}

/// Confirma el resumen y espera a que acabe aplicado (y el gestor libre).
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

/// Deshace el cambio `id` y lo aplica.
async fn deshacer(p: &Panel, cookie: &str, id: i64) {
    let g = ok(p, "deshacer_ajuste", json!({ "cambio": id }), cookie).await;
    aplicar(p, cookie, &g).await;
}

/// «Fijar esta IP»: la reserva nueva con la MAC y la IP actuales del
/// aparato, activa. Aparece en las reservas y en el aparato; deshacerla la
/// borra.
#[tokio::test]
async fn fijar_ip_crea_reserva_y_deshacer_la_borra() {
    let (p, cookie) = panel_con_gestor().await;
    let antes = reservas(&p, &cookie).await;
    let d = dispositivos(&p, &cookie).await;
    let host = por_mac(&d, "02:00:00:00:00:03");
    assert_eq!(host["reserva"], Value::Null, "{host}");
    let (mac, ipv4) = (host["mac"].clone(), host["ip"].clone());
    let (status, body) = preparar(
        &p,
        &cookie,
        None,
        false,
        json!({ "mac": mac, "ip": ipv4, "activa": "true" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["titulo"], "ajuste-red_local-reservas-titulo");
    let id = aplicar(&p, &cookie, &g).await;

    let ahora = reservas(&p, &cookie).await;
    assert_eq!(ahora.len(), antes.len() + 1, "{ahora:?}");
    let nueva = ahora
        .iter()
        .find(|(_, m, _)| m == "02:00:00:00:00:03")
        .unwrap_or_else(|| panic!("sin la reserva nueva: {ahora:?}"));
    assert_eq!(nueva.2, "192.168.1.62");
    let d = dispositivos(&p, &cookie).await;
    assert_eq!(
        por_mac(&d, "02:00:00:00:00:03")["reserva"],
        json!(nueva.0),
        "{d:?}"
    );

    deshacer(&p, &cookie, id).await;
    assert_eq!(reservas(&p, &cookie).await, antes);
    let d = dispositivos(&p, &cookie).await;
    assert_eq!(por_mac(&d, "02:00:00:00:00:03")["reserva"], Value::Null);
}

#[tokio::test]
async fn borrar_reserva_y_deshacer_la_recrea() {
    let (p, cookie) = panel_con_gestor().await;
    let antes = reservas(&p, &cookie).await;
    assert!(
        antes.contains(&(
            "1".into(),
            "02:00:00:00:00:01".into(),
            "192.168.1.69".into()
        )),
        "{antes:?}"
    );
    let (status, body) = preparar(&p, &cookie, Some("1"), true, json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let id = aplicar(&p, &cookie, &g).await;
    let sin = reservas(&p, &cookie).await;
    assert!(
        !sin.iter().any(|(_, m, _)| m == "02:00:00:00:00:01"),
        "{sin:?}"
    );
    assert_eq!(sin.len(), antes.len() - 1);

    deshacer(&p, &cookie, id).await;
    let otra_vez = reservas(&p, &cookie).await;
    assert_eq!(otra_vez.len(), antes.len(), "{otra_vez:?}");
    assert!(
        otra_vez
            .iter()
            .any(|(_, m, ip)| m == "02:00:00:00:00:01" && ip == "192.168.1.69"),
        "{otra_vez:?}"
    );
}

/// Una reserva que no vale no se prepara (400, junto al campo y en el
/// idioma) y no escribe nada.
#[tokio::test]
async fn reserva_invalida_da_400() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        None,
        false,
        json!({ "mac": "02:00:00:00:00:03", "ip": "192.0.2.10", "activa": "true" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("Dirección IP"), "{body}");
    assert!(body.contains("no es de tu red"), "{body}");
    let en = format!("{cookie}; rp_lang=en");
    let (status, body) = preparar(
        &p,
        &en,
        None,
        false,
        json!({ "mac": "02:00:00:00:00:03", "ip": "192.0.2.10", "activa": "true" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("isn't on your network"), "{body}");
    // La IP del router, una MAC que ya tiene reserva y, al editar, la IP de
    // otra reserva.
    for (instancia, valores, texto) in [
        (
            None,
            json!({ "mac": "02:00:00:00:00:03", "ip": "192.168.1.1", "activa": "true" }),
            "IP del router",
        ),
        (
            None,
            json!({ "mac": "02:00:00:00:00:04", "ip": "192.168.1.90", "activa": "true" }),
            "ya tiene una IP fija",
        ),
        (
            Some("0"),
            json!({ "ip": "192.168.1.69" }),
            "IP fija de otro aparato",
        ),
    ] {
        let (status, body) = preparar(&p, &cookie, instancia, false, valores).await;
        assert_eq!(status, 400, "{body}");
        assert!(body.contains(texto), "{texto}: {body}");
    }
    // Una nueva sin IP.
    let (status, body) = preparar(
        &p,
        &cookie,
        None,
        false,
        json!({ "mac": "02:00:00:00:00:03", "activa": "true" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("Dirección IP"), "{body}");
    assert!(p.sim.control.writes().is_empty());
    // Editar la misma reserva con su propia IP no es repetirla.
    let (status, body) =
        preparar(&p, &cookie, Some("0"), false, json!({ "activa": "false" })).await;
    assert_eq!(status, 200, "{body}");
}

/// Cada aparato con su nombre, su conexión (la banda por el punto de acceso
/// al que está asociado; si no, el cable) y su reserva.
#[tokio::test]
async fn leer_dispositivos_con_banda() {
    let (p, cookie) = panel_con_gestor_y_capa(&variante("dispositivos-wifi")).await;
    let d = dispositivos(&p, &cookie).await;
    assert_eq!(d.len(), 10, "{d:?}");
    let conexion = |mac: &str| por_mac(&d, mac)["conexion"].clone();
    assert_eq!(conexion("02:00:00:00:00:03"), json!("Wifi24"));
    assert_eq!(conexion("02:00:00:00:00:06"), json!("Wifi5"));
    assert_eq!(conexion("02:00:00:00:00:05"), json!("Invitados"));
    assert_eq!(conexion("02:00:00:00:00:01"), json!({ "Cable": "1:3" }));
    let apagado = por_mac(&d, "02:00:00:00:00:02");
    assert_eq!(apagado["activo"], false);
    assert_eq!(apagado["conexion"], json!("Desconocida"));
    assert_eq!(apagado["nombre"], "host-1");
    assert_eq!(apagado["ip"], "192.168.1.61");
    let reservado = por_mac(&d, "02:00:00:00:00:04");
    assert_eq!(reservado["reserva"], "0");
    assert_eq!(reservado["nombre"], "host-3");
    assert_eq!(por_mac(&d, "02:00:00:00:00:01")["reserva"], "1");
    // Los conectados primero, por IP.
    assert_eq!(d[0]["mac"], "02:00:00:00:00:01");
    assert_eq!(d.last().unwrap()["activo"], false);
}

#[tokio::test]
async fn leer_dispositivos_exige_sesion() {
    let (p, _) = panel_con_gestor().await;
    let (status, body) = llamar(&p, "leer_dispositivos", json!({}), None).await;
    assert_eq!(status, 401, "{body}");
}

/// La página de Red local pinta los dos componentes (sin el hueco de
/// «todavía no está lista»).
#[tokio::test]
async fn pagina_red_local_con_dispositivos_y_reservas() {
    let p = common::panel().await;
    let cookie = sesion(&p).await;
    let resp = common::get_con_cookie(&p.app, "/ajustes/red_local", &cookie).await;
    assert_eq!(resp.status(), 200);
    let html = common::body_text(resp).await;
    assert!(html.contains("Fijar esta IP"), "{html}");
    assert!(html.contains("Añadir IP fija"), "{html}");
    assert!(html.contains("192.168.1.62"), "{html}");
    assert!(!html.contains("todavía no está lista"), "{html}");
}
