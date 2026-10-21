//! Deshacer un cambio aplicado, una sola vez.

use super::*;

#[tokio::test]
async fn deshacer_restaura_valor_anterior() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    // Solo se deshace un cambio ya aplicado.
    let r = m.deshacer(id).await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;

    let deshacer = m.deshacer(id).await.unwrap();
    let c = &deshacer.diff.campos[0];
    assert_eq!(
        (c.nombre.as_str(), c.antes.as_str(), c.despues.as_str()),
        ("ipv4_addr", "192.168.1.80", "192.168.1.68")
    );
    let id2 = m.confirmar(&deshacer.token, None, "admin").await.unwrap();
    esperar_estado(&m, id2, Estado::Aplicado).await;
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

#[tokio::test]
async fn deshacer_una_reserva_nueva_la_borra() {
    let (p, m) = preparado().await;
    let prep = m.preparar(nueva_reserva()).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    let deshacer = m.deshacer(id).await.unwrap();
    assert!(
        deshacer
            .diff
            .campos
            .iter()
            .any(|c| c.nombre == "ipv4_addr" && c.antes == "192.168.1.99" && c.despues.is_empty())
    );
    let id2 = m.confirmar(&deshacer.token, None, "admin").await.unwrap();
    esperar_estado(&m, id2, Estado::Aplicado).await;
    assert_eq!(ip_de_lease(&p, "2").await, None);
    let w = p.sim.control.writes();
    let borrado: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(borrado["del"], "DEL");
    assert_eq!(borrado["rtm_dhcpv4_svr_pool_static_id"], "2");
    assert_eq!(borrado[POOL], "0");
    esperar_libre_gestor(&m).await;
    // Y deshacer el borrado la vuelve a crear.
    let otra = m.deshacer(id2).await.unwrap();
    let id3 = m.confirmar(&otra.token, None, "admin").await.unwrap();
    esperar_estado(&m, id3, Estado::Aplicado).await;
    let leases = read::static_leases(&p.state.hgu).await.unwrap();
    assert!(
        leases
            .iter()
            .any(|l| l.ip == "192.168.1.99" && l.mac == "02:00:00:00:00:99")
    );
}

#[tokio::test]
async fn no_se_deshace_dos_veces() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    let u1 = m.deshacer(id).await.unwrap();
    let u2 = m.deshacer(id).await.unwrap();
    let id2 = m.confirmar(&u1.token, None, "admin").await.unwrap();
    // Con el primero en curso, ni otro deshacer ni el segundo token.
    let r = m.deshacer(id).await;
    assert!(
        matches!(&r, Err(CambioError::Invalida(e)) if e.contains("ya se ha deshecho")),
        "{r:?}"
    );
    esperar_estado(&m, id2, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    let r = m.confirmar(&u2.token, None, "admin").await;
    assert!(
        matches!(&r, Err(CambioError::Invalida(e)) if e.contains("ya se ha deshecho")),
        "{r:?}"
    );
    let r = m.deshacer(id).await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    assert_eq!(p.sim.control.writes().len(), 2);
    assert_eq!(m.historial(10).await.len(), 2);
}
