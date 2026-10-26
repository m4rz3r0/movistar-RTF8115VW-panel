//! Las condiciones del pool DHCP (opción 60, 61, 77 y MAC) son casillas sin
//! `value` en la web del router: la web las manda como `on` si están
//! marcadas y no las manda si no lo están, y el router las da por marcadas
//! si el campo llega, valga lo que valga (el simulador hace lo mismo). Una
//! condición activa con valor vacío deja sin IP nueva a todos.

use crate::comun::{
    Panel, PanelPrueba, aplicar_ajuste, campo, esperar_final, fijar, llamar, ok, valores_de,
};
use crate::pool_por_id::{CASA, OTRO, del_pool, pool, preparar};
use serde_json::json;

const FUNCION: &str = "cbSetDhcpv4ServerPool";
const LECTURA: &str = "cbGetDhcpv4ServerPool";
const ID: &str = "rtm_cfg_dhcpv4_svr_pool_id";
/// Las casillas del pool: las cuatro condiciones y la concesión ilimitada.
const CASILLAS: [&str; 5] = [
    "dhcpv4_opt60",
    "dhcpv4_opt61",
    "dhcpv4_opt77",
    "mac_addr",
    "lease_time_u",
];

/// Cambia la concesión del pool de casa desde Ajustes.
async fn cambiar_concesion(p: &Panel, cookie: &str) {
    let g = preparar(p, cookie, "red_local.dhcp", json!({ "concesion": "dia" })).await;
    aplicar_ajuste(p, cookie, &g).await;
}

/// Parámetros de la única escritura del pool.
fn escritura_del_pool(p: &Panel) -> Vec<(String, String)> {
    let w: Vec<_> = p
        .sim
        .control
        .writes()
        .into_iter()
        .filter(|(f, _)| f == FUNCION)
        .collect();
    assert_eq!(w.len(), 1, "{w:?}");
    w.into_iter().next().unwrap().1
}

fn enviado<'a>(ps: &'a [(String, String)], k: &str) -> Option<&'a str> {
    ps.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
}

#[tokio::test]
async fn cambiar_solo_la_concesion_no_envia_las_condiciones() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    cambiar_concesion(&p, &cookie).await;
    let ps = escritura_del_pool(&p);
    for c in CASILLAS {
        assert_eq!(enviado(&ps, c), None, "{c} no se envía: {ps:?}");
    }
    // Lo demás va como lo manda la web, con el id del pool de casa.
    assert_eq!(enviado(&ps, ID), Some("0"), "{ps:?}");
    assert_eq!(enviado(&ps, "lease_time"), Some("86400"), "{ps:?}");
    assert!(enviado(&ps, "mac_addr_value").is_some(), "{ps:?}");
    assert!(enviado(&ps, "mac_addr_mask").is_some(), "{ps:?}");
    for c in &CASILLAS[..4] {
        assert_eq!(del_pool(&p, CASA, c).await, "0", "{c} sigue desmarcada");
    }
}

#[tokio::test]
async fn el_resto_del_pool_se_mantiene() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = pool(&p, CASA).await;
    let otro = pool(&p, OTRO).await;
    cambiar_concesion(&p, &cookie).await;
    let despues = pool(&p, CASA).await;
    let distintos: Vec<_> = antes
        .iter()
        .zip(&despues)
        .filter(|(a, d)| a != d)
        .map(|(a, d)| (a.0.as_str(), a.1.as_str(), d.1.as_str()))
        .collect();
    assert_eq!(antes.len(), despues.len());
    assert_eq!(distintos, [("lease_time", "43200", "86400")]);
    assert_eq!(pool(&p, OTRO).await, otro, "el otro pool no se toca");
}

/// Una condición puesta en la web del router, con su valor: el panel lee
/// el `1` como marcada y, al cambiar otra cosa, la reenvía marcada (`on`)
/// con su valor.
#[tokio::test]
async fn una_condicion_con_valor_se_lee_marcada_y_se_conserva() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let c = &p.sim.control;
    assert!(c.fijar_en_lista(LECTURA, ID, "0", "dhcpv4_opt60", "1"));
    assert!(c.fijar_en_lista(LECTURA, ID, "0", "dhcpv4_opt60_value", "MSFT 5.0"));
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": FUNCION, "instancia": "0", "claves": [] }),
        &cookie,
    )
    .await;
    assert_eq!(campo(&f, "dhcpv4_opt60").unwrap()["valor"], "1", "{f}");
    assert_eq!(campo(&f, "dhcpv4_opt61").unwrap()["valor"], "0", "{f}");

    cambiar_concesion(&p, &cookie).await;
    let ps = escritura_del_pool(&p);
    assert_eq!(enviado(&ps, "dhcpv4_opt60"), Some("on"), "{ps:?}");
    assert_eq!(enviado(&ps, "dhcpv4_opt60_value"), Some("MSFT 5.0"));
    assert_eq!(enviado(&ps, "dhcpv4_opt61"), None, "{ps:?}");
    assert_eq!(del_pool(&p, CASA, "dhcpv4_opt60").await, "1");
    assert_eq!(del_pool(&p, CASA, "dhcpv4_opt60_value").await, "MSFT 5.0");
}

/// El fallo del router real: las cuatro condiciones activas sin valor. Al
/// cambiar la concesión, el panel las desactiva; en la Biblioteca, el diff
/// lo dice.
#[tokio::test]
async fn una_condicion_activa_sin_valor_se_desactiva() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let c = &p.sim.control;
    for k in ["dhcpv4_opt60", "dhcpv4_opt61", "dhcpv4_opt77", "mac_addr"] {
        assert!(c.fijar_en_lista(LECTURA, ID, "0", k, "1"));
    }
    assert!(c.fijar_en_lista(LECTURA, ID, "0", "mac_addr_value", "00:00:00:00:00:00"));
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": FUNCION, "instancia": "0", "claves": [] }),
        &cookie,
    )
    .await;
    let mut valores = valores_de(&f);
    fijar(&mut valores, "lease_time", "86400");
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": FUNCION, "instancia": "0",
                              "valores": valores, "borrar": false } }),
        &cookie,
    )
    .await;
    let mut nombres: Vec<&str> = prep["campos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["nombre"].as_str().unwrap())
        .collect();
    nombres.sort_unstable();
    assert_eq!(
        nombres,
        [
            "dhcpv4_opt60",
            "dhcpv4_opt61",
            "dhcpv4_opt77",
            "lease_time",
            "mac_addr"
        ],
        "{prep}"
    );

    cambiar_concesion(&p, &cookie).await;
    let ps = escritura_del_pool(&p);
    for k in ["dhcpv4_opt60", "dhcpv4_opt61", "dhcpv4_opt77", "mac_addr"] {
        assert_eq!(enviado(&ps, k), None, "{k}: {ps:?}");
        assert_eq!(del_pool(&p, CASA, k).await, "0", "{k}");
    }
}

/// Prepara en la Biblioteca el pool `inst` con `cambios` sobre lo que lee.
async fn preparar_en(
    p: &Panel,
    cookie: &str,
    inst: &str,
    cambios: &[(&str, &str)],
) -> (u16, String) {
    let f = ok(
        p,
        "formulario",
        json!({ "funcion": FUNCION, "instancia": inst, "claves": [] }),
        cookie,
    )
    .await;
    let mut valores = valores_de(&f);
    for (k, v) in cambios {
        fijar(&mut valores, k, v);
    }
    llamar(
        p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": FUNCION, "instancia": inst,
                              "valores": valores, "borrar": false } }),
        Some(cookie),
    )
    .await
}

/// Las condiciones se cambian como en la web oficial, pero activar una con
/// el valor vacío da error: dejaría sin IP nueva a todos.
#[tokio::test]
async fn activar_una_condicion_pide_su_valor() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar_en(&p, &cookie, "0", &[("dhcpv4_opt61", "1")]).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("dhcpv4_opt61"), "{body}");

    let (status, body) = preparar_en(
        &p,
        &cookie,
        "0",
        &[("dhcpv4_opt60", "1"), ("dhcpv4_opt60_value", "MSFT 5.0")],
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let prep: serde_json::Value = serde_json::from_str(&body).unwrap();
    let id = ok(
        &p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    let c = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
    let ps = escritura_del_pool(&p);
    assert_eq!(enviado(&ps, "dhcpv4_opt60"), Some("on"), "{ps:?}");
    assert_eq!(del_pool(&p, CASA, "dhcpv4_opt60").await, "1");
}

/// El pool del operador (id 1) también se edita, como en la web oficial.
#[tokio::test]
async fn el_pool_del_operador_tambien_se_edita() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar_en(&p, &cookie, "1", &[("lease_time", "86400")]).await;
    assert_eq!(status, 200, "{body}");
}
