//! Clasificación, filtrado y marcado de QoS: la web desactiva el valor de
//! cada condición mientras su casilla está desmarcada, y `order` si la
//! posición es «primera» o «última» (`order_p` 1 o 3). Lo desactivado no
//! viaja con `.serialize()`, así que el panel tampoco lo manda.

use crate::comun::{Panel, PanelPrueba, esperar_final, fijar, ok, valores_de};
use serde_json::json;

/// Edita la instancia `inst` de `funcion` (o crea una, sin `inst`) con
/// `cambios` y devuelve lo que se envió al router.
async fn editar(
    p: &Panel,
    cookie: &str,
    funcion: &str,
    inst: Option<&str>,
    cambios: &[(&str, &str)],
) -> Vec<(String, String)> {
    let f = ok(
        p,
        "formulario",
        json!({ "funcion": funcion, "instancia": inst, "claves": [] }),
        cookie,
    )
    .await;
    let mut valores = valores_de(&f);
    for (k, v) in cambios {
        // `order_p` no sale en la lectura: al crear, se añade.
        if valores.iter().any(|(n, _)| n == k) {
            fijar(&mut valores, k, v);
        } else {
            valores.push(((*k).to_owned(), (*v).to_owned()));
        }
    }
    let prep = ok(
        p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": funcion, "instancia": inst,
                              "valores": valores, "borrar": false } }),
        cookie,
    )
    .await;
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
    let w = p.sim.control.writes();
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].0, funcion);
    w[0].1.clone()
}

fn enviado<'a>(ps: &'a [(String, String)], k: &str) -> Option<&'a str> {
    ps.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
}

#[tokio::test]
async fn clasificacion_sin_los_valores_de_las_condiciones_apagadas() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let ps = editar(
        &p,
        &cookie,
        "cbSetClassification",
        Some("0"),
        &[("traffic_class", "3")],
    )
    .await;
    // Las condiciones de la regla 0 están desmarcadas: sus valores no van.
    for k in [
        "cls_src_mac_addr",
        "cls_src_mac_mask",
        "cls_dst_mac_addr",
        "cls_vid_value",
        "cls_src_ipv4_addr",
        "cls_src_l4_port_value_min",
    ] {
        assert_eq!(enviado(&ps, k), None, "{k}: {ps:?}");
    }
}

/// Una regla nueva en la primera posición va sin `order`.
#[tokio::test]
async fn la_posicion_primera_no_lleva_orden() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let ps = editar(
        &p,
        &cookie,
        "cbSetClassification",
        None,
        &[("order_p", "1"), ("order", "5"), ("traffic_class", "3")],
    )
    .await;
    assert_eq!(enviado(&ps, "order_p"), Some("1"), "{ps:?}");
    assert_eq!(enviado(&ps, "order"), None, "{ps:?}");
}

#[tokio::test]
async fn una_condicion_marcada_lleva_su_valor() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let ps = editar(
        &p,
        &cookie,
        "cbSetClassification",
        Some("0"),
        &[
            ("cls_dst_ipv4", "1"),
            ("cls_dst_ipv4_addr", "192.0.2.1"),
            ("cls_dst_ipv4_mask", "255.255.255.255"),
        ],
    )
    .await;
    assert_eq!(enviado(&ps, "cls_dst_ipv4"), Some("on"), "{ps:?}");
    assert_eq!(enviado(&ps, "cls_dst_ipv4_addr"), Some("192.0.2.1"));
    assert_eq!(enviado(&ps, "cls_dst_ipv4_mask"), Some("255.255.255.255"));
}

#[tokio::test]
async fn marcado_sin_vid_si_no_se_sobrescribe() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let ps = editar(
        &p,
        &cookie,
        "cbSetTR98L2Marking",
        Some("0"),
        &[("vid_override", "0")],
    )
    .await;
    assert_eq!(enviado(&ps, "vid_override"), None, "{ps:?}");
    assert_eq!(enviado(&ps, "vid_value"), None, "{ps:?}");
    // La prioridad sigue sobrescrita: su valor va.
    assert_eq!(enviado(&ps, "pbit_override"), Some("on"), "{ps:?}");
    assert_eq!(enviado(&ps, "pbit_value"), Some("1"), "{ps:?}");
}
