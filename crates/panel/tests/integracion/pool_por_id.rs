//! El pool DHCP de casa con dos pools en el orden del router real: en la
//! posición 0 está el pool de otros dispositivos del operador (id 1) y en
//! la 1, el de casa (id 0). Los ajustes leen, escriben y deshacen siempre el
//! de id 0, y el otro no se toca.

use crate::comun::{
    Panel, PanelPrueba, ajustes_de, aplicar_ajuste, cambios_del_grupo, leer, llamar, ok,
};
use serde_json::{Value, json};

/// Posición del pool en la lectura (no es su id).
const POSICION: &str = "rtm_cfg_dhcpv4_svr_pool_0i";
/// En las fixtures, la posición del pool de casa (id 0) y la del otro (id 1).
const CASA: &str = "1";
const OTRO: &str = "0";

/// Hojas del pool de la posición `pos` del simulador, sin el `Result`.
async fn pool(p: &Panel, pos: &str) -> Vec<(String, String)> {
    let n = p
        .state
        .hgu
        .call("cbGetDhcpv4ServerPool", &[(POSICION, pos)])
        .await
        .unwrap();
    let mut hojas = Vec::new();
    juntar_hojas(&n, &mut hojas);
    hojas
}

fn juntar_hojas(n: &hgu::xml::XmlNode, out: &mut Vec<(String, String)>) {
    for c in &n.children {
        if c.children.is_empty() {
            if c.tag != "Result" {
                out.push((c.tag.clone(), c.text.clone()));
            }
        } else {
            juntar_hojas(c, out);
        }
    }
}

/// Valor de `nombre` en el pool de la posición `pos`.
async fn del_pool(p: &Panel, pos: &str, nombre: &str) -> String {
    pool(p, pos)
        .await
        .into_iter()
        .find(|(k, _)| k == nombre)
        .map(|(_, v)| v)
        .unwrap_or_else(|| panic!("falta {nombre} en la posición {pos}"))
}

/// Toda escritura del pool lleva el id 0.
fn solo_el_pool_de_casa(p: &Panel) {
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

/// Valor legible del campo `campo` del ajuste `ajuste` en la página.
fn en_la_pagina(pagina: &Value, ajuste: &str, campo: &str) -> Value {
    let a = ajustes_de(pagina)
        .into_iter()
        .find(|a| a["id"] == ajuste)
        .unwrap_or_else(|| panic!("falta {ajuste}: {pagina}"));
    assert_eq!(a["estado"], "Leido", "{a}");
    a["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == campo)
        .unwrap_or_else(|| panic!("falta {campo}: {a}"))["valor"]
        .clone()
}

/// Prepara `valores` en `ajuste` (200) y devuelve el resumen.
async fn preparar(p: &Panel, cookie: &str, ajuste: &str, valores: Value) -> Value {
    ok(
        p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": ajuste, "instancia": null, "borrar": false, "valores": valores } }),
        cookie,
    )
    .await
}

#[tokio::test]
async fn las_fixtures_tienen_el_pool_de_casa_en_la_posicion_1() {
    let (p, _) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    assert_eq!(del_pool(&p, CASA, "rtm_cfg_dhcpv4_svr_pool_id").await, "0");
    assert_eq!(del_pool(&p, OTRO, "rtm_cfg_dhcpv4_svr_pool_id").await, "1");
}

#[tokio::test]
async fn dhcp_y_dns_de_casa_leen_el_pool_de_casa() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let pagina = leer(&p, &cookie, "red_local").await;
    assert_eq!(
        en_la_pagina(&pagina, "red_local.dhcp", "inicio"),
        json!({ "Texto": "192.168.1.100" })
    );
    assert_eq!(
        en_la_pagina(&pagina, "red_local.dhcp", "fin"),
        json!({ "Texto": "192.168.1.199" })
    );
    assert_eq!(
        en_la_pagina(&pagina, "red_local.dns_casa", "principal"),
        json!({ "Texto": "192.168.1.1" })
    );
}

#[tokio::test]
async fn dhcp_parte_del_valor_del_pool_de_casa() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = preparar(
        &p,
        &cookie,
        "red_local.dhcp",
        json!({ "fin": "192.168.1.98" }),
    )
    .await;
    let campos = cambios_del_grupo(&g);
    assert_eq!(campos.len(), 1, "{g}");
    assert_eq!(campos[0]["antes"], "192.168.1.199", "{g}");
}

#[tokio::test]
async fn dns_casa_guarda_y_deshace_solo_el_pool_de_casa() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let otro = pool(&p, OTRO).await;
    let g = preparar(
        &p,
        &cookie,
        "red_local.dns_casa",
        json!({ "principal": "9.9.9.9", "secundario": "149.112.112.112" }),
    )
    .await;
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(del_pool(&p, CASA, "ipv4_dns_addr_0").await, "9.9.9.9");
    assert_eq!(
        del_pool(&p, CASA, "ipv4_dns_addr_1").await,
        "149.112.112.112"
    );
    assert_eq!(pool(&p, OTRO).await, otro, "el otro pool no se toca");

    let inverso = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    aplicar_ajuste(&p, &cookie, &inverso).await;
    assert_eq!(del_pool(&p, CASA, "ipv4_dns_addr_0").await, "192.168.1.1");
    assert_eq!(del_pool(&p, CASA, "ipv4_dns_addr_1").await, "192.168.1.1");
    assert_eq!(pool(&p, OTRO).await, otro, "el otro pool no se toca");
    solo_el_pool_de_casa(&p);
}

/// «Privacidad máxima» de ida y vuelta: los DNS del pool de casa cambian y
/// vuelven; el otro pool sigue igual.
#[tokio::test]
async fn privacidad_ida_y_vuelta_con_dos_pools() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let otro = pool(&p, OTRO).await;
    let (status, body) = llamar(
        &p,
        "preparar_lote",
        json!({ "id": "privacidad", "opciones": {} }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(del_pool(&p, CASA, "ipv4_dns_addr_0").await, "9.9.9.9");
    assert_eq!(
        del_pool(&p, CASA, "ipv4_dns_addr_1").await,
        "149.112.112.112"
    );
    assert_eq!(pool(&p, OTRO).await, otro, "el otro pool no se toca");

    let inverso = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    aplicar_ajuste(&p, &cookie, &inverso).await;
    assert_eq!(del_pool(&p, CASA, "ipv4_dns_addr_0").await, "192.168.1.1");
    assert_eq!(del_pool(&p, CASA, "ipv4_dns_addr_1").await, "192.168.1.1");
    assert_eq!(pool(&p, OTRO).await, otro, "el otro pool no se toca");
    solo_el_pool_de_casa(&p);
}

/// Sin el pool de casa (su posición no responde), los ajustes del pool
/// quedan «sin leer» con su motivo, sin caer en el otro pool.
#[tokio::test]
async fn sin_el_pool_de_casa_no_lee_el_otro() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.sim
        .control
        .fail_reads_con("cbGetDhcpv4ServerPool", POSICION, CASA, true);
    let pagina = leer(&p, &cookie, "red_local").await;
    for id in ["red_local.dhcp", "red_local.dns_casa"] {
        let a = ajustes_de(&pagina)
            .into_iter()
            .find(|a| a["id"] == id)
            .unwrap_or_else(|| panic!("falta {id}: {pagina}"));
        let motivo = a["estado"]["SinLeer"]
            .as_str()
            .unwrap_or_else(|| panic!("{a}"));
        assert!(!motivo.contains("cbGet"), "{motivo}");
        assert!(!a.to_string().contains("192.168.1.230"), "{a}");
    }
    let (status, body) = llamar(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "red_local.dhcp", "instancia": null, "borrar": false,
                      "valores": { "fin": "192.168.1.98" } } }),
        Some(&cookie),
    )
    .await;
    assert_ne!(status, 200, "{body}");
    assert!(
        p.sim
            .control
            .writes()
            .iter()
            .all(|(f, _)| f != "cbSetDhcpv4ServerPool"),
        "no se escribe nada"
    );
}
