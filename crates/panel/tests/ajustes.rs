//! Lectura de los ajustes del router por tema (`leer_tema`): valores
//! legibles, secretos que no salen del servidor, lecturas que fallan solas y
//! el catálogo comprobado al arrancar.

mod common;

use axum::http::header;
use common::{
    Panel, aplicar, body_text, esperar_final, esperar_libre, llamar, ok, panel, panel_con_ajustes,
    panel_con_capa, panel_con_gestor, panel_con_tiempos, post_json, send, sesion, tiempos_rapidos,
    variante,
};
use panel::instancia::{self, claves};
use serde_json::{Value, json};
use std::time::Duration;

/// Ajuste `id` de la respuesta de `leer_tema`.
fn ajuste<'a>(tema: &'a Value, id: &str) -> &'a Value {
    tema["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id)
        .unwrap_or_else(|| panic!("falta el ajuste {id}: {tema}"))
}

/// Campo `id` de un ajuste (o de una instancia).
fn campo<'a>(ajuste: &'a Value, id: &str) -> &'a Value {
    ajuste["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("falta el campo {id}: {ajuste}"))
}

/// Primer elemento `nombre` de una respuesta del router, a cualquier
/// profundidad.
fn elemento(n: &hgu::xml::XmlNode, nombre: &str) -> Option<String> {
    n.text_of(nombre)
        .map(str::to_owned)
        .or_else(|| n.children.iter().find_map(|c| elemento(c, nombre)))
}

async fn leer(p: &Panel, cookie: &str, tema: &str) -> Value {
    ok(p, "leer_tema", json!({ "tema": tema }), cookie).await
}

/// Escribe en el punto de acceso 0 del simulador, como lo haría el router.
async fn escribir_red_24(p: &Panel, pares: &[(&str, &str)]) {
    let mut params = vec![("dpm_cfg_wifi_acc_pt_id", "0")];
    params.extend_from_slice(pares);
    p.state
        .hgu
        .write("wifi.asp", "cbSetWifiAccPt", &params)
        .await
        .unwrap();
}

/// `leer_tema` con la sesión `cookie` (o sin ella): estado, `Cache-Control`
/// y cuerpo.
async fn peticion(p: &Panel, tema: &str, cookie: Option<&str>) -> (u16, String, String) {
    let mut req = post_json(
        "/api/leer_tema",
        &json!({ "tema": tema }).to_string(),
        "127.0.0.1",
    );
    if let Some(c) = cookie {
        req.headers_mut().insert("cookie", c.parse().unwrap());
    }
    let resp = send(&p.app, req).await;
    let status = resp.status().as_u16();
    let cache = resp
        .headers()
        .get(header::CACHE_CONTROL)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    (status, cache, body_text(resp).await)
}

#[tokio::test]
async fn leer_tema_exige_sesion() {
    let p = panel().await;
    let (status, cache, body) = peticion(&p, "wifi", None).await;
    assert_eq!(status, 401, "{body}");
    assert_eq!(cache, "no-store");
}

#[tokio::test]
async fn leer_tema_wifi_con_valores_legibles() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let ssid = elemento(
        &p.state
            .hgu
            .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "0")])
            .await
            .unwrap(),
        "ssid",
    )
    .unwrap();
    let tema = leer(&p, &cookie, "wifi").await;
    assert_eq!(tema["tema"], "wifi");
    let red = ajuste(&tema, "wifi.red_24");
    assert_eq!(red["estado"], "Leido", "{red}");
    assert_eq!(red["nivel"], "basico");
    assert_eq!(red["lista"], false);
    assert_eq!(
        campo(red, "seguridad")["valor"],
        json!({ "Opcion": "wpa2" })
    );
    assert_eq!(campo(red, "nombre")["valor"], json!({ "Texto": ssid }));
    // Lo que la página necesita para pintar sin el catálogo.
    let seguridad = campo(red, "seguridad");
    assert_eq!(seguridad["tipo"], "opcion");
    assert_eq!(seguridad["recomendado"], "wpa2");
    let abierta = seguridad["opciones"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["clave"] == "abierta")
        .unwrap();
    assert_eq!(abierta["recomendada"], false);
    assert_eq!(abierta["aviso"], "aviso-wifi_abierta");
    assert_eq!(campo(red, "visible")["recomendado"], "true");
    assert_eq!(campo(red, "contrasena")["ascii"], true);
    assert_eq!(campo(red, "contrasena")["min"], 8);
    assert_eq!(campo(red, "contrasena")["max"], 63);
    assert_eq!(red["efecto"], "efecto-wifi_red");
    // Los nombres de la API solo van en los detalles técnicos.
    assert_eq!(red["tecnico"]["escrituras"], json!(["cbSetWifiAccPt"]));
    assert!(
        red["tecnico"]["parametros"]
            .as_array()
            .unwrap()
            .contains(&json!(["seguridad", "authentication"])),
        "{red}"
    );
    let mut sin_tecnico = red.clone();
    sin_tecnico.as_object_mut().unwrap().remove("tecnico");
    let texto = sin_tecnico.to_string();
    assert!(
        !texto.contains("cbGet") && !texto.contains("cbSet") && !texto.contains("authentication"),
        "{texto}"
    );
}

#[tokio::test]
async fn secretos_no_salen_del_servidor() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let clave = "clave-conocida-del-sim-7319";
    escribir_red_24(&p, &[("key_phrase", clave)]).await;
    let (status, body) = llamar(&p, "leer_tema", json!({ "tema": "wifi" }), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert!(!body.contains(clave), "la clave sale del servidor");
    let tema: Value = serde_json::from_str(&body).unwrap();
    let red = ajuste(&tema, "wifi.red_24");
    assert_eq!(campo(red, "contrasena")["valor"], "Secreto");
}

#[tokio::test]
async fn usuarios_del_router_por_pares_y_secretos() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let tema = leer(&p, &cookie, "seguridad").await;
    let usuarios = ajuste(&tema, "seguridad.usuarios_router");
    assert_eq!(usuarios["lista"], true);
    let instancias = usuarios["instancias"].as_array().unwrap();
    let papeles: Vec<&Value> = instancias
        .iter()
        .map(|i| &campo(i, "papel")["valor"])
        .collect();
    assert_eq!(
        papeles,
        [
            &json!({ "Opcion": "operador" }),
            &json!({ "Opcion": "administrador" }),
            &json!({ "Opcion": "usuario" }),
        ]
    );
    for i in instancias {
        assert_eq!(campo(i, "nombre")["valor"], "Secreto", "{i}");
        assert_eq!(
            campo(i, "nombre")["solo_lectura"],
            "ajuste-seguridad-usuarios_router-motivo"
        );
    }
    // Los nombres de las fixtures no aparecen.
    assert!(!tema.to_string().contains("X-REDACTED-X"), "{tema}");
    // TR-069 es una opción, con la apagada recomendada.
    let remota = ajuste(&tema, "seguridad.gestion_remota");
    assert_eq!(campo(remota, "activa")["recomendado"], "apagada");
}

#[tokio::test]
async fn listas_con_sus_instancias() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let tema = leer(&p, &cookie, "puertos").await;
    let reglas = ajuste(&tema, "puertos.reglas");
    assert_eq!(reglas["componente"], "puertos");
    // En una lista, `campos` es la plantilla de una instancia nueva.
    let plantilla: Vec<&Value> = reglas["campos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["id"])
        .collect();
    let instancias = reglas["instancias"].as_array().unwrap();
    assert_eq!(instancias.len(), 1, "{reglas}");
    assert_eq!(instancias[0]["id"], "0");
    let de_la_instancia: Vec<&Value> = instancias[0]["campos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["id"])
        .collect();
    assert_eq!(plantilla, de_la_instancia);
    assert_eq!(
        campo(reglas, "protocolo")["opciones"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        campo(&instancias[0], "protocolo")["valor"],
        json!({ "Opcion": "tcp" })
    );
    assert_eq!(
        campo(&instancias[0], "externo_desde")["valor"],
        json!({ "Texto": "8080" })
    );
    let tema = leer(&p, &cookie, "red_local").await;
    let reservas = ajuste(&tema, "red_local.reservas");
    let ids: Vec<&Value> = reservas["instancias"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| &i["id"])
        .collect();
    assert_eq!(ids, [&json!("0"), &json!("1")]);
}

#[tokio::test]
async fn lectura_fallida_deja_solo_ese_ajuste_sin_leer() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    p.sim.control.fail_reads("cbGetUpnpCfg", true);
    let tema = leer(&p, &cookie, "puertos").await;
    let upnp = ajuste(&tema, "puertos.upnp");
    let motivo = upnp["estado"]["SinLeer"]
        .as_str()
        .unwrap_or_else(|| panic!("{upnp}"));
    assert!(!motivo.is_empty() && !motivo.contains("cbGet"), "{motivo}");
    assert!(upnp["campos"].as_array().unwrap().is_empty(), "{upnp}");
    assert_eq!(ajuste(&tema, "puertos.reglas")["estado"], "Leido");
    assert_eq!(ajuste(&tema, "puertos.alg")["estado"], "Leido");
}

#[tokio::test]
async fn opcion_desconocida_se_muestra_y_se_conserva() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    escribir_red_24(&p, &[("authentication", "99")]).await;
    let tema = leer(&p, &cookie, "wifi").await;
    let red = ajuste(&tema, "wifi.red_24");
    assert_eq!(
        campo(red, "seguridad")["valor"],
        json!({ "Desconocido": "99" })
    );
}

#[tokio::test]
async fn tema_desconocido_404() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let (status, cache, body) = peticion(&p, "nada", Some(&cookie)).await;
    assert_eq!(status, 404, "{body}");
    assert_eq!(cache, "no-store");
    assert!(body.contains("no existe"), "{body}");
}

/// Toda clave de traducción que da la vista existe en los dos idiomas.
#[tokio::test]
async fn claves_de_la_vista_existen() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let mut claves = Vec::new();
    let de_campo = |c: &Value, claves: &mut Vec<String>| {
        let k = c["clave"].as_str().unwrap();
        claves.push(k.to_owned());
        claves.push(format!("{k}-desc"));
        for o in c["opciones"].as_array().unwrap() {
            let op = o["clave"].as_str().unwrap();
            claves.push(format!("{k}-{op}"));
            claves.push(format!("{k}-{op}-desc"));
            claves.extend(o["aviso"].as_str().map(str::to_owned));
        }
        claves.extend(c["solo_lectura"].as_str().map(str::to_owned));
        claves.extend(c["motivo_recomendado"].as_str().map(str::to_owned));
    };
    for tema in [
        "internet",
        "wifi",
        "red_local",
        "puertos",
        "seguridad",
        "telefono",
        "sistema",
    ] {
        let t = leer(&p, &cookie, tema).await;
        assert!(!t["ajustes"].as_array().unwrap().is_empty(), "{tema}");
        for a in t["ajustes"].as_array().unwrap() {
            assert_eq!(a["estado"], "Leido", "{a}");
            for sufijo in ["titulo", "desc", "ayuda"] {
                claves.push(format!("{}-{sufijo}", a["clave"].as_str().unwrap()));
            }
            claves.extend(a["efecto"].as_str().map(str::to_owned));
            for c in a["campos"].as_array().unwrap() {
                de_campo(c, &mut claves);
            }
            for i in a["instancias"].as_array().unwrap() {
                for c in i["campos"].as_array().unwrap() {
                    de_campo(c, &mut claves);
                }
            }
        }
    }
    for idioma in i18n::Idioma::TODOS {
        let faltan: Vec<&String> = claves.iter().filter(|k| !i18n::existe(idioma, k)).collect();
        assert!(faltan.is_empty(), "faltan en {idioma:?}: {faltan:#?}");
    }
}

#[tokio::test]
async fn respuesta_no_store() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let (status, cache, body) = peticion(&p, "puertos", Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
}

/// Cada lectura distinta (función y parámetros) se hace una sola vez por
/// petición, aunque la compartan varios ajustes (`wifi.red_24` y
/// `wifi.red_24_avanzado`).
#[tokio::test]
async fn cada_lectura_una_sola_vez() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    for tema in ["wifi", "seguridad"] {
        let antes = p.sim.control.reads().len();
        leer(&p, &cookie, tema).await;
        let leidas: Vec<_> = p.sim.control.reads().into_iter().skip(antes).collect();
        assert!(!leidas.is_empty(), "{tema}: ninguna lectura");
        let mut vistas = Vec::new();
        for l in &leidas {
            assert!(!vistas.contains(l), "{tema}: {l:?} repetida");
            vistas.push(l.clone());
        }
        let distintas: std::collections::BTreeSet<_> = p
            .state
            .ajustes
            .por_tema(ajustes::Tema::from_slug(tema).unwrap())
            .map(|a| (a.leer.funcion.clone(), a.leer.params.clone()))
            .collect();
        assert_eq!(leidas.len(), distintas.len(), "{tema}: {leidas:#?}");
    }
    let red_24 = (
        "cbGetWifiAccPt".to_owned(),
        vec![("dpm_cfg_wifi_acc_pt_id".to_owned(), "0".to_owned())],
    );
    let veces = p
        .sim
        .control
        .reads()
        .iter()
        .filter(|l| **l == red_24)
        .count();
    assert_eq!(veces, 1);
}

/// El id de una instancia es el del router, no su posición: borrada la
/// primera reserva, la que queda conserva el suyo.
#[tokio::test]
async fn ids_de_instancia_no_son_la_posicion() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("del", "DEL"),
            ],
        )
        .await
        .unwrap();
    let tema = leer(&p, &cookie, "red_local").await;
    let reservas = ajuste(&tema, "red_local.reservas");
    let ids: Vec<&Value> = reservas["instancias"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| &i["id"])
        .collect();
    assert_eq!(ids, [&json!("1")], "{reservas}");
}

/// En una lista que se escribe, una respuesta sin el id de las instancias
/// deja el ajuste sin leer: editar o borrar por posición podría tocar otra
/// instancia. El resto del tema se lee.
#[tokio::test]
async fn lista_escrita_sin_id_queda_sin_leer() {
    let p = panel_con_capa(&variante("listas-irregulares")).await;
    let cookie = sesion(&p).await;
    let tema = leer(&p, &cookie, "red_local").await;
    let reservas = ajuste(&tema, "red_local.reservas");
    assert!(reservas["estado"]["SinLeer"].is_string(), "{reservas}");
    assert!(reservas["instancias"].as_array().unwrap().is_empty());
    assert!(reservas["campos"].as_array().unwrap().is_empty());
    assert_eq!(ajuste(&tema, "red_local.dhcp")["estado"], "Leido");
}

/// Los usuarios del router se emparejan solo con los elementos de sus
/// campos: una hoja suelta en la respuesta no crea un usuario de más.
#[tokio::test]
async fn usuarios_emparejados_solo_por_sus_elementos() {
    let p = panel_con_capa(&variante("listas-irregulares")).await;
    let cookie = sesion(&p).await;
    let tema = leer(&p, &cookie, "seguridad").await;
    let usuarios = ajuste(&tema, "seguridad.usuarios_router");
    let instancias = usuarios["instancias"].as_array().unwrap();
    let papeles: Vec<&Value> = instancias
        .iter()
        .map(|i| &campo(i, "papel")["valor"])
        .collect();
    assert_eq!(
        papeles,
        [
            &json!({ "Opcion": "operador" }),
            &json!({ "Opcion": "administrador" }),
            &json!({ "Opcion": "usuario" }),
        ],
        "{usuarios}"
    );
    let ids: Vec<&Value> = instancias.iter().map(|i| &i["id"]).collect();
    assert_eq!(ids, [&json!("0"), &json!("1"), &json!("2")]);
}

/// La plantilla de una lista nunca dice «valor no reconocido»: una opción
/// empieza en la recomendada o la primera, un sí/no en el recomendado o en
/// no, y el resto vacío.
#[tokio::test]
async fn plantilla_de_lista_sin_desconocidos() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let tema = leer(&p, &cookie, "puertos").await;
    let reglas = ajuste(&tema, "puertos.reglas");
    assert_eq!(
        campo(reglas, "protocolo")["valor"],
        json!({ "Opcion": "tcp" })
    );
    assert_eq!(
        campo(reglas, "activa")["valor"],
        json!({ "Booleano": false })
    );
    assert_eq!(campo(reglas, "nombre")["valor"], json!({ "Texto": "" }));
    assert_eq!(campo(reglas, "destino")["valor"], json!({ "Texto": "" }));
    for tema in [
        "internet",
        "wifi",
        "red_local",
        "puertos",
        "seguridad",
        "telefono",
        "sistema",
    ] {
        let t = leer(&p, &cookie, tema).await;
        for a in t["ajustes"].as_array().unwrap() {
            if a["lista"] != true {
                continue;
            }
            for c in a["campos"].as_array().unwrap() {
                assert!(c["valor"].get("Desconocido").is_none(), "{}: {c}", a["id"]);
            }
        }
    }
}

/// Un catálogo de ajustes que no cuadra con el de funciones no deja
/// arrancar el panel, y cada fallo se dice.
#[test]
fn catalogo_roto_no_arranca() {
    let roto = ajustes::CatalogoAjustes::from_toml(
        &[(
            "wifi",
            r#"
[[ajuste]]
id = "wifi.prueba"
tema = "wifi"
nivel = "basico"
leer = { funcion = "cbNoExiste" }
"#,
        )],
        "",
    )
    .unwrap();
    let f = catalogo::Catalogo::load().unwrap();
    let m = catalogo::mask::Mascara::load().unwrap();
    let e = panel::state::comprobar_ajustes(roto, &f, &m).unwrap_err();
    assert!(e.contains("wifi.prueba"), "{e}");
    let real = ajustes::CatalogoAjustes::load().unwrap();
    assert!(panel::state::comprobar_ajustes(real, &f, &m).is_ok());
}

// ---------------------------------------------------------------- guardar

/// `preparar_ajuste` del ajuste `ajuste` con los valores legibles `valores`
/// (`{campo: valor}`): estado y cuerpo.
async fn preparar_ajuste(p: &Panel, cookie: &str, ajuste: &str, valores: Value) -> (u16, String) {
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": ajuste, "instancia": null, "borrar": false, "valores": valores } }),
        Some(cookie),
    )
    .await
}

/// Como [`preparar_ajuste`], exigiendo 200: el resumen (`GrupoVista`).
async fn resumen(p: &Panel, cookie: &str, ajuste: &str, valores: Value) -> Value {
    let (status, body) = preparar_ajuste(p, cookie, ajuste, valores).await;
    assert_eq!(status, 200, "preparar_ajuste {ajuste}: {body}");
    serde_json::from_str(&body).unwrap()
}

/// Confirma el resumen `grupo` sin los pasos `desmarcados`; devuelve el id.
async fn confirmar(p: &Panel, cookie: &str, grupo: &Value, desmarcados: &[usize]) -> i64 {
    ok(
        p,
        "confirmar_ajuste",
        json!({ "token": grupo["token"], "desmarcados": desmarcados }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap()
}

/// Elemento `nombre` del punto de acceso `id` en el simulador.
async fn del_punto_de_acceso(p: &Panel, id: &str, nombre: &str) -> String {
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", id)])
        .await
        .unwrap();
    elemento(&n, nombre).unwrap_or_else(|| panic!("falta {nombre}"))
}

/// Campos cambiados de todos los pasos de un resumen.
fn cambios(grupo: &Value) -> Vec<&Value> {
    grupo["pasos"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["campos"].as_array().unwrap())
        .collect()
}

#[tokio::test]
async fn guardar_nombre_no_toca_la_contrasena() {
    let (p, cookie) = panel_con_gestor().await;
    let clave = del_punto_de_acceso(&p, "0", "key_phrase").await;
    let seguridad = del_punto_de_acceso(&p, "0", "authentication").await;
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    let campos = cambios(&g);
    assert_eq!(campos.len(), 1, "{g}");
    assert_eq!(campos[0]["campo"], "ajuste-wifi-red_24-nombre");
    assert_eq!(campos[0]["despues"], "casa-nueva");
    assert_eq!(campos[0]["secreto"], false);
    assert_eq!(g["titulo"], "ajuste-wifi-red_24-titulo");
    let id = confirmar(&p, &cookie, &g, &[]).await;
    let c = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
    assert_eq!(del_punto_de_acceso(&p, "0", "ssid").await, "casa-nueva");
    assert_eq!(del_punto_de_acceso(&p, "0", "key_phrase").await, clave);
    assert_eq!(
        del_punto_de_acceso(&p, "0", "authentication").await,
        seguridad
    );
}

#[tokio::test]
async fn guardar_otro_campo_conserva_la_opcion_desconocida() {
    let (p, cookie) = panel_con_gestor().await;
    escribir_red_24(&p, &[("authentication", "99")]).await;
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    assert_eq!(cambios(&g).len(), 1, "{g}");
    let id = confirmar(&p, &cookie, &g, &[]).await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    assert_eq!(del_punto_de_acceso(&p, "0", "authentication").await, "99");
    assert_eq!(del_punto_de_acceso(&p, "0", "ssid").await, "casa-nueva");
}

#[tokio::test]
async fn antes_es_el_valor_del_router_no_el_de_la_pagina() {
    let (p, cookie) = panel_con_gestor().await;
    let tema = leer(&p, &cookie, "wifi").await;
    let en_la_pagina = campo(ajuste(&tema, "wifi.red_24"), "nombre")["valor"]["Texto"].clone();
    escribir_red_24(&p, &[("ssid", "cambiado-fuera")]).await;
    assert_ne!(en_la_pagina, "cambiado-fuera");
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    let campos = cambios(&g);
    assert_eq!(campos[0]["antes"], "cambiado-fuera", "{g}");
    assert_eq!(campos[0]["despues"], "casa-nueva");
}

/// Ajuste de prueba con dos funciones de escritura: UPnP (`adm_state` 0 en
/// las fixtures) y el primer servidor de hora.
const DOS_FUNCIONES: &str = r#"
[[ajuste]]
id = "sistema.prueba"
tema = "sistema"
nivel = "basico"
leer = { funcion = "cbGetUpnpCfg" }

[[ajuste.campo]]
id = "upnp"
escribe = { funcion = "cbSetUpnpCfg", parametro = "adm_state" }
lee = "adm_state"
tipo = "booleano"

[[ajuste.campo]]
id = "hora"
escribe = { funcion = "cbSetTimeNtp", parametro = "ntp_svr_1" }
lee = "ntp_svr_1"
tipo = "texto"
"#;

#[tokio::test]
async fn desmarcar_un_paso_no_lo_escribe() {
    let (p, cookie) = panel_con_ajustes(DOS_FUNCIONES).await;
    let g = resumen(
        &p,
        &cookie,
        "sistema.prueba",
        json!({ "upnp": "true", "hora": "hora.example" }),
    )
    .await;
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(pasos.len(), 2, "{g}");
    assert_eq!(pasos[0]["indice"], 0);
    assert_eq!(pasos[0]["campos"][0]["campo"], "ajuste-sistema-prueba-upnp");
    assert_eq!(pasos[0]["campos"][0]["antes"], "false");
    assert_eq!(pasos[0]["campos"][0]["despues"], "true");
    assert_eq!(pasos[1]["campos"][0]["campo"], "ajuste-sistema-prueba-hora");
    let id = confirmar(&p, &cookie, &g, &[1]).await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    let escritas: Vec<String> = p.sim.control.writes().into_iter().map(|(f, _)| f).collect();
    assert_eq!(escritas, ["cbSetUpnpCfg"]);
}

/// Panel con una vigilancia larga: el primer cambio sigue vigilándose
/// mientras se prepara el segundo.
async fn panel_vigilando() -> (Panel, String) {
    let mut t = tiempos_rapidos();
    t.bajo = Duration::from_secs(30);
    t.medio = Duration::from_secs(30);
    t.alto = Duration::from_secs(30);
    panel_con_tiempos(t).await
}

/// Aplica `primero` y, mientras se vigila, prepara `segundo`: 409.
async fn segundo_durante_la_vigilancia(primero: (&str, Value), segundo: (&str, Value)) {
    let (p, cookie) = panel_vigilando().await;
    let g = resumen(&p, &cookie, primero.0, primero.1).await;
    let id = confirmar(&p, &cookie, &g, &[]).await;
    let c = ok(&p, "estado_cambio", json!({ "id": id }), &cookie).await;
    assert_eq!(c["estado"], "Vigilando", "{c}");
    let (status, body) = preparar_ajuste(&p, &cookie, segundo.0, segundo.1).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("otro cambio en curso"), "{body}");
}

#[tokio::test]
async fn segundo_ajuste_con_cambio_activo_da_409() {
    segundo_durante_la_vigilancia(
        ("wifi.red_24", json!({ "nombre": "casa-nueva" })),
        ("wifi.red_5", json!({ "nombre": "casa-nueva-5" })),
    )
    .await;
}

#[tokio::test]
async fn par_basico_y_avanzado_con_cambio_activo_da_409() {
    segundo_durante_la_vigilancia(
        ("wifi.red_24", json!({ "nombre": "casa-nueva" })),
        ("wifi.red_24_avanzado", json!({ "max_clientes": "7" })),
    )
    .await;
}

#[tokio::test]
async fn guardar_lo_mismo_da_409_ya_esta_asi() {
    let (p, cookie) = panel_con_gestor().await;
    let ssid = del_punto_de_acceso(&p, "0", "ssid").await;
    let (status, body) =
        preparar_ajuste(&p, &cookie, "wifi.red_24", json!({ "nombre": ssid })).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("Ya está así"), "{body}");
}

#[tokio::test]
async fn solo_lectura_da_403() {
    let (p, cookie) = panel_con_gestor().await;
    instancia::escribir(&p.state.db, claves::SOLO_LECTURA, &json!(true))
        .await
        .unwrap();
    let (status, body) = preparar_ajuste(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("solo se pueden consultar"), "{body}");
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn valor_invalido_da_400_en_el_idioma() {
    let (p, cookie) = panel_con_gestor().await;
    let largo = "x".repeat(33);
    let (status, body) =
        preparar_ajuste(&p, &cookie, "wifi.red_24", json!({ "nombre": largo })).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("Demasiado largo"), "{body}");
    let en = format!("{cookie}; rp_lang=en");
    let (status, body) = preparar_ajuste(&p, &en, "wifi.red_24", json!({ "nombre": "" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("cannot be left empty"), "{body}");
    // Ajuste o campo que no existen.
    let (status, body) =
        preparar_ajuste(&p, &cookie, "wifi.no_existe", json!({ "nombre": "x" })).await;
    assert_eq!(status, 404, "{body}");
    let (status, body) = preparar_ajuste(&p, &cookie, "wifi.red_24", json!({ "ssid": "x" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(p.sim.control.writes().is_empty());
}

/// `texto` no nombra funciones de la API (`cbGet…`, `cbSet…`).
fn sin_funciones_de_la_api(texto: &str) -> bool {
    let b = texto.as_bytes();
    !(0..b.len().saturating_sub(2)).any(|i| {
        let limite = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        limite && &b[i..i + 2] == b"cb" && b[i + 2].is_ascii_uppercase()
    })
}

#[test]
fn detecta_funciones_de_la_api() {
    assert!(!sin_funciones_de_la_api("{\"f\":\"cbSetWifiAccPt\"}"));
    assert!(!sin_funciones_de_la_api("cbGetX"));
    assert!(sin_funciones_de_la_api("abcbSet cb cbx"));
}

#[tokio::test]
async fn resumen_sin_nombres_de_api() {
    let (p, cookie) = panel_con_gestor().await;
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva", "seguridad": "abierta", "contrasena": "clave-secreta-1" }),
    )
    .await;
    let texto = g.to_string();
    assert!(sin_funciones_de_la_api(&texto), "{texto}");
    for api in ["ssid", "authentication", "encryption", "key_phrase"] {
        assert!(!texto.contains(api), "{api}: {texto}");
    }
    assert!(!texto.contains("clave-secreta-1"), "{texto}");
    let clave = cambios(&g)
        .into_iter()
        .find(|c| c["campo"] == "ajuste-wifi-red_24-contrasena")
        .unwrap()
        .clone();
    assert_eq!(clave["secreto"], true);
    assert_eq!(clave["antes"], clave["despues"]);
    assert_eq!(clave["antes"], "••••••");
    let seguridad = cambios(&g)
        .into_iter()
        .find(|c| c["campo"] == "ajuste-wifi-red_24-seguridad")
        .unwrap()
        .clone();
    assert_eq!(seguridad["antes"], "wpa2");
    assert_eq!(seguridad["despues"], "abierta");
}

#[tokio::test]
async fn guardar_exige_sesion() {
    let (p, _) = panel_con_gestor().await;
    let (status, body) = llamar(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "wifi.red_24", "instancia": null, "borrar": false, "valores": { "nombre": "x" } } }),
        None,
    )
    .await;
    assert_eq!(status, 401, "{body}");
}

#[tokio::test]
async fn borrar_en_un_ajuste_que_no_es_lista_da_400() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, body) = llamar(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "wifi.red_24", "instancia": null, "borrar": true, "valores": {} } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("no ha podido preparar"), "{body}");
    assert!(p.sim.control.writes().is_empty());
}

/// `deshacer_ajuste` del cambio `id`: estado y cuerpo.
async fn deshacer(p: &Panel, cookie: &str, id: i64) -> (u16, String) {
    llamar(p, "deshacer_ajuste", json!({ "cambio": id }), Some(cookie)).await
}

#[tokio::test]
async fn deshacer_un_cambio_que_no_es_de_ajustes_da_404() {
    let (p, cookie) = panel_con_gestor().await;
    // Un cambio del explorador: sin título de ajuste.
    let id = aplicar(
        &p,
        &cookie,
        json!({ "funcion": "cbSetUpnpCfg", "instancia": null, "valores": [["adm_state", "1"]], "borrar": false }),
    )
    .await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    esperar_libre(&p).await;
    let (status, body) = deshacer(&p, &cookie, id).await;
    assert_eq!(status, 404, "{body}");
    assert!(body.contains("no se hizo desde Ajustes"), "{body}");
}

#[tokio::test]
async fn deshacer_en_vigilancia_da_400() {
    let (p, cookie) = panel_vigilando().await;
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    let id = confirmar(&p, &cookie, &g, &[]).await;
    let (status, body) = deshacer(&p, &cookie, id).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("ya no se puede deshacer"), "{body}");
}

#[tokio::test]
async fn deshacer_dos_veces_da_400() {
    let (p, cookie) = panel_con_gestor().await;
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    let id = confirmar(&p, &cookie, &g, &[]).await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    esperar_libre(&p).await;
    let (status, body) = deshacer(&p, &cookie, id).await;
    assert_eq!(status, 200, "{body}");
    let inverso: Value = serde_json::from_str(&body).unwrap();
    let otro = confirmar(&p, &cookie, &inverso, &[]).await;
    assert_eq!(esperar_final(&p, &cookie, otro).await["estado"], "Aplicado");
    esperar_libre(&p).await;
    let (status, body) = deshacer(&p, &cookie, id).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("ya no se puede deshacer"), "{body}");
    // El deshacer sí se puede deshacer, y sigue siendo del mismo ajuste.
    let (status, body) = deshacer(&p, &cookie, otro).await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("ajuste-wifi-red_24-titulo"), "{body}");
}

/// Espejo de puerto: encenderlo sin elegir la toma escribiría la toma 0. El
/// servidor lo rechaza (400, en el idioma de la petición, junto a la toma) y
/// lo acepta con la toma; la vista lleva la regla para la página.
#[tokio::test]
async fn espejo_sin_toma_da_400() {
    let (p, cookie) = panel_con_gestor().await;
    let tema = leer(&p, &cookie, "sistema").await;
    let espejo = campo(ajuste(&tema, "sistema.diagnostico"), "espejo");
    assert_eq!(
        espejo["requiere"],
        json!({ "campo": "puerto", "cuando": "true" }),
        "{espejo}"
    );
    let (status, body) = preparar_ajuste(
        &p,
        &cookie,
        "sistema.diagnostico",
        json!({ "espejo": "true" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("Toma que recibe la copia"), "{body}");
    assert!(body.contains("Copia encendida"), "{body}");
    let en = format!("{cookie}; rp_lang=en");
    let (status, body) =
        preparar_ajuste(&p, &en, "sistema.diagnostico", json!({ "espejo": "true" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("Socket that receives the copy"), "{body}");
    assert!(p.sim.control.writes().is_empty());
    // Apagarlo no exige toma; encenderlo con ella, sí vale.
    let (status, body) = preparar_ajuste(
        &p,
        &cookie,
        "sistema.diagnostico",
        json!({ "espejo": "true", "puerto": "puerto_2" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
}

// ------------------------------------------------- páginas /ajustes/<tema>

/// La página `ruta` como la pide el navegador: estado y HTML.
async fn pagina(p: &Panel, cookie: &str, ruta: &str) -> (u16, String, String) {
    let resp = send(
        &p.app,
        axum::http::Request::get(ruta)
            .header("cookie", cookie)
            .header("accept", "text/html")
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await;
    let status = resp.status().as_u16();
    let location = resp
        .headers()
        .get(header::LOCATION)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    (status, location, body_text(resp).await)
}

/// `html` sin los bloques `<abre …>…</cierra>` cuya apertura empieza por
/// `abre` (sin anidar), y esos bloques aparte.
fn partir<'a>(html: &'a str, abre: &str, cierra: &str) -> (String, Vec<&'a str>) {
    let mut fuera = String::new();
    let mut dentro = Vec::new();
    let mut resto = html;
    while let Some(i) = resto.find(abre) {
        fuera.push_str(&resto[..i]);
        let fin = i + resto[i..].find(cierra).expect("bloque sin cerrar") + cierra.len();
        dentro.push(&resto[i..fin]);
        resto = &resto[fin..];
    }
    fuera.push_str(resto);
    (fuera, dentro)
}

/// Texto que se lee en pantalla (o en voz alta) de un trozo de HTML: el de
/// fuera de las etiquetas y los atributos que se leen.
fn texto_legible(html: &str) -> String {
    let atributos =
        regex::Regex::new(r#"\b(?:aria-label|title|placeholder|alt)="([^"]*)""#).unwrap();
    let mut out: String = atributos
        .captures_iter(html)
        .map(|c| format!("{} ", &c[1]))
        .collect();
    let etiquetas = regex::Regex::new(r"<[^>]*>").unwrap();
    out.push_str(&etiquetas.replace_all(html, " "));
    out
}

/// Nombres de parámetro del catálogo de ajustes (lecturas, fijos, campos y
/// sus `ademas`).
fn parametros_del_catalogo() -> Vec<String> {
    let c = ajustes::CatalogoAjustes::load().unwrap();
    let mut nombres = std::collections::BTreeSet::new();
    for a in c.todos() {
        nombres.extend(a.leer.params.iter().map(|(k, _)| k.clone()));
        nombres.extend(a.fijos.keys().cloned());
        for x in &a.campos {
            nombres.insert(x.lee.clone());
            nombres.extend(x.escribe.iter().map(|e| e.parametro.clone()));
            for ad in &x.ademas {
                nombres.insert(ad.parametro.clone());
                nombres.insert(ad.lee.clone());
            }
        }
    }
    nombres.into_iter().collect()
}

/// Palabras corrientes que también son parámetros (las mismas que deja
/// pasar `ajustes::catalogo_real`).
const FORMAS_CORRIENTES: [&str; 9] = [
    "account",
    "Account",
    "all",
    "All",
    "beamforming",
    "isolation",
    "password",
    "Password",
    "WMM",
];

#[tokio::test]
async fn pagina_tema_sin_nombres_de_api_fuera_de_detalles() {
    let (p, cookie) = panel_con_gestor().await;
    let funcion = regex::Regex::new(r"\bcb[A-Z]\w+").unwrap();
    let pagina_api = regex::Regex::new(r"\bte_\w+\.(asp|cmd|cgi|wl)\b").unwrap();
    let parametros: Vec<regex::Regex> = parametros_del_catalogo()
        .iter()
        .map(|n| regex::Regex::new(&format!(r"(?i)\b{}\b", regex::escape(n))).unwrap())
        .collect();
    for tema in ["wifi", "puertos", "seguridad"] {
        let ruta = format!("/ajustes/{tema}");
        let (status, _, html) = pagina(&p, &cookie, &ruta).await;
        assert_eq!(status, 200, "{ruta}: {html}");
        let (sin_detalles, detalles) =
            partir(&html, r#"<details class="detalles-tecnicos""#, "</details>");
        assert!(!detalles.is_empty(), "{ruta}: sin detalles técnicos");
        // Los datos de la hidratación (`<script>`) no se pintan: llevan los
        // detalles técnicos para hidratar sus bloques plegados.
        let (visible, _) = partir(&sin_detalles, "<script", "</script>");
        let api: Vec<&str> = funcion
            .find_iter(&visible)
            .chain(pagina_api.find_iter(&visible))
            .map(|m| m.as_str())
            .collect();
        assert!(
            api.is_empty(),
            "{ruta}: {api:?} fuera de los detalles técnicos"
        );
        let texto = texto_legible(&visible);
        for p in &parametros {
            for m in p.find_iter(&texto) {
                assert!(
                    FORMAS_CORRIENTES.contains(&m.as_str()),
                    "{ruta}: parámetro «{}» en el texto",
                    m.as_str()
                );
            }
        }
        if tema == "wifi" {
            assert!(
                detalles.iter().any(|d| d.contains("cbSetWifiAccPt")),
                "{ruta}: {detalles:#?}"
            );
        }
    }
}

#[tokio::test]
async fn pagina_tema_sin_secretos() {
    let (p, cookie) = panel_con_gestor().await;
    let clave = "clave-conocida-del-sim-7";
    escribir_red_24(&p, &[("key_phrase", clave)]).await;
    let (status, _, html) = pagina(&p, &cookie, "/ajustes/wifi").await;
    assert_eq!(status, 200, "{html}");
    assert!(html.contains("Red WiFi 2,4 GHz"), "{html}");
    assert!(!html.contains(clave), "la clave sale en el HTML");
}

#[tokio::test]
async fn ajustes_lleva_a_internet() {
    let (p, cookie) = panel_con_gestor().await;
    for ruta in ["/ajustes", "/ajustes/"] {
        let (status, location, body) = pagina(&p, &cookie, ruta).await;
        assert_eq!(status, 302, "{ruta}: {body}");
        assert_eq!(location, "/ajustes/internet", "{ruta}");
    }
}

#[tokio::test]
async fn tema_desconocido_no_encontrada() {
    let (p, cookie) = panel_con_gestor().await;
    let (status, _, html) = pagina(&p, &cookie, "/ajustes/no-existe").await;
    assert_eq!(status, 404, "{html}");
    assert!(html.contains("Página no encontrada"), "{html}");
    assert!(p.sim.control.writes().is_empty());
}
