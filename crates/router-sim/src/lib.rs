//! Router Askey RTF8115VW simulado.
//!
//! Sirve las fixtures saneadas con el mismo protocolo que el router real
//! (login básico y avanzado, cookies, `sessionKey` en las páginas `.asp`) y
//! refleja en memoria cualquier escritura avanzada en su lectura pareada del
//! catálogo (`catalogo`), con listas (crear, editar y borrar), `cbModifyPwd`,
//! los usuarios de su web (`usuarios`: alta, baja, cambio de otro y lista) y
//! el cortafuegos (`cortafuegos`: sus dos páginas y el alta y la baja de
//! reglas) y el registro del sistema (`registro`: exportarlo y servirlo).

mod conexiones;
mod control;
mod copia;
mod cortafuegos;
mod escrituras;
mod estado;
mod fixtures;
mod http;
mod lecturas;
mod ofuscado;
mod registro;
pub mod sanitize;
#[cfg(test)]
mod tests;
mod usuarios;
mod xml;

pub use control::SimControl;
pub use http::PAGE_VARS;

use estado::{Inner, SimState};
use fixtures::{load_fixtures, wan_de_las_fixtures};
use http::{
    RUTA_RETARDO, basic_cgi, basic_login, cgi, fijar_retardo, hang_if_unresponsive, logout, page,
    post_needs_content_length, root,
};

use axum::{
    Router, middleware,
    routing::{get, post},
};
use catalogo::Catalogo;
use std::{
    net::SocketAddr,
    path::{Path as FsPath, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Cuerpo que devuelve el router cuando la sesión no es válida.
const LOGGED_OUT: &str = "<script>top.location.href='te_acceso_router.asp';</script>";
const XML_HEADER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\r\n";
/// Puerto LAN del equipo de `counters_bump`.
const LAN_PORT: &str = "eth0.4";

/// Configuración del simulador.
#[derive(Clone, Debug)]
pub struct SimConfig {
    /// Usuario aceptado en el login.
    pub user: String,
    /// Contraseña aceptada en el login.
    pub password: String,
    /// Directorio con las fixtures `<name>[__<params>].xml`.
    pub fixtures_dir: PathBuf,
    /// Directorio con fixtures que sustituyen a las de `fixtures_dir` (una
    /// variante de línea, p. ej. `fixtures-variantes/dhcp-sin-voip`).
    pub capa: Option<PathBuf>,
    /// Dirección de escucha; `None` = `127.0.0.1` en un puerto libre.
    pub listen: Option<SocketAddr>,
    /// Retardo de cada respuesta, como la latencia del router real (ver
    /// [`SimControl::set_retardo`]); cero por defecto.
    pub retardo: Duration,
}

impl SimConfig {
    /// Configuración que escucha en `127.0.0.1` en un puerto libre.
    pub fn new(user: &str, password: &str, fixtures_dir: impl Into<PathBuf>) -> Self {
        Self {
            user: user.to_owned(),
            password: password.to_owned(),
            fixtures_dir: fixtures_dir.into(),
            capa: None,
            listen: None,
            retardo: Duration::ZERO,
        }
    }

    /// La misma configuración con una capa de fixtures: cada fixture se busca
    /// primero en `dir` y, si no está, en `fixtures_dir`.
    #[must_use]
    pub fn con_capa(mut self, dir: impl Into<PathBuf>) -> SimConfig {
        self.capa = Some(dir.into());
        self
    }
}

/// Simulador en marcha.
pub struct SimHandle {
    /// URL base, p. ej. `http://127.0.0.1:8081`.
    pub base_url: String,
    /// Control del estado del simulador.
    pub control: SimControl,
}

/// Punto de entrada del simulador.
pub struct Sim;

impl Sim {
    /// Carga las fixtures y arranca el servidor en segundo plano.
    ///
    /// # Panics
    /// Si las fixtures no se pueden leer o no se puede abrir el puerto.
    pub async fn start(cfg: SimConfig) -> SimHandle {
        let cargar = |dir: &FsPath| {
            load_fixtures(dir).unwrap_or_else(|e| panic!("fixtures en {}: {e}", dir.display()))
        };
        let mut fixtures = cargar(&cfg.fixtures_dir);
        if let Some(capa) = &cfg.capa {
            fixtures.extend(cargar(capa));
        }
        let wan = wan_de_las_fixtures(&fixtures);
        let usuarios = usuarios::de_partida(&fixtures, &cfg.user, &cfg.password);
        let inner = Arc::new(Inner {
            user: cfg.user,
            usuarios: Mutex::new(usuarios),
            fixtures,
            catalogo: Catalogo::load().expect("el catálogo de funciones carga"),
            wan,
            unresponsive: AtomicBool::new(false),
            epoca: Arc::new(AtomicU64::new(0)),
            retardo_ms: AtomicU64::new(u64::try_from(cfg.retardo.as_millis()).unwrap_or(u64::MAX)),
            state: Mutex::new(SimState {
                seed: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.subsec_nanos()),
                wan_up: true,
                cortafuegos: cortafuegos::de_partida(),
                ..SimState::default()
            }),
        });
        let addr = cfg
            .listen
            .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 0)));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .unwrap_or_else(|e| panic!("no se puede escuchar en {addr}: {e}"));
        let local = listener.local_addr().expect("dirección local");
        let listener = conexiones::Escucha {
            tcp: listener,
            epoca: inner.epoca.clone(),
        };
        let app = Router::new()
            .route("/", get(root))
            .route("/cgi-bin/te_acceso_router.cgi", post(basic_login))
            .route("/cgi-bin/te_logout.cgi", get(logout))
            .route("/cgi-bin/{file}", post(cgi).get(basic_cgi))
            .route(registro::RUTA, get(registro::servir))
            .route("/{page}", get(page))
            .route(RUTA_RETARDO, post(fijar_retardo))
            .layer(middleware::from_fn_with_state(
                inner.clone(),
                hang_if_unresponsive,
            ))
            .layer(middleware::from_fn(post_needs_content_length))
            .with_state(inner.clone());
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("router-sim: {e}");
            }
        });
        SimHandle {
            base_url: format!("http://{local}"),
            control: SimControl { inner },
        }
    }
}
