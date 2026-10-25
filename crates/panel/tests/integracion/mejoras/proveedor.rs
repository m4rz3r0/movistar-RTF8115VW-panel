//! El proveedor de DNS de Privacidad máxima: los DNS que ya son de un
//! proveedor de la lista cuentan como hechos, y la revisión se prepara con
//! ese proveedor, no con el de por defecto.

use super::{del_grupo, dns_del_pool, escribir, grupo, mejoras, paso, preparar};
use crate::comun::PanelPrueba;
use serde_json::{Value, json};

/// Vigilancia 4: los DNS que ya son de un proveedor cuentan como hechos.
#[tokio::test]
async fn dns_ya_del_proveedor() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    dns_del_pool(&p, "9.9.9.9", "149.112.112.112").await;
    let m = mejoras(&p, &cookie).await;
    assert_eq!(paso(&m, "privacidad", "dns_casa")["estado"], "Hecho", "{m}");
    let g = grupo(
        &p,
        &cookie,
        "privacidad",
        json!({ "proveedor_dns": "quad9" }),
    )
    .await;
    assert_eq!(del_grupo(&g, "dns_casa")["sin_cambios"], true, "{g}");

    let g = grupo(
        &p,
        &cookie,
        "privacidad",
        json!({ "proveedor_dns": "cloudflare" }),
    )
    .await;
    let dns = del_grupo(&g, "dns_casa");
    assert_eq!(dns["sin_cambios"], false, "{g}");
    let despues: Vec<&Value> = dns["campos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["despues"])
        .collect();
    assert_eq!(despues, [&json!("1.1.1.1"), &json!("1.0.0.1")], "{dns}");

    // Con lo demás también hecho, el lote entero ya está.
    escribir(
        &p,
        "cwmp.asp",
        "cbSetTr69Config",
        &[("enableCWMP", "0"), ("periodicInformEnable", "0")],
    )
    .await;
    escribir(
        &p,
        "ntp.asp",
        "cbSetTimeNtp",
        &[("ntp_svr_1", "hora.roa.es")],
    )
    .await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "privacidad",
        json!({ "proveedor_dns": "quad9" }),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("ya está aplicado"), "{body}");
}

/// Sin proveedor elegido, la revisión se prepara con el que ya tiene el
/// pool 0 (Cloudflare): el paso de los DNS sale sin cambios y la respuesta
/// dice qué proveedor marcar.
#[tokio::test]
async fn revision_con_el_proveedor_del_pool() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    dns_del_pool(&p, "1.1.1.1", "1.0.0.1").await;
    let m = mejoras(&p, &cookie).await;
    assert_eq!(paso(&m, "privacidad", "dns_casa")["estado"], "Hecho", "{m}");
    let g = grupo(&p, &cookie, "privacidad", json!({})).await;
    assert_eq!(del_grupo(&g, "dns_casa")["sin_cambios"], true, "{g}");
    assert_eq!(g["proveedor_dns"], "cloudflare", "{g}");
}

/// Con DNS que no son de ningún proveedor de la lista, el de por defecto.
#[tokio::test]
async fn revision_sin_proveedor_conocido_usa_quad9() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = grupo(&p, &cookie, "privacidad", json!({})).await;
    let dns = del_grupo(&g, "dns_casa");
    assert_eq!(dns["sin_cambios"], false, "{g}");
    assert_eq!(dns["campos"][0]["despues"], "9.9.9.9", "{dns}");
    assert_eq!(g["proveedor_dns"], "quad9", "{g}");
    // Uno elegido manda, aunque el pool tenga otro.
    let g = grupo(
        &p,
        &cookie,
        "privacidad",
        json!({ "proveedor_dns": "adguard" }),
    )
    .await;
    assert_eq!(g["proveedor_dns"], "adguard", "{g}");
}

/// El DNS familiar es un proveedor más: con él en el pool 0, el paso está
/// hecho, la revisión lo marca y los DNS de casa no salen como desconocidos.
#[tokio::test]
async fn dns_familiar_cuenta_como_proveedor() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    dns_del_pool(&p, "1.1.1.3", "1.0.0.3").await;
    let m = mejoras(&p, &cookie).await;
    assert_eq!(paso(&m, "privacidad", "dns_casa")["estado"], "Hecho", "{m}");
    let g = grupo(&p, &cookie, "privacidad", json!({})).await;
    assert_eq!(del_grupo(&g, "dns_casa")["sin_cambios"], true, "{g}");
    assert_eq!(g["proveedor_dns"], "cloudflare_familiar", "{g}");
    let red = crate::comun::leer(&p, &cookie, "red_local").await;
    let dns = red["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "dns-casa")
        .unwrap_or_else(|| panic!("sin dns-casa: {red}"));
    assert!(dns["recomendacion"].is_null(), "{dns}");
}
