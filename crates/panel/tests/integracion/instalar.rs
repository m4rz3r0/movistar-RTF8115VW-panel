//! Pruebas del asistente de primera instalación: token de un solo uso,
//! bloqueo de todo lo demás sin usuarios, foto de partida cifrada y modo solo
//! lectura con un firmware no probado.

use crate::comun::{
    PANEL_PASS, PANEL_USER, PASS, Panel, PanelPrueba, USER, body_text, get, get_con_cookie,
    post_json, send, set_cookie,
};
use axum::{body::Body, http::Request};
use panel::{
    changes::{CambioError, ChangeManager, ChangeRequest, Tiempos},
    fotos::{self, TipoFoto},
    instalar,
    instancia::{self, claves},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, net::Ipv4Addr, path::PathBuf};

/// SSID de una de las redes de las fixtures.
const SSID: &str = "RED-1";

/// Fichero del token: junto a la base de datos.
fn fichero_token(p: &Panel) -> PathBuf {
    p.state.cfg.db_path.parent().unwrap().join("instalar.token")
}

/// Panel recién instalado (sin usuarios ni router) con el token generado al
/// arrancar.
async fn recien_instalado() -> Panel {
    let p = PanelPrueba::nuevo().sin_router().arrancar().await;
    assert!(instalar::token_inicial(&p.state).await.is_some());
    p
}

/// Llama a una server function desde `peer`, con cookie opcional.
async fn llamar_desde(
    p: &Panel,
    nombre: &str,
    cuerpo: Value,
    cookie: Option<&str>,
    peer: &str,
) -> (u16, String, Option<String>) {
    let mut req = post_json(&format!("/api/{nombre}"), &cuerpo.to_string(), peer);
    if let Some(c) = cookie {
        req.headers_mut().insert("cookie", c.parse().unwrap());
    }
    let resp = send(&p.app, req).await;
    let status = resp.status().as_u16();
    let cookie = resp.headers().get("set-cookie").map(|_| {
        set_cookie(&resp)
            .split(';')
            .next()
            .unwrap()
            .trim()
            .to_owned()
    });
    (status, body_text(resp).await, cookie)
}

async fn llamar(p: &Panel, nombre: &str, cuerpo: Value, cookie: Option<&str>) -> (u16, String) {
    let (s, b, _) = llamar_desde(p, nombre, cuerpo, cookie, "127.0.0.1").await;
    (s, b)
}

async fn usuarios(p: &Panel) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&p.state.db)
        .await
        .unwrap()
}

/// `crear_admin` con el token del fichero; devuelve la cookie de sesión.
async fn crear_admin(p: &Panel) -> String {
    let token = std::fs::read_to_string(fichero_token(p)).unwrap();
    let (status, body, cookie) = llamar_desde(
        p,
        "crear_admin",
        json!({ "token": token.trim(), "usuario": PANEL_USER, "contrasena": PANEL_PASS }),
        None,
        "127.0.0.1",
    )
    .await;
    assert_eq!(status, 200, "{body}");
    cookie.expect("crear_admin abre la sesión")
}

fn router(p: &Panel) -> Value {
    json!({ "token": "", "url": p.sim.base_url, "usuario": USER, "contrasena": PASS })
}

fn json_de(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|e| panic!("{e}: {body}"))
}

#[tokio::test]
async fn sin_usuarios_redirige_a_instalar() {
    let p = recien_instalado().await;
    for ruta in ["/", "/login", "/historial", "/explorador"] {
        let resp = get(&p.app, ruta).await;
        assert_eq!(resp.status(), 303, "{ruta}");
        assert_eq!(resp.headers()["location"], "/instalar", "{ruta}");
    }
    assert_eq!(get(&p.app, "/instalar").await.status(), 200);
    assert_eq!(get(&p.app, "/salud").await.status(), 200);
    assert_eq!(get(&p.app, "/pkg/panel.css").await.status(), 200);
}

#[tokio::test]
async fn instalar_sin_token_403() {
    let p = recien_instalado().await;
    let admin =
        |token: Value| json!({ "token": token, "usuario": PANEL_USER, "contrasena": PANEL_PASS });
    for token in [json!("malo"), json!("")] {
        let (status, body) = llamar(&p, "crear_admin", admin(token.clone()), None).await;
        assert_eq!(status, 403, "{token}: {body}");
    }
    // Sin el campo: tampoco.
    let (status, _) = llamar(
        &p,
        "crear_admin",
        json!({ "usuario": PANEL_USER, "contrasena": PANEL_PASS }),
        None,
    )
    .await;
    assert_eq!(status, 403);
    // El resto de pasos, igual.
    let (status, _) = llamar(&p, "probar_router", router(&p), None).await;
    assert_eq!(status, 403);
    let (status, _) = llamar(&p, "guardar_router", router(&p), None).await;
    assert_eq!(status, 403);
    let (status, _) = llamar(&p, "lectura_inicial", json!({ "token": "malo" }), None).await;
    assert_eq!(status, 403);
    // Con el token bueno, pero desde fuera de la red local.
    let token = std::fs::read_to_string(fichero_token(&p)).unwrap();
    let (status, _, _) = llamar_desde(
        &p,
        "crear_admin",
        admin(json!(token.trim())),
        None,
        "203.0.113.9",
    )
    .await;
    assert_eq!(status, 403);
    assert_eq!(usuarios(&p).await, 0);
    assert!(!p.state.hgu.configurado());
}

/// `RP_INSTALL_NETS` amplía la red local del asistente (p. ej. con la red de
/// una VPN): sin ella, un cliente de esa red recibe 403; con ella, pasa.
#[tokio::test]
async fn red_de_instalacion_opcional() {
    const VPN: &str = "203.0.113.5";
    let crear_desde_la_vpn = |p: Panel| async move {
        assert!(instalar::token_inicial(&p.state).await.is_some());
        let token = std::fs::read_to_string(fichero_token(&p)).unwrap();
        let admin =
            json!({ "token": token.trim(), "usuario": PANEL_USER, "contrasena": PANEL_PASS });
        let (status, body, _) = llamar_desde(&p, "crear_admin", admin, None, VPN).await;
        (status, body, usuarios(&p).await)
    };
    let (status, body, n) =
        crear_desde_la_vpn(PanelPrueba::nuevo().sin_router().arrancar().await).await;
    assert_eq!((status, n), (403, 0), "{body}");
    let p = PanelPrueba::nuevo()
        .sin_router()
        .config(|c| {
            c.install_nets = panel::redes::lista("203.0.113.0/24").unwrap();
        })
        .arrancar()
        .await;
    let (status, body, n) = crear_desde_la_vpn(p).await;
    assert_eq!((status, n), (200, 1), "{body}");
}

#[tokio::test]
async fn con_usuarios_no_hay_asistente() {
    let p = PanelPrueba::nuevo().sin_router().arrancar().await;
    // Un token que quedó de otra vez.
    std::fs::write(fichero_token(&p), "viejo\n").unwrap();
    panel::auth::create_user(&p.state.db, PANEL_USER, PANEL_PASS)
        .await
        .unwrap();
    assert_eq!(instalar::token_inicial(&p.state).await, None);
    assert!(!fichero_token(&p).exists(), "el fichero del token sobra");
    assert_eq!(get(&p.app, "/instalar").await.status(), 404);
    let (status, _) = llamar(
        &p,
        "crear_admin",
        json!({ "token": "viejo", "usuario": "otro", "contrasena": PANEL_PASS }),
        None,
    )
    .await;
    assert_eq!(status, 404);
    // Con sesión tampoco: la instalación no está en curso.
    let cookie = crate::comun::sesion_de(&p).await;
    assert_eq!(
        get_con_cookie(&p.app, "/instalar", &cookie).await.status(),
        404
    );
    let (status, _) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 404);
    assert_eq!(usuarios(&p).await, 1);
    // Y las páginas piden sesión como siempre.
    let resp = get(&p.app, "/").await;
    assert_eq!(resp.status(), 303);
    assert_eq!(resp.headers()["location"], "/login?next=%2F");
}

#[tokio::test]
async fn flujo_completo_con_sim() {
    let p = recien_instalado().await;
    let cookie = crear_admin(&p).await;
    assert!(!fichero_token(&p).exists(), "el token es de un solo uso");
    assert_eq!(usuarios(&p).await, 1);
    // El token ya no vale para otro administrador.
    let (status, _) = llamar(
        &p,
        "crear_admin",
        json!({ "token": "", "usuario": "otro", "contrasena": PANEL_PASS }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 404);
    // Sin sesión, el resto del asistente no responde.
    let (status, _) = llamar(&p, "probar_router", router(&p), None).await;
    assert_eq!(status, 401);

    let (status, body) = llamar(&p, "probar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    let info = json_de(&body);
    assert_eq!(info["modelo"], "RTF8115VW");
    assert_eq!(info["firmware"], "ES_g14.17_RTF_TEF001_V9.17_V034");
    assert_eq!(info["probado"], true);
    assert!(!p.state.hgu.configurado(), "probar no guarda nada");

    let mut malo = router(&p);
    malo["contrasena"] = json!("otra");
    let (status, _) = llamar(&p, "guardar_router", malo, Some(&cookie)).await;
    assert_eq!(status, 502, "no guarda credenciales que no entran");
    assert!(!p.state.hgu.configurado());

    let (status, body) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert!(p.state.hgu.configurado());
    let guardadas = instancia::leer_credenciales(&p.state.db, &p.state.history_key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(guardadas.url, p.sim.base_url);
    assert_ne!(
        instancia::leer(&p.state.db, claves::SOLO_LECTURA).await,
        Some(json!(true))
    );

    let (status, body) = llamar(&p, "lectura_inicial", json!({ "token": "" }), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    let resumen = json_de(&body);
    assert!(
        resumen["funciones_leidas"].as_u64().unwrap() >= 40,
        "{resumen}"
    );
    assert_eq!(resumen["fallidas"], json!([]), "{resumen}");
    assert!(
        fotos::ultima(&p.state.db, &p.state.history_key, TipoFoto::Partida)
            .await
            .is_some()
    );

    // Terminado: sesión abierta, el asistente ya no está.
    assert!(!fichero_token(&p).exists());
    assert_eq!(get_con_cookie(&p.app, "/", &cookie).await.status(), 200);
    assert_eq!(
        get_con_cookie(&p.app, "/instalar", &cookie).await.status(),
        404
    );
    let (status, _) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 404);
}

/// Petición de escritura cualquiera.
fn escritura() -> ChangeRequest {
    ChangeRequest {
        funcion: "cbSetUpnpCfg".into(),
        instancia: None,
        valores: BTreeMap::from([("adm_state".to_owned(), "1".to_owned())]),
        borrar: false,
        identidad: Default::default(),
    }
}

#[tokio::test]
async fn firmware_no_probado_queda_en_solo_lectura() {
    let p = recien_instalado().await;
    p.sim
        .control
        .set_firmware("ES_g99.00_RTF_TEF001_V99.00_V001");
    let cookie = crear_admin(&p).await;
    let (status, body) = llamar(&p, "probar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(json_de(&body)["probado"], false);
    let (status, body) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        instancia::leer(&p.state.db, claves::SOLO_LECTURA).await,
        Some(json!(true))
    );

    let gestor = ChangeManager::instalar(&p.state, Tiempos::default());
    match gestor.preparar(escritura()).await {
        Err(CambioError::Bloqueada(m)) => assert_eq!(m, "panel-solo-lectura"),
        r => panic!("{r:?}"),
    }
    // Desde la interfaz, con el motivo traducido.
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetUpnpCfg", "instancia": null,
            "valores": [["adm_state", "1"]], "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("solo lectura"), "{body}");

    // Confirmado el firmware (lo hará «Configuración del router → Conexión
    // con el router»), se escribe.
    instancia::escribir(&p.state.db, claves::SOLO_LECTURA, &json!(false))
        .await
        .unwrap();
    assert!(gestor.preparar(escritura()).await.is_ok());
}

#[tokio::test]
async fn firmware_confirmado_no_bloquea() {
    let p = recien_instalado().await;
    p.sim
        .control
        .set_firmware("ES_g99.00_RTF_TEF001_V99.00_V001");
    instancia::escribir(
        &p.state.db,
        claves::FIRMWARE_CONFIRMADO,
        &json!("ES_g99.00_RTF_TEF001_V99.00_V001"),
    )
    .await
    .unwrap();
    let cookie = crear_admin(&p).await;
    let (status, body) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert_ne!(
        instancia::leer(&p.state.db, claves::SOLO_LECTURA).await,
        Some(json!(true))
    );
}

#[tokio::test]
async fn foto_de_partida_cifrada() {
    let p = recien_instalado().await;
    let cookie = crear_admin(&p).await;
    let (status, _) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200);
    let (status, body) = llamar(&p, "lectura_inicial", json!({ "token": "" }), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");

    let (tipo, blob): (String, Vec<u8>) = sqlx::query_as("SELECT tipo, contenido FROM fotos")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert_eq!(tipo, "partida");
    assert!(
        !blob.windows(SSID.len()).any(|w| w == SSID.as_bytes()),
        "la foto está en claro"
    );
    let (id, foto) = fotos::ultima(&p.state.db, &p.state.history_key, TipoFoto::Partida)
        .await
        .unwrap();
    assert!(id > 0);
    assert!(
        foto.get("cbListWifiAccPt")
            .is_some_and(|xml| xml.contains(&format!("<ssid>{SSID}</ssid>"))),
        "{:?}",
        foto.keys().collect::<Vec<_>>()
    );
    assert!(foto.contains_key("cbGetSummary"));
    // La foto manual es otra serie.
    assert!(
        fotos::ultima(&p.state.db, &p.state.history_key, TipoFoto::Manual)
            .await
            .is_none()
    );
    let manual = BTreeMap::from([("a".to_owned(), "<a>1</a>".to_owned())]);
    let id2 = fotos::guardar(&p.state.db, &p.state.history_key, TipoFoto::Manual, &manual)
        .await
        .unwrap();
    assert_eq!(
        fotos::ultima(&p.state.db, &p.state.history_key, TipoFoto::Manual).await,
        Some((id2, manual))
    );
}

#[tokio::test]
async fn pagina_del_asistente_traducida_y_sin_cache() {
    let p = recien_instalado().await;
    let resp = send(
        &p.app,
        Request::get("/instalar")
            .header("accept-language", "en")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(resp.status(), 200);
    let html = body_text(resp).await;
    assert!(html.contains("idioma-selector"), "selector de idioma");
    assert!(html.contains("Set up the panel"), "{html}");
    // Las respuestas del asistente no se guardan en caché.
    let req = post_json("/api/probar_router", &router(&p).to_string(), "127.0.0.1");
    let resp = send(&p.app, req).await;
    assert_eq!(resp.headers()["cache-control"], "no-store");
}

#[tokio::test]
async fn asistente_no_cambia_credenciales_con_cambio_activo() {
    let p = recien_instalado().await;
    let cookie = crear_admin(&p).await;
    let gestor = ChangeManager::instalar(&p.state, Tiempos::default());
    // Mientras el gestor esté ocupado (como con un cambio aplicándose), no.
    let ocupado = gestor.excluir_cambios().unwrap();
    let (status, body) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    assert!(!p.state.hgu.configurado());
    assert!(
        instancia::leer_credenciales(&p.state.db, &p.state.history_key)
            .await
            .unwrap()
            .is_none()
    );
    drop(ocupado);
    let (status, body) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
}

#[tokio::test]
async fn lectura_inicial_fija_solo_lectura_con_el_firmware_que_lee() {
    let p = recien_instalado().await;
    p.sim
        .control
        .set_firmware("ES_g99.00_RTF_TEF001_V99.00_V001");
    let cookie = crear_admin(&p).await;
    // Router configurado sin pasar por el paso del router (importado del
    // entorno, o una recarga del asistente tras guardarlo).
    instancia::guardar_credenciales(
        &p.state.db,
        &p.state.history_key,
        &instancia::CredencialesRouter {
            url: p.sim.base_url.clone(),
            usuario: USER.into(),
            contrasena: PASS.into(),
        },
    )
    .await
    .unwrap();
    instancia::arrancar_router(&p.state).await.unwrap();
    assert!(!instancia::solo_lectura(&p.state.db).await);
    let (status, body) = llamar(&p, "lectura_inicial", json!({ "token": "" }), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert!(instancia::solo_lectura(&p.state.db).await);
}

#[tokio::test]
async fn lectura_inicial_con_firmware_probado_no_bloquea() {
    let p = recien_instalado().await;
    let cookie = crear_admin(&p).await;
    let (status, _) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200);
    // Quedó en solo lectura de antes (otro router): la lectura lo corrige.
    instancia::escribir(&p.state.db, claves::SOLO_LECTURA, &json!(true))
        .await
        .unwrap();
    let (status, body) = llamar(&p, "lectura_inicial", json!({ "token": "" }), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert!(!instancia::solo_lectura(&p.state.db).await);
}

/// Petición desde el proxy de confianza (127.0.0.1) por cuenta de `cliente`.
async fn por_el_proxy(
    p: &Panel,
    uri: &str,
    cuerpo: &str,
    json: bool,
    cliente: &str,
) -> (u16, String) {
    let mut req = if json {
        post_json(uri, cuerpo, "127.0.0.1")
    } else {
        crate::comun::post_form(uri, cuerpo, "127.0.0.1")
    };
    req.headers_mut()
        .insert("x-forwarded-for", cliente.parse().unwrap());
    let resp = send(&p.app, req).await;
    let status = resp.status().as_u16();
    (status, body_text(resp).await)
}

async fn fallos_de_ip(p: &Panel, ip: &str) -> Option<i64> {
    sqlx::query_scalar("SELECT failures FROM login_attempts WHERE kind = 'ip' AND key = ?")
        .bind(ip)
        .fetch_optional(&p.state.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn tras_el_proxy_cuenta_la_ip_del_cliente() {
    const PUBLICA: &str = "203.0.113.9";
    let p = recien_instalado().await;
    let token = std::fs::read_to_string(fichero_token(&p)).unwrap();
    let admin = json!({ "token": token.trim(), "usuario": PANEL_USER, "contrasena": PANEL_PASS })
        .to_string();
    // El proxy es local, pero el cliente que trae no: el asistente lo rechaza.
    let (status, body) = por_el_proxy(&p, "/api/crear_admin", &admin, true, PUBLICA).await;
    assert_eq!(status, 403, "{body}");
    assert_eq!(usuarios(&p).await, 0);
    let (status, body) = por_el_proxy(&p, "/api/crear_admin", &admin, true, "192.168.1.90").await;
    assert_eq!(status, 200, "{body}");

    // El login y su freno cuentan la IP del cliente, no la del proxy.
    let malo = crate::comun::login_body(PANEL_USER, "no es la buena");
    let (status, _) = por_el_proxy(&p, "/api/login", &malo, false, PUBLICA).await;
    assert_eq!(status, 401);
    assert_eq!(fallos_de_ip(&p, PUBLICA).await, Some(1));
    assert_eq!(fallos_de_ip(&p, "127.0.0.1").await, None);
    let bueno = crate::comun::login_body(PANEL_USER, PANEL_PASS);
    let (status, body) = por_el_proxy(&p, "/api/login", &bueno, false, PUBLICA).await;
    assert_eq!(status, 429, "en espera: {body}");
}

#[tokio::test]
async fn lectura_inicial_sin_resumen_no_decide_el_modo() {
    let p = recien_instalado().await;
    let cookie = crear_admin(&p).await;
    let (status, _) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200);
    assert!(!instancia::solo_lectura(&p.state.db).await);
    // El resumen no se puede leer (ni al reintentar): no se adivina.
    p.sim.control.fail_reads("cbGetSummary", true);
    let (status, body) = llamar(&p, "lectura_inicial", json!({ "token": "" }), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert!(!instancia::solo_lectura(&p.state.db).await);
}

/// Construir el router de otro panel del mismo proceso no deja colgada la
/// página de uno que ya sirve. Leptos genera la lista de rutas renderizando la
/// aplicación con la carga de recursos suspendida mediante un indicador
/// global; si se repetía en cada `build_router`, un recurso que empezaba a
/// cargar en ese momento (la fase del asistente) se quedaba esperando para
/// siempre y la respuesta no terminaba.
#[tokio::test]
async fn construir_otro_router_no_cuelga_la_pagina() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::Duration;
    let p = recien_instalado().await;
    let parar = Arc::new(AtomicBool::new(false));
    let constructor = std::thread::spawn({
        let (state, parar) = (p.state.clone(), parar.clone());
        move || {
            while !parar.load(Ordering::Relaxed) {
                drop(panel::build_router(state.clone()));
            }
        }
    });
    for i in 0..200 {
        let pagina = async {
            let resp = get(&p.app, "/instalar").await;
            assert_eq!(resp.status(), 200);
            body_text(resp).await
        };
        let r = tokio::time::timeout(Duration::from_secs(5), pagina).await;
        if r.is_err() {
            parar.store(true, Ordering::Relaxed);
            panic!("la página {i} no terminó");
        }
    }
    parar.store(true, Ordering::Relaxed);
    constructor.join().unwrap();
}

/// La lectura inicial devuelve la red detectada del router.
#[tokio::test]
async fn lectura_inicial_incluye_red() {
    let p = recien_instalado().await;
    let cookie = crear_admin(&p).await;
    let (status, body) = llamar(&p, "guardar_router", router(&p), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    let (status, body) = llamar(&p, "lectura_inicial", json!({ "token": "" }), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    let red = &json_de(&body)["red"];
    assert_eq!(red["wan"], "ip2", "{body}");
    assert_eq!(red["tipo"], "Pppoe", "{body}");
    assert_eq!(red["voip"], true, "{body}");
    assert_eq!(red["iptv"], true, "{body}");
    assert!(red["lan"].is_string(), "{body}");
}

/// La puerta de enlace por defecto del equipo, de `/proc/net/route` (Linux)
/// y de `route -n get default` (FreeBSD).
#[test]
fn gw_de_proc_net_route_y_route_get() {
    let linux = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n\
                 eth0\t0002000A\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0\n\
                 eth0\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0\n";
    assert_eq!(
        instalar::gw_de_proc_net_route(linux),
        Some("192.168.1.1".parse().unwrap())
    );
    let docker = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n\
                  eth0\t00000000\t010010AC\t0003\t0\t0\t0\t00000000\t0\t0\t0\n";
    assert_eq!(
        instalar::gw_de_proc_net_route(docker),
        Some("172.16.0.1".parse().unwrap())
    );
    // Sin ruta por defecto, o sin pasarela (sin el bit RTF_GATEWAY).
    assert_eq!(instalar::gw_de_proc_net_route(""), None);
    let sin_gw = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n\
                  eth0\t00000000\t00000000\t0001\t0\t0\t0\t00000000\t0\t0\t0\n";
    assert_eq!(instalar::gw_de_proc_net_route(sin_gw), None);

    let freebsd = "   route to: default\ndestination: default\n       mask: default\n\
                   \x20   gateway: 192.168.1.1\n        fib: 0\n  interface: em0\n\
                   \x20     flags: <UP,GATEWAY,DONE,STATIC>\n";
    assert_eq!(
        instalar::gw_de_route_get(freebsd),
        Some("192.168.1.1".parse().unwrap())
    );
    let jaula = "   route to: default\n    gateway: 172.16.0.1\n";
    assert_eq!(
        instalar::gw_de_route_get(jaula),
        Some("172.16.0.1".parse().unwrap())
    );
    assert_eq!(
        instalar::gw_de_route_get("route: route has not been found\n"),
        None
    );
}

/// Solo se sugiere una pasarela de red privada que no sea 172.16/12 (la de
/// Docker y otros contenedores; en las pruebas, `172.16.0.1`).
#[test]
fn url_sugerida_ignora_redes_docker() {
    for si in ["192.168.1.1", "10.255.0.1"] {
        assert!(instalar::sugerible(si.parse().unwrap()), "{si}");
    }
    for no in ["172.16.0.1", "203.0.113.1", "127.0.0.1", "0.0.0.0"] {
        assert!(!instalar::sugerible(no.parse().unwrap()), "{no}");
    }
    // El final de 172.16/12 (172.16 a 172.31), fuera.
    assert!(!instalar::sugerible(Ipv4Addr::new(172, 31, 255, 1)));
    assert!(!instalar::sugerible(Ipv4Addr::new(172, 20, 1, 1)));
    // Justo fuera de 172.16/12 ya no es red privada: tampoco (solo se sugiere
    // una puerta de enlace privada).
    assert!(!instalar::sugerible(Ipv4Addr::new(172, 15, 255, 254)));
    assert!(!instalar::sugerible(Ipv4Addr::new(172, 32, 0, 1)));
    // Ni la red compartida de los operadores, ni la de enlace local, ni el
    // propio equipo, ni una pública.
    for no in [
        Ipv4Addr::new(100, 64, 0, 1),
        Ipv4Addr::new(169, 254, 1, 1),
        Ipv4Addr::LOCALHOST,
        Ipv4Addr::new(8, 8, 8, 8),
        Ipv4Addr::new(198, 51, 100, 1),
    ] {
        assert!(!instalar::sugerible(no), "{no}");
    }
}

/// La sugerencia de dirección del router es parte del asistente: con token
/// (o la sesión del administrador mientras dura la instalación).
#[tokio::test]
async fn url_sugerida_exige_token() {
    let p = recien_instalado().await;
    let (status, _) = llamar(&p, "url_sugerida", json!({ "token": "malo" }), None).await;
    assert_eq!(status, 403);
    let cookie = crear_admin(&p).await;
    let (status, body) = llamar(&p, "url_sugerida", json!({ "token": "" }), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    let v = json_de(&body);
    assert!(
        v.is_null() || v.as_str().is_some_and(|u| u.starts_with("http://")),
        "{v}"
    );
}
