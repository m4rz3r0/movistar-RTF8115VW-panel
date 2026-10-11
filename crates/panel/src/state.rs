//! Estado compartido del servidor.

use crate::{changes, config::Config, crypto::HistoryKey, db, poller::PollerHandle};
use axum::extract::FromRef;
use catalogo::{Catalogo, mask::Mascara};
use hgu::deteccion::RedDetectada;
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
}

/// Estado del servidor, compartido por las rutas Axum y las server functions.
/// Barato de clonar.
#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub db: SqlitePool,
    pub hgu: hgu::Client,
    pub catalogo: Arc<Catalogo>,
    pub mascara: Arc<Mascara>,
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
    /// Idioma por defecto de la instancia (`claves::IDIOMA`), si se ha fijado:
    /// copia en memoria para decidir el idioma de cada página sin ir a la
    /// base de datos. Lo carga [`AppState::new`] y lo cambia
    /// [`AppState::fijar_idioma_por_defecto`].
    idioma_por_defecto: Arc<std::sync::RwLock<Option<Idioma>>>,
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

    /// La conexión a internet que vale: la elegida a mano
    /// (`claves::WAN_MANUAL`) si existe; si no, la detectada.
    pub fn wan(&self) -> Option<String> {
        self.red.wan()
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

    /// Anota el idioma por defecto recién guardado en la base de datos.
    pub fn fijar_idioma_por_defecto(&self, idioma: Idioma) {
        *self
            .idioma_por_defecto
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(idioma);
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
            hgu,
            catalogo: Arc::new(catalogo),
            mascara: Arc::new(mascara),
            leptos,
            login_lock: Arc::new(Mutex::new(())),
            poller: PollerHandle::default(),
            history_key: Arc::new(history_key),
            cambios: changes::Ranura::default(),
            instalacion: Arc::default(),
            idioma_por_defecto: Arc::new(std::sync::RwLock::new(idioma_por_defecto)),
            red,
        })
    }
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos.clone()
    }
}
