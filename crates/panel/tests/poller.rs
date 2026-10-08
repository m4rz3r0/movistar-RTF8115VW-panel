//! Pruebas del sondeo en segundo plano contra `router-sim`, con intervalos
//! cortos.

mod common;

use common::{Panel, panel};
use panel::{
    poller::{Poller, PollerConfig, PollerHandle},
    series::{self, Rango},
};
use std::time::{Duration, Instant};

/// Sondeo rápido: 50 ms entre lecturas, 200 ms para hosts/WiFi, espera
/// máxima de 400 ms tras errores.
fn config() -> PollerConfig {
    PollerConfig {
        fast: Duration::from_millis(50),
        slow: Duration::from_millis(200),
        backoff_max: Duration::from_millis(400),
        auth_retry: Duration::from_secs(1),
        compact_every: Duration::from_millis(200),
    }
}

/// Arranca el sondeo con un cliente del router de tiempo de espera corto
/// (200 ms) y espera la primera instantánea completa.
async fn sondeo(p: &mut Panel) -> PollerHandle {
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, common::USER, common::PASS);
    cfg.timeout = Duration::from_millis(200);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    let h = Poller::spawn_with(p.state.clone(), config());
    esperar("primera instantánea con datos de 60 s", || {
        h.latest().is_some_and(|s| s.slow_at.is_some())
    })
    .await;
    h
}

async fn esperar(que: &str, mut cond: impl FnMut() -> bool) {
    let start = Instant::now();
    while !cond() {
        assert!(start.elapsed() < Duration::from_secs(5), "no llega: {que}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn eventos_wan(p: &Panel) -> Vec<(i64, Option<i64>)> {
    sqlx::query_as("SELECT desde, hasta FROM wan_events ORDER BY id")
        .fetch_all(&p.state.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn poller_lee_estado_y_guarda_muestras() {
    let mut p = panel().await;
    let h = sondeo(&mut p).await;
    let s = h.latest().unwrap();
    assert_eq!(s.summary.model, "RTF8115VW");
    assert!(s.wan.iter().any(|w| w.name == "ip2" && w.up));
    assert!(s.counters.iter().any(|c| c.name == "ip2"));
    assert!(!s.hosts.is_empty());
    assert!(!s.leases.is_empty());
    assert!(!s.statics.is_empty());
    assert_eq!(s.wifi.radios.len(), 2);
    assert_eq!(s.tr69_activo, Some(true));
    assert_eq!(s.error, None);
    assert!(h.healthy());

    let muestras = series::query(&p.state.db, Rango::Hora).await.unwrap();
    assert!(!muestras.is_empty());
    let ultima = muestras.last().unwrap();
    assert!(ultima.wan_up);
    assert!(ultima.ram_used_pct > 0.0 && ultima.ram_used_pct < 100.0);
    let activos = s.hosts.iter().filter(|h| h.active).count();
    let con_clientes = series::query(&p.state.db, Rango::Hora).await.unwrap();
    assert!(
        con_clientes
            .iter()
            .any(|m| m.clients == Some(u32::try_from(activos).unwrap())),
        "{con_clientes:?}"
    );
}

#[tokio::test]
async fn poller_registra_corte_wan() {
    let mut p = panel().await;
    let _h = sondeo(&mut p).await;
    assert!(eventos_wan(&p).await.is_empty());

    p.sim.control.set_wan_up(false);
    let start = Instant::now();
    loop {
        let ev = eventos_wan(&p).await;
        if !ev.is_empty() {
            assert_eq!(ev.len(), 1);
            assert_eq!(ev[0].1, None);
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(5), "no hay corte");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let cortes = series::cortes(&p.state.db, Rango::Hora).await.unwrap();
    assert_eq!(cortes.len(), 1);
    assert_eq!(cortes[0].hasta, None);
    let caidas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM samples_raw WHERE wan_up = 0")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert!(caidas >= 1);

    p.sim.control.set_wan_up(true);
    let start = Instant::now();
    loop {
        let ev = eventos_wan(&p).await;
        assert_eq!(ev.len(), 1, "{ev:?}");
        if let Some(hasta) = ev[0].1 {
            assert!(hasta >= ev[0].0);
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "el corte no se cierra"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let cortes = series::cortes(&p.state.db, Rango::Hora).await.unwrap();
    assert_eq!(cortes.len(), 1);
    assert!(cortes[0].hasta.is_some());
}

#[tokio::test]
async fn poller_backoff_si_router_no_responde() {
    let mut p = panel().await;
    let h = sondeo(&mut p).await;
    let antes = h.latest().unwrap();

    p.sim.control.set_unresponsive(true);
    esperar("error visible", || h.latest().unwrap().error.is_some()).await;
    assert!(!h.healthy());
    // Sin espera creciente habría un intento cada 250 ms (200 de tiempo de
    // espera + 50 de intervalo), unos 8 en 2 s; con ella (100, 200, 400,
    // 400… ms entre intentos) no pasan de 5.
    tokio::time::sleep(Duration::from_secs(2)).await;
    let fallos = h.failures();
    assert!((2..=5).contains(&fallos), "fallos seguidos: {fallos}");
    let s = h.latest().unwrap();
    assert!(s.error.is_some());
    // Se conservan los últimos datos y su hora.
    assert_eq!(s.at, antes.at);
    assert_eq!(s.summary, antes.summary);

    p.sim.control.set_unresponsive(false);
    esperar("recuperación", || {
        h.latest().unwrap().error.is_none() && h.failures() == 0
    })
    .await;
    assert!(h.healthy());
}

#[tokio::test]
async fn pausa_espera_al_ciclo_en_curso_y_detiene_el_sondeo() {
    let mut p = panel().await;
    let h = sondeo(&mut p).await;

    // Con el router colgado, el ciclo en curso tarda el tiempo de espera del
    // cliente: `pause` no vuelve hasta que termina.
    p.sim.control.set_unresponsive(true);
    tokio::time::sleep(Duration::from_millis(60)).await;
    let pausa = h.pause().await;
    let fallos = h.failures();
    assert!(fallos >= 1, "pause volvió con un ciclo en curso");

    // En pausa no hay peticiones: los fallos no aumentan.
    tokio::time::sleep(Duration::from_millis(700)).await;
    assert_eq!(h.failures(), fallos);

    p.sim.control.set_unresponsive(false);
    pausa.resume();
    esperar("sondeo reanudado", || {
        h.latest().unwrap().error.is_none() && h.failures() == 0
    })
    .await;
}

#[tokio::test]
async fn fallo_de_lectura_lenta_no_descarta_el_ciclo() {
    let mut p = panel().await;
    let h = sondeo(&mut p).await;
    let antes = h.latest().unwrap();

    p.sim.control.fail_reads("cbGetTr69Config", true);
    esperar("error de las lecturas de 60 s", || h.slow_error().is_some()).await;
    let filas = || async {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM samples_raw")
            .fetch_one(&p.state.db)
            .await
            .unwrap()
    };
    let n = filas().await;
    // Las lecturas de 10 s siguen: hay muestras nuevas (una por segundo como
    // mucho, `ts` es la clave) y el sondeo está sano.
    tokio::time::sleep(Duration::from_millis(2100)).await;
    assert!(filas().await >= n + 2, "no se guardan muestras");
    assert_eq!(h.error(), None);
    assert_eq!(h.failures(), 0);
    assert!(h.healthy());
    let s = h.latest().unwrap();
    assert!(s.slow_error.is_some());
    assert!(s.at > antes.at);
    // Se conservan los datos de 60 s anteriores.
    assert_eq!(s.tr69_activo, Some(true));
    assert_eq!(s.hosts, antes.hosts);
    assert_eq!(s.slow_at, antes.slow_at);

    // Se reintentan en el ciclo siguiente.
    p.sim.control.fail_reads("cbGetTr69Config", false);
    esperar("lecturas de 60 s recuperadas", || {
        h.slow_error().is_none() && h.latest().unwrap().slow_at > antes.slow_at
    })
    .await;
}

#[tokio::test]
async fn sigue_sano_durante_una_pausa() {
    let mut p = panel().await;
    let h = sondeo(&mut p).await;
    let pausa = h.pause().await;
    // Más de 3 intervalos (150 ms) sin datos nuevos.
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(h.healthy());
    pausa.resume();
    assert!(h.healthy(), "deja de estar sano justo al reanudar");
}
