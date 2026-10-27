//! Voz en Teléfono: el estado real leído y «Apagar la línea de voz».

use crate::comun::{Panel, PanelPrueba, aplicar_ajuste, elemento, leer, llamar, ok, variante};
use serde_json::{Value, json};

/// El elemento `id` de la página de Teléfono.
async fn de_telefono(p: &Panel, cookie: &str, id: &str) -> Value {
    let tema = leer(p, cookie, "telefono").await;
    tema["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .cloned()
        .unwrap_or_else(|| panic!("sin {id}: {tema}"))
}

/// Valor del campo `campo` del ajuste `ajuste` en el detalle del elemento.
fn valor<'e>(e: &'e Value, ajuste: &str, campo: &str) -> &'e Value {
    e["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == ajuste)
        .and_then(|a| a["campos"].as_array())
        .and_then(|cs| cs.iter().find(|c| c["id"] == campo))
        .map_or_else(|| panic!("sin {ajuste}.{campo}: {e}"), |c| &c["valor"])
}

/// El valor del dato de la tarjeta con la clave `clave`.
fn dato<'e>(e: &'e Value, clave: &str) -> &'e Value {
    e["datos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["clave"] == clave)
        .map_or_else(|| panic!("sin el dato {clave}: {e}"), |d| &d["valor"])
}

/// Los campos del ajuste `ajuste` en el detalle del elemento, buscado por id.
fn campos<'e>(e: &'e Value, ajuste: &str) -> &'e Vec<Value> {
    e["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == ajuste)
        .and_then(|a| a["campos"].as_array())
        .unwrap_or_else(|| panic!("sin {ajuste}: {e}"))
}

/// `adm_state` del teléfono por internet en el simulador.
async fn sip_encendido(p: &Panel) -> String {
    let n = p.state.hgu.call("cbGetVoiceSip", &[]).await.unwrap();
    elemento(&n, "adm_state").expect("falta adm_state")
}

#[tokio::test]
async fn voz_estado_leido() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let servidores = de_telefono(&p, &cookie, "servidores").await;
    assert_eq!(servidores["texto_estado"], "Registrado", "{servidores}");
    assert_eq!(servidores["estado"], "emite", "{servidores}");
    assert_eq!(
        dato(&servidores, "ajuste-telefono-sip_oculto-estado_registro"),
        &json!({ "Opcion": "registrada" })
    );
    let linea = de_telefono(&p, &cookie, "linea").await;
    assert_eq!(
        valor(&linea, "telefono.linea", "en_servicio"),
        &json!({ "Opcion": "activa" })
    );
    assert_eq!(
        valor(&linea, "telefono.linea", "llamada"),
        &json!({ "Opcion": "sin_llamada" })
    );
    assert_eq!(
        dato(&linea, "ajuste-telefono-linea-en_servicio"),
        &json!({ "Opcion": "activa" })
    );
    let fax = de_telefono(&p, &cookie, "fax-y-modem").await;
    assert_eq!(
        valor(&fax, "telefono.fisico", "puerto"),
        &json!({ "Booleano": true })
    );
    assert_eq!(
        dato(&fax, "ajuste-telefono-fisico-puerto"),
        &json!({ "Booleano": true })
    );
    // Solo se lee: nada de esto se escribe.
    assert!(p.sim.control.writes().is_empty());
}

/// Encendida pero sin registrar en el operador: en ámbar, «Sin registrar».
#[tokio::test]
async fn voz_sin_registrar_en_ambar() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("voz-sin-registrar"))
        .arrancar_con_gestor()
        .await;
    let servidores = de_telefono(&p, &cookie, "servidores").await;
    assert_eq!(servidores["texto_estado"], "Sin registrar", "{servidores}");
    assert_eq!(servidores["estado"], "atencion", "{servidores}");
    assert_eq!(
        dato(&servidores, "ajuste-telefono-sip_oculto-estado_registro"),
        &json!({ "Opcion": "sin_registrar" })
    );
    // La recomendación no afirma un registro que la tarjeta niega.
    let problema = servidores["recomendacion"]["problema"]
        .as_str()
        .unwrap_or_default();
    assert!(
        !problema.is_empty() && !problema.contains("registrado"),
        "{servidores}"
    );
}

/// Encendida con un registro que el panel no reconoce (`net_status=2`):
/// solo «Encendido», nunca «Registrado».
#[tokio::test]
async fn voz_registro_desconocido_solo_encendido() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("voz-registro-desconocido"))
        .arrancar_con_gestor()
        .await;
    let servidores = de_telefono(&p, &cookie, "servidores").await;
    assert_eq!(servidores["texto_estado"], "Encendido", "{servidores}");
    assert_eq!(servidores["estado"], "emite", "{servidores}");
}

#[tokio::test]
async fn apagar_voz_recomendado_con_aviso() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let servidores = de_telefono(&p, &cookie, "servidores").await;
    let r = &servidores["recomendacion"];
    assert_eq!(r["valor"], "false", "{servidores}");
    assert_eq!(r["campo"], "activo", "{r}");
    assert!(
        r["texto"].as_str().is_some_and(|t| t
            .starts_with("Recomendado si no tienes teléfono fijo conectado")
            && t.contains("A favor:")
            && t.contains("En contra:")
            && t.contains("volver a encenderlo")),
        "{r}"
    );
    assert!(r["lotes"].as_array().is_none_or(Vec::is_empty), "{r}");
    // La línea ya no recomienda estar encendida.
    let linea = de_telefono(&p, &cookie, "linea").await;
    let activa = campos(&linea, "telefono.linea")
        .iter()
        .find(|c| c["id"] == "activa")
        .unwrap();
    assert!(activa["recomendado"].is_null(), "{activa}");

    let (status, body) = llamar(
        &p,
        "preparar_accion",
        json!({ "tema": "telefono", "accion": "apagar_voz", "elementos": ["servidores"] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["efecto"], "efecto-telefono_corte", "{g}");
    // El botón de la recomendación prepara el ajuste: mismo aviso.
    let (status, body) = llamar(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "telefono.sip_oculto", "instancia": null, "borrar": false,
                       "valores": { "activo": "false" } } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let b: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(b["efecto"], "efecto-telefono_corte", "{b}");
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(sip_encendido(&p).await, "0");
    let inverso = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    aplicar_ajuste(&p, &cookie, &inverso).await;
    assert_eq!(sip_encendido(&p).await, "1");
}
