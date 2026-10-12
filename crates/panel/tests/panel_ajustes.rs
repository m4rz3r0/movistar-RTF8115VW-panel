//! Pruebas de «Ajustes del panel» (`/panel`): credenciales del router,
//! parámetros de la instancia, usuarios y ventana de reautenticación.

mod common;

use common::{
    PANEL_PASS, PANEL_USER, PASS, Panel, USER, body_text, get_con_cookie, llamar, login_body, ok,
    panel, panel_con_gestor, post_form, post_json, send, sesion, set_cookie,
};
use panel::instancia::{self, claves};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const NUEVA: &str = "nueva-clave-del-router";

/// Abre la ventana de reautenticación de la sesión `cookie`.
async fn reautenticar(p: &Panel, cookie: &str) {
    ok(
        p,
        "reautenticar",
        json!({ "contrasena": PANEL_PASS }),
        cookie,
    )
    .await;
}

fn router(p: &Panel, contrasena: &str) -> Value {
    json!({ "url": p.sim.base_url, "usuario": USER, "contrasena": contrasena })
}

async fn credenciales(p: &Panel) -> instancia::CredencialesRouter {
    instancia::leer_credenciales(&p.state.db, &p.state.history_key)
        .await
        .unwrap()
        .unwrap()
}

fn campos() -> Value {
    json!({
        "idioma": { "Fijo": "Es" },
        "zona_horaria": "UTC+01:00",
        "dns_preferidos": [],
        "dispositivos_confianza": [],
        "mantenimiento": null,
        "servidor_propio": null,
    })
}

#[tokio::test]
async fn credenciales_no_se_cambian_con_cambio_activo() {
    let (p, cookie) = panel_con_gestor().await;
    reautenticar(&p, &cookie).await;
    let antes = credenciales(&p).await;
    // Un cambio que se queda a medias: el router deja de responder tras la
    // escritura.
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetUpnpCfg", "instancia": null,
            "valores": [["adm_state", "1"]], "borrar": false } }),
        &cookie,
    )
    .await;
    p.sim.control.hang_after_next_write();
    let cambio = {
        let app = p.app.clone();
        let mut req = post_json(
            "/api/confirmar_cambio",
            &json!({ "token": prep["token"], "palabra": null }).to_string(),
            "127.0.0.1",
        );
        req.headers_mut().insert("cookie", cookie.parse().unwrap());
        tokio::spawn(async move { send(&app, req).await.status() })
    };
    let gestor = p.state.cambios().unwrap();
    let start = Instant::now();
    while gestor.activo().is_none() || p.sim.control.writes().is_empty() {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "sin cambio activo"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    p.sim.control.set_password(NUEVA);
    let (status, body) = llamar(&p, "panel_guardar_router", router(&p, NUEVA), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("cambio en curso"), "{body}");
    assert_eq!(credenciales(&p).await, antes, "la base de datos no cambia");

    // Al terminar el cambio, ya se puede.
    p.sim.control.set_password(PASS);
    p.sim.control.set_unresponsive(false);
    let _ = cambio.await;
    let start = Instant::now();
    while gestor.activo().is_some() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "el cambio no acaba"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn guardar_router_exige_reautenticacion() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    p.sim.control.set_password(NUEVA);
    let (status, body) = llamar(&p, "panel_guardar_router", router(&p, NUEVA), Some(&cookie)).await;
    assert_eq!(status, 401, "{body}");
    assert!(body.contains(panel::auth::ERROR_REAUTENTICACION), "{body}");
    assert_eq!(credenciales(&p).await.contrasena, PASS);

    // Contraseña del panel mala: 403 y no abre la ventana.
    let (status, _) = llamar(
        &p,
        "reautenticar",
        json!({ "contrasena": "no es la buena" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 403);
    // Sin sesión, ni reautenticar.
    let (status, _) = llamar(
        &p,
        "reautenticar",
        json!({ "contrasena": PANEL_PASS }),
        None,
    )
    .await;
    assert_eq!(status, 401);

    // Espera del freno tras el fallo de antes.
    sqlx::query("UPDATE login_attempts SET last_failure_ms = last_failure_ms - 60000")
        .execute(&p.state.db)
        .await
        .unwrap();
    reautenticar(&p, &cookie).await;
    let r = ok(&p, "panel_guardar_router", router(&p, NUEVA), &cookie).await;
    assert_eq!(r["modelo"], "RTF8115VW", "{r}");
    assert_eq!(credenciales(&p).await.contrasena, NUEVA);
    // El cliente del panel entra ya con la nueva: la vieja no vale.
    p.sim.control.invalidate_sessions();
    hgu::read::summary(&p.state.hgu).await.unwrap();
}

#[tokio::test]
async fn reautenticacion_dura_5_minutos() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    reautenticar(&p, &cookie).await;
    let hasta: i64 = sqlx::query_scalar("SELECT reautenticado_hasta FROM sessions")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    let ahora = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    assert!((ahora + 295..=ahora + 301).contains(&hasta), "{hasta}");
    ok(&p, "confirmar_firmware", json!({}), &cookie).await;
    // Pasada la ventana, otra vez 401.
    sqlx::query("UPDATE sessions SET reautenticado_hasta = reautenticado_hasta - 301")
        .execute(&p.state.db)
        .await
        .unwrap();
    let (status, body) = llamar(&p, "confirmar_firmware", json!({}), Some(&cookie)).await;
    assert_eq!(status, 401, "{body}");
    assert!(body.contains(panel::auth::ERROR_REAUTENTICACION), "{body}");
}

#[tokio::test]
async fn guardar_router_prueba_antes_de_guardar() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    reautenticar(&p, &cookie).await;
    let (status, body) = llamar(
        &p,
        "panel_guardar_router",
        router(&p, "contrasena-mala"),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 502, "{body}");
    assert_eq!(credenciales(&p).await.contrasena, PASS);
    // La prueba sola no guarda nada.
    let r = ok(&p, "panel_probar_router", router(&p, PASS), &cookie).await;
    assert_eq!(r["probado"], true, "{r}");
    let (status, _) = llamar(&p, "panel_probar_router", router(&p, PASS), None).await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn confirmar_firmware_quita_solo_lectura() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    instancia::escribir(&p.state.db, claves::SOLO_LECTURA, &json!(true))
        .await
        .unwrap();
    let (status, _) = llamar(&p, "confirmar_firmware", json!({}), Some(&cookie)).await;
    assert_eq!(status, 401);
    assert!(instancia::solo_lectura(&p.state.db).await);
    reautenticar(&p, &cookie).await;
    ok(&p, "confirmar_firmware", json!({}), &cookie).await;
    assert!(!instancia::solo_lectura(&p.state.db).await);
    assert_eq!(
        instancia::leer(&p.state.db, claves::FIRMWARE_CONFIRMADO).await,
        Some(json!(instancia::FIRMWARES_PROBADOS[0]))
    );
    assert_eq!(
        ok(&p, "leer_panel", json!({}), &cookie).await["solo_lectura"],
        false
    );
}

async fn usuarios(p: &Panel, cookie: &str) -> Vec<String> {
    let v = ok(p, "leer_panel", json!({}), cookie).await;
    serde_json::from_value(v["usuarios"].clone()).unwrap()
}

#[tokio::test]
async fn no_se_borra_el_ultimo_usuario() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let (status, _) = llamar(
        &p,
        "borrar_usuario",
        json!({ "usuario": PANEL_USER }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 401, "sin reautenticar");
    reautenticar(&p, &cookie).await;
    // Ni el último ni uno mismo.
    let (status, body) = llamar(
        &p,
        "borrar_usuario",
        json!({ "usuario": PANEL_USER }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    ok(
        &p,
        "crear_usuario",
        json!({ "usuario": "otro", "contrasena": "otra contraseña larga" }),
        &cookie,
    )
    .await;
    assert_eq!(usuarios(&p, &cookie).await, [PANEL_USER, "otro"]);
    let (status, body) = llamar(
        &p,
        "borrar_usuario",
        json!({ "usuario": PANEL_USER }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "uno mismo: {body}");
    let (status, _) = llamar(
        &p,
        "crear_usuario",
        json!({ "usuario": "otro", "contrasena": "otra contraseña larga" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 409, "ya existe");
    let (status, _) = llamar(
        &p,
        "crear_usuario",
        json!({ "usuario": "corta", "contrasena": "corta" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400);
    let (status, _) = llamar(
        &p,
        "borrar_usuario",
        json!({ "usuario": "nadie" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 404);

    // El otro usuario, con sesión, se borra y su sesión deja de valer.
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body("otro", "otra contraseña larga"),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 200);
    let cookie_otro = set_cookie(&resp).split(';').next().unwrap().to_owned();
    ok(&p, "borrar_usuario", json!({ "usuario": "otro" }), &cookie).await;
    assert_eq!(usuarios(&p, &cookie).await, [PANEL_USER]);
    let (status, _) = llamar(&p, "leer_panel", json!({}), Some(&cookie_otro)).await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn cambiar_contrasena_propia_mantiene_la_sesion() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let nueva = json!({ "usuario": PANEL_USER, "contrasena": "contraseña nueva y larga" });
    let (status, _) = llamar(&p, "cambiar_contrasena", nueva.clone(), Some(&cookie)).await;
    assert_eq!(status, 401);
    reautenticar(&p, &cookie).await;
    let mut req = post_json("/api/cambiar_contrasena", &nueva.to_string(), "127.0.0.1");
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 200);
    // Las sesiones viejas se cierran; la respuesta trae una nueva.
    let nueva_cookie = set_cookie(&resp).split(';').next().unwrap().to_owned();
    assert_ne!(nueva_cookie, cookie);
    let (status, _) = llamar(&p, "leer_panel", json!({}), Some(&cookie)).await;
    assert_eq!(status, 401);
    ok(&p, "leer_panel", json!({}), &nueva_cookie).await;
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body(PANEL_USER, "contraseña nueva y larga"),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 200, "entra con la nueva");
}

#[tokio::test]
async fn instancia_valida_dns_y_macs() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let mut c = campos();
    c["dns_preferidos"] = json!(["9.9.9.9", " 2620:fe::fe ", "194.242.2.2"]);
    c["dispositivos_confianza"] =
        json!(["AA-BB-CC-00-11-22", "aabb.cc00.1133", "AA:BB:CC:00:11:44"]);
    c["mantenimiento"] = json!("03:00-05:30");
    c["servidor_propio"] = json!("servidor.home.arpa");
    c["idioma"] = json!({ "Fijo": "En" });
    ok(&p, "guardar_instancia", json!({ "campos": c }), &cookie).await;
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(
        v["dns_preferidos"],
        json!(["9.9.9.9", "2620:fe::fe", "194.242.2.2"])
    );
    assert_eq!(
        v["dispositivos_confianza"],
        json!([
            "aa:bb:cc:00:11:22",
            "aa:bb:cc:00:11:33",
            "aa:bb:cc:00:11:44"
        ])
    );
    assert_eq!(v["mantenimiento"], "03:00-05:30");
    assert_eq!(v["servidor_propio"], "servidor.home.arpa");
    assert_eq!(v["zona_horaria"], "UTC+01:00");
    assert_eq!(v["idioma"], json!({ "Fijo": "En" }));

    let malos: Vec<(&str, Value)> = vec![
        ("dns_preferidos", json!(["9.9.9.9x"])),
        ("dns_preferidos", json!(["dns.quad9.net"])),
        (
            "dns_preferidos",
            json!([
                "1.1.1.1",
                "1.0.0.1",
                "9.9.9.9",
                "149.112.112.112",
                "194.242.2.2"
            ]),
        ),
        ("dispositivos_confianza", json!(["aa:bb:cc:00:11"])),
        ("dispositivos_confianza", json!(["aa:bb:cc:00:11:zz"])),
        ("mantenimiento", json!("3-5")),
        ("mantenimiento", json!("25:00-05:00")),
        ("mantenimiento", json!("03:00-03:60")),
        ("zona_horaria", json!("America/New_York")),
        ("zona_horaria", json!("UTC+15:00")),
        ("zona_horaria", json!("UTC+1")),
        ("servidor_propio", json!("http://servidor.home.arpa/")),
    ];
    for (campo, valor) in malos {
        let mut c = campos();
        c[campo] = valor.clone();
        let (status, body) = llamar(
            &p,
            "guardar_instancia",
            json!({ "campos": c }),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, 400, "{campo} = {valor}: {body}");
    }
    // Nada de lo malo se ha guardado.
    let v2 = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v, v2);
    assert_eq!(
        instancia::leer(&p.state.db, claves::MANTENIMIENTO).await,
        Some(json!("03:00-05:30"))
    );
    // Vacío: sin mantenimiento ni servidor propio.
    let mut c = campos();
    c["mantenimiento"] = json!("");
    c["servidor_propio"] = json!("  ");
    ok(&p, "guardar_instancia", json!({ "campos": c }), &cookie).await;
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["mantenimiento"], Value::Null);
    assert_eq!(v["servidor_propio"], Value::Null);
}

#[tokio::test]
async fn leer_panel_no_devuelve_contrasena() {
    let p = panel().await;
    let (status, _) = llamar(&p, "leer_panel", json!({}), None).await;
    assert_eq!(status, 401);
    let cookie = sesion(&p).await;
    let (status, body) = llamar(&p, "leer_panel", json!({}), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert!(!body.contains(PASS), "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["router"]["url"], p.sim.base_url);
    assert_eq!(v["router"]["usuario"], USER);
    assert_eq!(v["usuarios"], json!([PANEL_USER]));
    assert_eq!(v["solo_lectura"], false);
    assert_eq!(v["dns_preferidos"], json!([]));
    // Sin zona guardada, la peninsular.
    assert_eq!(v["zona_horaria"], "Europe/Madrid");
    // La página tampoco la lleva.
    let resp = get_con_cookie(&p.app, "/panel", &cookie).await;
    assert_eq!(resp.status(), 200);
    let html = body_text(resp).await;
    assert!(!html.contains(PASS));
    assert!(html.contains("Ajustes del panel"), "{html}");
}

// ---- Ronda de correcciones 1

/// Sondeo rápido sobre un cliente del router rápido (la página se rehace
/// con él).
fn sondeo_rapido(p: &mut Panel) {
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, USER, PASS);
    cfg.timeout = Duration::from_millis(300);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    p.app = panel::build_router(p.state.clone());
    panel::poller::Poller::spawn_with(
        p.state.clone(),
        panel::poller::PollerConfig {
            fast: Duration::from_millis(50),
            slow: Duration::from_millis(200),
            backoff_max: Duration::from_millis(200),
            ..panel::poller::PollerConfig::default()
        },
    );
}

async fn esperar_solo_lectura(p: &Panel, valor: bool) {
    let start = Instant::now();
    while instancia::solo_lectura(&p.state.db).await != valor {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "solo lectura no pasa a {valor}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

const SIN_PROBAR_A: &str = "ES_g99.00_RTF_TEF001_V99.00_V001";
const SIN_PROBAR_B: &str = "ES_g99.01_RTF_TEF001_V99.01_V001";

#[tokio::test]
async fn firmware_confirmado_solo_vale_para_esa_version() {
    let mut p = panel().await;
    p.sim.control.set_firmware(SIN_PROBAR_A);
    sondeo_rapido(&mut p);
    let cookie = sesion(&p).await;
    // El sondeo aplica la regla al leer el firmware.
    esperar_solo_lectura(&p, true).await;
    reautenticar(&p, &cookie).await;
    ok(&p, "confirmar_firmware", json!({}), &cookie).await;
    assert!(!instancia::solo_lectura(&p.state.db).await);
    assert_eq!(
        instancia::leer(&p.state.db, claves::FIRMWARE_CONFIRMADO).await,
        Some(json!(SIN_PROBAR_A))
    );
    // Una actualización a otro firmware sin probar vuelve a solo lectura.
    p.sim.control.set_firmware(SIN_PROBAR_B);
    esperar_solo_lectura(&p, true).await;
    // Y uno probado la quita.
    p.sim
        .control
        .set_firmware(panel::instancia::FIRMWARES_PROBADOS[0]);
    esperar_solo_lectura(&p, false).await;
}

/// Falla algún login del usuario del panel desde otra IP: el freno por
/// usuario queda activo.
async fn frenar_usuario(p: &Panel) {
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body(PANEL_USER, "no es la buena"),
            "192.168.1.60",
        ),
    )
    .await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn cambiar_contrasena_propia_con_usuario_frenado() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    reautenticar(&p, &cookie).await;
    frenar_usuario(&p).await;
    let nueva = json!({ "usuario": PANEL_USER, "contrasena": "contraseña nueva y larga" });
    let mut req = post_json("/api/cambiar_contrasena", &nueva.to_string(), "127.0.0.1");
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 200);
    let nueva_cookie = set_cookie(&resp).split(';').next().unwrap().to_owned();
    ok(&p, "leer_panel", json!({}), &nueva_cookie).await;
}

#[tokio::test]
async fn contrasena_demasiado_larga() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    reautenticar(&p, &cookie).await;
    let larga = "x".repeat(1025);
    let (status, body) = llamar(
        &p,
        "crear_usuario",
        json!({ "usuario": "otro", "contrasena": larga }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert_eq!(usuarios(&p, &cookie).await, [PANEL_USER]);
    let (status, body) = llamar(
        &p,
        "cambiar_contrasena",
        json!({ "usuario": PANEL_USER, "contrasena": larga }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    // Sigue entrando con la de antes, y con 1024 bytes justos se puede.
    ok(&p, "leer_panel", json!({}), &cookie).await;
    ok(
        &p,
        "crear_usuario",
        json!({ "usuario": "otro", "contrasena": "x".repeat(1024) }),
        &cookie,
    )
    .await;
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body("otro", &"x".repeat(1024)),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn cada_seccion_guarda_solo_lo_suyo() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": campos() }),
        &cookie,
    )
    .await;
    // «Privacidad e instancia» no manda la zona ni el idioma.
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "dns_preferidos": ["9.9.9.9"], "dispositivos_confianza": [],
            "mantenimiento": "", "servidor_propio": "" } }),
        &cookie,
    )
    .await;
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["zona_horaria"], "UTC+01:00");
    assert_eq!(v["dns_preferidos"], json!(["9.9.9.9"]));
    // «Panel» no manda los DNS.
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "idioma": { "Fijo": "En" }, "zona_horaria": "UTC-03:00" } }),
        &cookie,
    )
    .await;
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["zona_horaria"], "UTC-03:00");
    assert_eq!(v["idioma"], json!({ "Fijo": "En" }));
    assert_eq!(v["dns_preferidos"], json!(["9.9.9.9"]));
}

/// La conexión con el router vive en «Router → Conexión con el router»
/// (`/router/conexion`), no en «Ajustes del panel»; `/router` lleva allí.
#[tokio::test]
async fn conexion_con_el_router_en_su_seccion() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let resp = get_con_cookie(&p.app, "/router/conexion", &cookie).await;
    assert_eq!(resp.status(), 200);
    let html = body_text(resp).await;
    assert!(html.contains("Conexión con el router"), "{html}");
    assert!(html.contains(r#"id="panel-router-url""#), "{html}");
    // La red detectada, en la misma página.
    assert!(html.contains("Red del router"), "{html}");
    assert!(html.contains("Internet: ip2 (PPPoE)"), "{html}");
    assert!(html.contains(r#"id="router-red-conexion""#), "{html}");
    assert!(
        html.contains(r#"href="/router/conexion""#),
        "enlace de la navegación: {html}"
    );
    assert!(!html.contains(PASS));

    let html = body_text(get_con_cookie(&p.app, "/panel", &cookie).await).await;
    assert!(html.contains("Ajustes del panel"), "{html}");
    assert!(!html.contains(r#"id="panel-router-url""#), "{html}");
    assert!(html.contains(r#"id="panel-idioma""#), "{html}");
    // Y lleva a la conexión con el router (además de la navegación).
    assert_eq!(
        html.matches(r#"href="/router/conexion""#).count(),
        2,
        "{html}"
    );

    // Como la pide el navegador.
    let resp = send(
        &p.app,
        axum::http::Request::get("/router")
            .header("cookie", &cookie)
            .header("accept", "text/html")
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await;
    assert!(resp.status().is_redirection(), "{}", resp.status());
    assert_eq!(resp.headers()["location"], "/router/conexion");
}

/// La red detectada y las conexiones del router llegan a la página; elegir
/// la conexión a internet a mano exige reautenticación y un nombre que el
/// router tenga. Volver a detectar, también reautenticación.
#[tokio::test]
async fn guardar_wan_exige_reautenticacion_y_nombre_valido() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["red"]["wan"], "ip2", "{v}");
    assert_eq!(v["wan_manual"], Value::Null, "{v}");
    // Sin lectura del sondeo, solo la detectada.
    assert_eq!(v["conexiones"], json!(["ip2"]), "{v}");

    let (status, body) = llamar(&p, "guardar_wan", json!({ "manual": "ip4" }), Some(&cookie)).await;
    assert_eq!(status, 401, "{body}");
    assert!(body.contains(panel::auth::ERROR_REAUTENTICACION), "{body}");
    let (status, body) = llamar(&p, "redetectar", json!({}), Some(&cookie)).await;
    assert_eq!(status, 401, "{body}");
    assert_eq!(p.state.wan().as_deref(), Some("ip2"));

    reautenticar(&p, &cookie).await;
    let (status, body) = llamar(&p, "guardar_wan", json!({ "manual": "ip9" }), Some(&cookie)).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("ip9"), "{body}");
    assert_eq!(p.state.wan().as_deref(), Some("ip2"));
    assert_eq!(instancia::leer(&p.state.db, claves::WAN_MANUAL).await, None);

    ok(&p, "guardar_wan", json!({ "manual": "ip4" }), &cookie).await;
    assert_eq!(p.state.wan().as_deref(), Some("ip4"));
    assert_eq!(
        instancia::leer(&p.state.db, claves::WAN_MANUAL).await,
        Some(json!("ip4"))
    );
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["wan_manual"], "ip4", "{v}");

    // Sin la manual, vuelve la detectada.
    ok(&p, "guardar_wan", json!({ "manual": null }), &cookie).await;
    assert_eq!(p.state.wan().as_deref(), Some("ip2"));
    assert!(
        instancia::leer(&p.state.db, claves::WAN_MANUAL)
            .await
            .is_none_or(|v| v.is_null())
    );

    // Volver a detectar devuelve la red y la anota.
    p.state.fijar_red(None, None);
    let red = ok(&p, "redetectar", json!({}), &cookie).await;
    assert_eq!(red["wan"], "ip2", "{red}");
    assert_eq!(p.state.wan().as_deref(), Some("ip2"));
}

/// Con un cambio aplicándose o vigilándose, ni se elige la conexión a
/// internet ni se vuelve a detectar: la red de seguridad del cambio no
/// cambia de conexión a mitad.
#[tokio::test]
async fn guardar_wan_rechazada_con_cambio_activo() {
    let (p, cookie) = panel_con_gestor().await;
    reautenticar(&p, &cookie).await;
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetUpnpCfg", "instancia": null,
            "valores": [["adm_state", "1"]], "borrar": false } }),
        &cookie,
    )
    .await;
    p.sim.control.hang_after_next_write();
    let cambio = {
        let app = p.app.clone();
        let mut req = post_json(
            "/api/confirmar_cambio",
            &json!({ "token": prep["token"], "palabra": null }).to_string(),
            "127.0.0.1",
        );
        req.headers_mut().insert("cookie", cookie.parse().unwrap());
        tokio::spawn(async move { send(&app, req).await.status() })
    };
    let gestor = p.state.cambios().unwrap();
    let start = Instant::now();
    while gestor.activo().is_none() || p.sim.control.writes().is_empty() {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "sin cambio activo"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let (status, body) = llamar(&p, "guardar_wan", json!({ "manual": "ip4" }), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    let (status, body) = llamar(&p, "redetectar", json!({}), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(p.state.wan().as_deref(), Some("ip2"));
    assert_eq!(instancia::leer(&p.state.db, claves::WAN_MANUAL).await, None);

    p.sim.control.set_unresponsive(false);
    let _ = cambio.await;
    let start = Instant::now();
    while gestor.activo().is_some() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "el cambio no acaba"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    ok(&p, "guardar_wan", json!({ "manual": "ip4" }), &cookie).await;
    assert_eq!(p.state.wan().as_deref(), Some("ip4"));
}

/// Sin ninguna lectura del sondeo, la página no pregunta al router (caído,
/// la retrasaría; con credenciales malas, sumaría intentos de entrada):
/// ofrece las conexiones que conoce el panel.
#[tokio::test]
async fn leer_panel_sin_sondeo_no_pregunta_al_router() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    assert!(p.state.poller.latest().is_none());
    p.state.fijar_red(p.state.red(), Some("ip4".into()));
    p.sim.control.set_unresponsive(true);
    let antes = Instant::now();
    let v = tokio::time::timeout(
        Duration::from_secs(2),
        ok(&p, "leer_panel", json!({}), &cookie),
    )
    .await
    .expect("leer_panel no espera al router");
    assert!(antes.elapsed() < Duration::from_secs(2));
    assert_eq!(v["conexiones"], json!(["ip2", "ip4"]), "{v}");
    assert_eq!(v["wan_manual"], "ip4", "{v}");

    // Con una lectura del sondeo, todas las del router.
    p.sim.control.set_unresponsive(false);
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, USER, PASS);
    cfg.timeout = Duration::from_millis(500);
    cfg.min_gap = Duration::from_millis(1);
    let mut state = p.state.clone();
    state.hgu = hgu::Client::new(cfg);
    let h = panel::poller::Poller::spawn_with(
        state,
        panel::poller::PollerConfig {
            fast: Duration::from_millis(50),
            slow: Duration::from_millis(200),
            backoff_max: Duration::from_millis(200),
            ..panel::poller::PollerConfig::default()
        },
    );
    let start = Instant::now();
    while !h.healthy() {
        assert!(start.elapsed() < Duration::from_secs(5), "sin primer dato");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["conexiones"], json!(["ip2", "ip3", "ip4"]), "{v}");
}

/// Mientras se retoman los cambios del arranque (uno quedó vigilándose), ni
/// se elige la conexión a internet, ni se vuelve a detectar, ni se anota una
/// detección que acabe entonces.
#[tokio::test]
async fn red_bloqueada_mientras_se_retoman_cambios() {
    use panel::changes::{ChangeManager, ChangeRequest, Tiempos};
    let mut p = panel().await;
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, USER, PASS);
    cfg.timeout = Duration::from_millis(500);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    p.app = panel::build_router(p.state.clone());
    let cookie = sesion(&p).await;
    reautenticar(&p, &cookie).await;
    let tiempos = |vigilancia: Duration| Tiempos {
        bajo: vigilancia,
        medio: vigilancia,
        alto: vigilancia,
        intervalo: Duration::from_millis(30),
        fallos_para_revertir: 3,
        pausa_max: Duration::from_secs(2),
    };
    // Un proceso anterior deja un cambio vigilándose.
    let a = ChangeManager::new(p.state.clone(), tiempos(Duration::from_secs(60)));
    a.reanudar_pendientes().await;
    let peticion = ChangeRequest {
        funcion: "cbSetUpnpCfg".into(),
        instancia: None,
        valores: [("adm_state".to_owned(), "1".to_owned())].into(),
        borrar: false,
    };
    let prep = a.preparar(peticion).await.unwrap();
    a.confirmar(&prep.token, None, "admin").await.unwrap();
    a.detener();

    // El gestor del proceso nuevo, con el hueco ocupado: retomar espera.
    let b = ChangeManager::instalar(&p.state, tiempos(Duration::from_millis(300)));
    let hueco = b.excluir_cambios().unwrap();
    let retomar = tokio::spawn({
        let b = b.clone();
        async move { b.reanudar_pendientes().await }
    });
    let start = Instant::now();
    while !b.ocupado() {
        assert!(start.elapsed() < Duration::from_secs(5), "no retoma");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(b.activo(), None, "aún sin cambio activo: solo retomando");

    // Estos 409 los da la reserva que mantiene `hueco`, no la comprobación de
    // `reanudando`: la ventana en que solo se está retomando dura hasta
    // `esperar_hueco` y no se puede sostener desde aquí. Lo que sí depende
    // de la fase de retomar es el descarte de la detección, más abajo.
    let (status, body) = llamar(&p, "guardar_wan", json!({ "manual": "ip4" }), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    let (status, body) = llamar(&p, "redetectar", json!({}), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    let mut anterior = p.state.red().unwrap();
    anterior.wan = Some("ip4".into());
    p.state.fijar_red(Some(anterior.clone()), None);
    assert!(instancia::detectar_y_guardar(&p.state).await.is_err());
    assert_eq!(p.state.red(), Some(anterior), "detección descartada");

    drop(hueco);
    retomar.await.unwrap();
    let start = Instant::now();
    while b.ocupado() {
        assert!(start.elapsed() < Duration::from_secs(10), "no termina");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    ok(&p, "guardar_wan", json!({ "manual": "ip4" }), &cookie).await;
}

/// Zonas horarias que se aceptan (Península, Canarias, UTC y desfases fijos
/// de hasta 14 h) y que pasan a usarse en el acto; las demás, 400 sin tocar
/// nada.
#[tokio::test]
async fn zona_valida_y_no_valida() {
    use panel::ui::charts::Zona;
    let p = panel().await;
    let cookie = sesion(&p).await;
    assert_eq!(p.state.zona(), Zona::Peninsular);
    for (zona, guardada, en_memoria) in [
        ("Atlantic/Canary", "Atlantic/Canary", Zona::Canarias),
        ("UTC", "UTC", Zona::Fija(0)),
        (" UTC+00:00 ", "UTC", Zona::Fija(0)),
        ("UTC-03:30", "UTC-03:30", Zona::Fija(-12_600)),
        ("UTC+14:00", "UTC+14:00", Zona::Fija(50_400)),
        ("Europe/Madrid", "Europe/Madrid", Zona::Peninsular),
        ("", "Europe/Madrid", Zona::Peninsular),
    ] {
        let c = ok(
            &p,
            "guardar_instancia",
            json!({ "campos": { "zona_horaria": zona } }),
            &cookie,
        )
        .await;
        assert_eq!(c["zona_horaria"], guardada, "{zona}");
        let v = ok(&p, "leer_panel", json!({}), &cookie).await;
        assert_eq!(v["zona_horaria"], guardada, "{zona}");
        assert_eq!(p.state.zona(), en_memoria, "{zona}");
    }
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "zona_horaria": "Atlantic/Canary" } }),
        &cookie,
    )
    .await;
    for zona in [
        "America/New_York",
        "europe/madrid",
        "UTC+15:00",
        "UTC-14:30",
        "UTC+1",
    ] {
        let (status, body) = llamar(
            &p,
            "guardar_instancia",
            json!({ "campos": { "zona_horaria": zona } }),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, 400, "{zona}: {body}");
        assert_eq!(p.state.zona(), Zona::Canarias, "{zona}");
    }
    // Al arrancar, la guardada.
    let dir = p.state.cfg.db_path.parent().unwrap();
    let otro = common::estado((*p.state.cfg).clone(), dir).await;
    assert_eq!(otro.zona(), Zona::Canarias);
}

/// La zona va en `<html data-zona>` (el navegador la lee de ahí al hidratar)
/// y las fechas de la página salen en ella.
#[tokio::test]
async fn html_lleva_data_zona() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let html = body_text(get_con_cookie(&p.app, "/panel", &cookie).await).await;
    assert!(html.contains(r#"data-zona="Europe/Madrid""#), "{html}");
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "zona_horaria": "Atlantic/Canary" } }),
        &cookie,
    )
    .await;
    for ruta in ["/panel", "/", "/historial", "/login"] {
        let html = body_text(get_con_cookie(&p.app, ruta, &cookie).await).await;
        assert!(
            html.contains(r#"data-zona="Atlantic/Canary""#),
            "{ruta}: {html}"
        );
    }
    // El selector, en «Canarias».
    let html = body_text(get_con_cookie(&p.app, "/panel", &cookie).await).await;
    assert!(
        html.contains(r#"<option value="Atlantic/Canary" selected"#),
        "{html}"
    );
    // Un desfase fijo distinto de UTC va en «Otra», con su propio selector.
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "zona_horaria": "UTC+05:45" } }),
        &cookie,
    )
    .await;
    let html = body_text(get_con_cookie(&p.app, "/panel", &cookie).await).await;
    assert!(html.contains(r#"data-zona="UTC+05:45""#), "{html}");
    assert!(html.contains(r#"<option value="otra" selected"#), "{html}");
    assert!(html.contains(r#"id="panel-zona-otra""#), "{html}");
    assert!(
        html.contains(r#"<option value="UTC+05:45" selected"#),
        "{html}"
    );
    // Uno válido que no está en la lista, también se ve elegido.
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "zona_horaria": "UTC-14:00" } }),
        &cookie,
    )
    .await;
    let html = body_text(get_con_cookie(&p.app, "/panel", &cookie).await).await;
    assert!(
        html.contains(r#"<option value="UTC-14:00" selected"#),
        "{html}"
    );
}

/// Migración al arrancar: un `UTC` guardado por la versión anterior (era lo
/// que mostraba el selector por defecto) se quita una sola vez; un `UTC`
/// elegido después se respeta y cualquier otra zona no se toca.
#[tokio::test]
async fn migracion_zona_utc_por_defecto() {
    use panel::ui::charts::Zona;
    let p = panel().await;
    let db = &p.state.db;
    let dir = p.state.cfg.db_path.parent().unwrap();
    let arrancar = || common::estado((*p.state.cfg).clone(), dir);
    // Base de datos de la versión anterior: sin la marca de la migración.
    let anterior = |zona: &'static str| async move {
        sqlx::query("DELETE FROM ajustes_instancia WHERE clave = ?")
            .bind(claves::MIGRACION_ZONA_UTC)
            .execute(db)
            .await
            .unwrap();
        instancia::escribir(db, claves::ZONA_HORARIA, &json!(zona))
            .await
            .unwrap();
    };

    // `UTC` sin la marca: se quita (la peninsular) y queda la marca.
    anterior("UTC").await;
    let s = arrancar().await;
    assert_eq!(s.zona(), Zona::Peninsular);
    assert_eq!(instancia::leer(db, claves::ZONA_HORARIA).await, None);
    assert!(
        instancia::leer(db, claves::MIGRACION_ZONA_UTC)
            .await
            .is_some()
    );

    // Un `UTC` guardado después sobrevive a otro arranque.
    instancia::escribir(db, claves::ZONA_HORARIA, &json!("UTC"))
        .await
        .unwrap();
    let s = arrancar().await;
    assert_eq!(s.zona(), Zona::Fija(0));
    assert_eq!(
        instancia::leer(db, claves::ZONA_HORARIA).await,
        Some(json!("UTC"))
    );

    // Otra zona sin la marca: no se toca.
    for zona in ["Atlantic/Canary", "UTC+01:00", "Europe/Madrid"] {
        anterior(zona).await;
        arrancar().await;
        assert_eq!(
            instancia::leer(db, claves::ZONA_HORARIA).await,
            Some(json!(zona)),
            "{zona}"
        );
        assert!(
            instancia::leer(db, claves::MIGRACION_ZONA_UTC)
                .await
                .is_some()
        );
    }
}
