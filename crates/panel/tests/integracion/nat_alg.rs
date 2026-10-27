//! ALG de NAT (`cbSetNatConfig`): en natalg.asp son casillas sin `value`
//! (el router las da por activas si el campo llega) y la web manda siempre
//! `set_nf_conntrack=1`, que la lectura no da. Sin eso, el router aceptaba
//! la escritura y no cambiaba nada. El NAT loopback va por la misma CGI
//! con otros campos: es otra función (`cbSetNatConfig.loopback`).

use crate::comun::{PanelPrueba, aplicar_ajuste, campo, esperar_final, fijar, ok, valores_de};
use serde_json::json;

const FUNCION: &str = "cbSetNatConfig";

#[tokio::test]
async fn apagar_y_encender_una_alg_como_la_web() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let form = || async {
        ok(
            &p,
            "formulario",
            json!({ "funcion": FUNCION, "instancia": null, "claves": [] }),
            &cookie,
        )
        .await
    };
    let f = form().await;
    assert!(campo(&f, "set_nf_conntrack").is_none(), "no se enseña: {f}");
    let antes = campo(&f, "h323_enable").unwrap()["valor"]
        .as_str()
        .unwrap()
        .to_owned();
    let nuevo = if antes == "1" { "0" } else { "1" };
    let mut valores = valores_de(&f);
    fijar(&mut valores, "h323_enable", nuevo);
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": FUNCION, "instancia": null,
                              "valores": valores, "borrar": false } }),
        &cookie,
    )
    .await;
    let id = ok(
        &p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");

    let (_, ps) = p
        .sim
        .control
        .writes()
        .into_iter()
        .find(|(n, _)| n == FUNCION)
        .unwrap();
    let enviado = |k: &str| ps.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    assert_eq!(enviado("set_nf_conntrack"), Some("1"), "{ps:?}");
    // Como el formulario NatAlgForm: solo las casillas y set_nf_conntrack,
    // nunca los campos del NAT loopback.
    let alg = [
        "ftp_enable",
        "h323_enable",
        "pptp_enable",
        "sip_enable",
        "set_nf_conntrack",
    ];
    assert!(ps.iter().all(|(k, _)| alg.contains(&k.as_str())), "{ps:?}");
    let esperado = if nuevo == "1" { Some("on") } else { None };
    assert_eq!(enviado("h323_enable"), esperado, "{ps:?}");
    assert_eq!(campo(&form().await, "h323_enable").unwrap()["valor"], nuevo);
}

/// Prepara y aplica `peticion` (de la Biblioteca o un deshacer ya
/// preparado); devuelve el id del cambio aplicado.
async fn confirmar(p: &crate::comun::Panel, cookie: &str, prep: &serde_json::Value) -> i64 {
    let id = ok(
        p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap();
    assert_eq!(esperar_final(p, cookie, id).await["estado"], "Aplicado");
    id
}

/// El loopback se manda a la CGI del ALG, pero el historial anota la
/// variante del catálogo: deshacerlo vuelve con sus campos (en el router
/// real, antes, fallaba con «parámetro desconocido adm_state»).
#[tokio::test]
async fn deshacer_el_loopback_vuelve_con_su_variante() {
    const LOOPBACK: &str = "cbSetNatConfig.loopback";
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": LOOPBACK, "instancia": null, "claves": [] }),
        &cookie,
    )
    .await;
    let antes = campo(&f, "adm_state").unwrap()["valor"]
        .as_str()
        .unwrap()
        .to_owned();
    let nuevo = if antes == "1" { "0" } else { "1" };
    let mut valores = valores_de(&f);
    fijar(&mut valores, "adm_state", nuevo);
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": LOOPBACK, "instancia": null,
                              "valores": valores, "borrar": false } }),
        &cookie,
    )
    .await;
    let id = confirmar(&p, &cookie, &prep).await;

    let deshacer = ok(&p, "deshacer", json!({ "change_id": id }), &cookie).await;
    confirmar(&p, &cookie, &deshacer).await;
    let (_, ps) = p.sim.control.writes().pop().unwrap();
    assert!(ps.iter().all(|(k, _)| k != "h323_enable"), "{ps:?}");
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": LOOPBACK, "instancia": null, "claves": [] }),
        &cookie,
    )
    .await;
    assert_eq!(campo(&f, "adm_state").unwrap()["valor"], antes.as_str());
}

/// El prefijo ULA de la red local es una casilla, como en la web: el router
/// toma cualquier valor como marcado (con `0` siguió en `1`), así que
/// desmarcado no se manda.
#[tokio::test]
async fn prefijo_ula_por_presencia() {
    const IPV6: &str = "cbSetLanIPv6Config";
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    // Marcada en el router, como el del usuario.
    p.state
        .hgu
        .write("ipv6.asp", IPV6, &[("ss_ipv6_prefix_adm_state", "1")])
        .await
        .unwrap();
    p.state.lecturas.invalidar();
    let valores = [("ss_ipv6_prefix_adm_state", "0")];
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": IPV6, "instancia": null,
                              "valores": valores, "borrar": false } }),
        &cookie,
    )
    .await;
    confirmar(&p, &cookie, &prep).await;
    let (_, ps) = p
        .sim
        .control
        .writes()
        .into_iter()
        .rev()
        .find(|(n, _)| n == IPV6)
        .unwrap();
    assert!(
        ps.iter().all(|(k, _)| k != "ss_ipv6_prefix_adm_state"),
        "{ps:?}"
    );
}

/// Elemento `nombre` de la lectura del NAT en el simulador.
async fn del_nat(p: &crate::comun::Panel, nombre: &str) -> String {
    let n = p.state.hgu.call("cbGetNatConfig", &[]).await.unwrap();
    crate::comun::elemento(&n, nombre).unwrap_or_else(|| panic!("falta {nombre}"))
}

/// Prepara en Ajustes `valores` del ajuste `ajuste`, lo aplica y devuelve
/// el id del cambio.
async fn ajuste(
    p: &crate::comun::Panel,
    cookie: &str,
    ajuste: &str,
    valores: serde_json::Value,
) -> i64 {
    let g = ok(
        p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": ajuste, "instancia": null, "borrar": false, "valores": valores } }),
        cookie,
    )
    .await;
    aplicar_ajuste(p, cookie, &g).await
}

/// Lo deshace desde Ajustes.
async fn deshacer_ajuste(p: &crate::comun::Panel, cookie: &str, id: i64) {
    let d = ok(p, "deshacer_ajuste", json!({ "cambio": id }), cookie).await;
    aplicar_ajuste(p, cookie, &d).await;
}

/// Comprobados en el router real, los ALG se cambian
/// desde Ajustes → Puertos, como en natalg.asp.
#[tokio::test]
async fn alg_desde_ajustes_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = del_nat(&p, "h323_enable").await;
    let nuevo = if antes == "1" { "false" } else { "true" };
    let id = ajuste(&p, &cookie, "puertos.alg", json!({ "h323": nuevo })).await;
    assert_ne!(del_nat(&p, "h323_enable").await, antes);
    let (_, ps) = p.sim.control.writes().pop().unwrap();
    assert!(
        ps.contains(&("set_nf_conntrack".to_owned(), "1".to_owned())),
        "{ps:?}"
    );
    deshacer_ajuste(&p, &cookie, id).await;
    assert_eq!(del_nat(&p, "h323_enable").await, antes);
}

/// El NAT loopback, comprobado en el router real, en Ajustes → Puertos.
#[tokio::test]
async fn loopback_desde_ajustes_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = del_nat(&p, "adm_state").await;
    let nuevo = if antes == "1" { "false" } else { "true" };
    let id = ajuste(&p, &cookie, "puertos.loopback", json!({ "activo": nuevo })).await;
    assert_ne!(del_nat(&p, "adm_state").await, antes);
    let (_, ps) = p.sim.control.writes().pop().unwrap();
    assert!(ps.iter().all(|(k, _)| k != "h323_enable"), "{ps:?}");
    deshacer_ajuste(&p, &cookie, id).await;
    assert_eq!(del_nat(&p, "adm_state").await, antes);
}
