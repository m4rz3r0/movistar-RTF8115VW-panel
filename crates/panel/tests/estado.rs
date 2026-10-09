//! Pruebas de la página de estado y sus server functions contra `router-sim`.

mod common;

use common::{Panel, body_text, get_con_cookie, panel, post_form, send, sesion};
use panel::poller::{Poller, PollerConfig, PollerHandle};
use serde_json::Value;
use std::time::{Duration, Instant};

/// Arranca el sondeo con intervalos cortos y un cliente del router de tiempo
/// de espera corto.
fn arrancar_sondeo(p: &mut Panel) -> PollerHandle {
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, common::USER, common::PASS);
    cfg.timeout = Duration::from_millis(200);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(50),
            slow: Duration::from_millis(200),
            backoff_max: Duration::from_millis(200),
            ..PollerConfig::default()
        },
    )
}

async fn esperar(que: &str, mut cond: impl FnMut() -> bool) {
    let start = Instant::now();
    while !cond() {
        assert!(start.elapsed() < Duration::from_secs(5), "no llega: {que}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Llama a la server function `nombre` con la sesión `cookie` y devuelve el
/// estado HTTP y el cuerpo.
async fn llamar(p: &Panel, nombre: &str, cuerpo: &str, cookie: Option<&str>) -> (u16, String) {
    let mut req = post_form(&format!("/api/{nombre}"), cuerpo, "127.0.0.1");
    if let Some(c) = cookie {
        req.headers_mut().insert("cookie", c.parse().unwrap());
    }
    let resp = send(&p.app, req).await;
    let status = resp.status().as_u16();
    (status, body_text(resp).await)
}

async fn estado(p: &Panel, cookie: &str) -> (String, Value) {
    let (status, body) = llamar(p, "estado_actual", "", Some(cookie)).await;
    assert_eq!(status, 200, "{body}");
    let json = serde_json::from_str(&body).unwrap();
    (body, json)
}

#[tokio::test]
async fn estado_actual_con_sim() {
    let mut p = panel().await;
    let cookie = sesion(&p).await;
    let h = arrancar_sondeo(&mut p);
    esperar("datos de 60 s", || {
        h.latest().is_some_and(|s| s.slow_at.is_some())
    })
    .await;
    let (body, v) = estado(&p, &cookie).await;
    assert!(body.contains("RTF8115VW"), "{body}");
    assert_eq!(v["router_ok"], true, "{body}");
    assert_eq!(v["error"], Value::Null, "{body}");
    assert!(v["clientes"].as_u64().unwrap() > 0, "{body}");
    assert_eq!(v["internet_up"], true, "{body}");
    assert_eq!(v["ip_publica"], "203.0.113.6", "{body}");
    // Fixtures: 9 equipos activos de 10; la reserva fija 02:00:00:00:00:04.
    assert_eq!(v["clientes"], 9, "{body}");
    let hosts = v["hosts"].as_array().unwrap();
    assert_eq!(hosts.len(), 10, "{body}");
    let fijo = hosts
        .iter()
        .find(|h| h["mac"] == "02:00:00:00:00:04")
        .unwrap();
    assert_eq!(fijo["fijo"], true, "{body}");
    // La fixture tiene TR-069 activo; WPS está configurado, pero en redes
    // desactivadas, así que no se avisa.
    let avisos = v["avisos"].to_string();
    assert!(avisos.contains("TR-069"), "{avisos}");
    assert!(!avisos.contains("WPS"), "{avisos}");
    assert!(!v["wifi"].as_array().unwrap().is_empty(), "{body}");
}

#[tokio::test]
async fn estado_sin_router_muestra_error() {
    let mut p = panel().await;
    let cookie = sesion(&p).await;
    p.sim.control.set_unresponsive(true);
    let h = arrancar_sondeo(&mut p);
    esperar("error del sondeo", || h.error().is_some()).await;
    let (body, v) = estado(&p, &cookie).await;
    assert_eq!(v["router_ok"], false, "{body}");
    assert_eq!(v["at"], Value::Null, "{body}");
    assert!(v["error"].as_str().is_some_and(|e| !e.is_empty()), "{body}");
    assert!(!body.contains(common::PASS), "{body}");
    let avisos = v["avisos"].to_string();
    assert!(avisos.contains("Sin conexión con el router"), "{avisos}");

    // La página lo dice sin esperar al navegador.
    let html = body_text(get_con_cookie(&p.app, "/", &cookie).await).await;
    assert!(html.contains("Sin conexión con el router"), "{html}");
    assert!(html.contains("sin datos todavía"), "{html}");
}

#[tokio::test]
async fn estado_con_datos_y_router_caido_avisa_con_la_hora() {
    let mut p = panel().await;
    let cookie = sesion(&p).await;
    let h = arrancar_sondeo(&mut p);
    esperar("primer dato", || h.healthy()).await;
    p.sim.control.set_unresponsive(true);
    esperar("router caído", || !h.healthy() && h.error().is_some()).await;
    let (body, v) = estado(&p, &cookie).await;
    assert_eq!(v["router_ok"], false, "{body}");
    // Los datos anteriores siguen ahí.
    assert!(body.contains("RTF8115VW"), "{body}");
    assert!(v["at"].as_i64().is_some(), "{body}");
    assert!(
        v["avisos"]
            .to_string()
            .contains("Sin conexión con el router desde"),
        "{body}"
    );
}

#[tokio::test]
async fn estado_no_envia_claves_wifi() {
    let mut p = panel().await;
    let cookie = sesion(&p).await;
    let h = arrancar_sondeo(&mut p);
    esperar("datos de 60 s", || {
        h.latest().is_some_and(|s| s.slow_at.is_some())
    })
    .await;
    let (body, v) = estado(&p, &cookie).await;
    assert!(!v["wifi"].as_array().unwrap().is_empty(), "{body}");
    // Ni valores (la fixture los trae como X-REDACTED-X) ni campos de clave.
    for prohibido in [
        "X-REDACTED-X",
        "key_phrase",
        "pre_shared_key",
        "wps_device_pin",
        "acs_password",
        common::PASS,
    ] {
        assert!(!body.contains(prohibido), "{prohibido} en {body}");
    }
    // La red sabe si tiene clave, pero no cuál es.
    let red = &v["wifi"][0];
    assert!(red["con_clave"].is_boolean(), "{body}");
}

#[tokio::test]
async fn series_y_cortes_con_sim() {
    let mut p = panel().await;
    let cookie = sesion(&p).await;
    let h = arrancar_sondeo(&mut p);
    esperar("primer dato", || h.healthy()).await;
    p.sim.control.counters_bump(1_000_000, 100_000);
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let (status, body) = llamar(&p, "series", "rango=Hora", Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    let muestras: Vec<Value> = serde_json::from_str(&body).unwrap();
    assert!(!muestras.is_empty(), "{body}");
    assert!(muestras.iter().all(|m| m["wan_up"] == true), "{body}");
    assert!(muestras.iter().any(|m| m["rx_bps"].is_f64()), "{body}");

    let (status, body) = llamar(&p, "cortes", "rango=Dia", Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body, "[]");
    p.sim.control.set_wan_up(false);
    esperar("corte registrado", || {
        h.latest()
            .is_some_and(|s| s.wan.iter().any(|w| w.name == "ip2" && !w.up))
    })
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let (_, body) = llamar(&p, "cortes", "rango=Hora", Some(&cookie)).await;
    let cortes: Vec<Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(cortes.len(), 1, "{body}");
    assert_eq!(cortes[0]["hasta"], Value::Null, "{body}");
    let (_, v) = estado(&p, &cookie).await;
    assert_eq!(v["internet_up"], false);
    assert!(v["caida_desde"].as_i64().is_some(), "{v}");
}

#[tokio::test]
async fn server_fns_de_estado_exigen_sesion() {
    let p = panel().await;
    for (nombre, cuerpo) in [
        ("estado_actual", ""),
        ("series", "rango=Hora"),
        ("cortes", "rango=Hora"),
    ] {
        let (status, _) = llamar(&p, nombre, cuerpo, None).await;
        assert_eq!(status, 401, "{nombre}");
    }
}

/// Regresión de la medida del 09/10 en el router real: una descarga de 1 GiB
/// en el puerto LAN 4 movió `ip2` rx sólo +51 973 B (reenvío por hardware)
/// y `eth0.4` tx +1 146 955 629 B. La bajada tiene que seguir al puerto LAN.
#[tokio::test]
async fn la_bajada_sigue_a_los_puertos_lan_y_no_a_ip2() {
    let mut p = panel().await;
    let h = arrancar_sondeo(&mut p);
    esperar("primer dato", || h.healthy()).await;
    // ~1 GiB en 100 pasos de 10 ms por eth0.4 (tx); ip2 casi quieto.
    let inicio = Instant::now();
    for _ in 0..100 {
        p.sim
            .control
            .counters_bump_iface("eth0.4", 0, 1_146_955_629 / 100);
        p.sim.control.counters_bump_iface("ip2", 51_973 / 100, 0);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let segundos = inicio.elapsed().as_secs_f64();
    tokio::time::sleep(Duration::from_millis(60)).await;
    let muestras = panel::series::query(&p.state.db, panel::series::Rango::Hora)
        .await
        .unwrap();
    let maxima = muestras
        .iter()
        .filter_map(|m| m.rx_bps)
        .fold(0.0_f64, f64::max);
    let media_eth = 1_146_955_629.0 * 8.0 / segundos;
    let media_ip2 = 51_973.0 * 8.0 / segundos;
    // Por muestra la tasa varía con el reparto de los pasos; lo que importa
    // es el orden de magnitud: el del puerto LAN, no el de ip2.
    assert!(
        maxima > media_eth / 4.0,
        "bajada máxima {maxima} b/s; puerto LAN {media_eth}, ip2 {media_ip2}"
    );
    assert!(
        muestras
            .iter()
            .all(|m| m.tx_bps.is_none_or(|t| t < media_eth / 100.0))
    );
}
