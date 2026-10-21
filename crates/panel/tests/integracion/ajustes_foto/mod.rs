//! Cómo estaba al instalar (la foto de partida): comparar con el router de
//! ahora, volver un ajuste a su valor de entonces (y deshacerlo), los
//! secretos solo en su campo con el ojo, y sin foto.

use crate::ajustes_secretos::{CLAVE_1, CLAVE_2, escribir_punto, html, llamada, texto_visible};
use crate::comun::{
    Panel, PanelPrueba, aplicar_ajuste, elemento, llamar, ok, partir, registro_global,
};
use panel::fotos::{self, TipoFoto};
use serde_json::{Value, json};
use std::collections::BTreeMap;

mod pagina;

const RUTA: &str = "/ajustes/foto-de-partida";

/// Toma la foto de partida como el asistente: cada lectura de partida del
/// simulador, cifrada en la tabla `fotos`.
pub(crate) async fn tomar_foto(p: &Panel) {
    let mut foto = BTreeMap::new();
    for (nombre, params) in panel::instalar::lecturas_de_partida(&p.state.catalogo) {
        if let Ok(n) = p.state.hgu.call(nombre, &params).await {
            foto.insert(nombre.to_owned(), n.to_xml());
        }
    }
    fotos::guardar(&p.state.db, &p.state.history_key, TipoFoto::Partida, &foto)
        .await
        .unwrap();
    // Lo de ahora, del router (no de una lectura anterior a la foto).
    p.state.lecturas.invalidar();
}

/// Las diferencias de la comparación del ajuste `ajuste`.
fn del_ajuste<'a>(c: &'a Value, ajuste: &str) -> Vec<&'a Value> {
    c["diferencias"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["ajuste"] == ajuste)
        .collect()
}

/// Pone UPnP a `activo` desde Ajustes; el id del cambio.
async fn cambiar_upnp(p: &Panel, cookie: &str, activo: bool) -> i64 {
    let g = ok(
        p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "puertos.upnp", "instancia": null, "borrar": false,
                       "valores": { "activo": activo.to_string() } } }),
        cookie,
    )
    .await;
    aplicar_ajuste(p, cookie, &g).await
}

/// UPnP en el simulador.
async fn upnp(p: &Panel) -> String {
    let n = p.state.hgu.call("cbGetUpnpCfg", &[]).await.unwrap();
    elemento(&n, "adm_state").unwrap()
}

/// Pide volver `campo` del ajuste `ajuste` a la foto: estado y cuerpo.
async fn volver(
    p: &Panel,
    cookie: Option<&str>,
    ajuste: &str,
    campo: Option<&str>,
    instancia: Option<&str>,
) -> (u16, String) {
    llamar(
        p,
        "preparar_volver_a_partida",
        json!({ "ajuste": ajuste, "campo": campo, "instancia": instancia }),
        cookie,
    )
    .await
}

#[tokio::test]
async fn comparar_tras_un_cambio() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    tomar_foto(&p).await;
    let c = ok(&p, "comparar_con_partida", json!({}), &cookie).await;
    assert!(c["momento"].as_i64().is_some_and(|m| m > 0), "{c}");
    assert_eq!(c["diferencias"], json!([]), "recién tomada");
    // Las IP fijas se leen con un parámetro que la foto no lleva.
    let sin_dato = c["sin_dato"].as_array().unwrap();
    assert!(
        sin_dato.contains(&json!("ajuste-red_local-reservas")),
        "{c}"
    );
    // Los dispositivos conectados no son un ajuste que comparar.
    assert!(
        !sin_dato.contains(&json!("ajuste-red_local-dispositivos")),
        "{c}"
    );

    cambiar_upnp(&p, &cookie, true).await;
    let (status, cache, body) = llamada(&p, "comparar_con_partida", json!({}), &cookie).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
    let c: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(c["diferencias"].as_array().unwrap().len(), 1, "{c}");
    let d = &del_ajuste(&c, "puertos.upnp")[0];
    assert_eq!(d["tema"], "puertos");
    assert_eq!(d["clave"], "ajuste-puertos-upnp");
    assert_eq!(d["instancia"], Value::Null);
    let v = &d["cambio"]["Valor"];
    assert_eq!(v["partida"]["id"], "activo");
    assert_eq!(v["partida"]["valor"], json!({ "Booleano": false }));
    assert_eq!(v["actual"]["valor"], json!({ "Booleano": true }));
    assert_eq!(v["restaurable"], true);
    // Sin sesión, nada.
    let (status, _) = llamar(&p, "comparar_con_partida", json!({}), None).await;
    assert_eq!(status, 401);
}

/// Una regla de puertos nueva sale como alta, solo informativa.
#[tokio::test]
async fn comparar_una_lista_por_instancias() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    tomar_foto(&p).await;
    let g = ok(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "puertos.reglas", "instancia": null, "borrar": false,
                       "valores": { "nombre": "juegos", "protocolo": "tcp",
                                    "externo_desde": "9000", "externo_hasta": "9000",
                                    "destino": "192.168.1.62", "interno_desde": "9000",
                                    "interno_hasta": "9000", "activa": "true" } } }),
        &cookie,
    )
    .await;
    aplicar_ajuste(&p, &cookie, &g).await;
    let c = ok(&p, "comparar_con_partida", json!({}), &cookie).await;
    let d = del_ajuste(&c, "puertos.reglas");
    assert_eq!(d.len(), 1, "{c}");
    assert!(d[0]["instancia"].is_string(), "{c}");
    let alta = d[0]["cambio"]["Alta"].as_array().expect("una alta");
    assert!(
        alta.iter()
            .any(|campo| campo["valor"] == json!({ "Texto": "juegos" })),
        "{c}"
    );
}

#[tokio::test]
async fn volver_a_partida_restaura_y_se_puede_deshacer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    tomar_foto(&p).await;
    cambiar_upnp(&p, &cookie, true).await;
    assert_eq!(upnp(&p).await, "1");

    let pedir = json!({ "ajuste": "puertos.upnp", "campo": "activo", "instancia": null });
    let g = ok(&p, "preparar_volver_a_partida", pedir, &cookie).await;
    assert_eq!(g["titulo"], "ajuste-puertos-upnp-titulo");
    let campos = &g["pasos"][0]["campos"];
    assert_eq!(campos[0]["antes"], "true", "{g}");
    assert_eq!(campos[0]["despues"], "false", "{g}");
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(upnp(&p).await, "0");
    let c = ok(&p, "comparar_con_partida", json!({}), &cookie).await;
    assert_eq!(c["diferencias"], json!([]), "{c}");
    // Ya está como en la foto.
    let (status, _) = volver(&p, Some(&cookie), "puertos.upnp", Some("activo"), None).await;
    assert_eq!(status, 409);

    // Como cualquier cambio de Ajustes, se deshace.
    let g = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(upnp(&p).await, "1");

    // Todo el ajuste (sin campo), también.
    let todo = json!({ "ajuste": "puertos.upnp", "campo": null, "instancia": null });
    let g = ok(&p, "preparar_volver_a_partida", todo, &cookie).await;
    aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(upnp(&p).await, "0");
}

#[tokio::test]
async fn volver_a_partida_rechaza_lo_que_no_se_restaura() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    tomar_foto(&p).await;
    let c = Some(cookie.as_str());
    // Una lista no se restaura.
    let (status, body) = volver(&p, c, "puertos.reglas", None, Some("0")).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("listas"), "{body}");
    // Un ajuste sin dato en la foto.
    assert_eq!(
        volver(&p, c, "red_local.dhcp", Some("activo"), None)
            .await
            .0,
        404
    );
    // Ni un ajuste o un campo que no existen.
    assert_eq!(volver(&p, c, "puertos.nada", None, None).await.0, 404);
    assert_eq!(
        volver(&p, c, "puertos.upnp", Some("nada"), None).await.0,
        404
    );
    // Un campo que solo se lee.
    assert_eq!(volver(&p, c, "puertos.alg", Some("ftp"), None).await.0, 400);
    // Un secreto que el router no deja leer (tapado al leer) no se reenvía
    // vacío desde la foto.
    let (status, body) = volver(&p, c, "telefono.linea", Some("contrasena"), None).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("no dejaba leer"), "{body}");
    // Sin sesión, nada.
    let (status, _) = volver(&p, None, "puertos.upnp", Some("activo"), None).await;
    assert_eq!(status, 401);
}

/// Un secreto que ha cambiado sale con su valor de la foto y el de ahora,
/// solo en el `value` de sus campos censurados (y en los datos de la
/// página), con `no-store` y nunca en el registro. Volver a él lo escribe
/// por el flujo normal, con el resumen enmascarado.
#[tokio::test]
async fn comparar_secreto_con_ojo() {
    let registro = registro_global();
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    tomar_foto(&p).await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_2)]).await;

    let (status, cache, body) = llamada(&p, "comparar_con_partida", json!({}), &cookie).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
    let c: Value = serde_json::from_str(&body).unwrap();
    let d = del_ajuste(&c, "wifi.red_5");
    assert_eq!(d.len(), 1, "{c}");
    let v = &d[0]["cambio"]["Valor"];
    assert_eq!(v["partida"]["id"], "contrasena");
    assert_eq!(v["partida"]["valor"], "Secreto");
    assert_eq!(v["partida"]["valor_secreto"], CLAVE_1);
    assert_eq!(v["actual"]["valor_secreto"], CLAVE_2);
    assert_eq!(v["restaurable"], true);

    let pagina = html(&p, &cookie, RUTA).await;
    let (fuera, _) = partir(&pagina, "<script", "</script>");
    let visible = texto_visible(&fuera);
    for clave in [CLAVE_1, CLAVE_2] {
        assert!(fuera.contains(clave), "la página lleva {clave}");
        assert!(!visible.contains(clave), "a la vista: {visible}");
        assert_eq!(
            fuera.matches(clave).count(),
            fuera.matches(&format!("value=\"{clave}\"")).count(),
            "solo en el value del campo"
        );
    }
    assert!(pagina.contains("type=\"password\""), "censurado");

    let g = ok(
        &p,
        "preparar_volver_a_partida",
        json!({ "ajuste": "wifi.red_5", "campo": "contrasena", "instancia": null }),
        &cookie,
    )
    .await;
    let g_texto = g.to_string();
    assert!(
        !g_texto.contains(CLAVE_1) && !g_texto.contains(CLAVE_2),
        "{g_texto}"
    );
    aplicar_ajuste(&p, &cookie, &g).await;
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "1")])
        .await
        .unwrap();
    assert_eq!(elemento(&n, "key_phrase").unwrap(), CLAVE_1);

    // Las demás páginas no la llevan.
    for ruta in ["/", "/historial", "/ajustes/puertos"] {
        let otra = html(&p, &cookie, ruta).await;
        assert!(!otra.contains(CLAVE_1) && !otra.contains(CLAVE_2), "{ruta}");
    }
    let texto = registro.texto();
    assert!(
        texto.contains("cbSetWifiAccPt"),
        "el registro no recoge nada"
    );
    assert!(
        !texto.contains(CLAVE_1) && !texto.contains(CLAVE_2),
        "{texto}"
    );
}

#[tokio::test]
async fn sin_foto_no_falla() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let c = ok(&p, "comparar_con_partida", json!({}), &cookie).await;
    assert_eq!(c["momento"], Value::Null, "{c}");
    assert_eq!(c["diferencias"], json!([]));
    assert_eq!(c["sin_dato"], json!([]));
    let (status, _) = volver(&p, Some(&cookie), "puertos.upnp", Some("activo"), None).await;
    assert_eq!(status, 404);
    // Los temas dicen si hay foto (para su enlace).
    let tema = ok(&p, "leer_tema", json!({ "tema": "puertos" }), &cookie).await;
    assert_eq!(tema["foto_de_partida"], false);
    tomar_foto(&p).await;
    let tema = ok(&p, "leer_tema", json!({ "tema": "puertos" }), &cookie).await;
    assert_eq!(tema["foto_de_partida"], true);
}
