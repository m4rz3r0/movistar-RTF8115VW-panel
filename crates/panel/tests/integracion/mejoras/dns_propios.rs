//! DNS propios en la red de casa: un DNS principal que no es el router ni
//! del operador da el paso por hecho, con «DNS propios», y la revisión los
//! mantiene salvo que se elija un proveedor.

use super::{del_grupo, del_pool, dns_del_pool, grupo, lote, mejoras, paso, pendientes};
use crate::comun::{PanelPrueba, aplicar_ajuste, leer};
use serde_json::json;

/// Un resolutor de la red de casa y otro público que no está en la lista.
const PROPIO: &str = "192.168.1.70";
const PROPIO_PUBLICO: &str = "192.0.2.53";

#[tokio::test]
async fn dns_propios_dan_el_paso_por_hecho() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    dns_del_pool(&p, PROPIO, PROPIO_PUBLICO).await;
    let m = mejoras(&p, &cookie).await;
    let dns = paso(&m, "privacidad", "dns_casa");
    assert_eq!(dns["estado"], "Hecho", "{m}");
    assert_eq!(dns["nota"], "mejoras-paso-estado-dns-propios", "{dns}");
    assert_eq!(lote(&m, "privacidad")["estado"], pendientes(2, 5), "{m}");
    assert_eq!(m["pendientes"], 5, "{m}");
    // Un proveedor de la lista no es «propio».
    dns_del_pool(&p, "9.9.9.9", "149.112.112.112").await;
    let m = mejoras(&p, &cookie).await;
    let dns = paso(&m, "privacidad", "dns_casa");
    assert_eq!(dns["estado"], "Hecho", "{m}");
    assert!(dns["nota"].is_null(), "{dns}");
}

/// El router (su IP de casa, vacío o 0.0.0.0) o un DNS del operador: el
/// paso sigue pendiente.
#[tokio::test]
async fn dns_del_router_o_del_operador_siguen_pendientes() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for principal in ["192.168.1.1", "", "0.0.0.0", "80.58.61.250", "80.58.61.254"] {
        dns_del_pool(&p, principal, "").await;
        let m = mejoras(&p, &cookie).await;
        let dns = paso(&m, "privacidad", "dns_casa");
        assert_eq!(dns["estado"], "Pendiente", "{principal}: {dns}");
        assert!(dns["nota"].is_null(), "{dns}");
    }
}

/// Sin proveedor elegido, la revisión mantiene los DNS propios (el paso sin
/// cambios, «Mantener los actuales» marcado); con uno elegido, los cambia.
#[tokio::test]
async fn la_revision_mantiene_los_dns_propios() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    dns_del_pool(&p, PROPIO, PROPIO_PUBLICO).await;
    let g = grupo(&p, &cookie, "privacidad", json!({})).await;
    assert_eq!(del_grupo(&g, "dns_casa")["sin_cambios"], true, "{g}");
    assert_eq!(g["proveedor_dns"], "mantener", "{g}");
    let g = grupo(
        &p,
        &cookie,
        "privacidad",
        json!({ "proveedor_dns": "mantener" }),
    )
    .await;
    assert_eq!(del_grupo(&g, "dns_casa")["sin_cambios"], true, "{g}");
    aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(del_pool(&p, "ipv4_dns_addr_0").await, PROPIO);
    assert_eq!(del_pool(&p, "ipv4_dns_addr_1").await, PROPIO_PUBLICO);

    let g = grupo(
        &p,
        &cookie,
        "privacidad",
        json!({ "proveedor_dns": "quad9" }),
    )
    .await;
    let dns = del_grupo(&g, "dns_casa");
    assert_eq!(dns["sin_cambios"], false, "{g}");
    assert_eq!(dns["campos"][0]["despues"], "9.9.9.9", "{dns}");
    assert_eq!(g["proveedor_dns"], "quad9", "{g}");
}

/// «Mantener los actuales» es una opción del selector, la primera.
#[tokio::test]
async fn el_selector_ofrece_mantener_los_actuales() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let m = mejoras(&p, &cookie).await;
    let proveedores = m["proveedores"].as_array().unwrap();
    assert_eq!(proveedores[0]["id"], "mantener", "{m}");
    assert_eq!(proveedores[0]["defecto"], false, "{m}");
    // Con los DNS del router, sin elegir, sigue el de por defecto.
    let g = grupo(&p, &cookie, "privacidad", json!({})).await;
    assert_eq!(g["proveedor_dns"], "quad9", "{g}");
}

/// En Ajustes, la tarjeta de los DNS de casa no recomienda cambiar unos DNS
/// propios; los del operador, sí.
#[tokio::test]
async fn la_tarjeta_no_recomienda_cambiar_dns_propios() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let recomendacion = async || {
        let pagina = leer(&p, &cookie, "red_local").await;
        pagina["elementos"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == "dns-casa")
            .unwrap_or_else(|| panic!("falta dns-casa: {pagina}"))["recomendacion"]
            .clone()
    };
    dns_del_pool(&p, PROPIO, PROPIO_PUBLICO).await;
    assert!(recomendacion().await.is_null());
    dns_del_pool(&p, "80.58.61.250", "80.58.61.254").await;
    assert_eq!(recomendacion().await["valor"], "9.9.9.9");
}

/// La tarjeta de Privacidad máxima dice «DNS propios» en el paso de los DNS.
#[tokio::test]
async fn la_tarjeta_dice_dns_propios() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    dns_del_pool(&p, PROPIO, PROPIO_PUBLICO).await;
    let html = crate::comun::pagina(&p, &cookie, "/mejoras").await;
    assert!(html.contains("DNS propios"), "{html}");
}
