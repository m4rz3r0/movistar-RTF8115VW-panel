//! Retomar al arrancar grupos que quedaron a medias.

use super::*;

#[tokio::test]
async fn reanuda_grupo_a_medias() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        ..tiempos_rapidos()
    };
    let a = gestor(&p, lento).await;
    let prep = a
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.80")]))
        .await
        .unwrap();
    // El router se cuelga tras la escritura del paso 2 y el panel muere
    // antes de terminar.
    p.sim.control.hang_after_write_n(2);
    let a2 = a.clone();
    let token = prep.token.clone();
    let tarea = tokio::spawn(async move { a2.confirmar_grupo(&token, &[], "admin").await });
    esperar_hasta(Duration::from_secs(5), "sin paso 2", async || {
        p.sim.control.writes().len() >= 2
    })
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    a.detener();
    tarea.abort();
    let id = a.historial(1).await.remove(0).id;
    p.sim.control.set_unresponsive(false);

    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    b.reanudar_pendientes().await;
    let c = esperar_final(&b, id).await;
    let (upnp, ip) = (upnp_actual(&p).await, ip_de_lease0(&p).await);
    match c.estado {
        Estado::Aplicado => assert_eq!((upnp.as_str(), ip.as_str()), ("1", "192.168.1.80")),
        Estado::Fallido | Estado::Revertido => {
            assert_eq!((upnp.as_str(), ip.as_str()), ("0", "192.168.1.68"))
        }
        e => panic!("estado {e:?}: {:?}", c.error),
    }
    esperar_libre_gestor(&b).await;
}

/// Interrumpido antes de escribir el paso 2: al retomar se revierte el 1.
#[tokio::test]
async fn reanuda_grupo_sin_terminar_revierte_lo_escrito() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let a = gestor(&p, tiempos_rapidos()).await;
    let prep = a
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.80")]))
        .await
        .unwrap();
    // El router se cuelga tras el paso 1: su verificación no llega a leer.
    p.sim.control.hang_after_write_n(1);
    let a2 = a.clone();
    let token = prep.token.clone();
    let tarea = tokio::spawn(async move { a2.confirmar_grupo(&token, &[], "admin").await });
    esperar_hasta(Duration::from_secs(5), "sin paso 1", async || {
        !p.sim.control.writes().is_empty()
    })
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    a.detener();
    tarea.abort();
    let id = a.historial(1).await.remove(0).id;
    let antes = a.cambio(id).await.unwrap();
    assert_eq!(antes.pasos[1].estado, EstadoPaso::Pendiente);
    p.sim.control.set_unresponsive(false);

    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    b.reanudar_pendientes().await;
    let c = esperar_final(&b, id).await;
    assert_eq!(c.estado, Estado::Fallido, "{:?}", c.error);
    assert_eq!(upnp_actual(&p).await, "0");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    assert_eq!(escritas(&p), [UPNP, UPNP]);
    assert_eq!(c.pasos[0].estado, EstadoPaso::Revertido);
    esperar_libre_gestor(&b).await;
}

/// Un cambio de credenciales que empezó antes de retomar no hace que el
/// grupo pendiente se cierre sin revertir: se espera al hueco y se revierte.
#[tokio::test]
async fn credenciales_durante_la_reanudacion_no_cierran_el_grupo() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let a = gestor(&p, tiempos_rapidos()).await;
    let prep = a
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.80")]))
        .await
        .unwrap();
    // Cortado entre el paso 1 y el 2.
    p.sim.control.hang_after_write_n(1);
    let tarea = confirmar_en_segundo_plano(&a, &prep.token);
    esperar_escrituras(&p, 1).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    a.detener();
    tarea.abort();
    let id = a.historial(1).await.remove(0).id;
    p.sim.control.set_unresponsive(false);

    // Lo que se anota mientras se retoma (las tareas de este hilo).
    let registro = Registro::default();
    let _registro = tracing::subscriber::set_default(
        tracing_subscriber::fmt()
            .with_writer(registro.clone())
            .with_ansi(false)
            .finish(),
    );
    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    // Guardar credenciales ganó la carrera: tiene el hueco.
    let credenciales = b.excluir_cambios().unwrap();
    let b2 = b.clone();
    let retomar = tokio::spawn(async move { b2.reanudar_pendientes().await });
    tokio::time::sleep(Duration::from_millis(200)).await;
    // Mientras se retoma, nadie más aparta el gestor.
    assert!(matches!(b.excluir_cambios(), Err(CambioError::EnCurso)));
    let c = b.cambio(id).await.unwrap();
    assert_eq!(
        c.estado,
        Estado::Aplicando,
        "espera al hueco: {:?}",
        c.error
    );
    assert_eq!(escritas(&p), [UPNP]);
    // La espera queda anotada, una vez.
    let esperas = registro
        .texto()
        .matches("espera a que quede libre el gestor de cambios")
        .count();
    assert_eq!(esperas, 1, "{}", registro.texto());
    drop(credenciales);
    retomar.await.unwrap();
    let c = esperar_final(&b, id).await;
    assert_eq!(c.estado, Estado::Fallido, "{:?}", c.error);
    assert_eq!(upnp_actual(&p).await, "0");
    assert_eq!(escritas(&p), [UPNP, UPNP]);
    esperar_libre_gestor(&b).await;
    assert!(b.excluir_cambios().is_ok(), "retomado: ya se puede");
}

/// Cortado a mitad de revertir un grupo de tres pasos (el router se cuelga
/// al revertir el paso 2): al retomar se termina la reversión.
#[tokio::test]
async fn reanuda_a_mitad_de_revertir() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let a = gestor(&p, tiempos_rapidos()).await;
    let prep = a
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.80"), ssid("OTRA")]))
        .await
        .unwrap();
    p.sim.control.fail_write_n(3);
    p.sim.control.hang_after_write_n(4);
    let tarea = confirmar_en_segundo_plano(&a, &prep.token);
    esperar_escrituras(&p, 4).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    a.detener();
    tarea.abort();
    let id = a.historial(1).await.remove(0).id;
    assert_eq!(a.cambio(id).await.unwrap().estado, Estado::Aplicando);
    p.sim.control.set_unresponsive(false);

    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    b.reanudar_pendientes().await;
    let c = esperar_final(&b, id).await;
    assert_eq!(c.estado, Estado::Fallido, "{:?}", c.error);
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
    esperar_libre_gestor(&b).await;
}

/// Cortado mientras se vigilaba un grupo; al retomar la WAN cae: se
/// revierte el grupo entero.
#[tokio::test]
async fn reanuda_vigilancia_de_grupo_y_revierte_si_cae_la_wan() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        ..tiempos_rapidos()
    };
    let a = gestor(&p, lento).await;
    let prep = a
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.80")]))
        .await
        .unwrap();
    let id = a.confirmar_grupo(&prep.token, &[], "admin").await.unwrap();
    assert_eq!(a.cambio(id).await.unwrap().estado, Estado::Vigilando);
    a.detener();

    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    b.reanudar_pendientes().await;
    assert_eq!(b.activo(), Some(id));
    p.sim.control.set_wan_up(false);
    let c = esperar_estado(&b, id, Estado::Revertido).await;
    assert!(c.error.as_deref().unwrap().contains("WAN"));
    assert_eq!(upnp_actual(&p).await, "0");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    assert_eq!(escritas(&p), [UPNP, LEASES, LEASES, UPNP]);
    esperar_libre_gestor(&b).await;
}

/// Un paso sin verificar cuya reversión falló sigue pendiente de revertir
/// si el panel se corta antes de cerrar el grupo: al retomar se revierte.
#[tokio::test]
async fn reversion_fallida_sigue_pendiente_al_retomar() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let a = gestor(&p, tiempos_rapidos()).await;
    let prep = a
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.80")]))
        .await
        .unwrap();
    // El paso 2 se aplica pero el router se cuelga: no se verifica y su
    // reversión agota los reintentos; el panel muere revirtiendo el 1.
    p.sim.control.hang_after_write_n(2);
    let tarea = confirmar_en_segundo_plano(&a, &prep.token);
    esperar_escrituras(&p, 2).await;
    let id = a.historial(1).await.remove(0).id;
    esperar_hasta(Duration::from_secs(5), "sin fallo del paso 2", async || {
        a.cambio(id).await.unwrap().pasos[1].error.is_some()
    })
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    a.detener();
    tarea.abort();
    assert_eq!(a.cambio(id).await.unwrap().estado, Estado::Aplicando);
    p.sim.control.set_unresponsive(false);

    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    b.reanudar_pendientes().await;
    let c = esperar_final(&b, id).await;
    assert_eq!(c.estado, Estado::Fallido, "{:?}", c.error);
    assert_eq!(upnp_actual(&p).await, "0");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68", "el paso 2 también");
    esperar_libre_gestor(&b).await;
}
