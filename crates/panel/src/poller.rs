//! Sondeo compartido del router en segundo plano.
//!
//! Una sola tarea lee el estado cada 10 s (`cbGetSummary`, `cbGetStatistic`,
//! `cbGetWanIPIntf`) y cada 60 s los hosts, concesiones DHCP, WiFi y el
//! estado de TR-069. Guarda la última instantánea para las páginas, una
//! muestra por lectura en `samples_raw`, los cortes de la conexión a internet
//! ([`AppState::wan`]) en `wan_events`, y compacta las series cada 10 min.
//!
//! Vuelve a detectar la red del router ([`instancia::detectar_y_guardar`])
//! cada [`DETECCION_CADA`] o en cuanto la conexión a internet deja de
//! aparecer en `cbGetWanIPIntf`, salvo con un cambio aplicándose o
//! vigilándose (la red de seguridad del gestor no cambia de conexión a
//! mitad). Una conexión elegida a mano que ya no existe no cuenta: vale la
//! detectada ([`AppState::wan_en`]) y no se vuelve a detectar por ella.
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
//! Mientras el router no esté configurado (sin credenciales), no se hace
//! ninguna petición: el estado es «sin configurar»
//! ([`PollerHandle::sin_configurar`]), no cuenta como fallo y se vuelve a
//! mirar cada 10 s.
//!
//! El gestor de cambios llama a [`PollerHandle::pause`] antes de escribir:
//! espera a que termine el ciclo en curso y, mientras viva la
//! [`PauseGuard`] devuelta, el sondeo no hace peticiones al router.

use crate::{
    changes::plan::wan_arriba,
    instancia,
    series::{self, Sample},
    state::AppState,
};
use hgu::{
    Error,
    models::{DhcpLease, IfaceCounters, LanHost, StaticLease, Summary, WanConn, WifiStatus},
    read,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant},
};
use tokio::sync::watch;

/// Cada cuánto se vuelve a detectar la red del router.
pub const DETECCION_CADA: Duration = Duration::from_secs(3600);

/// Lectura de los contadores del lado LAN: `(ts_ms, nombre → (rx, tx))` de
/// los puertos físicos y los puntos de acceso WiFi.
type LanBytes = (i64, BTreeMap<String, (u64, u64)>);

/// Contadores del lado LAN de una lectura de `cbGetStatistic`; `None` si no
/// hay ninguna interfaz de ese tipo.
fn lan_bytes(ms: i64, counters: &[IfaceCounters]) -> Option<LanBytes> {
    let ifaces: BTreeMap<_, _> = counters
        .iter()
        .filter(|c| c.kind.is_lan_side())
        .map(|c| (c.name.clone(), (c.rx_bytes, c.tx_bytes)))
        .collect();
    (!ifaces.is_empty()).then_some((ms, ifaces))
}

/// Bajada y subida (bit/s) de los dispositivos de la red entre dos lecturas.
///
/// El router reenvía el tráfico de internet por hardware y no lo cuenta en
/// `ip2`, así que se mide en los puertos LAN y los puntos de acceso: lo que
/// el router **envía** a los dispositivos (Σ tx) es la bajada y lo que
/// **recibe** de ellos (Σ rx), la subida. Incluye el tráfico entre
/// dispositivos de puertos distintos del router.
///
/// Si alguna interfaz ha reiniciado su contador (ha bajado) o el conjunto de
/// interfaces ha cambiado, la muestra entera queda sin tasa: sumar sólo las
/// demás daría un valor más bajo que parecería real; un hueco no engaña.
fn lan_rates(prev: &LanBytes, cur: &LanBytes) -> (Option<f64>, Option<f64>) {
    if prev.1.len() != cur.1.len() {
        return (None, None);
    }
    let (mut rx, mut tx) = (0u64, 0u64);
    for (name, &(crx, ctx)) in &cur.1 {
        let Some(&(prx, ptx)) = prev.1.get(name) else {
            return (None, None);
        };
        let (Some(drx), Some(dtx)) = (crx.checked_sub(prx), ctx.checked_sub(ptx)) else {
            return (None, None);
        };
        rx = rx.saturating_add(drx);
        tx = tx.saturating_add(dtx);
    }
    (
        series::rate((prev.0, 0), (cur.0, tx)),
        series::rate((prev.0, 0), (cur.0, rx)),
    )
}

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
    /// Conexión a internet con la que se leyó ([`AppState::wan`]); `None` si
    /// está sin detectar.
    pub wan_intf: Option<String>,
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
    /// El router no está configurado: no se ha intentado leer.
    sin_configurar: bool,
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
    /// Adelanta el siguiente ciclo ([`PollerHandle::despertar`]).
    despertar: tokio::sync::Notify,
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
                despertar: tokio::sync::Notify::new(),
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

    /// `true` si el último ciclo no leyó porque el router no está configurado.
    pub fn sin_configurar(&self) -> bool {
        self.status().sin_configurar
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

    /// Adelanta el siguiente ciclo: el sondeo deja de esperar y lee ya (p.
    /// ej. nada más configurar el router). Si llega durante un ciclo, el
    /// siguiente empieza sin esperar.
    pub fn despertar(&self) {
        self.shared.despertar.notify_one();
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
    /// Última lectura de los contadores del lado LAN.
    prev: Option<LanBytes>,
    last_slow: Option<Instant>,
    last_compact: Option<Instant>,
    /// Firmware con el que se fijó el modo solo lectura por última vez.
    firmware: Option<String>,
    /// Último intento de detectar la red desde el sondeo.
    deteccion: Option<Instant>,
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
            tracing::error!("error interno en el sondeo ({what}): {e}");
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
            firmware: None,
            deteccion: None,
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
            let espera = match next {
                Next::Interval(d) => start + d,
                Next::Backoff(d) => tokio::time::Instant::now() + d,
            };
            tokio::select! {
                () = tokio::time::sleep_until(espera) => {}
                () = shared.despertar.notified() => {}
            }
        }
    }

    /// Un ciclo de lectura; devuelve cuándo hacer el siguiente.
    async fn cycle(&mut self) -> Next {
        let now = series::now();
        let slow_due = self.last_slow.is_none_or(|t| t.elapsed() >= self.cfg.slow);
        let leido = if self.state.hgu.configurado() {
            self.read(slow_due).await
        } else {
            Err(Error::NoConfigurado)
        };
        match leido {
            Ok(r) => {
                self.store(now, r).await;
                Next::Interval(self.cfg.delay(0, None))
            }
            // Sin credenciales no hay nada que leer ni que contar como fallo;
            // los datos de otro router (si los había) ya no valen.
            Err(Error::NoConfigurado) => {
                self.prev = None;
                self.last_slow = None;
                let mut st = self.handle.status();
                *st = Status {
                    sin_configurar: true,
                    ..Status::default()
                };
                Next::Interval(self.cfg.delay(0, None))
            }
            Err(e) => {
                self.prev = None;
                let mut st = self.handle.status();
                st.sin_configurar = false;
                st.failures = st.failures.saturating_add(1);
                // Una vez por racha de fallos, con la cadena del error (sin
                // secretos: ver `hgu::Error`).
                if st.failures == 1 {
                    tracing::warn!("el sondeo no puede leer el router: {e}");
                }
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
        let radios = self
            .state
            .red()
            .map(|r| r.radios)
            .filter(|r| !r.is_empty())
            .unwrap_or_else(|| read::RADIOS_POR_DEFECTO.map(String::from).to_vec());
        Ok(Slow {
            hosts: read::lan_hosts(c).await?,
            leases: read::dhcp_leases(c).await?,
            statics: read::static_leases(c).await?,
            wifi: read::wifi(c, &radios).await?,
            tr69_activo: c
                .call("cbGetTr69Config", &[])
                .await?
                .get("cwmp_data")
                .and_then(|n| n.text_of("enableCWMP"))
                .map(|v| v == "1"),
        })
    }

    /// Actualiza la instantánea y guarda la muestra y el estado de la WAN.
    /// El modo solo lectura sigue al firmware que se lee: se fija con la
    /// primera lectura y cada vez que cambia (actualización del router), con
    /// la misma regla que el asistente ([`instancia::fijar_modo_firmware`]).
    async fn seguir_firmware(&mut self, firmware: &str) {
        if firmware.is_empty() || self.firmware.as_deref() == Some(firmware) {
            return;
        }
        match instancia::fijar_modo_firmware(&self.state.db, firmware).await {
            Ok(probado) => {
                if self.firmware.is_some() || !probado {
                    tracing::info!(
                        "firmware del router: {firmware}{}",
                        if probado {
                            " (probado)"
                        } else {
                            " (sin probar)"
                        }
                    );
                }
                self.firmware = Some(firmware.to_owned());
            }
            Err(e) => tracing::error!("no se puede fijar el modo según el firmware: {e}"),
        }
    }

    /// Vuelve a detectar la red si toca (ver el comentario del módulo). Tras
    /// un intento, el siguiente espera al menos un intervalo de 60 s.
    async fn seguir_red(&mut self, now: i64, wan: &[WanConn]) {
        let cada = i64::try_from(DETECCION_CADA.as_secs()).unwrap_or(i64::MAX);
        let vieja = self
            .state
            .red()
            .is_none_or(|r| now.saturating_sub(r.momento) >= cada);
        // La que vale entre las del router: una elegida a mano que ya no
        // existe no cuenta (volver a detectar no la traería).
        let falta = self
            .state
            .wan_en(wan)
            .is_some_and(|n| !wan.iter().any(|w| w.name == n));
        let reciente = self.deteccion.is_some_and(|t| t.elapsed() < self.cfg.slow);
        let cambio_en_curso = self.state.cambios().is_some_and(|m| m.ocupado());
        if !(vieja || falta) || reciente || cambio_en_curso {
            return;
        }
        self.deteccion = Some(Instant::now());
        if let Err(e) = instancia::detectar_y_guardar(&self.state).await {
            tracing::warn!("{e}");
        }
    }

    async fn store(&mut self, now: i64, r: Reading) {
        self.seguir_firmware(&r.summary.firmware).await;
        self.seguir_red(now, &r.wan).await;
        let wan_intf = self.state.wan_en(&r.wan);
        let wan_up = wan_arriba(&r.wan, wan_intf.as_deref());
        let bytes = lan_bytes(r.counters_ms, &r.counters);
        // Tras un hueco (pausa larga) no se promedia sobre él.
        let gap_ms = i64::try_from((self.cfg.fast * 3).as_millis()).unwrap_or(i64::MAX);
        let (rx_bps, tx_bps) = match (&self.prev, &bytes) {
            (Some(prev), Some(cur)) if cur.0 - prev.0 <= gap_ms => lan_rates(prev, cur),
            _ => (None, None),
        };
        self.prev = bytes;

        let sample = {
            let mut st = self.handle.status();
            let mut snap = match st.snapshot.take() {
                Some(mut s) => {
                    s.summary = r.summary;
                    s.wan = r.wan;
                    s.wan_intf = wan_intf;
                    s.counters = r.counters;
                    s
                }
                None => Snapshot {
                    at: now,
                    summary: r.summary,
                    wan: r.wan,
                    wan_intf,
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
            if st.failures > 0 {
                tracing::info!(
                    "el sondeo vuelve a leer el router tras {} fallos",
                    st.failures
                );
            }
            st.failures = 0;
            st.sin_configurar = false;
            st.last_ok = Some(Instant::now());
            sample
        };

        let db = &self.state.db;
        if let Err(e) = series::insert(db, &sample).await {
            tracing::error!("no se puede guardar la muestra: {e}");
        }
        if let Err(e) = series::wan_state(db, wan_up, now).await {
            tracing::error!("no se puede registrar el estado de la WAN: {e}");
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
            tracing::error!("no se pueden compactar las series: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hgu::models::IfaceKind;

    fn iface(name: &str, kind: IfaceKind, rx: u64, tx: u64) -> IfaceCounters {
        IfaceCounters {
            name: name.into(),
            kind,
            rx_bytes: rx,
            tx_bytes: tx,
            rx_err: 0,
            tx_err: 0,
            rx_drop: 0,
            tx_drop: 0,
        }
    }

    /// Lectura con los contadores de una medida en un router real: (ip2,
    /// eth0.1, eth0.4, ra0) como (rx, tx).
    fn lectura(ms: i64, v: [(u64, u64); 4]) -> LanBytes {
        let c = [
            iface("ip2", IfaceKind::IpIntf, v[0].0, v[0].1),
            iface("eth0.1", IfaceKind::LanPort, v[1].0, v[1].1),
            iface("eth0.4", IfaceKind::LanPort, v[2].0, v[2].1),
            iface("ra0", IfaceKind::WifiAp, v[3].0, v[3].1),
        ];
        lan_bytes(ms, &c).unwrap()
    }

    #[test]
    fn la_bajada_sale_de_los_puertos_lan_no_de_ip2() {
        // Descarga de 1 GiB por un puerto LAN (eth0.4) durante 40 s: ip2 rx
        // apenas se mueve (+51 973 B, el resto va reenviado por hardware) y
        // eth0.4 tx sube 1 146 955 629 B; eth0.1 rx +1,9 MB.
        let antes = lectura(
            0,
            [
                (1_000, 2_000),
                (30_960_130_983, 22_812_082_599),
                (2_394_882_504, 30_028_210_591),
                (21_331, 59_075),
            ],
        );
        let despues = lectura(
            40_000,
            [
                (1_000 + 51_973, 2_000 + 40_000),
                (30_960_130_983 + 1_900_000, 22_812_082_599),
                (2_394_882_504 + 20_000_000, 30_028_210_591 + 1_146_955_629),
                (21_331, 59_075),
            ],
        );
        let (bajada, subida) = lan_rates(&antes, &despues);
        let bajada = bajada.unwrap();
        let esperada = 1_146_955_629.0 * 8.0 / 40.0;
        assert!((bajada - esperada).abs() / esperada < 0.001, "{bajada}");
        assert!(
            bajada > 220e6,
            "≈ 229 Mbit/s, no los ~10 kbit/s de ip2: {bajada}"
        );
        let subida = subida.unwrap();
        let esperada = (1_900_000.0 + 20_000_000.0) * 8.0 / 40.0;
        assert!((subida - esperada).abs() / esperada < 0.001, "{subida}");
    }

    #[test]
    fn reinicio_de_una_interfaz_descarta_la_muestra() {
        let antes = lectura(0, [(0, 0), (1_000, 1_000), (5_000, 5_000), (10, 10)]);
        // eth0.4 vuelve a cero (rx) y las demás suben: ni negativa ni enorme.
        let despues = lectura(10_000, [(0, 0), (2_000, 2_000), (100, 9_000), (20, 20)]);
        assert_eq!(lan_rates(&antes, &despues), (None, None));
    }

    #[test]
    fn cambio_de_interfaces_descarta_la_muestra() {
        let antes = lectura(0, [(0, 0), (1_000, 1_000), (5_000, 5_000), (10, 10)]);
        let mut despues = lectura(10_000, [(0, 0), (2_000, 2_000), (6_000, 6_000), (20, 20)]);
        despues.1.remove("ra0");
        assert_eq!(lan_rates(&antes, &despues), (None, None));
        assert_eq!(lan_rates(&despues, &antes), (None, None));
    }

    #[test]
    fn sin_interfaces_lan_no_hay_lectura() {
        assert!(lan_bytes(0, &[iface("ip2", IfaceKind::IpIntf, 1, 1)]).is_none());
    }

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
