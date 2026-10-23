//! Los dos ajustes de las mejoras contra `router-sim`: el registro del
//! sistema (sin envío remoto y sin `export`) y los DNS que el router reparte
//! a la red de casa (pool DHCP de id 0), con su ida, su vuelta y su recomendación.

use crate::comun::{Panel, PanelPrueba, aplicar_ajuste, elemento, leer, llamar, ok};
use serde_json::{Value, json};

/// Escribe en el simulador como lo haría el router (otro que no es el
/// panel) y olvida la caché de lecturas.
async fn escribir(p: &Panel, pagina: &str, funcion: &str, pares: &[(&str, &str)]) {
    p.state.hgu.write(pagina, funcion, pares).await.unwrap();
    p.state.lecturas.invalidar();
}

/// Elemento `nombre` de la lectura del registro en el simulador.
async fn del_registro(p: &Panel, nombre: &str) -> String {
    let n = p.state.hgu.call("cbGetSyslog", &[]).await.unwrap();
    elemento(&n, nombre).unwrap_or_else(|| panic!("falta {nombre}"))
}

/// Elemento `nombre` del pool DHCP de casa (id 0, en la posición 1 de las
/// fixtures) en el simulador.
async fn del_pool(p: &Panel, nombre: &str) -> String {
    let n = p
        .state
        .hgu
        .call(
            "cbGetDhcpv4ServerPool",
            &[("rtm_cfg_dhcpv4_svr_pool_0i", "1")],
        )
        .await
        .unwrap();
    elemento(&n, nombre).unwrap_or_else(|| panic!("falta {nombre}"))
}

/// Prepara `valores` en `ajuste` (200) y devuelve el resumen.
async fn preparar(p: &Panel, cookie: &str, ajuste: &str, valores: Value) -> Value {
    let (status, body) = llamar(
        p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": ajuste, "instancia": null, "borrar": false, "valores": valores } }),
        Some(cookie),
    )
    .await;
    assert_eq!(status, 200, "preparar {ajuste}: {body}");
    serde_json::from_str(&body).unwrap()
}

/// Deshace el cambio `id` y espera a que quede aplicado.
async fn deshacer(p: &Panel, cookie: &str, id: i64) {
    let inverso = ok(p, "deshacer_ajuste", json!({ "cambio": id }), cookie).await;
    aplicar_ajuste(p, cookie, &inverso).await;
}

/// Elemento `id` de la página de un tema.
fn de_la_pagina<'a>(pagina: &'a Value, id: &str) -> &'a Value {
    pagina["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .unwrap_or_else(|| panic!("falta el elemento {id}: {pagina}"))
}

/// Parámetros de la última escritura de `funcion` en el simulador.
fn ultima_escritura(p: &Panel, funcion: &str) -> Vec<(String, String)> {
    p.sim
        .control
        .writes()
        .into_iter()
        .rev()
        .find(|(f, _)| f == funcion)
        .map(|(_, ps)| ps)
        .unwrap_or_else(|| panic!("ninguna escritura de {funcion}"))
}

#[tokio::test]
async fn registro_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    escribir(
        &p,
        "syslog.asp",
        "cbSetSyslog",
        &[("syslog_mode", "2"), ("remote_ip", "192.0.2.5")],
    )
    .await;
    let pagina = leer(&p, &cookie, "sistema").await;
    let e = de_la_pagina(&pagina, "registro");
    assert_eq!(e["interruptor"]["encendido"], true, "{e}");
    assert_eq!(e["recomendacion"]["valor"], "false", "{e}");

    let g = preparar(&p, &cookie, "sistema.registro", json!({ "envio": "false" })).await;
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(del_registro(&p, "syslog_mode").await, "0");
    let escrita = ultima_escritura(&p, "cbSetSyslog");
    assert!(
        escrita.iter().all(|(k, _)| k != "export"),
        "el panel nunca exporta el registro: {escrita:?}"
    );

    deshacer(&p, &cookie, id).await;
    assert_eq!(del_registro(&p, "syslog_mode").await, "2");
    assert!(
        ultima_escritura(&p, "cbSetSyslog")
            .iter()
            .all(|(k, _)| k != "export")
    );
}

/// Toda escritura del pool DHCP va al de id 0: los demás no se tocan.
fn solo_pool_0(p: &Panel) {
    for (f, ps) in p.sim.control.writes() {
        if f == "cbSetDhcpv4ServerPool" {
            assert!(
                ps.iter()
                    .any(|(k, v)| k == "rtm_cfg_dhcpv4_svr_pool_id" && v == "0"),
                "{ps:?}"
            );
        }
    }
}

#[tokio::test]
async fn dns_casa_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    assert_eq!(del_pool(&p, "ipv4_dns_addr_0").await, "192.168.1.1");
    assert_eq!(del_pool(&p, "ipv4_dns_addr_1").await, "192.168.1.1");
    let g = preparar(
        &p,
        &cookie,
        "red_local.dns_casa",
        json!({ "principal": "9.9.9.9", "secundario": "149.112.112.112" }),
    )
    .await;
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(del_pool(&p, "ipv4_dns_addr_0").await, "9.9.9.9");
    assert_eq!(del_pool(&p, "ipv4_dns_addr_1").await, "149.112.112.112");
    solo_pool_0(&p);

    deshacer(&p, &cookie, id).await;
    assert_eq!(del_pool(&p, "ipv4_dns_addr_0").await, "192.168.1.1");
    assert_eq!(del_pool(&p, "ipv4_dns_addr_1").await, "192.168.1.1");
    solo_pool_0(&p);
}

/// Con un solo DNS (el secundario vacío), el deshacer lo deja vacío.
#[tokio::test]
async fn dns_casa_con_un_solo_dns_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    escribir(
        &p,
        "dhcp.asp",
        "cbSetDhcpv4ServerPool",
        &[("rtm_cfg_dhcpv4_svr_pool_id", "0"), ("ipv4_dns_addr_1", "")],
    )
    .await;
    let g = preparar(
        &p,
        &cookie,
        "red_local.dns_casa",
        json!({ "principal": "9.9.9.9", "secundario": "149.112.112.112" }),
    )
    .await;
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(del_pool(&p, "ipv4_dns_addr_1").await, "149.112.112.112");
    deshacer(&p, &cookie, id).await;
    assert_eq!(del_pool(&p, "ipv4_dns_addr_1").await, "");
    assert_eq!(del_pool(&p, "ipv4_dns_addr_0").await, "192.168.1.1");
}

#[tokio::test]
async fn dns_casa_en_su_pagina() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let pagina = leer(&p, &cookie, "red_local").await;
    let e = de_la_pagina(&pagina, "dns-casa");
    let r = &e["recomendacion"];
    assert_eq!(
        (&r["ajuste"], &r["campo"], &r["valor"]),
        (
            &json!("red_local.dns_casa"),
            &json!("principal"),
            &json!("9.9.9.9")
        ),
        "{e}"
    );

    // Otro proveedor neutro (aceptado) vale como el recomendado.
    escribir(
        &p,
        "dhcp.asp",
        "cbSetDhcpv4ServerPool",
        &[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("ipv4_dns_addr_0", "1.1.1.1"),
        ],
    )
    .await;
    let pagina = leer(&p, &cookie, "red_local").await;
    let e = de_la_pagina(&pagina, "dns-casa");
    assert!(e["recomendacion"].is_null(), "{e}");
}
