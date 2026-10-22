//! Conexión a internet (WAN) elegida a mano y detección de la red.

use super::*;

/// La red detectada y las conexiones del router llegan a la página; elegir
/// la conexión a internet a mano exige reautenticación y un nombre que el
/// router tenga. Volver a detectar, también reautenticación.
#[tokio::test]
async fn guardar_wan_exige_reautenticacion_y_nombre_valido() {
    let p = PanelPrueba::nuevo().arrancar().await;
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
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
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
    esperar_hasta(Duration::from_secs(5), "sin cambio activo", async || {
        gestor.activo().is_some() && !p.sim.control.writes().is_empty()
    })
    .await;
    let (status, body) = llamar(&p, "guardar_wan", json!({ "manual": "ip4" }), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    let (status, body) = llamar(&p, "redetectar", json!({}), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(p.state.wan().as_deref(), Some("ip2"));
    assert_eq!(instancia::leer(&p.state.db, claves::WAN_MANUAL).await, None);

    p.sim.control.set_unresponsive(false);
    let _ = cambio.await;
    esperar_hasta(Duration::from_secs(10), "el cambio no acaba", async || {
        gestor.activo().is_none()
    })
    .await;
    ok(&p, "guardar_wan", json!({ "manual": "ip4" }), &cookie).await;
    assert_eq!(p.state.wan().as_deref(), Some("ip4"));
}

/// Sin ninguna lectura del sondeo, la página no pregunta al router (caído,
/// la retrasaría; con credenciales malas, sumaría intentos de entrada):
/// ofrece las conexiones que conoce el panel.
#[tokio::test]
async fn leer_panel_sin_sondeo_no_pregunta_al_router() {
    let p = PanelPrueba::nuevo().arrancar().await;
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
    tokio::time::timeout(
        Duration::from_secs(2),
        ok(&p, "leer_preferencias", json!({}), &cookie),
    )
    .await
    .expect("leer_preferencias no espera al router");
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
    esperar_hasta(Duration::from_secs(5), "sin primer dato", async || {
        h.healthy()
    })
    .await;
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["conexiones"], json!(["ip2", "ip3", "ip4"]), "{v}");
}

/// Mientras se retoman los cambios del arranque (uno quedó vigilándose), ni
/// se elige la conexión a internet, ni se vuelve a detectar, ni se anota una
/// detección que acabe entonces.
#[tokio::test]
async fn red_bloqueada_mientras_se_retoman_cambios() {
    use panel::changes::{ChangeManager, ChangeRequest, Tiempos};
    let mut p = PanelPrueba::nuevo().arrancar().await;
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
        identidad: Default::default(),
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
    esperar_hasta(Duration::from_secs(5), "no retoma", async || b.ocupado()).await;
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
    esperar_hasta(Duration::from_secs(10), "no termina", async || !b.ocupado()).await;
    ok(&p, "guardar_wan", json!({ "manual": "ip4" }), &cookie).await;
}
