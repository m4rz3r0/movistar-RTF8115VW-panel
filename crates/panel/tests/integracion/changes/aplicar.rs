//! Preparar, confirmar y verificar cambios: bloqueos, secretos, tokens y reinicio.

use super::*;

#[tokio::test]
async fn aplica_y_verifica_static_lease() {
    let (p, m) = preparado().await;
    // Con el sondeo en marcha: el gestor lo pausa para escribir.
    let h = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(20),
            slow: Duration::from_millis(100),
            ..PollerConfig::default()
        },
    );
    let prep = m.preparar(nueva_reserva()).await.unwrap();
    assert_eq!(prep.riesgo, Riesgo::Bajo);
    assert_eq!(prep.palabra, None);
    let campos: Vec<_> = prep
        .diff
        .campos
        .iter()
        .map(|c| (c.nombre.as_str(), c.antes.as_str(), c.despues.as_str()))
        .collect();
    assert!(
        campos.contains(&("ipv4_addr", "", "192.168.1.99")),
        "{campos:?}"
    );
    assert!(campos.contains(&("mac_addr", "", "02:00:00:00:00:99")));

    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    assert_eq!(m.activo(), Some(id));
    let c = esperar_estado(&m, id, Estado::Aplicado).await;
    assert_eq!(c.usuario.as_deref(), Some("admin"));
    assert_eq!(c.funcion, LEASES);
    // La instancia creada queda en el historial (para deshacer).
    assert_eq!(c.instancia.as_deref(), Some("2"));
    assert_eq!(ip_de_lease(&p, "2").await.as_deref(), Some("192.168.1.99"));
    let w = p.sim.control.writes();
    assert_eq!(w.len(), 1);
    let params: BTreeMap<_, _> = w[0].1.iter().cloned().collect();
    assert_eq!(params["rtm_dhcpv4_svr_pool_static_id"], "-1");
    assert_eq!(params["mac_addr"], "02:00:00:00:00:99");
    esperar_libre_gestor(&m).await;
    assert!(h.latest().is_some());
}

#[tokio::test]
async fn edicion_envia_la_instancia_completa() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    assert_eq!(prep.diff.campos.len(), 1);
    let c = &prep.diff.campos[0];
    assert_eq!(
        (c.nombre.as_str(), c.antes.as_str(), c.despues.as_str()),
        ("ipv4_addr", "192.168.1.68", "192.168.1.80")
    );
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    let w = p.sim.control.writes();
    let params: BTreeMap<_, _> = w[0].1.iter().cloned().collect();
    // Los campos no tocados se envían con su valor leído.
    assert_eq!(params["mac_addr"], "02:00:00:00:00:04");
    assert_eq!(params["adm_state"], "1");
    assert_eq!(params["rtm_dhcpv4_svr_pool_static_id"], "0");
    assert_eq!(params[POOL], "0");
    assert!(!params.contains_key("del"));
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
}

#[tokio::test]
async fn sin_cambios_rechazado() {
    let (p, m) = preparado().await;
    let r = m.preparar(lease0("192.168.1.68")).await;
    assert!(matches!(r, Err(CambioError::SinCambios)), "{r:?}");
    // IP/MAC se comparan sin distinguir mayúsculas.
    let r = m
        .preparar(req(
            LEASES,
            Some("0"),
            &[(POOL, "0"), ("mac_addr", "02:00:00:00:00:0A")],
        ))
        .await;
    assert!(r.is_ok(), "MAC distinta: {r:?}");
    let r = m
        .preparar(req(
            LEASES,
            Some("1"),
            &[(POOL, "0"), ("ipv4_addr", " 192.168.1.69 ")],
        ))
        .await;
    assert!(matches!(r, Err(CambioError::SinCambios)), "{r:?}");
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn bloqueada_rechazada() {
    let (p, m) = preparado().await;
    let r = m
        .preparar(req("cbRestoreConfig", None, &[("file", "x")]))
        .await;
    assert!(
        matches!(&r, Err(CambioError::Bloqueada(motivo)) if motivo.contains("configuración")),
        "{r:?}"
    );
    let r = m.preparar(req("cbGetSummary", None, &[])).await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    let r = m.preparar(req("cbNoExiste", None, &[])).await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    let r = m
        .preparar(req(LEASES, Some("0"), &[(POOL, "0"), ("inventado", "1")]))
        .await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    let r = m.preparar(lease0("192.168.1.300")).await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn verificacion_falla_si_router_ignora() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    p.sim.control.ignore_next_write();
    let r = m.confirmar(&prep.token, None, "admin").await;
    let Err(CambioError::Verificacion(fallos)) = r else {
        panic!("{r:?}");
    };
    assert!(fallos.iter().any(|f| f.contains("ipv4_addr")), "{fallos:?}");
    let ultimo = m.historial(1).await.remove(0);
    assert_eq!(ultimo.estado, Estado::Fallido);
    // Sin reversión: una sola escritura.
    assert_eq!(p.sim.control.writes().len(), 1);
    assert_eq!(m.activo(), None);
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

#[tokio::test]
async fn error_del_router_deja_fallido() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    p.sim.control.fail_next_write("no se puede");
    let r = m.confirmar(&prep.token, None, "admin").await;
    assert!(
        matches!(&r, Err(CambioError::Router(e)) if e.contains("no se puede")),
        "{r:?}"
    );
    let ultimo = m.historial(1).await.remove(0);
    assert_eq!(ultimo.estado, Estado::Fallido);
    assert!(crate::comun::legible(&ultimo.error.unwrap()).contains("no se puede"));
    assert_eq!(m.activo(), None);
}

#[tokio::test]
async fn cambios_concurrentes_no_se_intercalan() {
    let (p, m) = preparado().await;
    let a = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let b = m
        .preparar(req("cbSetUpnpCfg", None, &[("adm_state", "1")]))
        .await
        .unwrap();
    let (ra, rb) = tokio::join!(
        m.confirmar(&a.token, None, "admin"),
        m.confirmar(&b.token, None, "admin")
    );
    let (ok, otro, token_otro) = match (ra, rb) {
        (Ok(id), Err(e)) => (id, e, b.token.clone()),
        (Err(e), Ok(id)) => (id, e, a.token.clone()),
        r => panic!("se esperaba uno aceptado y otro en curso: {r:?}"),
    };
    assert!(matches!(otro, CambioError::EnCurso), "{otro:?}");
    assert_eq!(p.sim.control.writes().len(), 1);
    // Mientras se vigila, sigue ocupado.
    let r = m.confirmar(&token_otro, None, "admin").await;
    assert!(matches!(r, Err(CambioError::EnCurso)), "{r:?}");
    esperar_estado(&m, ok, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    // El token rechazado por estar ocupado sigue valiendo.
    let id = m.confirmar(&token_otro, None, "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    assert_eq!(p.sim.control.writes().len(), 2);
}

#[tokio::test]
async fn token_de_un_solo_uso() {
    let (_p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    let r = m.confirmar(&prep.token, None, "admin").await;
    assert!(matches!(r, Err(CambioError::TokenInvalido)), "{r:?}");
    let r = m.confirmar("inventado", None, "admin").await;
    assert!(matches!(r, Err(CambioError::TokenInvalido)), "{r:?}");
}

#[tokio::test]
async fn pausa_que_no_llega_no_escribe() {
    let p = panel_rapido(Duration::from_secs(5)).await;
    let m = gestor(
        &p,
        Tiempos {
            pausa_max: Duration::from_millis(300),
            ..tiempos_rapidos()
        },
    )
    .await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(20),
            ..PollerConfig::default()
        },
    );
    // El ciclo del sondeo queda colgado esperando al router.
    p.sim.control.set_unresponsive(true);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let r = m.confirmar(&prep.token, None, "admin").await;
    assert!(
        matches!(&r, Err(CambioError::Router(e)) if e.contains("el sondeo no se detuvo a tiempo")),
        "{r:?}"
    );
    p.sim.control.set_unresponsive(false);
    assert!(p.sim.control.writes().is_empty());
    assert_eq!(m.activo(), None);
    assert!(m.historial(10).await.is_empty());
}

#[tokio::test]
async fn reboot_exige_palabra() {
    let (p, m) = preparado().await;
    let prep = m.preparar(req("cbReboot", None, &[])).await.unwrap();
    assert_eq!(prep.riesgo, Riesgo::Alto);
    assert_eq!(prep.palabra.as_deref(), Some("REINICIAR"));
    for palabra in [None, Some("reiniciar"), Some("SI")] {
        let r = m.confirmar(&prep.token, palabra, "admin").await;
        assert!(matches!(r, Err(CambioError::PalabraIncorrecta)), "{r:?}");
    }
    assert!(p.sim.control.writes().is_empty());
    let id = m
        .confirmar(&prep.token, Some("REINICIAR"), "admin")
        .await
        .unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    let w = p.sim.control.writes();
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].0, "cbReboot");
}

#[tokio::test]
async fn reboot_sin_vuelta_es_fallido() {
    let p = panel_rapido(Duration::from_millis(100)).await;
    let m = gestor(&p, tiempos_rapidos()).await;
    let prep = m.preparar(req("cbReboot", None, &[])).await.unwrap();
    let id = m
        .confirmar(&prep.token, Some("REINICIAR"), "admin")
        .await
        .unwrap();
    // El router no vuelve a responder tras reiniciar.
    p.sim.control.set_unresponsive(true);
    let c = esperar_estado(&m, id, Estado::Fallido).await;
    p.sim.control.set_unresponsive(false);
    assert!(crate::comun::legible(&c.error.unwrap()).contains("no volvió"));
}

#[tokio::test]
async fn bloqueos_de_la_v1() {
    let (p, m) = preparado().await;
    // Web básica con parámetros secretos (aunque no se toque el secreto).
    for f in ["te_wifi.wl", "te_wifi_5ghz.wl", "te_wifi_guest.wl"] {
        let r = m.preparar(req(f, None, &[("wlSsid", "Casa")])).await;
        assert!(
            matches!(&r, Err(CambioError::Bloqueada(e)) if e.contains("bloqueada en v1") && e.contains("web básica")),
            "{f}: {r:?}"
        );
    }
    // Cambios de la IP LAN del router.
    for f in [
        "cbSetLanIPv4Addr",
        "cbSetLanIPv4Intf",
        "te_red_local.cgi",
        "te_mapa_red_local.cgi",
    ] {
        let r = m.preparar(req(f, None, &[])).await;
        assert!(
            matches!(&r, Err(CambioError::Bloqueada(e)) if e.contains("bloqueada en v1") && e.contains("IP del router")),
            "{f}: {r:?}"
        );
    }
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn editar_un_campo_ilegible_se_rechaza() {
    let (p, m) = preparado().await;
    // `validation_only` no aparece en la lectura pareada.
    let r = m
        .preparar(req(
            "cbSetWifiAccPt",
            None,
            &[("dpm_cfg_wifi_acc_pt_id", "1"), ("validation_only", "0")],
        ))
        .await;
    assert!(
        matches!(&r, Err(CambioError::Invalida(e)) if e.contains("no se puede leer el valor actual de validation_only")),
        "{r:?}"
    );
    // Web básica sin secretos cuya lectura no muestra el campo.
    let r = m
        .preparar(req("te_upnp.cmd", None, &[("enblUpnp", "1")]))
        .await;
    assert!(matches!(&r, Err(CambioError::Invalida(_))), "{r:?}");
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn historial_en_orden() {
    let (_p, m) = preparado().await;
    let mut ids = Vec::new();
    for ip in ["192.168.1.80", "192.168.1.81"] {
        let prep = m.preparar(lease0(ip)).await.unwrap();
        let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
        esperar_estado(&m, id, Estado::Aplicado).await;
        esperar_libre_gestor(&m).await;
        ids.push(id);
    }
    let h = m.historial(10).await;
    assert_eq!(h.iter().map(|c| c.id).collect::<Vec<_>>(), [ids[1], ids[0]]);
    assert_eq!(h[0].riesgo, Riesgo::Bajo);
    assert_eq!(h[0].diff.campos[0].despues, "192.168.1.81");
    assert!(h[0].updated >= h[0].ts);
    assert_eq!(m.historial(1).await.len(), 1);
}

#[tokio::test]
async fn confirmar_abandonado_termina_igual() {
    let (_p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    // Quien confirma se va (p. ej. se cierra la pestaña) a mitad de camino.
    let r = tokio::time::timeout(
        Duration::from_millis(2),
        m.confirmar(&prep.token, None, "admin"),
    )
    .await;
    assert!(r.is_err(), "debía abandonarse antes de terminar: {r:?}");
    let id = esperar_hasta_con(Duration::from_secs(10), async || {
        m.historial(1)
            .await
            .first()
            .map(|c| c.id)
            .ok_or_else(|| "sin cambio".to_owned())
    })
    .await;
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
}

#[tokio::test]
async fn instalar_deja_un_solo_gestor() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    assert!(p.state.cambios().is_none());
    let a = ChangeManager::instalar(&p.state, tiempos_rapidos());
    let b = ChangeManager::instalar(&p.state.clone(), tiempos_rapidos());
    assert!(Arc::ptr_eq(&a, &b));
    assert!(Arc::ptr_eq(&p.state.cambios().unwrap(), &a));
}
