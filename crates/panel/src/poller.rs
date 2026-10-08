//! Sondeo compartido del router en segundo plano.
//!
//! Una sola tarea lee el estado cada 10 s (`cbGetSummary`, `cbGetStatistic`,
//! `cbGetWanIPIntf`) y cada 60 s los hosts, concesiones DHCP, WiFi y el
//! estado de TR-069. Guarda la última instantánea para las páginas, una
//! muestra por lectura en `samples_raw`, los cortes de `ip2` en `wan_events`,
//! y compacta las series cada 10 min.
//!
//! Ante errores de las lecturas de 10 s reintenta con espera creciente hasta
//! 60 s; si el router rechaza la contraseña, cada 5 min. Esas muestras no se
//! guardan (se ven como huecos) y el error queda en [`Snapshot::error`]. Si
//! solo fallan las lecturas de 60 s, el ciclo sigue adelante con los datos de
//! 60 s anteriores, el error queda aparte en [`Snapshot::slow_error`] (no
//! afecta a [`PollerHandle::healthy`]) y se reintentan en el ciclo siguiente.
//!
//! Cada ciclo se ejecuta en su propia tarea: si una entra en pánico, se
//! registra, cuenta como un fallo y el sondeo continúa.
//!
//! El gestor de cambios llama a [`PollerHandle::pause`] antes de escribir:
//! espera a que termine el ciclo en curso y, mientras viva la
//! [`PauseGuard`] devuelta, el sondeo no hace peticiones al router.

use crate::{
    series::{self, Sample},
    state::AppState,
};
use hgu::{
    Error,
    models::{DhcpLease, IfaceCounters, LanHost, StaticLease, Summary, WanConn, WifiStatus},
    read,
};
use std::{
    sync::{Arc, Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant},
};
use tokio::sync::watch;

/// Interfaz de la conexión PPP principal.
const WAN: &str = "ip2";

/// Intervalos del sondeo.
#[derive(Clone, Debug)]
pub struct PollerConfig {
    /// Resumen, contadores y WAN (10 s).
    pub fast: Duration,
    /// Hosts, concesiones, WiFi y TR-069 (60 s).
    pub slow: Duration,
    /// Espera máxima entre reintentos tras errores (60 s).
    pub backoff_max: Duration,
    /// Espera tras un rechazo de credenciales (5 min).
    pub auth_retry: Duration,
    /// Compactación de las series (10 min).
    pub compact_every: Duration,
}

impl Default for PollerConfig {
    fn default() -> Self {
        PollerConfig {
            fast: Duration::from_secs(10),
            slow: Duration::from_secs(60),
            backoff_max: Duration::from_secs(60),
            auth_retry: Duration::from_secs(5 * 60),
            compact_every: Duration::from_secs(10 * 60),
        }
    }
}

impl PollerConfig {
    /// Espera hasta el siguiente ciclo tras `failures` errores seguidos, el
    /// último `err`: el intervalo normal sin errores; el doble por cada fallo
    /// (desde 2 × `fast`) hasta `backoff_max`; `auth_retry` si el router
    /// rechazó las credenciales.
    pub fn delay(&self, failures: u32, err: Option<&Error>) -> Duration {
        match err {
            None => self.fast,
            Some(Error::Auth(_)) => self.auth_retry,
            Some(_) => self
                .fast
                .saturating_mul(2u32.saturating_pow(failures.min(16)))
                .min(self.backoff_max),
        }
    }
}

/// Último estado leído del router.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// Hora (servidor, segundos Unix) de la última lectura correcta.
    pub at: i64,
    pub summary: Summary,
    pub wan: Vec<WanConn>,
    pub counters: Vec<IfaceCounters>,
    /// Datos de 60 s: se conservan entre lecturas.
    pub hosts: Vec<LanHost>,
    pub leases: Vec<DhcpLease>,
    pub statics: Vec<StaticLease>,
    /// Radios, SSID (con su WPS) y estaciones.
    pub wifi: WifiStatus,
    /// `enableCWMP` de `cbGetTr69Config`; `None` si no se ha leído.
    pub tr69_activo: Option<bool>,
    /// Hora de la última lectura de 60 s; `None` si aún no la hay.
    pub slow_at: Option<i64>,
    /// Error del último ciclo, si falló (los datos son los anteriores).
    pub error: Option<String>,
    /// Error de las últimas lecturas de 60 s, si fallaron (los datos de 60 s
    /// son los de `slow_at`).
    pub slow_error: Option<String>,
}

#[derive(Default)]
struct Status {
    snapshot: Option<Snapshot>,
    error: Option<String>,
    slow_error: Option<String>,
    failures: u32,
    /// Cuándo terminó la última lectura correcta.
    last_ok: Option<Instant>,
    /// Cuándo se soltó la última pausa: el plazo de `healthy` cuenta desde
    /// aquí si es posterior a `last_ok`.
    resumed: Option<Instant>,
}

struct Shared {
    cfg: OnceLock<PollerConfig>,
    status: Mutex<Status>,
    /// Pausas activas (`PauseGuard` vivas).
    pauses: watch::Sender<u32>,
    /// Lo tiene el sondeo durante cada ciclo.
    cycle: tokio::sync::Mutex<()>,
}

/// Acceso al sondeo. Barato de clonar; vive en `AppState` desde el arranque,
/// aunque el sondeo no se haya iniciado.
#[derive(Clone)]
pub struct PollerHandle {
    shared: Arc<Shared>,
}

impl Default for PollerHandle {
    fn default() -> Self {
        PollerHandle {
            shared: Arc::new(Shared {
                cfg: OnceLock::new(),
                status: Mutex::new(Status::default()),
                pauses: watch::Sender::new(0),
                cycle: tokio::sync::Mutex::new(()),
            }),
        }
    }
}

impl PollerHandle {
    fn status(&self) -> MutexGuard<'_, Status> {
        self.shared.status.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Última instantánea (con el error del último ciclo, si lo hubo);
    /// `None` hasta la primera lectura correcta.
    pub fn latest(&self) -> Option<Snapshot> {
        let st = self.status();
        st.snapshot.clone().map(|mut s| {
            s.error.clone_from(&st.error);
            s.slow_error.clone_from(&st.slow_error);
            s
        })
    }

    /// Error del último ciclo, también antes de tener ningún dato (p. ej. la
    /// contraseña del router no es válida desde el arranque).
    pub fn error(&self) -> Option<String> {
        self.status().error.clone()
    }

    /// Error de las últimas lecturas de 60 s, si fallaron.
    pub fn slow_error(&self) -> Option<String> {
        self.status().slow_error.clone()
    }

    /// Ciclos fallidos seguidos.
    pub fn failures(&self) -> u32 {
        self.status().failures
    }

    /// `true` si hay datos sin error y de hace menos de 3 intervalos. Durante
    /// una pausa no caduca (se mantiene el último estado) y, al reanudar, el
    /// plazo vuelve a contar desde ese momento.
    pub fn healthy(&self) -> bool {
        let Some(cfg) = self.shared.cfg.get() else {
            return false;
        };
        let paused = *self.shared.pauses.borrow() > 0;
        let st = self.status();
        let Some(last_ok) = st.last_ok else {
            return false;
        };
        let since = st.resumed.map_or(last_ok, |r| r.max(last_ok));
        st.error.is_none() && (paused || since.elapsed() < cfg.fast * 3)
    }

    /// Detiene el sondeo: espera a que acabe el ciclo en curso (si lo hay)
    /// y, hasta que se suelte la guarda devuelta (o se llame a
    /// [`PauseGuard::resume`]), no se hace ninguna petición al router. Admite
    /// pausas simultáneas: se reanuda al soltar la última.
    pub async fn pause(&self) -> PauseGuard {
        self.shared.pauses.send_modify(|n| *n += 1);
        let guard = PauseGuard {
            shared: self.shared.clone(),
        };
        // El sondeo comprueba las pausas con el candado del ciclo tomado:
        // conseguirlo aquí garantiza que no queda ningún ciclo en marcha.
        drop(self.shared.cycle.lock().await);
        guard
    }
}

/// Mientras viva, el sondeo está en pausa (ver [`PollerHandle::pause`]).
#[must_use = "el sondeo se reanuda al soltar la guarda"]
pub struct PauseGuard {
    shared: Arc<Shared>,
}

impl PauseGuard {
    /// Reanuda el sondeo (igual que soltar la guarda).
    pub fn resume(self) {}
}

impl Drop for PauseGuard {
    fn drop(&mut self) {
        let mut last = false;
        self.shared.pauses.send_modify(|n| {
            *n -= 1;
            last = *n == 0;
        });
        if last {
            let mut st = self.shared.status.lock().unwrap_or_else(|e| e.into_inner());
            st.resumed = Some(Instant::now());
        }
    }
}

/// Tarea de sondeo.
pub struct Poller {
    state: AppState,
    handle: PollerHandle,
    cfg: PollerConfig,
    /// Última lectura de los contadores de `ip2`: `(ts_ms, rx, tx)`.
    prev: Option<(i64, u64, u64)>,
    last_slow: Option<Instant>,
    last_compact: Option<Instant>,
}

/// Espera hasta el siguiente ciclo.
enum Next {
    /// Lectura correcta: desde el inicio del ciclo, para tener una muestra
    /// cada `fast` aunque la lectura tarde.
    Interval(Duration),
    /// Tras un error: desde el final del intento (que puede haber agotado el
    /// tiempo de espera del cliente).
    Backoff(Duration),
}

/// Datos de una lectura de 10 s correcta.
struct Reading {
    summary: Summary,
    counters: Vec<IfaceCounters>,
    /// Hora (ms) justo después de leer los contadores.
    counters_ms: i64,
    wan: Vec<WanConn>,
    /// Lecturas de 60 s, si tocaban.
    slow: Option<Result<Slow, Error>>,
}

struct Slow {
    hosts: Vec<LanHost>,
    leases: Vec<DhcpLease>,
    statics: Vec<StaticLease>,
    wifi: WifiStatus,
    tr69_activo: Option<bool>,
}

/// Ejecuta `fut` en su propia tarea; si entra en pánico lo registra y
/// devuelve `None`.
async fn supervised<T: Send + 'static>(
    what: &str,
    fut: impl Future<Output = T> + Send + 'static,
) -> Option<T> {
    match tokio::spawn(fut).await {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("panel: error interno en el sondeo ({what}): {e}");
            None
        }
    }
}

impl Poller {
    /// Arranca el sondeo con los intervalos de producción.
    pub fn spawn(state: AppState) -> PollerHandle {
        Self::spawn_with(state, PollerConfig::default())
    }

    /// Arranca el sondeo con `cfg` y devuelve el `PollerHandle` del estado.
    ///
    /// # Panics
    /// Si ya se había arrancado para ese `AppState`.
    pub fn spawn_with(state: AppState, cfg: PollerConfig) -> PollerHandle {
        let handle = state.poller.clone();
        assert!(
            handle.shared.cfg.set(cfg.clone()).is_ok(),
            "el sondeo ya estaba en marcha"
        );
        tokio::spawn(Self::run(state, handle.clone(), cfg));
        handle
    }

    fn new(state: AppState, handle: PollerHandle, cfg: PollerConfig) -> Poller {
        Poller {
            state,
            handle,
            cfg,
            prev: None,
            last_slow: None,
            last_compact: None,
        }
    }

    /// Bucle del sondeo. Cada ciclo (y cada compactación) corre en su propia
    /// tarea con el `Poller` prestado; si entra en pánico, se pierde su
    /// memoria (contadores previos, horas de las lecturas de 60 s) y se sigue
    /// con uno nuevo.
    async fn run(state: AppState, handle: PollerHandle, cfg: PollerConfig) {
        let shared = handle.shared.clone();
        let fresh = || Poller::new(state.clone(), handle.clone(), cfg.clone());
        let mut poller = fresh();
        let mut pauses = shared.pauses.subscribe();
        loop {
            if pauses.wait_for(|n| *n == 0).await.is_err() {
                return;
            }
            let start = tokio::time::Instant::now();
            let next = {
                let _cycle = shared.cycle.lock().await;
                if *shared.pauses.borrow() > 0 {
                    continue;
                }
                let run = supervised("ciclo", async move {
                    let next = poller.cycle().await;
                    (poller, next)
                })
                .await;
                match run {
                    Some((p, next)) => {
                        poller = p;
                        next
                    }
                    None => {
                        poller = fresh();
                        let mut st = handle.status();
                        st.failures = st.failures.saturating_add(1);
                        st.error = Some("error interno del sondeo".into());
                        let err = Error::Unexpected(String::new());
                        Next::Backoff(cfg.delay(st.failures, Some(&err)))
                    }
                }
            };
            match supervised("compactación", async move {
                poller.compact_if_due().await;
                poller
            })
            .await
            {
                Some(p) => poller = p,
                None => {
                    // Sin memoria de la compactación fallida: no repetirla
                    // en bucle.
                    poller = fresh();
                    poller.last_compact = Some(Instant::now());
                }
            }
            match next {
                Next::Interval(d) => tokio::time::sleep_until(start + d).await,
                Next::Backoff(d) => tokio::time::sleep(d).await,
            }
        }
    }

    /// Un ciclo de lectura; devuelve cuándo hacer el siguiente.
    async fn cycle(&mut self) -> Next {
        let now = series::now();
        let slow_due = self.last_slow.is_none_or(|t| t.elapsed() >= self.cfg.slow);
        match self.read(slow_due).await {
            Ok(r) => {
                self.store(now, r).await;
                Next::Interval(self.cfg.delay(0, None))
            }
            Err(e) => {
                self.prev = None;
                let mut st = self.handle.status();
                st.failures = st.failures.saturating_add(1);
                st.error = Some(e.to_string());
                Next::Backoff(self.cfg.delay(st.failures, Some(&e)))
            }
        }
    }

    /// Lecturas de 10 s (un error descarta el ciclo) y, si `slow`, las de
    /// 60 s (su error se devuelve aparte).
    async fn read(&self, slow: bool) -> Result<Reading, Error> {
        let c = &self.state.hgu;
        let summary = read::summary(c).await?;
        let counters = read::counters(c).await?;
        let counters_ms = series::now_ms();
        let wan = read::wan(c).await?;
        let slow = if slow {
            Some(self.read_slow().await)
        } else {
            None
        };
        Ok(Reading {
            summary,
            counters,
            counters_ms,
            wan,
            slow,
        })
    }

    async fn read_slow(&self) -> Result<Slow, Error> {
        let c = &self.state.hgu;
        Ok(Slow {
            hosts: read::lan_hosts(c).await?,
            leases: read::dhcp_leases(c).await?,
            statics: read::static_leases(c).await?,
            wifi: read::wifi(c).await?,
            tr69_activo: c
                .call("cbGetTr69Config", &[])
                .await?
                .get("cwmp_data")
                .and_then(|n| n.text_of("enableCWMP"))
                .map(|v| v == "1"),
        })
    }

    /// Actualiza la instantánea y guarda la muestra y el estado de la WAN.
    async fn store(&mut self, now: i64, r: Reading) {
        let wan_up = r.wan.iter().any(|w| w.name == WAN && w.up);
        let bytes = r
            .counters
            .iter()
            .find(|c| c.name == WAN)
            .map(|c| (r.counters_ms, c.rx_bytes, c.tx_bytes));
        // Tras un hueco (pausa larga) no se promedia sobre él.
        let gap_ms = i64::try_from((self.cfg.fast * 3).as_millis()).unwrap_or(i64::MAX);
        let (rx_bps, tx_bps) = match (self.prev, bytes) {
            (Some((pms, prx, ptx)), Some((ms, rx, tx))) if ms - pms <= gap_ms => (
                series::rate((pms, prx), (ms, rx)),
                series::rate((pms, ptx), (ms, tx)),
            ),
            _ => (None, None),
        };
        self.prev = bytes;

        let sample = {
            let mut st = self.handle.status();
            let mut snap = match st.snapshot.take() {
                Some(mut s) => {
                    s.summary = r.summary;
                    s.wan = r.wan;
                    s.counters = r.counters;
                    s
                }
                None => Snapshot {
                    at: now,
                    summary: r.summary,
                    wan: r.wan,
                    counters: r.counters,
                    hosts: Vec::new(),
                    leases: Vec::new(),
                    statics: Vec::new(),
                    wifi: WifiStatus::default(),
                    tr69_activo: None,
                    slow_at: None,
                    error: None,
                    slow_error: None,
                },
            };
            snap.at = now;
            match r.slow {
                Some(Ok(slow)) => {
                    snap.hosts = slow.hosts;
                    snap.leases = slow.leases;
                    snap.statics = slow.statics;
                    snap.wifi = slow.wifi;
                    snap.tr69_activo = slow.tr69_activo;
                    snap.slow_at = Some(now);
                    st.slow_error = None;
                    self.last_slow = Some(Instant::now());
                }
                // Se conservan los datos anteriores y `last_slow` sigue
                // igual: se reintentan en el ciclo siguiente.
                Some(Err(e)) => st.slow_error = Some(e.to_string()),
                None => {}
            }
            let s = &snap.summary;
            let ram_used_pct = if s.ram_total_kb == 0 {
                0.0
            } else {
                s.ram_total_kb.saturating_sub(s.ram_free_kb) as f32 / s.ram_total_kb as f32 * 100.0
            };
            let sample = Sample {
                ts: now,
                rx_bps,
                tx_bps,
                load1: s.load1,
                ram_used_pct,
                clients: snap.slow_at.map(|_| {
                    u32::try_from(snap.hosts.iter().filter(|h| h.active).count())
                        .unwrap_or(u32::MAX)
                }),
                wan_up,
            };
            st.snapshot = Some(snap);
            st.error = None;
            st.failures = 0;
            st.last_ok = Some(Instant::now());
            sample
        };

        let db = &self.state.db;
        if let Err(e) = series::insert(db, &sample).await {
            eprintln!("panel: no se puede guardar la muestra: {e}");
        }
        if let Err(e) = series::wan_state(db, wan_up, now).await {
            eprintln!("panel: no se puede registrar el estado de la WAN: {e}");
        }
    }

    async fn compact_if_due(&mut self) {
        if self
            .last_compact
            .is_some_and(|t| t.elapsed() < self.cfg.compact_every)
        {
            return;
        }
        self.last_compact = Some(Instant::now());
        if let Err(e) = series::compact(&self.state.db, series::now()).await {
            eprintln!("panel: no se pueden compactar las series: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn espera_creciente_hasta_el_maximo() {
        let cfg = PollerConfig::default();
        let net = Error::Network("x".into());
        assert_eq!(cfg.delay(0, None), Duration::from_secs(10));
        let esperas: Vec<u64> = (1..=5)
            .map(|n| cfg.delay(n, Some(&net)).as_secs())
            .collect();
        assert_eq!(esperas, [20, 40, 60, 60, 60]);
        assert_eq!(cfg.delay(1000, Some(&net)), Duration::from_secs(60));
        assert_eq!(
            cfg.delay(1, Some(&Error::Auth("no"))),
            Duration::from_secs(300)
        );
    }

    #[tokio::test]
    async fn supervised_sobrevive_a_un_panico() {
        assert_eq!(supervised("prueba", async { 7 }).await, Some(7));
        let r: Option<()> = supervised("prueba", async { panic!("a propósito") }).await;
        assert_eq!(r, None);
    }
}
