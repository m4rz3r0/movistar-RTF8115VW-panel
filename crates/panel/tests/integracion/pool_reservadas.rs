//! IP reservadas de un pool DHCP (las que el pool no reparte nunca). El
//! router las guarda en 32 ranuras, `reserved_ipv4_addr_<i>` y
//! `reserved_ipv4_mask_<i>`, dentro de `<reserved_ipv4>`; una libre es
//! `0.0.0.0` (con máscara `255.255.255.255`). La web oficial añade en la
//! ranura siguiente a la última usada y borra con `0.0.0.0`/`0.0.0.0`,
//! siempre con `cbSetDhcpv4ServerPool`. El panel manda además todo el pool,
//! como al editarlo: el router podría tomar por desmarcada una condición
//! que no llega.

use crate::comun::{
    Panel, PanelPrueba, aplicar_ajuste, campo, esperar_final, fijar, llamar, ok, valores_de,
};
use crate::pool_por_id::{CASA, OTRO, pool, preparar};
use serde_json::{Value, json};

const FUNCION: &str = "cbSetDhcpv4ServerPool";
const LECTURA: &str = "cbGetDhcpv4ServerPool";
const ID: &str = "rtm_cfg_dhcpv4_svr_pool_id";
const IP: &str = "reserved_ipv4_addr";
const MASCARA: &str = "reserved_ipv4_mask";

/// Pone en la ranura `i` del pool de casa la IP reservada `ip` (una sola
/// dirección), como si se hubiera hecho en la web del router.
fn con_reservada(p: &Panel, i: usize, ip: &str) {
    let c = &p.sim.control;
    assert!(c.fijar_en_lista(LECTURA, ID, "0", &format!("{IP}_{i}"), ip));
    assert!(c.fijar_en_lista(
        LECTURA,
        ID,
        "0",
        &format!("{MASCARA}_{i}"),
        "255.255.255.255"
    ));
}

/// El formulario de la Biblioteca del pool de casa.
async fn formulario(p: &Panel, cookie: &str) -> Value {
    ok(
        p,
        "formulario",
        json!({ "funcion": FUNCION, "instancia": "0", "claves": [] }),
        cookie,
    )
    .await
}

/// Prepara en la Biblioteca el pool de casa con `cambios` sobre lo que lee.
async fn preparar_pool(p: &Panel, cookie: &str, cambios: &[(&str, &str)]) -> (u16, String) {
    let mut valores = valores_de(&formulario(p, cookie).await);
    for (k, v) in cambios {
        fijar(&mut valores, k, v);
    }
    llamar(
        p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": FUNCION, "instancia": "0",
                              "valores": valores, "borrar": false } }),
        Some(cookie),
    )
    .await
}

/// Prepara, confirma y espera el final del cambio; devuelve su id.
async fn aplicar_pool(p: &Panel, cookie: &str, cambios: &[(&str, &str)]) -> i64 {
    let (status, body) = preparar_pool(p, cookie, cambios).await;
    assert_eq!(status, 200, "{body}");
    let prep: Value = serde_json::from_str(&body).unwrap();
    let id = ok(
        p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap();
    let c = esperar_final(p, cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
    id
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

/// Los parámetros de IP reservada que lleva una escritura.
fn reservadas_enviadas(ps: &[(String, String)]) -> Vec<(&str, &str)> {
    ps.iter()
        .filter(|(k, _)| k.starts_with("reserved_ipv4"))
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect()
}

/// Lo que cambia entre dos lecturas del mismo pool: `(campo, antes, después)`.
fn distintos<'a>(
    antes: &'a [(String, String)],
    despues: &'a [(String, String)],
) -> Vec<(&'a str, &'a str, &'a str)> {
    assert_eq!(antes.len(), despues.len());
    antes
        .iter()
        .zip(despues)
        .filter(|(a, d)| a != d)
        .map(|(a, d)| (a.0.as_str(), a.1.as_str(), d.1.as_str()))
        .collect()
}

/// Lo demás del pool va en la escritura como al editarlo: el id, el rango,
/// la concesión y los campos que la web manda siempre; las casillas
/// desmarcadas, no.
fn lleva_todo_el_pool(ps: &[(String, String)]) {
    assert_eq!(enviado(ps, ID), Some("0"), "{ps:?}");
    assert_eq!(enviado(ps, "lease_time"), Some("43200"), "{ps:?}");
    assert_eq!(
        enviado(ps, "ipv4_addr_min"),
        Some("192.168.1.100"),
        "{ps:?}"
    );
    assert_eq!(
        enviado(ps, "ipv4_addr_max"),
        Some("192.168.1.199"),
        "{ps:?}"
    );
    assert_eq!(enviado(ps, "adm_state"), Some("1"), "{ps:?}");
    assert!(enviado(ps, "mac_addr_value").is_some(), "{ps:?}");
    for c in ["dhcpv4_opt60", "dhcpv4_opt61", "dhcpv4_opt77", "mac_addr"] {
        assert_eq!(enviado(ps, c), None, "{c}: {ps:?}");
    }
}

#[tokio::test]
async fn el_formulario_ensena_las_reservadas_y_una_nueva() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    con_reservada(&p, 2, "192.168.1.70");
    let f = formulario(&p, &cookie).await;
    assert_eq!(
        campo(&f, "reserved_ipv4_addr_2").unwrap()["valor"],
        "192.168.1.70"
    );
    assert_eq!(
        campo(&f, "reserved_ipv4_mask_2").unwrap()["valor"],
        "255.255.255.255"
    );
    assert!(campo(&f, "reserved_ipv4_addr_0").is_none(), "libre: {f}");
    assert_eq!(campo(&f, IP).unwrap()["valor"], "", "la nueva: {f}");
    assert_eq!(campo(&f, MASCARA).unwrap()["valor"], "", "la nueva: {f}");
}

#[tokio::test]
async fn una_reservada_nueva_va_en_la_ranura_siguiente_con_todo_el_pool() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    con_reservada(&p, 2, "192.168.1.70");
    let antes = pool(&p, CASA).await;
    let otro = pool(&p, OTRO).await;
    aplicar_pool(&p, &cookie, &[(IP, "192.168.1.71"), (MASCARA, "")]).await;

    let ps = escritura_del_pool(&p);
    assert_eq!(
        reservadas_enviadas(&ps),
        [
            ("reserved_ipv4_addr_3", "192.168.1.71"),
            ("reserved_ipv4_mask_3", "255.255.255.255")
        ],
        "sin máscara, una sola dirección: {ps:?}"
    );
    lleva_todo_el_pool(&ps);
    let despues = pool(&p, CASA).await;
    assert_eq!(
        distintos(&antes, &despues),
        [("reserved_ipv4_addr_3", "0.0.0.0", "192.168.1.71")]
    );
    assert_eq!(pool(&p, OTRO).await, otro, "el otro pool no se toca");
}

#[tokio::test]
async fn borrar_una_reservada_como_la_web() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    con_reservada(&p, 2, "192.168.1.70");
    let antes = pool(&p, CASA).await;
    let otro = pool(&p, OTRO).await;
    aplicar_pool(&p, &cookie, &[("reserved_ipv4_addr_2", "0.0.0.0")]).await;

    let ps = escritura_del_pool(&p);
    assert_eq!(
        reservadas_enviadas(&ps),
        [
            ("reserved_ipv4_addr_2", "0.0.0.0"),
            ("reserved_ipv4_mask_2", "0.0.0.0")
        ],
        "{ps:?}"
    );
    lleva_todo_el_pool(&ps);
    let despues = pool(&p, CASA).await;
    assert_eq!(
        distintos(&antes, &despues),
        [
            ("reserved_ipv4_addr_2", "192.168.1.70", "0.0.0.0"),
            ("reserved_ipv4_mask_2", "255.255.255.255", "0.0.0.0")
        ]
    );
    assert_eq!(pool(&p, OTRO).await, otro, "el otro pool no se toca");
}

#[tokio::test]
async fn deshacer_una_reservada_nueva_deja_libre_su_ranura() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = pool(&p, CASA).await;
    let id = aplicar_pool(&p, &cookie, &[(IP, "192.168.1.71"), (MASCARA, "")]).await;
    let prep = ok(&p, "deshacer", json!({ "change_id": id }), &cookie).await;
    let nuevo = ok(
        &p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    assert_eq!(
        esperar_final(&p, &cookie, nuevo).await["estado"],
        "Aplicado"
    );
    assert_eq!(pool(&p, CASA).await, antes);
}

#[tokio::test]
async fn sin_ranura_libre_no_se_anade() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for i in 0..32 {
        con_reservada(&p, i, &format!("192.168.1.{}", 10 + i));
    }
    let (status, body) = preparar_pool(&p, &cookie, &[(IP, "192.168.1.60")]).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains(IP), "{body}");
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn una_reservada_que_no_es_una_ip_no_vale() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar_pool(&p, &cookie, &[(IP, "192.168.1")]).await;
    assert_eq!(status, 400, "{body}");
    assert!(p.sim.control.writes().is_empty());
}

/// Lo de antes se mantiene: cambiar la concesión no manda ninguna ranura.
#[tokio::test]
async fn cambiar_la_concesion_no_envia_las_reservadas() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    con_reservada(&p, 2, "192.168.1.70");
    let g = preparar(&p, &cookie, "red_local.dhcp", json!({ "concesion": "dia" })).await;
    aplicar_ajuste(&p, &cookie, &g).await;
    let ps = escritura_del_pool(&p);
    assert_eq!(reservadas_enviadas(&ps), [], "{ps:?}");
    assert_eq!(enviado(&ps, "lease_time"), Some("86400"), "{ps:?}");
}
