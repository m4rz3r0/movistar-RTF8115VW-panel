//! ALG de NAT (`cbSetNatConfig`): en natalg.asp son casillas sin `value`
//! (el router las da por activas si el campo llega) y la web manda siempre
//! `set_nf_conntrack=1`, que la lectura no da. Sin eso, el router aceptaba
//! la escritura y no cambiaba nada.

use crate::comun::{PanelPrueba, campo, esperar_final, fijar, ok, valores_de};
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
    let esperado = if nuevo == "1" { Some("on") } else { None };
    assert_eq!(enviado("h323_enable"), esperado, "{ps:?}");
    assert_eq!(campo(&form().await, "h323_enable").unwrap()["valor"], nuevo);
}
