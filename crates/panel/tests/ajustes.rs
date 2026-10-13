//! Lectura de los ajustes del router por tema (`leer_tema`): valores
//! legibles, secretos que no salen del servidor, lecturas que fallan solas y
//! el catálogo comprobado al arrancar.

mod common;

use axum::http::header;
use common::{
    Panel, body_text, llamar, ok, panel, panel_con_capa, post_json, send, sesion, variante,
};
use serde_json::{Value, json};

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
