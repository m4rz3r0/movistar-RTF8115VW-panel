//! Conexión con el router: credenciales, reautenticación y firmware sin probar.

use super::*;

#[tokio::test]
async fn credenciales_no_se_cambian_con_cambio_activo() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
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
    esperar_hasta(Duration::from_secs(5), "sin cambio activo", async || {
        gestor.activo().is_some() && !p.sim.control.writes().is_empty()
    })
    .await;
    p.sim.control.set_password(NUEVA);
    let (status, body) = llamar(&p, "panel_guardar_router", router(&p, NUEVA), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("cambio en curso"), "{body}");
    assert_eq!(credenciales(&p).await, antes, "la base de datos no cambia");

    // Al terminar el cambio, ya se puede.
    p.sim.control.set_password(PASS);
    p.sim.control.set_unresponsive(false);
    let _ = cambio.await;
    esperar_hasta(Duration::from_secs(10), "el cambio no acaba", async || {
        gestor.activo().is_none()
    })
    .await;
}

#[tokio::test]
async fn guardar_router_exige_reautenticacion() {
    let p = PanelPrueba::nuevo().arrancar().await;
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
    let p = PanelPrueba::nuevo().arrancar().await;
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
    let p = PanelPrueba::nuevo().arrancar().await;
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
    let p = PanelPrueba::nuevo().arrancar().await;
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

async fn esperar_solo_lectura(p: &Panel, valor: bool) {
    esperar_hasta(
        Duration::from_secs(5),
        &format!("solo lectura no pasa a {valor}"),
        async || instancia::solo_lectura(&p.state.db).await == valor,
    )
    .await;
}

const SIN_PROBAR_A: &str = "ES_g99.00_RTF_TEF001_V99.00_V001";

const SIN_PROBAR_B: &str = "ES_g99.01_RTF_TEF001_V99.01_V001";

#[tokio::test]
async fn firmware_confirmado_solo_vale_para_esa_version() {
    let mut p = PanelPrueba::nuevo().arrancar().await;
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

/// La conexión con el router vive en «Configuración del router → Conexión
/// con el router» (`/router/conexion`), no en «Ajustes del panel»; `/router`
/// lleva allí.
#[tokio::test]
async fn conexion_con_el_router_en_su_seccion() {
    let p = PanelPrueba::nuevo().arrancar().await;
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
    // La contraseña del router guardada llega con la página (decisión del
    // usuario del 2026-10-18): fuera de los datos de la página, solo en el
    // `value` de su campo, censurado con el ojo.
    let fuera: String = html
        .split("<script")
        .enumerate()
        .map(|(i, t)| {
            if i == 0 {
                t
            } else {
                t.split_once("</script>").map_or("", |(_, r)| r)
            }
        })
        .collect();
    assert_eq!(fuera.matches(PASS).count(), 1, "{fuera}");
    // Y en el `<input>` de su campo, no en otro.
    let input = fuera
        .split("<input")
        .skip(1)
        .map(|t| t.split('>').next().unwrap_or(""))
        .find(|t| t.contains(r#"id="panel-router-contrasena""#))
        .expect("el campo de la contraseña");
    assert!(input.contains(&format!(r#"value="{PASS}""#)), "{input}");
    assert!(input.contains(r#"type="password""#), "{input}");
    // Ninguna otra página la lleva: ni el Estado ni Ajustes.
    for ruta in [
        "/",
        "/ajustes/internet",
        "/ajustes/wifi",
        "/ajustes/red_local",
        "/ajustes/puertos",
        "/ajustes/seguridad",
        "/ajustes/telefono",
        "/ajustes/sistema",
    ] {
        let resp = get_con_cookie(&p.app, ruta, &cookie).await;
        assert_eq!(resp.status(), 200, "{ruta}");
        assert!(!body_text(resp).await.contains(PASS), "{ruta}");
    }

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
