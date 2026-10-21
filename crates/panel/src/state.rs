//! Estado compartido del servidor.

use crate::{
    changes,
    config::Config,
    crypto::HistoryKey,
    db,
    lecturas::{CADUCIDAD_LECTURAS, CacheLecturas},
    poller::PollerHandle,
    ui::charts::Zona,
};
use ajustes::CatalogoAjustes;
use axum::extract::FromRef;
use catalogo::{Catalogo, mask::Mascara};
use hgu::{deteccion::RedDetectada, models::WanConn};
use i18n::Idioma;
use leptos::prelude::LeptosOptions;
use sqlx::SqlitePool;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

/// Red del router en memoria: la última detección y la conexión a internet
/// elegida a mano, si la hay. La comparten [`AppState`] y el gestor de
/// cambios (su red de seguridad WAN). Barata de clonar.
#[derive(Clone, Default)]
pub struct RedActual(Arc<RwLock<(Option<RedDetectada>, Option<String>)>>);

impl RedActual {
    /// La conexión a internet que vale: la elegida a mano si existe; si no,
    /// la detectada. `None` si no hay ninguna (entonces cuenta cualquier
    /// conexión de datos: ver `changes::plan::wan_arriba`).
    pub fn wan(&self) -> Option<String> {
        let g = self.0.read().unwrap_or_else(|e| e.into_inner());
        g.1.clone()
            .or_else(|| g.0.as_ref().and_then(|r| r.wan.clone()))
    }

    /// La conexión a internet que vale entre las que tiene el router
    /// (`conns`): la elegida a mano si sigue existiendo; si no, la detectada.
    /// Así una elegida que ya no existe no deja la red de seguridad mirando
    /// una conexión que nunca estará arriba.
    pub fn wan_en(&self, conns: &[WanConn]) -> Option<String> {
        let g = self.0.read().unwrap_or_else(|e| e.into_inner());
        g.1.clone()
            .filter(|m| conns.iter().any(|c| &c.name == m))
            .or_else(|| g.0.as_ref().and_then(|r| r.wan.clone()))
    }

    /// La conexión a internet elegida a mano, exista o no en el router.
    pub fn manual(&self) -> Option<String> {
        self.0.read().unwrap_or_else(|e| e.into_inner()).1.clone()
    }

    /// La última detección.
    pub fn red(&self) -> Option<RedDetectada> {
        self.0.read().unwrap_or_else(|e| e.into_inner()).0.clone()
    }

    /// Sustituye la detección y la conexión elegida a mano.
    pub fn fijar(&self, red: Option<RedDetectada>, manual: Option<String>) {
        *self.0.write().unwrap_or_else(|e| e.into_inner()) = (red, manual);
    }

    /// Sustituye solo la detección (la elegida a mano no cambia).
    pub fn fijar_detectada(&self, red: RedDetectada) {
        self.0.write().unwrap_or_else(|e| e.into_inner()).0 = Some(red);
    }

    /// Sustituye solo la conexión elegida a mano.
    pub fn fijar_manual(&self, manual: Option<String>) {
        self.0.write().unwrap_or_else(|e| e.into_inner()).1 = manual;
    }
}

/// Espera de cada prueba de acceso al cambiar la contraseña del router (ver
/// [`AppState::espera_prueba_router`]).
pub const ESPERA_PRUEBA_ROUTER: std::time::Duration = std::time::Duration::from_secs(20);

/// Estado del servidor, compartido por las rutas Axum y las server functions.
/// Barato de clonar.
#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub db: SqlitePool,
    pub hgu: hgu::Client,
    /// Caché corta de las lecturas del router para pintar páginas (ver
    /// [`crate::lecturas`]).
    pub lecturas: Arc<CacheLecturas>,
    pub catalogo: Arc<Catalogo>,
    pub mascara: Arc<Mascara>,
    /// Catálogo de ajustes, ya comprobado contra `catalogo` y `mascara`
    /// ([`comprobar_ajustes`]).
    pub ajustes: Arc<CatalogoAjustes>,
    pub leptos: LeptosOptions,
    /// Serializa los intentos de login: el freno de fuerza bruta se comprueba
    /// y se actualiza sin carreras entre peticiones simultáneas.
    pub login_lock: Arc<Mutex<()>>,
    /// Sondeo del router: última instantánea y pausa para escribir. Inactivo
    /// hasta `Poller::spawn`.
    pub poller: PollerHandle,
    /// Clave con la que el historial cifra los valores secretos.
    pub history_key: Arc<HistoryKey>,
    /// Gestor de cambios del servidor (vacío hasta
    /// `ChangeManager::instalar`). Toda escritura al router pasa por él.
    pub cambios: changes::Ranura,
    /// Asistente de instalación: token y fase (ver `instalar`).
    pub instalacion: Arc<crate::instalar::Instalacion>,
    /// Lo más que espera cada prueba de acceso al cambiar la contraseña del
    /// router ([`crate::contrasena_router`]): si vence, cuenta como «no
    /// entra». Por defecto [`ESPERA_PRUEBA_ROUTER`]; las pruebas la acortan.
    pub espera_prueba_router: std::time::Duration,
    /// Idioma por defecto de la instancia (`claves::IDIOMA`), si se ha fijado:
    /// copia en memoria para decidir el idioma de cada página sin ir a la
    /// base de datos. Lo carga [`AppState::new`] y lo cambia
    /// [`AppState::fijar_idioma_por_defecto`].
    idioma_por_defecto: Arc<std::sync::RwLock<Option<Idioma>>>,
    /// Zona horaria de la instancia (`claves::ZONA_HORARIA`; sin guardar, la
    /// peninsular): copia en memoria para pintar las fechas de cada página.
    /// La carga [`AppState::new`] y la cambia [`AppState::fijar_zona`].
    zona: Arc<std::sync::RwLock<Zona>>,
    /// Red del router (`claves::RED_DETECTADA` y `claves::WAN_MANUAL`):
    /// copia en memoria cargada por [`AppState::new`].
    red: RedActual,
}

impl AppState {
    /// Gestor de cambios del servidor, si ya se instaló.
    pub fn cambios(&self) -> Option<Arc<changes::ChangeManager>> {
        self.cambios.get().cloned()
    }

    /// Idioma por defecto de la instancia, si se ha fijado.
    pub fn idioma_por_defecto(&self) -> Option<Idioma> {
        *self
            .idioma_por_defecto
            .read()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// Zona horaria de la instancia.
    pub fn zona(&self) -> Zona {
        *self.zona.read().unwrap_or_else(|e| e.into_inner())
    }

    /// Anota la zona horaria recién guardada en la base de datos.
    pub fn fijar_zona(&self, zona: Zona) {
        *self.zona.write().unwrap_or_else(|e| e.into_inner()) = zona;
    }

    /// La conexión a internet que vale: la elegida a mano
    /// (`claves::WAN_MANUAL`) si existe; si no, la detectada.
    pub fn wan(&self) -> Option<String> {
        self.red.wan()
    }

    /// La conexión a internet que vale entre las del router (ver
    /// [`RedActual::wan_en`]).
    pub fn wan_en(&self, conns: &[WanConn]) -> Option<String> {
        self.red.wan_en(conns)
    }

    /// La conexión a internet elegida a mano (`claves::WAN_MANUAL`), exista
    /// o no en el router.
    pub fn wan_manual(&self) -> Option<String> {
        self.red.manual()
    }

    /// La última detección de la red del router.
    pub fn red(&self) -> Option<RedDetectada> {
        self.red.red()
    }

    /// Anota en memoria la detección y la conexión elegida a mano (no las
    /// guarda en la base de datos: ver `instancia::detectar_y_guardar`).
    pub fn fijar_red(&self, r: Option<RedDetectada>, manual: Option<String>) {
        self.red.fijar(r, manual);
    }

    /// La red compartida con el gestor de cambios.
    pub(crate) fn red_actual(&self) -> RedActual {
        self.red.clone()
    }

    /// Anota el idioma por defecto recién guardado en la base de datos
    /// (`None`: el del navegador).
    pub fn fijar_idioma_por_defecto(&self, idioma: Option<Idioma>) {
        *self
            .idioma_por_defecto
            .write()
            .unwrap_or_else(|e| e.into_inner()) = idioma;
    }
}

impl AppState {
    /// Carga el catálogo, lee la clave maestra (la crea si no existe) y abre
    /// la base de datos. El cliente del router empieza sin configurar: lo
    /// configura `instancia::arrancar_router`. Cualquier fallo impide
    /// arrancar; el mensaje nunca incluye secretos.
    pub async fn new(cfg: Config, leptos: LeptosOptions) -> Result<AppState, String> {
        tokio::task::spawn_blocking(crate::auth::warm_up)
            .await
            .map_err(|e| e.to_string())?;
        let catalogo = Catalogo::load().map_err(|e| e.to_string())?;
        let mascara = Mascara::load().map_err(|e| e.to_string())?;
        let ajustes = comprobar_ajustes(
            CatalogoAjustes::load().map_err(|e| e.to_string())?,
            &catalogo,
            &mascara,
        )?;
        let history_key = HistoryKey::load_or_create(&cfg.history_key_file)?;
        let hgu = hgu::Client::sin_configurar();
        let db = db::open(&cfg.db_path).await.map_err(|e| {
            format!(
                "no se puede abrir la base de datos {}: {e}",
                cfg.db_path.display()
            )
        })?;
        let idioma_por_defecto = crate::instancia::leer(&db, crate::instancia::claves::IDIOMA)
            .await
            .and_then(|v| serde_json::from_value(v).ok());
        crate::instancia::migrar_zona_utc_por_defecto(&db).await?;
        let zona = crate::instancia::leer(&db, crate::instancia::claves::ZONA_HORARIA)
            .await
            .and_then(|v| v.as_str().and_then(Zona::desde_texto))
            .unwrap_or_default();
        let red = RedActual::default();
        red.fijar(
            crate::instancia::leer(&db, crate::instancia::claves::RED_DETECTADA)
                .await
                .and_then(|v| serde_json::from_value(v).ok()),
            crate::instancia::leer(&db, crate::instancia::claves::WAN_MANUAL)
                .await
                .and_then(|v| v.as_str().map(str::to_owned))
                .filter(|w| !w.is_empty()),
        );
        Ok(AppState {
            cfg: Arc::new(cfg),
            db,
            lecturas: Arc::new(CacheLecturas::new(CADUCIDAD_LECTURAS)),
            hgu,
            catalogo: Arc::new(catalogo),
            mascara: Arc::new(mascara),
            ajustes: Arc::new(ajustes),
            leptos,
            login_lock: Arc::new(Mutex::new(())),
            poller: PollerHandle::default(),
            history_key: Arc::new(history_key),
            cambios: changes::Ranura::default(),
            instalacion: Arc::default(),
            espera_prueba_router: ESPERA_PRUEBA_ROUTER,
            idioma_por_defecto: Arc::new(std::sync::RwLock::new(idioma_por_defecto)),
            zona: Arc::new(std::sync::RwLock::new(zona)),
            red,
        })
    }
}

/// El catálogo de ajustes `c` si cuadra con el de funciones y la máscara de
/// secretos ([`ajustes::comprobar`]). Si no, registra cada fallo y devuelve
/// el error con todos: un catálogo roto es un fallo del proyecto, no de
/// quien instala, y el panel no arranca con él.
pub fn comprobar_ajustes(
    c: CatalogoAjustes,
    f: &Catalogo,
    m: &Mascara,
) -> Result<CatalogoAjustes, String> {
    let fallos = ajustes::comprobar(&c, f, m);
    if fallos.is_empty() {
        return Ok(c);
    }
    for fallo in &fallos {
        tracing::error!("catálogo de ajustes: {fallo}");
    }
    Err(format!(
        "el catálogo de ajustes no cuadra con el de funciones: {}",
        fallos.join("; ")
    ))
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos.clone()
    }
}
