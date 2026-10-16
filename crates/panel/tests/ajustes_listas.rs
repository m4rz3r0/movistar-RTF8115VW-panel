//! Red local: los dispositivos conectados (`leer_dispositivos`) y las IP fijas
//! (reservas DHCP) creadas, borradas y deshechas como grupos de cambios,
//! con la validación de una reserva (`validar_reserva`) igual en el
//! servidor y en la página.

mod common;

use common::{
    Panel, esperar_final, esperar_libre, llamar, ok, panel_con_gestor, panel_con_gestor_y_capa,
    sesion, variante,
};
use panel::ajustes_router::{ErrorReserva, validar_reserva};
use panel::ajustes_router::{PeticionAjuste, valida_la_reserva};
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
        " 02-00-00-00-00-04 ",
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

/// `preparar_ajuste` de las reservas: estado y cuerpo. Al editar o borrar,
/// envía como la página la MAC que tiene ahora la reserva en el router (si
/// existe).
async fn preparar(
    p: &Panel,
    cookie: &str,
    instancia: Option<&str>,
    borrar: bool,
    valores: Value,
) -> (u16, String) {
    let mut esperados = json!({});
    if let Some(id) = instancia {
        let actuales = hgu::read::static_leases(&p.state.hgu).await.unwrap();
        if let Some(r) = actuales.iter().find(|r| r.id == id) {
            esperados = json!({ "mac": r.mac });
        }
    }
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": {
            "ajuste": "red_local.reservas",
            "instancia": instancia,
            "borrar": borrar,
            "valores": valores,
            "esperados": esperados,
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
/// dispositivo, activa. Aparece en las reservas y en el dispositivo; deshacerla la
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
            "IP fija de otro dispositivo",
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

/// Cada dispositivo con su nombre, su conexión (la banda por el punto de acceso
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

// ------------------------------------------- ronda de corrección 1

/// HTML de `/ajustes/red_local`.
async fn pagina(p: &Panel, cookie: &str) -> String {
    let resp = common::get_con_cookie(&p.app, "/ajustes/red_local", cookie).await;
    assert_eq!(resp.status(), 200);
    common::body_text(resp).await
}

/// El `<article>` de la página cuyo título es `titulo` (sin lo que llega
/// después por streaming).
fn tarjeta<'a>(html: &'a str, titulo: &str) -> &'a str {
    html.split("<article")
        .find(|a| a.contains(&format!(">{titulo}</h2>")))
        .unwrap_or_else(|| panic!("sin la tarjeta {titulo}"))
        .split("</article>")
        .next()
        .unwrap()
}

/// El fragmento que el servidor envía aparte (streaming fuera de orden, un
/// `<template>`) con `marca`: lo que pinta una tarjeta tras leer la red.
fn fragmento<'a>(html: &'a str, marca: &str) -> &'a str {
    html.split("<template")
        .map(|t| t.split("</template>").next().unwrap())
        .find(|t| t.contains(marca) && !t.contains("<article"))
        .unwrap_or_else(|| panic!("sin el fragmento con {marca}"))
}

/// Lo que se ve de los dispositivos y de las IP fijas, ya leídos.
fn lista_de_dispositivos(html: &str) -> &str {
    fragmento(html, r#"class="dispositivos""#)
}

fn ip_fijas(html: &str) -> &str {
    fragmento(html, "tabla-apilada")
}

/// El nombre accesible de «Fijar esta IP» empieza por lo que se ve
/// (WCAG 2.5.3): quien lo pide por voz lo encuentra.
#[tokio::test]
async fn fijar_esta_ip_se_nombra_como_se_ve() {
    let (p, cookie) = panel_con_gestor().await;
    let html = pagina(&p, &cookie).await;
    assert!(
        html.contains(r#"aria-label="Fijar esta IP: 192.168.1.62 (host-2)""#),
        "{html}"
    );
}

/// Un dispositivo con IP fija dice cuál es, aunque ahora tenga otra.
#[tokio::test]
async fn ya_tiene_ip_fija_dice_la_reservada() {
    let (p, cookie) = panel_con_gestor().await;
    let d = dispositivos(&p, &cookie).await;
    assert_eq!(por_mac(&d, "02:00:00:00:00:01")["ip"], "192.168.1.60");
    assert_eq!(por_mac(&d, "02:00:00:00:00:01")["ip_fija"], "192.168.1.69");
    assert_eq!(por_mac(&d, "02:00:00:00:00:03")["ip_fija"], Value::Null);
    let html = pagina(&p, &cookie).await;
    assert!(html.contains("Ya tiene IP fija: 192.168.1.69"), "{html}");
}

/// Sin «Fijar esta IP» donde el servidor la rechazaría siempre (una IP de
/// otra red, la de otra reserva) ni para un dispositivo desconectado (su última
/// IP puede tenerla ya otro): una nota breve en su lugar.
#[tokio::test]
async fn sin_fijar_si_no_se_puede() {
    let (p, cookie) = panel_con_gestor_y_capa(&variante("dispositivos-irregulares")).await;
    // Otra reserva con la IP de host-2.
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "-1"),
                ("mac_addr", "02:00:00:00:00:0b"),
                ("ipv4_addr", "192.168.1.62"),
                ("adm_state", "1"),
            ],
        )
        .await
        .unwrap();
    let html = pagina(&p, &cookie).await;
    let dispositivos = lista_de_dispositivos(&html);
    for ip in ["198.51.100.76", "192.168.1.61", "192.168.1.62"] {
        assert!(
            !dispositivos.contains(&format!("Fijar esta IP: {ip} ")),
            "{ip}: {dispositivos}"
        );
    }
    assert!(dispositivos.contains("Fijar esta IP: 192.168.1.64 (host-4)"));
    assert_eq!(dispositivos.matches("Esta IP no se puede fijar").count(), 2);
    assert_eq!(
        dispositivos.matches("Conéctalo para fijar su IP").count(),
        1
    );
}

/// El nombre del dispositivo de una reserva se busca con las MAC normalizadas.
#[tokio::test]
async fn nombre_de_la_reserva_con_mac_normalizada() {
    let (p, cookie) = panel_con_gestor_y_capa(&variante("dispositivos-irregulares")).await;
    let html = pagina(&p, &cookie).await;
    let fijas = ip_fijas(&html);
    let fila = fijas
        .split("<tr")
        .find(|f| f.contains("02:00:00:00:00:04"))
        .unwrap_or_else(|| panic!("{fijas}"));
    assert!(fila.contains("host-3"), "{fila}");
}

/// Una sola región de alerta por tarjeta, no una por dispositivo.
#[tokio::test]
async fn una_alerta_por_tarjeta() {
    let (p, cookie) = panel_con_gestor().await;
    let html = pagina(&p, &cookie).await;
    let alertas = |t: &str| t.matches(r#"role="alert""#).count();
    let n =
        alertas(tarjeta(&html, "Dispositivos conectados")) + alertas(lista_de_dispositivos(&html));
    assert_eq!(n, 1, "dispositivos");
    let n = alertas(tarjeta(&html, "IP fijas")) + alertas(ip_fijas(&html));
    assert_eq!(n, 1, "IP fijas");
}

/// Desactivar una reserva que ya no valdría (comparte IP con otra) no exige
/// corregir la IP; activarla, sí.
#[tokio::test]
async fn desactivar_sin_corregir_la_ip() {
    let (p, cookie) = panel_con_gestor().await;
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("ipv4_addr", "192.168.1.69"),
                ("adm_state", "1"),
            ],
        )
        .await
        .unwrap();
    let (status, body) =
        preparar(&p, &cookie, Some("0"), false, json!({ "activa": "false" })).await;
    assert_eq!(status, 200, "{body}");
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("adm_state", "0"),
            ],
        )
        .await
        .unwrap();
    let (status, body) = preparar(&p, &cookie, Some("0"), false, json!({ "activa": "true" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("IP fija de otro dispositivo"), "{body}");
}

/// Qué peticiones de una reserva se validan: una nueva, cambiar la MAC o la
/// IP y activarla; desactivarla o borrarla, no.
#[test]
fn que_se_valida_de_una_reserva() {
    let p = |instancia: Option<&str>, borrar: bool, valores: &[(&str, &str)]| PeticionAjuste {
        ajuste: "red_local.reservas".into(),
        instancia: instancia.map(str::to_owned),
        borrar,
        valores: valores
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        esperados: Default::default(),
    };
    assert!(valida_la_reserva(&p(
        None,
        false,
        &[("mac", "x"), ("ip", "y")]
    )));
    assert!(valida_la_reserva(&p(Some("0"), false, &[("ip", "y")])));
    assert!(valida_la_reserva(&p(Some("0"), false, &[("mac", "x")])));
    assert!(valida_la_reserva(&p(
        Some("0"),
        false,
        &[("activa", "true")]
    )));
    assert!(!valida_la_reserva(&p(
        Some("0"),
        false,
        &[("activa", "false")]
    )));
    assert!(!valida_la_reserva(&p(Some("0"), true, &[])));
}

/// El resumen de una reserva nombra el dispositivo, si se conoce.
#[tokio::test]
async fn el_resumen_nombra_el_dispositivo() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        None,
        false,
        json!({ "mac": "02:00:00:00:00:03", "ip": "192.168.1.62", "activa": "true" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["detalle"], "host-2", "{g}");
    let (status, body) = preparar(&p, &cookie, Some("0"), true, json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["detalle"], "host-3", "{g}");
}

/// `preparar_ajuste` con lo que la página espera de la instancia.
async fn preparar_esperando(
    p: &Panel,
    cookie: &str,
    instancia: &str,
    borrar: bool,
    valores: Value,
    mac: &str,
) -> (u16, String) {
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": {
            "ajuste": "red_local.reservas",
            "instancia": instancia,
            "borrar": borrar,
            "valores": valores,
            "esperados": { "mac": mac },
        } }),
        Some(cookie),
    )
    .await
}

/// Editar o borrar una reserva que otra sesión ha cambiado entretanto (se
/// borró la de id máximo y se creó otra, que el router numera igual): 409
/// y nada escrito. Con la que se esperaba, sí se prepara.
#[tokio::test]
async fn editar_o_borrar_otra_reserva_da_409() {
    let (p, cookie) = panel_con_gestor().await;
    let base = &[("rtm_cfg_dhcpv4_svr_pool_id", "0")];
    let escribir = |extra: &[(&'static str, &'static str)]| {
        let mut params = base.to_vec();
        params.extend_from_slice(extra);
        params
    };
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &escribir(&[("rtm_dhcpv4_svr_pool_static_id", "1"), ("del", "DEL")]),
        )
        .await
        .unwrap();
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &escribir(&[
                ("rtm_dhcpv4_svr_pool_static_id", "-1"),
                ("mac_addr", "02:00:00:00:00:0b"),
                ("ipv4_addr", "192.168.1.90"),
            ]),
        )
        .await
        .unwrap();
    assert!(
        reservas(&p, &cookie).await.contains(&(
            "1".into(),
            "02:00:00:00:00:0b".into(),
            "192.168.1.90".into()
        )),
        "el sim reutiliza el id"
    );
    let escritas = p.sim.control.writes().len();
    let (status, body) =
        preparar_esperando(&p, &cookie, "1", true, json!({}), "02:00:00:00:00:01").await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("ha cambiado"), "{body}");
    let (status, body) = preparar_esperando(
        &p,
        &cookie,
        "1",
        false,
        json!({ "activa": "false" }),
        "02-00-00-00-00-01",
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(p.sim.control.writes().len(), escritas);
    let (status, body) =
        preparar_esperando(&p, &cookie, "1", true, json!({}), "02-00-00-00-00-0B").await;
    assert_eq!(status, 200, "{body}");
}

/// Editar una reserva que no existe y crear una con la IP de difusión.
#[tokio::test]
async fn reserva_inexistente_o_de_difusion_da_400() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, body) =
        preparar(&p, &cookie, Some("77"), false, json!({ "activa": "false" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("no ha podido preparar"), "{body}");
    let (status, body) = preparar(
        &p,
        &cookie,
        None,
        false,
        json!({ "mac": "02:00:00:00:00:03", "ip": "192.168.1.0", "activa": "true" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("es de toda la red"), "{body}");
    assert!(p.sim.control.writes().is_empty());
}

/// Editar o borrar una reserva exige la MAC que la página espera: sin
/// ella, el servidor no puede saber si la reserva sigue siendo la misma
/// (400, nada escrito).
#[tokio::test]
async fn editar_o_borrar_sin_mac_esperada_da_400() {
    let (p, cookie) = panel_con_gestor().await;
    for (borrar, valores) in [(true, json!({})), (false, json!({ "activa": "false" }))] {
        let (status, body) = llamar(
            &p,
            "preparar_ajuste",
            json!({ "p": {
                "ajuste": "red_local.reservas",
                "instancia": "1",
                "borrar": borrar,
                "valores": valores,
            } }),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, 400, "borrar={borrar}: {body}");
        assert!(body.contains("no ha podido preparar"), "{body}");
    }
    assert!(p.sim.control.writes().is_empty());
}

// =========================================== Tarea 10: apertura de puertos

use panel::ajustes_router::{
    DispositivoVista, ErrorRegla, Protocolo, ReglaPuertos, destino_inexistente, validar_regla,
};

fn dispositivo(ip: &str, activo: bool) -> DispositivoVista {
    DispositivoVista {
        nombre: "host-2".into(),
        ip: ip.into(),
        mac: "02:00:00:00:00:03".into(),
        conexion: panel::ajustes_router::Conexion::Desconocida,
        activo,
        reserva: None,
        ip_fija: None,
    }
}

#[test]
fn destino_de_un_dispositivo_o_de_una_ip_fija_existe() {
    let d = [
        dispositivo("192.168.1.62", true),
        dispositivo("192.168.1.61", false),
    ];
    let r = existentes();
    assert!(!destino_inexistente("192.168.1.62", &d, &r), "conectado");
    assert!(!destino_inexistente("192.168.1.61", &d, &r), "desconectado");
    assert!(!destino_inexistente("192.168.1.69", &d, &r), "IP fija");
    assert!(
        !destino_inexistente(" 192.168.1.68 ", &[], &r),
        "con espacios"
    );
    assert!(destino_inexistente("192.168.1.75", &d, &r), "de nadie");
    assert!(destino_inexistente("192.168.1.75", &[], &[]), "sin nada");
}

fn regla(id: Option<&str>, protocolo: Protocolo, externo: (u16, u16)) -> ReglaPuertos {
    ReglaPuertos {
        id: id.map(str::to_owned),
        nombre: "Consola".into(),
        protocolo,
        externo,
        interno: externo,
        ip: ip("192.168.1.62"),
        activa: true,
    }
}

/// Las reglas del router de las pruebas: 8080 TCP y 3074–3075 UDP.
fn reglas_existentes() -> Vec<ReglaPuertos> {
    vec![
        regla(Some("0"), Protocolo::Tcp, (8080, 8080)),
        regla(Some("1"), Protocolo::Udp, (3074, 3075)),
    ]
}

#[test]
fn regla_valida() {
    let e = reglas_existentes();
    for r in [
        regla(None, Protocolo::Tcp, (9000, 9000)),
        regla(None, Protocolo::Udp, (8080, 8080)),
        regla(None, Protocolo::Tcp, (3074, 3075)),
        regla(None, Protocolo::Ambos, (9000, 9010)),
    ] {
        assert_eq!(validar_regla(&r, &e, LAN), Ok(()), "{r:?}");
    }
    // Otro puerto interno del mismo tamaño.
    let mut r = regla(None, Protocolo::Tcp, (9000, 9001));
    r.interno = (80, 81);
    assert_eq!(validar_regla(&r, &e, LAN), Ok(()));
}

#[test]
fn puerto_cero() {
    let r = regla(None, Protocolo::Tcp, (0, 10));
    assert_eq!(validar_regla(&r, &[], LAN), Err(ErrorRegla::PuertoCero));
    let mut r = regla(None, Protocolo::Tcp, (10, 10));
    r.interno = (0, 0);
    assert_eq!(validar_regla(&r, &[], LAN), Err(ErrorRegla::PuertoCero));
}

#[test]
fn rango_invertido() {
    let r = regla(None, Protocolo::Tcp, (9010, 9000));
    assert_eq!(validar_regla(&r, &[], LAN), Err(ErrorRegla::RangoInvertido));
    let mut r = regla(None, Protocolo::Tcp, (9000, 9000));
    r.interno = (90, 80);
    assert_eq!(validar_regla(&r, &[], LAN), Err(ErrorRegla::RangoInvertido));
}

#[test]
fn rangos_distintos() {
    let mut r = regla(None, Protocolo::Tcp, (9000, 9010));
    r.interno = (9000, 9005);
    assert_eq!(
        validar_regla(&r, &[], LAN),
        Err(ErrorRegla::RangosDistintos)
    );
}

#[test]
fn solapa() {
    let e = reglas_existentes();
    for r in [
        regla(None, Protocolo::Tcp, (8080, 8080)),
        regla(None, Protocolo::Tcp, (8000, 8080)),
        regla(None, Protocolo::Udp, (3075, 3080)),
        regla(None, Protocolo::Ambos, (8080, 8080)),
        regla(None, Protocolo::Ambos, (3070, 3074)),
        // Editar la 1 con los puertos de la 0.
        regla(Some("1"), Protocolo::Tcp, (8080, 8080)),
    ] {
        assert_eq!(validar_regla(&r, &e, LAN), Err(ErrorRegla::Solapa), "{r:?}");
    }
    // Editar la 0 sin cambiar sus puertos no se solapa consigo misma.
    let r = regla(Some("0"), Protocolo::Tcp, (8080, 8080));
    assert_eq!(validar_regla(&r, &e, LAN), Ok(()));
}

#[test]
fn fuera_de_la_red_regla() {
    for otra in ["192.0.2.10", "192.168.1.1", "192.168.1.0"] {
        let mut r = regla(None, Protocolo::Tcp, (9000, 9000));
        r.ip = ip(otra);
        assert_eq!(
            validar_regla(&r, &[], LAN),
            Err(ErrorRegla::FueraDeLaRed),
            "{otra}"
        );
    }
    // La última dirección es de toda la red (en una red de documentación).
    let doc = (Ipv4Addr::new(192, 0, 2, 1), Ipv4Addr::new(255, 255, 255, 0));
    let mut r = regla(None, Protocolo::Tcp, (9000, 9000));
    r.ip = ip("192.0.2.255");
    assert_eq!(validar_regla(&r, &[], doc), Err(ErrorRegla::FueraDeLaRed));
    r.ip = ip("192.0.2.254");
    assert_eq!(validar_regla(&r, &[], doc), Ok(()));
}

// ------------------------------------------------- puertos en el simulador

/// Reglas de `leer_tema` como `(id, nombre, protocolo, externo_desde,
/// destino)`.
async fn reglas_del_tema(p: &Panel, cookie: &str) -> Vec<[String; 5]> {
    let tema = ok(p, "leer_tema", json!({ "tema": "puertos" }), cookie).await;
    let a = tema["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "puertos.reglas")
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
        let v = &c["valor"];
        v["Texto"]
            .as_str()
            .or(v["Opcion"].as_str())
            .unwrap_or_default()
            .to_owned()
    };
    a["instancias"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            [
                i["id"].as_str().unwrap().to_owned(),
                valor(i, "nombre"),
                valor(i, "protocolo"),
                valor(i, "externo_desde"),
                valor(i, "destino"),
            ]
        })
        .collect()
}

async fn preparar_regla(
    p: &Panel,
    cookie: &str,
    instancia: Option<&str>,
    borrar: bool,
    valores: Value,
    esperados: Value,
) -> (u16, String) {
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": {
            "ajuste": "puertos.reglas",
            "instancia": instancia,
            "borrar": borrar,
            "valores": valores,
            "esperados": esperados,
        } }),
        Some(cookie),
    )
    .await
}

/// Una regla nueva completa hacia host-2 (192.168.1.62).
fn nueva(protocolo: &str, puerto: &str) -> Value {
    json!({
        "nombre": "Consola",
        "protocolo": protocolo,
        "externo_desde": puerto,
        "externo_hasta": puerto,
        "destino": "192.168.1.62",
        "interno_desde": puerto,
        "interno_hasta": puerto,
        "activa": "true",
    })
}

/// Lo que identifica la regla 0 de las fixtures (8080 TCP a .75).
fn esperados_0() -> Value {
    json!({ "protocolo": "tcp", "externo_desde": "8080", "destino": "192.168.1.75" })
}

#[tokio::test]
async fn abrir_puerto_y_deshacer() {
    let (p, cookie) = panel_con_gestor().await;
    let antes = reglas_del_tema(&p, &cookie).await;
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("tcp", "9000"), json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["titulo"], "ajuste-puertos-reglas-titulo");
    assert_eq!(g["detalle"], "Consola", "{g}");
    assert_eq!(g["pasos"].as_array().unwrap().len(), 1, "{g}");
    let id = aplicar(&p, &cookie, &g).await;
    let ahora = reglas_del_tema(&p, &cookie).await;
    assert_eq!(ahora.len(), antes.len() + 1, "{ahora:?}");
    let n = ahora
        .iter()
        .find(|r| r[3] == "9000")
        .expect("la regla nueva");
    assert_eq!(n[1..], ["Consola", "tcp", "9000", "192.168.1.62"]);
    deshacer(&p, &cookie, id).await;
    assert_eq!(reglas_del_tema(&p, &cookie).await, antes);
}

/// «Ambos»: dos reglas (TCP y UDP) en un único grupo, con un solo diálogo y
/// un solo deshacer que borra las dos.
#[tokio::test]
async fn abrir_puerto_en_ambos_es_un_grupo_y_un_deshacer() {
    let (p, cookie) = panel_con_gestor().await;
    let antes = reglas_del_tema(&p, &cookie).await;
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("ambos", "9000"), json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(pasos.len(), 2, "{g}");
    let protocolo = |paso: &Value| {
        paso["campos"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["campo"] == "ajuste-puertos-reglas-protocolo")
            .map(|c| c["despues"].clone())
    };
    assert_eq!(protocolo(&pasos[0]), Some(json!("tcp")), "{g}");
    assert_eq!(protocolo(&pasos[1]), Some(json!("udp")), "{g}");
    let id = aplicar(&p, &cookie, &g).await;
    let ahora = reglas_del_tema(&p, &cookie).await;
    let nuevas: Vec<&[String; 5]> = ahora.iter().filter(|r| r[3] == "9000").collect();
    assert_eq!(nuevas.len(), 2, "{ahora:?}");
    assert!(nuevas.iter().any(|r| r[2] == "tcp") && nuevas.iter().any(|r| r[2] == "udp"));
    assert_ne!(nuevas[0][0], nuevas[1][0], "dos instancias distintas");
    deshacer(&p, &cookie, id).await;
    assert_eq!(reglas_del_tema(&p, &cookie).await, antes);
}

/// Una regla que abre un puerto que ya abre otra con el mismo protocolo no
/// se prepara (400, junto al campo y en el idioma) y no escribe nada. Con
/// el otro protocolo, sí.
#[tokio::test]
async fn regla_solapada_da_400() {
    let (p, cookie) = panel_con_gestor().await;
    for protocolo in ["tcp", "ambos"] {
        let (status, body) = preparar_regla(
            &p,
            &cookie,
            None,
            false,
            nueva(protocolo, "8080"),
            json!({}),
        )
        .await;
        assert_eq!(status, 400, "{protocolo}: {body}");
        assert!(body.contains("ya está abierto"), "{body}");
    }
    let en = format!("{cookie}; rp_lang=en");
    let (status, body) =
        preparar_regla(&p, &en, None, false, nueva("tcp", "8080"), json!({})).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("already open"), "{body}");
    // Fuera de la red y rango al revés.
    let mut fuera = nueva("tcp", "9000");
    fuera["destino"] = json!("192.0.2.10");
    let (status, body) = preparar_regla(&p, &cookie, None, false, fuera, json!({})).await;
    assert_eq!(status, 400, "{body}");
    assert!(
        body.contains("no es la de un dispositivo de tu red"),
        "{body}"
    );
    let mut al_reves = nueva("tcp", "9000");
    al_reves["externo_hasta"] = json!("8990");
    al_reves["interno_hasta"] = json!("8990");
    let (status, body) = preparar_regla(&p, &cookie, None, false, al_reves, json!({})).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("menor"), "{body}");
    // A una nueva no le puede faltar el destino.
    let mut sin_destino = nueva("tcp", "9000");
    sin_destino.as_object_mut().unwrap().remove("destino");
    let (status, body) = preparar_regla(&p, &cookie, None, false, sin_destino, json!({})).await;
    assert_eq!(status, 400, "{body}");
    assert!(p.sim.control.writes().is_empty());
    // Con UDP no choca.
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("udp", "8080"), json!({})).await;
    assert_eq!(status, 200, "{body}");
}

/// Editar o borrar una regla exige lo que la identifica (protocolo, puerto
/// de internet y destino): sin ello, 400; si el router ya tiene otra cosa,
/// 409; nada escrito. «Ambos» solo vale al crear.
#[tokio::test]
async fn editar_o_borrar_regla_con_esperados() {
    let (p, cookie) = panel_con_gestor().await;
    for borrar in [true, false] {
        let valores = if borrar {
            json!({})
        } else {
            json!({ "activa": "false" })
        };
        let (status, body) =
            preparar_regla(&p, &cookie, Some("0"), borrar, valores.clone(), json!({})).await;
        assert_eq!(status, 400, "sin esperados, borrar={borrar}: {body}");
        let (status, body) = preparar_regla(
            &p,
            &cookie,
            Some("0"),
            borrar,
            valores.clone(),
            json!({ "protocolo": "tcp", "externo_desde": "8080" }),
        )
        .await;
        assert_eq!(status, 400, "a medias, borrar={borrar}: {body}");
        let (status, body) = preparar_regla(
            &p,
            &cookie,
            Some("0"),
            borrar,
            valores.clone(),
            json!({ "protocolo": "tcp", "externo_desde": "8081", "destino": "192.168.1.75" }),
        )
        .await;
        assert_eq!(status, 409, "otra, borrar={borrar}: {body}");
        assert!(body.contains("ha cambiado"), "{body}");
    }
    let (status, body) = preparar_regla(
        &p,
        &cookie,
        Some("0"),
        false,
        json!({ "protocolo": "ambos" }),
        esperados_0(),
    )
    .await;
    assert_eq!(status, 400, "ambos al editar: {body}");
    let (status, body) =
        preparar_regla(&p, &cookie, Some("77"), true, json!({}), esperados_0()).await;
    assert_eq!(status, 400, "no existe: {body}");
    assert!(p.sim.control.writes().is_empty());
    // Con lo que tiene, sí; y desactivarla no la valida (su destino ya no
    // está, pero eso no impide apagarla).
    let (status, body) =
        preparar_regla(&p, &cookie, Some("0"), true, json!({}), esperados_0()).await;
    assert_eq!(status, 200, "{body}");
    let (status, body) = preparar_regla(
        &p,
        &cookie,
        Some("0"),
        false,
        json!({ "activa": "false" }),
        esperados_0(),
    )
    .await;
    assert_eq!(status, 200, "{body}");
}

/// La regla de las fixtures va a 192.168.1.75, que no es de ningún
/// dispositivo ni de ninguna IP fija: la página lo avisa.
#[tokio::test]
async fn aviso_destino_inexistente_en_la_pagina() {
    let p = common::panel().await;
    let cookie = sesion(&p).await;
    let resp = common::get_con_cookie(&p.app, "/ajustes/puertos", &cookie).await;
    assert_eq!(resp.status(), 200);
    let html = common::body_text(resp).await;
    assert!(
        html.contains("El dispositivo de destino ya no está en la red"),
        "{html}"
    );
    assert!(!html.contains("todavía no está lista"), "{html}");
    assert!(html.contains("Abrir un puerto"), "{html}");
}

#[tokio::test]
async fn leer_reservas_de_la_cache() {
    let (p, cookie) = panel_con_gestor().await;
    let r = ok(&p, "leer_reservas", json!({}), &cookie).await;
    // Solo las IP: es lo único que usa la página.
    assert_eq!(r, json!(["192.168.1.68", "192.168.1.69"]));
    let (status, _) = llamar(&p, "leer_reservas", json!({}), None).await;
    assert_eq!(status, 401);
}

// ------------------------------------------- ronda de corrección 1 (T10)

/// «TCP y UDP» con la segunda escritura fallida: el gestor revierte la
/// primera (borra la regla TCP recién creada) y el grupo queda fallido; la
/// lista vuelve a ser la de antes.
#[tokio::test]
async fn ambos_con_la_segunda_escritura_fallida_no_deja_nada() {
    let (p, cookie) = panel_con_gestor().await;
    let antes = reglas_del_tema(&p, &cookie).await;
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("ambos", "9000"), json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    p.sim.control.fail_write_n(2);
    let (status, body) = llamar(
        &p,
        "confirmar_ajuste",
        json!({ "token": g["token"], "desmarcados": [] }),
        Some(&cookie),
    )
    .await;
    assert_ne!(status, 200, "{body}");
    esperar_libre(&p).await;
    let historial = p.state.cambios().unwrap();
    let ultimo = historial
        .historial(5)
        .await
        .into_iter()
        .next()
        .expect("el cambio");
    assert_eq!(ultimo.estado, panel::changes::Estado::Fallido, "{ultimo:?}");
    assert_eq!(reglas_del_tema(&p, &cookie).await, antes);
}

/// Editar una regla valida lo que resulta de mezclar lo pedido con lo que
/// tiene el router: chocar con otra regla o salir de la red da 400; un
/// cambio que vale se aplica.
#[tokio::test]
async fn editar_regla_valida_con_lo_del_router() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("tcp", "9000"), json!({})).await;
    assert_eq!(status, 200, "{body}");
    aplicar(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let escritas = p.sim.control.writes().len();
    // La 0 al puerto 9000 de internet (el de dentro sigue en 8080).
    let (status, body) = preparar_regla(
        &p,
        &cookie,
        Some("0"),
        false,
        json!({ "externo_desde": "9000", "externo_hasta": "9000" }),
        esperados_0(),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("ya está abierto"), "{body}");
    let (status, body) = preparar_regla(
        &p,
        &cookie,
        Some("0"),
        false,
        json!({ "destino": "192.0.2.10" }),
        esperados_0(),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(
        body.contains("no es la de un dispositivo de tu red"),
        "{body}"
    );
    assert_eq!(p.sim.control.writes().len(), escritas);
    let (status, body) = preparar_regla(
        &p,
        &cookie,
        Some("0"),
        false,
        json!({ "destino": "192.168.1.62" }),
        esperados_0(),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    aplicar(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let ahora = reglas_del_tema(&p, &cookie).await;
    let r0 = ahora.iter().find(|r| r[0] == "0").unwrap();
    assert_eq!(r0[2..], ["tcp", "8080", "192.168.1.62"]);
}

/// Un protocolo que el panel no conoce se ve como en el resto de ajustes
/// («Valor no reconocido (47)»), nunca con su clave de traducción.
#[tokio::test]
async fn protocolo_desconocido_en_la_pagina() {
    let (p, cookie) = panel_con_gestor_y_capa(&variante("puertos-protocolo-desconocido")).await;
    let resp = common::get_con_cookie(&p.app, "/ajustes/puertos", &cookie).await;
    let html = common::body_text(resp).await;
    assert!(html.contains("Valor no reconocido (47)"), "{html}");
    assert!(!html.contains("ajuste-puertos-reglas-protocolo-"), "{html}");
}

/// Con un protocolo desconocido no se sabe si choca: cuenta como TCP y UDP
/// a la vez (8080 UDP choca con ella).
#[tokio::test]
async fn protocolo_desconocido_solapa_con_los_dos() {
    let (p, cookie) = panel_con_gestor_y_capa(&variante("puertos-protocolo-desconocido")).await;
    for protocolo in ["tcp", "udp"] {
        let (status, body) = preparar_regla(
            &p,
            &cookie,
            None,
            false,
            nueva(protocolo, "8080"),
            json!({}),
        )
        .await;
        assert_eq!(status, 400, "{protocolo}: {body}");
        assert!(body.contains("ya está abierto"), "{body}");
    }
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("udp", "8081"), json!({})).await;
    assert_eq!(status, 200, "{body}");
}

/// «TCP y UDP»: cada paso lleva su título («TCP», «UDP») y el segundo solo
/// lo que cambia respecto al primero (el resto, en su nota).
#[tokio::test]
async fn ambos_con_un_titulo_por_paso() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("ambos", "9000"), json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(
        pasos[0]["titulo"], "ajuste-puertos-reglas-protocolo-tcp",
        "{g}"
    );
    assert_eq!(
        pasos[1]["titulo"], "ajuste-puertos-reglas-protocolo-udp",
        "{g}"
    );
    let campos = |i: usize| -> Vec<String> {
        pasos[i]["campos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["campo"].as_str().unwrap().to_owned())
            .collect()
    };
    assert!(campos(0).len() >= 7, "{g}");
    assert_eq!(campos(1), ["ajuste-puertos-reglas-protocolo"], "{g}");
    assert_eq!(pasos[1]["nota"], "ajuste-puertos-reglas-paso-igual", "{g}");
    // Una regla de un solo protocolo no lleva títulos.
    let (_, body) = preparar_regla(&p, &cookie, None, false, nueva("tcp", "9000"), json!({})).await;
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["pasos"][0]["titulo"], Value::Null, "{g}");
}
