//! Panel web del router Askey RTF8115VW: Leptos (SSR + hidratación) sobre Axum.
//!
//! Con la feature `ssr` se compila el servidor; con `hydrate`, la parte que se
//! ejecuta en el navegador (WASM).

pub mod app;

#[cfg(feature = "ssr")]
pub mod config;
#[cfg(feature = "ssr")]
pub mod db;
#[cfg(feature = "ssr")]
pub mod security;
#[cfg(feature = "ssr")]
pub mod state;

#[cfg(feature = "ssr")]
pub use server::build_router;

#[cfg(feature = "ssr")]
mod server {
    use crate::{
        app::{App, shell},
        security,
        state::AppState,
    };
    use axum::{Json, Router, extract::State, routing::get};
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use serde_json::{Value, json};
    use std::time::Duration;

    /// Tiempo máximo que `/salud` espera al router.
    const SALUD_TIMEOUT: Duration = Duration::from_secs(3);

    /// Construye el router Axum completo: `/salud`, rutas Leptos, ficheros
    /// estáticos y cabeceras de seguridad en todas las respuestas.
    pub fn build_router(state: AppState) -> Router {
        let routes = generate_route_list(App);
        Router::new()
            .route("/salud", get(salud))
            .leptos_routes(&state, routes, {
                let options = state.leptos.clone();
                move || shell(options.clone())
            })
            .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
            .layer(security::layer())
            .with_state(state)
    }

    /// Comprobación de vida para el actualizador (sin login).
    async fn salud(State(state): State<AppState>) -> Json<Value> {
        let router = match tokio::time::timeout(SALUD_TIMEOUT, hgu::read::summary(&state.hgu)).await
        {
            Ok(Ok(_)) => "ok",
            _ => "error",
        };
        Json(json!({
            "ok": true,
            "version": env!("CARGO_PKG_VERSION"),
            "router": router,
        }))
    }
}

/// Punto de entrada del WASM: hidrata el HTML que generó el servidor.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
