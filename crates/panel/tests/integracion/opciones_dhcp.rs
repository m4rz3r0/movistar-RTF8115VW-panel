//! Opciones DHCP de un pool (`cbDhcpV4SvrPoolSetOpt`) como las manda la web
//! oficial: el valor en hexadecimal sin espacios, la longitud en bytes si
//! no se escribe, el código por debajo de 255, `-1` como id al crear y, al
//! borrar, solo los dos ids con `del=DEL`.

use crate::comun::{Panel, PanelPrueba, aplicar, esperar_final, fijar, llamar, ok, valores_de};
use serde_json::{Value, json};

const FUNCION: &str = "cbDhcpV4SvrPoolSetOpt";
const LECTURA: &str = "cbDhcpV4SvrPoolGetOpt";
const POOL: &str = "rtm_cfg_dhcpv4_svr_pool_id";
const OPCION: &str = "rtm_dhcpv4_svr_pool_opt_id";

/// Las opciones del pool de casa: `(id, código, longitud, valor, activada)`.
async fn opciones(p: &Panel) -> Vec<[String; 5]> {
    let n = p
        .state
        .hgu
        .call(LECTURA, &[(POOL, "0"), ("all", "ALL")])
        .await
        .unwrap();
    n.children
        .iter()
        .filter(|c| !c.children.is_empty())
        .map(|o| {
            [
                "rtm_dhcpv4_svr_pool_opt_id",
                "opt_code",
                "opt_len",
                "opt_value",
                "adm_state",
            ]
            .map(|k| o.text_of(k).unwrap_or_default().to_owned())
        })
        .collect()
}

/// Los valores del formulario de una opción nueva del pool de casa, con
/// `cambios`.
async fn nueva(p: &Panel, cookie: &str, cambios: &[(&str, &str)]) -> Vec<(String, String)> {
    let f = ok(
        p,
        "formulario",
        json!({ "funcion": FUNCION, "instancia": null, "claves": [[POOL, "0"]] }),
        cookie,
    )
    .await;
    let mut v = valores_de(&f);
    for (k, valor) in cambios {
        fijar(&mut v, k, valor);
    }
    v
}

/// Prepara una opción nueva con `cambios` (estado y cuerpo).
async fn preparar_nueva(p: &Panel, cookie: &str, cambios: &[(&str, &str)]) -> (u16, String) {
    let v = nueva(p, cookie, cambios).await;
    llamar(
        p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": FUNCION, "instancia": null,
                              "valores": v, "borrar": false } }),
        Some(cookie),
    )
    .await
}

/// Parámetros de la única escritura de opciones.
fn escritura(p: &Panel) -> Vec<(String, String)> {
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

fn ordenados(mut ps: Vec<(String, String)>) -> Vec<(String, String)> {
    ps.sort();
    ps
}

fn pares(ps: &[(&str, &str)]) -> Vec<(String, String)> {
    ordenados(
        ps.iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
    )
}

#[tokio::test]
async fn crear_una_opcion_como_la_web() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = opciones(&p).await;
    let v = nueva(
        &p,
        &cookie,
        &[
            ("adm_state", "1"),
            ("opt_code", "66"),
            ("opt_len", ""),
            ("opt_value", "c0 a8 01 0A"),
        ],
    )
    .await;
    let id = aplicar(
        &p,
        &cookie,
        json!({ "funcion": FUNCION, "instancia": null, "valores": v, "borrar": false }),
    )
    .await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");

    assert_eq!(
        ordenados(escritura(&p)),
        pares(&[
            ("adm_state", "1"),
            ("opt_code", "66"),
            ("opt_len", "4"),
            ("opt_value", "c0a8010A"),
            (OPCION, "-1"),
            (POOL, "0"),
        ])
    );
    let despues = opciones(&p).await;
    assert_eq!(despues.len(), antes.len() + 1, "{despues:?}");
    assert_eq!(despues[..antes.len()], antes[..], "las demás no cambian");
    let n = despues.last().unwrap();
    assert_eq!(n[1..], ["66", "4", "c0a8010A", "1"].map(String::from));
}

#[tokio::test]
async fn la_longitud_escrita_se_respeta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let v = nueva(
        &p,
        &cookie,
        &[
            ("adm_state", "1"),
            ("opt_code", "66"),
            ("opt_len", "2"),
            ("opt_value", "0102"),
        ],
    )
    .await;
    let id = aplicar(
        &p,
        &cookie,
        json!({ "funcion": FUNCION, "instancia": null, "valores": v, "borrar": false }),
    )
    .await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    let ps = escritura(&p);
    assert!(ps.contains(&("opt_len".into(), "2".into())), "{ps:?}");
}

#[tokio::test]
async fn el_codigo_tiene_que_ser_menor_que_255() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar_nueva(
        &p,
        &cookie,
        &[("adm_state", "1"), ("opt_code", "255"), ("opt_value", "01")],
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("opt_code"), "{body}");
    assert!(p.sim.control.writes().is_empty());
}

/// Por encima de 223, la web convierte el texto a hexadecimal; el panel
/// lo pide ya en hexadecimal (lo que lee y compara el router).
#[tokio::test]
async fn el_valor_tiene_que_ir_en_hexadecimal() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for valor in ["hola", "abc"] {
        let (status, body) = preparar_nueva(
            &p,
            &cookie,
            &[
                ("adm_state", "1"),
                ("opt_code", "224"),
                ("opt_value", valor),
            ],
        )
        .await;
        assert_eq!(status, 400, "{valor}: {body}");
        assert!(body.contains("opt_value"), "{body}");
    }
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn borrar_una_opcion_como_la_web() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = opciones(&p).await;
    let id = aplicar(
        &p,
        &cookie,
        json!({ "funcion": FUNCION, "instancia": "4", "valores": [[POOL, "0"]], "borrar": true }),
    )
    .await;
    let c: Value = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
    assert_eq!(
        ordenados(escritura(&p)),
        pares(&[("del", "DEL"), (OPCION, "4"), (POOL, "0")])
    );
    let quedan: Vec<_> = antes.into_iter().filter(|o| o[0] != "4").collect();
    assert_eq!(opciones(&p).await, quedan);
}
