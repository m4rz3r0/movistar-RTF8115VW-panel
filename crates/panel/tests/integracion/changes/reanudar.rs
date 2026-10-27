//! Retomar al arrancar los cambios que quedaron a medias.

use super::*;
use crate::comun::ConfirmarLigado;

#[tokio::test]
async fn reanuda_tras_reinicio() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        ..tiempos_rapidos()
    };
    let a = gestor(&p, lento).await;
    let prep = a.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = a
        .confirmar_ligado(&prep.token, None, "admin")
        .await
        .unwrap();
    assert_eq!(a.cambio(id).await.unwrap().estado, Estado::Vigilando);
    // El proceso muere a mitad de la vigilancia.
    a.detener();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(a.cambio(id).await.unwrap().estado, Estado::Vigilando);

    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    b.reanudar_pendientes().await;
    assert_eq!(b.activo(), Some(id));
    let c = esperar_estado(&b, id, Estado::Aplicado).await;
    let nota = c.nota.unwrap();
    assert!(
        Nota::texto(&nota, Idioma::Es).contains("reinicio del panel"),
        "{nota}"
    );
    assert!(
        Nota::texto(&nota, Idioma::En).contains("panel restarted"),
        "{nota}"
    );
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
}

#[tokio::test]
async fn reanuda_y_marca_fallido_si_no_llego_a_aplicarse() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        ..tiempos_rapidos()
    };
    let a = gestor(&p, lento).await;
    let prep = a.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = a
        .confirmar_ligado(&prep.token, None, "admin")
        .await
        .unwrap();
    a.detener();
    // El router vuelve a tener el valor anterior (p. ej. no llegó a guardarlo).
    p.state
        .hgu
        .write(
            "dhcp.asp",
            LEASES,
            &[
                (POOL, "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("ipv4_addr", "192.168.1.68"),
            ],
        )
        .await
        .unwrap();
    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    b.reanudar_pendientes().await;
    let c = b.cambio(id).await.unwrap();
    assert_eq!(c.estado, Estado::Fallido);
    assert!(crate::comun::legible(&c.error.unwrap()).contains("interrumpido antes de aplicar"));
    assert_eq!(b.activo(), None);
}

#[tokio::test]
async fn reanuda_con_la_wan_de_antes_del_cambio() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        ..tiempos_rapidos()
    };
    let a = gestor(&p, lento).await;
    let prep = a.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = a
        .confirmar_ligado(&prep.token, None, "admin")
        .await
        .unwrap();
    a.detener();
    // El cambio tumbó la WAN y el panel se reinició: se vigila con la WAN de
    // antes (arriba), así que se revierte.
    p.sim.control.set_wan_up(false);
    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    b.reanudar_pendientes().await;
    let c = esperar_estado(&b, id, Estado::Revertido).await;
    assert!(crate::comun::legible(&c.error.unwrap()).contains("WAN"));
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

#[tokio::test]
async fn confirmar_espera_a_que_se_retome_lo_pendiente_y_se_encola() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        ..tiempos_rapidos()
    };
    let a = gestor(&p, lento).await;
    let prep = a.preparar(lease0("192.168.1.80")).await.unwrap();
    let pendiente = a
        .confirmar_ligado(&prep.token, None, "admin")
        .await
        .unwrap();
    a.detener();

    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    let otro = b
        .preparar(req("cbSetUpnpCfg", None, &[("adm_state", "1")]))
        .await
        .unwrap();
    let (r, ()) = tokio::join!(b.confirmar_ligado(&otro.token, None, "admin"), async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        b.reanudar_pendientes().await;
    });
    // El cambio nuevo esperó, el retomado se quedó el hueco y el nuevo
    // espera en la cola hasta que el retomado queda aplicado.
    let id = r.unwrap();
    assert_eq!(b.activo(), Some(pendiente));
    assert_eq!(b.en_cola(), vec![id]);
    esperar_estado(&b, pendiente, Estado::Aplicado).await;
    esperar_estado(&b, id, Estado::Aplicado).await;
}
