//! Estado compartido del servidor.

use crate::{
    config::{self, Config},
    crypto::HistoryKey,
    db,
    poller::PollerHandle,
};
use axum::extract::FromRef;
use catalogo::{Catalogo, mask::Mascara};
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
}

impl AppState {
    /// Carga el catálogo, lee la contraseña del router y la clave del
    /// historial (la crea si no existe) y abre la base de datos. Cualquier fallo impide arrancar; el mensaje nunca incluye
    /// secretos.
    pub async fn new(cfg: Config, leptos: LeptosOptions) -> Result<AppState, String> {
        tokio::task::spawn_blocking(crate::auth::warm_up)
            .await
            .map_err(|e| e.to_string())?;
        let catalogo = Catalogo::load().map_err(|e| e.to_string())?;
        let mascara = Mascara::load().map_err(|e| e.to_string())?;
        let password = config::read_router_secret(&cfg.router_secret_file)?;
        let history_key = HistoryKey::load_or_create(&cfg.history_key_file)?;
        let hgu = hgu::Client::new(hgu::ClientConfig::new(
            &cfg.router_url,
            &cfg.router_user,
            &password,
        ));
        let db = db::open(&cfg.db_path).await.map_err(|e| {
            format!(
                "no se puede abrir la base de datos {}: {e}",
                cfg.db_path.display()
            )
        })?;
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
        })
    }
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos.clone()
    }
}
