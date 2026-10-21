//! Revertir y deshacer grupos: fallos a mitad, vigilancia y escrituras inciertas.

use super::*;

#[tokio::test]
async fn fallo_en_paso_k_revierte_anteriores() {
    let (p, m) = preparado().await;
    let prep = m
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.80"), ssid("OTRA")]))
        .await
        .unwrap();
    assert_eq!(prep.riesgo, Riesgo::Medio, "el mayor de los pasos");
    p.sim.control.fail_write_n(3);
    let r = m.confirmar_grupo(&prep.token, &[], "admin").await;
    assert!(r.is_err(), "{r:?}");
    let c = m.historial(1).await.remove(0);
    assert_eq!(c.estado, Estado::Fallido, "{:?}", c.error);
    // Los pasos 2 y 1, en orden inverso.
    assert_eq!(escritas(&p), [UPNP, LEASES, WIFI, LEASES, UPNP]);
    assert_eq!(upnp_actual(&p).await, "0");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    assert_eq!(ssid_actual(&p).await, "RED-1");
    assert_eq!(
        estados_pasos(&c),
        [
            EstadoPaso::Revertido,
            EstadoPaso::Revertido,
            EstadoPaso::Fallido
        ]
    );
    assert_eq!(m.activo(), None);
}

#[tokio::test]
async fn fallo_al_revertir_es_revertido_parcial() {
    let (p, m) = preparado().await;
    let prep = m
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.80"), ssid("OTRA")]))
        .await
        .unwrap();
    // Falla el paso 3 y, al revertir, la reversión del paso 1 (escritura 5).
    p.sim.control.fail_write_n(3);
    p.sim.control.fail_write_n(5);
    let r = m.confirmar_grupo(&prep.token, &[], "admin").await;
    let Err(CambioError::Router(e)) = &r else {
        panic!("{r:?}");
    };
    assert!(e.contains("paso 1"), "{e}");
    let c = m.historial(1).await.remove(0);
    assert_eq!(c.estado, Estado::RevertidoParcial);
    let detalle = c.error.clone().unwrap();
    assert!(detalle.contains("paso 1"), "{detalle}");
    assert!(!detalle.contains("paso 2:"), "{detalle}");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    assert_eq!(upnp_actual(&p).await, "1", "el paso 1 sigue aplicado");
    assert_eq!(
        estados_pasos(&c),
        [
            EstadoPaso::Aplicado,
            EstadoPaso::Revertido,
            EstadoPaso::Fallido
        ]
    );
    assert_eq!(m.activo(), None);
}

#[tokio::test]
async fn vigilancia_revierte_grupo_entero() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    // El riesgo medio vigila más que el bajo: la vigilancia del grupo es la
    // del paso de mayor riesgo.
    let t = Tiempos {
        bajo: Duration::from_millis(50),
        medio: Duration::from_secs(5),
        ..tiempos_rapidos()
    };
    let m = gestor(&p, t).await;
    let prep = m
        .preparar_grupo(grupo(vec![lease0("192.168.1.80"), ssid("OTRA")]))
        .await
        .unwrap();
    assert_eq!(prep.riesgo, Riesgo::Medio);
    let id = m.confirmar_grupo(&prep.token, &[], "admin").await.unwrap();
    assert_eq!(m.cambio(id).await.unwrap().riesgo, Riesgo::Medio);
    // Pasado el plazo del riesgo bajo, la vigilancia sigue.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(m.cambio(id).await.unwrap().estado, Estado::Vigilando);
    p.sim.control.set_wan_up(false);
    let c = esperar_estado(&m, id, Estado::Revertido).await;
    assert!(c.error.as_deref().unwrap().contains("WAN"));
    assert_eq!(escritas(&p), [LEASES, WIFI, WIFI, LEASES]);
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    assert_eq!(ssid_actual(&p).await, "RED-1");
    assert_eq!(
        estados_pasos(&c),
        [EstadoPaso::Revertido, EstadoPaso::Revertido]
    );
    esperar_libre_gestor(&m).await;
}

#[tokio::test]
async fn deshacer_grupo() {
    let (p, m) = preparado().await;
    let prep = m
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.80")]))
        .await
        .unwrap();
    let id = m.confirmar_grupo(&prep.token, &[], "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    assert!(m.se_puede_deshacer(id).await);

    let d = m.deshacer(id).await.unwrap();
    // Los dos pasos, en orden inverso.
    assert_eq!(d.pasos.len(), 2);
    assert_eq!(d.pasos[0].funcion, LEASES);
    assert_eq!(d.pasos[1].funcion, UPNP);
    assert_eq!(d.diff.campos.len(), 2);
    let id2 = m.confirmar(&d.token, None, "admin").await.unwrap();
    let c2 = esperar_estado(&m, id2, Estado::Aplicado).await;
    assert_eq!(c2.pasos.len(), 2);
    assert_eq!(escritas(&p), [UPNP, LEASES, LEASES, UPNP]);
    assert_eq!(upnp_actual(&p).await, "0");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    esperar_libre_gestor(&m).await;
    assert!(!m.se_puede_deshacer(id).await, "ya deshecho");
}

/// Escritura sin respuesta tras la que el router no muestra ni lo de antes
/// ni lo nuevo: no se da por aplicada; se revierte.
#[tokio::test]
async fn sin_respuesta_con_otros_valores_se_revierte() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80").req).await.unwrap();
    p.sim.control.no_answer_next_write(true);
    let m2 = m.clone();
    let token = prep.token.clone();
    let tarea = tokio::spawn(async move { m2.confirmar(&token, None, "admin").await });
    esperar_hasta(Duration::from_secs(5), "sin escritura", async || {
        !p.sim.control.writes().is_empty()
    })
    .await;
    // Mientras el panel espera, otro cambia la reserva a un tercer valor.
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, crate::comun::USER, crate::comun::PASS);
    cfg.min_gap = Duration::from_millis(1);
    let otro = hgu::Client::new(cfg);
    otro.write(
        "dhcp.asp",
        LEASES,
        &[
            (POOL, "0"),
            ("rtm_dhcpv4_svr_pool_static_id", "0"),
            ("ipv4_addr", "192.168.1.77"),
        ],
    )
    .await
    .unwrap();
    let r = tarea.await.unwrap();
    assert!(
        matches!(&r, Err(CambioError::Router(e)) if e.contains("otros valores")),
        "{r:?}"
    );
    let c = m.historial(1).await.remove(0);
    assert_eq!(c.estado, Estado::Fallido, "{:?}", c.error);
    assert!(c.error.unwrap().contains("revertido"));
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    assert_eq!(m.activo(), None);
}

/// Tras una escritura sin respuesta, si la relectura falla, el sondeo deja
/// de estar en pausa: `/salud` refleja el router (aquí, caído) en vez de
/// quedarse congelado hasta el fin de la relectura.
#[tokio::test]
async fn pausa_se_suelta_en_relectura() {
    let p = panel_rapido(Duration::from_millis(300)).await;
    let sondeo = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(50),
            ..PollerConfig::default()
        },
    );
    esperar_hasta(Duration::from_secs(5), "sondeo sin datos", async || {
        sondeo.healthy()
    })
    .await;
    // La relectura se reintentaría durante un minuto.
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        intervalo: Duration::from_millis(50),
        ..tiempos_rapidos()
    };
    let m = gestor(&p, lento).await;
    let prep = m.preparar(lease0("192.168.1.80").req).await.unwrap();
    p.sim.control.no_answer_next_write(true);
    p.sim.control.hang_after_next_write();
    let m2 = m.clone();
    let token = prep.token.clone();
    tokio::spawn(async move { m2.confirmar(&token, None, "admin").await });
    esperar_hasta(Duration::from_secs(5), "sin escritura", async || {
        !p.sim.control.writes().is_empty()
    })
    .await;
    // Escritura (300 ms) + primera relectura (300 ms) + dos ciclos del
    // sondeo (cada uno agota su espera de 300 ms).
    let limite = Duration::from_millis(2500);
    esperar_hasta_con(limite, async || {
        let body = crate::comun::body_text(get(&p.app, "/salud").await).await;
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        if v["router"] == "error" {
            return Ok(());
        }
        Err(format!("/salud sigue congelado en {:?}", v["router"]))
    })
    .await;
    p.sim.control.set_unresponsive(false);
    m.detener();
}

/// En un grupo, una creación sin respuesta tras la que el router no muestra
/// lo de antes (aquí, dos instancias nuevas) no se da por no aplicada: se
/// intenta revertir y, sin saber qué instancia es, el grupo queda
/// `revertido_parcial` nombrando el paso.
#[tokio::test]
async fn creacion_incierta_en_grupo_es_revertido_parcial() {
    let (p, m) = preparado().await;
    let prep = m
        .preparar_grupo(grupo(vec![nueva_reserva(), upnp("1")]))
        .await
        .unwrap();
    p.sim.control.create_twice_next_write();
    p.sim.control.no_answer_next_write(true);
    let r = m.confirmar_grupo(&prep.token, &[], "admin").await;
    assert!(r.is_err(), "{r:?}");
    let c = m.historial(1).await.remove(0);
    assert_eq!(c.estado, Estado::RevertidoParcial, "{:?}", c.error);
    let e = c.error.clone().unwrap();
    assert!(e.contains("paso 1"), "{e}");
    assert!(!e.contains("revertido a los valores anteriores"), "{e}");
    assert_eq!(escritas(&p), [LEASES], "no sigue con el paso 2");
    assert_eq!(
        estados_pasos(&c),
        [EstadoPaso::Fallido, EstadoPaso::Pendiente]
    );
    assert_eq!(m.activo(), None);
}

/// Un deshacer que queda revertido en parte cuenta como deshecho: el
/// original no se puede deshacer otra vez.
#[tokio::test]
async fn deshacer_revertido_en_parte_no_se_repite() {
    let (p, m) = preparado().await;
    let prep = m
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.80")]))
        .await
        .unwrap();
    let id = m.confirmar_grupo(&prep.token, &[], "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    let d = m.deshacer(id).await.unwrap();
    // Falla el paso 2 del deshacer (UPnP) y la reversión de su paso 1.
    p.sim.control.fail_write_n(2);
    p.sim.control.fail_write_n(3);
    let r = m.confirmar(&d.token, None, "admin").await;
    assert!(r.is_err(), "{r:?}");
    let u = m.historial(1).await.remove(0);
    assert_eq!(u.estado, Estado::RevertidoParcial, "{:?}", u.error);
    assert!(!m.se_puede_deshacer(id).await);
    assert!(matches!(
        m.deshacer(id).await,
        Err(CambioError::Invalida(_))
    ));
}
