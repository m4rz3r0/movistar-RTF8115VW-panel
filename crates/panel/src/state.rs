//! Estado compartido del servidor.

use crate::{changes, config::Config, crypto::HistoryKey, db, poller::PollerHandle};
use axum::extract::FromRef;
use catalogo::{Catalogo, mask::Mascara};
use i18n::Idioma;
use leptos::prelude::LeptosOptions;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

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
        })
    }
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos.clone()
    }
}
