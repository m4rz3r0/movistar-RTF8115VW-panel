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
        "idioma": "Es",
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
        Some(json!(true))
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
    c["servidor_propio"] = json!("servidor.lan");
    c["idioma"] = json!("En");
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
    assert_eq!(v["servidor_propio"], "servidor.lan");
    assert_eq!(v["zona_horaria"], "UTC+01:00");
    assert_eq!(v["idioma"], "En");

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
        ("zona_horaria", json!("Europe/Madrid")),
        ("zona_horaria", json!("UTC+15:00")),
        ("zona_horaria", json!("UTC+1")),
        ("servidor_propio", json!("http://servidor.lan/")),
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
    assert_eq!(v["zona_horaria"], "UTC");
    // La página tampoco la lleva.
    let resp = get_con_cookie(&p.app, "/panel", &cookie).await;
    assert_eq!(resp.status(), 200);
    let html = body_text(resp).await;
    assert!(!html.contains(PASS));
    assert!(html.contains("Ajustes del panel"), "{html}");
}
