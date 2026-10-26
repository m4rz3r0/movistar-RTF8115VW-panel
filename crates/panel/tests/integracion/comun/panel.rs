//! Panel de prueba en proceso contra `router-sim`, con la base de datos y la
//! clave en un directorio temporal.

use super::{PASS, USER, sesion, tiempos_rapidos};
use leptos::prelude::LeptosOptions;
use panel::changes::{ChangeManager, Tiempos};
use panel::poller::{Poller, PollerConfig, PollerHandle};
use panel::{
    config::Config,
    instancia::{self, CredencialesRouter},
    redes,
    state::AppState,
};
use router_sim::{Sim, SimConfig, SimHandle};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tempfile::TempDir;

/// Cambio de la configuración de serie de las pruebas.
type Ajustar<'a> = Box<dyn FnOnce(&mut Config) + 'a>;

pub struct Panel {
    pub app: axum::Router,
    pub sim: SimHandle,
    pub state: AppState,
    _dir: TempDir,
}

/// Cómo arrancar un panel de prueba. Por defecto: el proxy de confianza
/// `127.0.0.1`, las fixtures de serie, las credenciales del simulador
/// guardadas (como las dejaría el asistente) y el catálogo de ajustes real.
///
/// ```ignore
/// let p = PanelPrueba::nuevo().arrancar().await;
/// let (p, cookie) = PanelPrueba::nuevo().capa(&variante("ip5")).arrancar_con_gestor().await;
/// ```
pub struct PanelPrueba<'a> {
    sim: SimConfig,
    con_router: bool,
    ajustar: Option<Ajustar<'a>>,
    ajustes: Option<String>,
    tiempos: Option<Tiempos>,
}

impl<'a> PanelPrueba<'a> {
    pub fn nuevo() -> Self {
        Self {
            sim: sim_config(),
            con_router: true,
            ajustar: None,
            ajustes: None,
            tiempos: None,
        }
    }

    /// Los proxies de confianza `proxies` (como `RP_TRUSTED_PROXY`).
    pub fn proxy(self, proxies: &str) -> Self {
        let proxies = redes::lista(proxies).unwrap();
        self.config(move |c| c.trusted_proxies = proxies)
    }

    /// El simulador con la capa de fixtures `capa` (una variante de línea,
    /// ver [`variante`]).
    pub fn capa(mut self, capa: &Path) -> Self {
        self.sim = self.sim.con_capa(capa);
        self
    }

    /// Panel recién instalado: sin variables del router y con la base de
    /// datos vacía (el simulador arranca, pero el panel no lo conoce).
    pub fn sin_router(mut self) -> Self {
        self.con_router = false;
        self
    }

    /// La configuración de serie de las pruebas, cambiada por `ajustar`.
    pub fn config(mut self, ajustar: impl FnOnce(&mut Config) + 'a) -> Self {
        assert!(self.ajustar.is_none(), "una sola configuración");
        self.ajustar = Some(Box::new(ajustar));
        self
    }

    /// El catálogo de ajustes sustituido por el del TOML `toml` (un fichero
    /// de ajustes; sin excluidas ni comprobación contra el catálogo de
    /// funciones, que pide cubrir todas las escrituras).
    pub fn ajustes(mut self, toml: &str) -> Self {
        self.ajustes = Some(toml.to_owned());
        self
    }

    /// Los tiempos del gestor de cambios (por defecto,
    /// [`tiempos_rapidos`]); solo con [`Self::arrancar_con_gestor`].
    pub fn tiempos(mut self, t: Tiempos) -> Self {
        self.tiempos = Some(t);
        self
    }

    /// Arranca el panel, sin gestor de cambios ni sesión.
    pub async fn arrancar(self) -> Panel {
        assert!(
            self.tiempos.is_none(),
            "los tiempos son del gestor: arrancar_con_gestor"
        );
        self.arrancar_sin_gestor().await
    }

    /// Arranca el panel con un cliente del router rápido y el gestor de
    /// cambios instalado (por defecto, con vigilancia de 300 ms), y devuelve
    /// también la cookie de una sesión.
    pub async fn arrancar_con_gestor(mut self) -> (Panel, String) {
        let t = self.tiempos.take().unwrap_or_else(tiempos_rapidos);
        let mut p = self.arrancar_sin_gestor().await;
        let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, USER, PASS);
        cfg.timeout = Duration::from_millis(500);
        cfg.min_gap = p.state.cfg.router_min_gap;
        p.state.hgu = hgu::Client::new(cfg);
        // Un solo cliente del router para las server functions y el gestor.
        p.app = panel::build_router(p.state.clone());
        let m = ChangeManager::instalar(&p.state, t);
        m.reanudar_pendientes().await;
        let cookie = sesion(&p).await;
        (p, cookie)
    }

    async fn arrancar_sin_gestor(self) -> Panel {
        let sim = Sim::start(self.sim).await;
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = config_en(dir.path());
        if let Some(ajustar) = self.ajustar {
            ajustar(&mut cfg);
        }
        let state = estado(cfg, dir.path()).await;
        instancia::arrancar_router(&state).await.unwrap();
        if self.con_router {
            // Las credenciales del simulador, como las dejaría el asistente.
            instancia::guardar_credenciales(
                &state.db,
                &state.history_key,
                &CredencialesRouter {
                    url: sim.base_url.clone(),
                    usuario: USER.into(),
                    contrasena: PASS.into(),
                },
            )
            .await
            .unwrap();
            instancia::arrancar_router(&state).await.unwrap();
        }
        let mut p = Panel {
            app: panel::build_router(state.clone()),
            sim,
            state,
            _dir: dir,
        };
        if let Some(toml) = self.ajustes {
            let c = ajustes::CatalogoAjustes::from_toml(&[("prueba", &toml)], "").unwrap();
            p.state.ajustes = std::sync::Arc::new(c);
            p.app = panel::build_router(p.state.clone());
        }
        p
    }
}

/// Directorio de la variante de fixtures `nombre` (`fixtures-variantes/`).
pub fn variante(nombre: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../hgu/fixtures-variantes")
        .join(nombre)
}

/// Configuración del simulador con las fixtures de serie.
pub fn sim_config() -> SimConfig {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../hgu/fixtures");
    SimConfig::new(USER, PASS, fixtures)
}

/// Configuración del panel con la base de datos y la clave en `dir`, sin
/// variables del router, con el proxy de confianza `127.0.0.1`, 1 ms entre
/// peticiones al simulador (en vez de 150 ms) y 200 ms de espera a la
/// detección de la red (en vez de 5 s).
pub fn config_en(dir: &Path) -> Config {
    Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        db_path: dir.join("panel.db"),
        router_url: None,
        router_user: None,
        router_secret_file: None,
        history_key_file: dir.join("history.key"),
        trusted_proxies: redes::lista("127.0.0.1").unwrap(),
        public_origins: vec![super::ORIGIN.into()],
        install_nets: Vec::new(),
        router_min_gap: Duration::from_millis(1),
        espera_deteccion: Duration::from_millis(200),
        // Contra router-sim se prueba todo, también lo no comprobado en el
        // router real; las pruebas de lo que hace la 1.0 lo apagan.
        habilitar_todo: true,
    }
}

/// Estado del servidor con `cfg` y los ficheros estáticos en `dir/site`
/// (sin arrancar el router).
pub async fn estado(cfg: Config, dir: &Path) -> AppState {
    let site = dir.join("site");
    std::fs::create_dir_all(site.join("pkg")).unwrap();
    std::fs::write(site.join("pkg/panel.css"), "main { padding: 0 }\n").unwrap();
    let leptos = LeptosOptions::builder()
        .output_name("panel")
        .site_root(site.to_str().unwrap())
        .build();
    AppState::new(cfg, leptos).await.unwrap()
}

/// Arranca el sondeo con intervalos cortos y un cliente del router de tiempo
/// de espera corto.
pub fn arrancar_sondeo(p: &mut Panel) -> PollerHandle {
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, USER, PASS);
    cfg.timeout = Duration::from_millis(200);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(50),
            slow: Duration::from_millis(200),
            backoff_max: Duration::from_millis(200),
            ..PollerConfig::default()
        },
    )
}
