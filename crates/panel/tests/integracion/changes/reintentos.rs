//! Reversiones cuya relectura falla: se reintentan sin duplicar.

use super::*;

#[tokio::test]
async fn relectura_fallida_tras_revertir_se_reintenta() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim
        .control
        .fail_reads_after_next_write("cbDhcpV4SvrPoolGetStatic");
    p.sim.control.set_wan_up(false);
    esperar_hasta(Duration::from_secs(5), "sin reversión", async || {
        p.sim.control.writes().len() >= 2
    })
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(m.cambio(id).await.unwrap().estado, Estado::Vigilando);
    p.sim.control.fail_reads("cbDhcpV4SvrPoolGetStatic", false);
    esperar_estado(&m, id, Estado::Revertido).await;
    assert_eq!(p.sim.control.writes().len(), 2, "no se reescribe");
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

/// Tras confirmar, la lectura de reservas falla justo después de la próxima
/// escritura (la de la reversión) y se pierde la WAN; al cabo de 100 ms la
/// lectura vuelve.
async fn revertir_con_relectura_fallida(p: &Panel, m: &ChangeManager, id: i64) {
    p.sim
        .control
        .fail_reads_after_next_write("cbDhcpV4SvrPoolGetStatic");
    p.sim.control.set_wan_up(false);
    esperar_hasta(Duration::from_secs(5), "sin reversión", async || {
        p.sim.control.writes().len() >= 2
    })
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    p.sim.control.fail_reads("cbDhcpV4SvrPoolGetStatic", false);
    esperar_estado(m, id, Estado::Revertido).await;
    assert_eq!(
        p.sim.control.writes().len(),
        2,
        "una sola escritura de reversión"
    );
}

#[tokio::test]
async fn reintento_de_revertir_un_borrado_no_duplica() {
    let (p, m) = preparado().await;
    let borrar = ChangeRequest {
        borrar: true,
        identidad: Default::default(),
        ..req(LEASES, Some("1"), &[(POOL, "0")])
    };
    let prep = m.preparar(borrar).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    assert_eq!(ip_de_lease(&p, "1").await, None);
    revertir_con_relectura_fallida(&p, &m, id).await;
    let recreadas: Vec<_> = read::static_leases(&p.state.hgu)
        .await
        .unwrap()
        .into_iter()
        .filter(|l| l.ip == "192.168.1.69")
        .collect();
    assert_eq!(recreadas.len(), 1, "{recreadas:?}");
    let w = p.sim.control.writes();
    let crear: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(crear["rtm_dhcpv4_svr_pool_static_id"], "-1");
}

#[tokio::test]
async fn reintento_de_revertir_una_creacion_acepta_que_ya_no_exista() {
    // Cliente con 100 ms de espera: la relectura tras la reversión agota el
    // tiempo (no es un "no existe" del router) y hay que reintentar.
    let p = panel_rapido(Duration::from_millis(100)).await;
    let m = gestor(&p, tiempos_rapidos()).await;
    let prep = m.preparar(nueva_reserva()).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.hang_after_next_write();
    p.sim.control.set_wan_up(false);
    esperar_hasta(Duration::from_secs(5), "sin reversión", async || {
        p.sim.control.writes().len() >= 2
    })
    .await;
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(m.cambio(id).await.unwrap().estado, Estado::Vigilando);
    p.sim.control.set_unresponsive(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    assert_eq!(
        p.sim.control.writes().len(),
        2,
        "una sola escritura de reversión"
    );
    assert_eq!(ip_de_lease(&p, "2").await, None);
    let w = p.sim.control.writes();
    let borrado: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(borrado["del"], "DEL");
}

#[tokio::test]
async fn router_que_rechaza_la_reversion_falla_sin_reintentar() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.fail_next_write("valores no admitidos");
    p.sim.control.set_wan_up(false);
    let c = esperar_estado(&m, id, Estado::Fallido).await;
    let e = crate::comun::legible(&c.error.unwrap());
    assert!(e.contains("valores no admitidos"), "{e}");
    // Sin reintentos: el cambio y un único intento de reversión.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(p.sim.control.writes().len(), 2);
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
}
