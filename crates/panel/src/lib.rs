//! Panel web del router Askey RTF8115VW: Leptos (SSR + hidratación) sobre Axum.
//!
//! Con la feature `ssr` se compila el servidor; con `hydrate`, la parte que se
//! ejecuta en el navegador (WASM).

// Los tipos de vista anidados de los gráficos (`ui/charts/franja.rs`) superan
// la profundidad de consultas por defecto (128) al compilar la hidratación en
// release; sin esto `cargo leptos build --release` no compila.
#![recursion_limit = "256"]

pub mod app;
pub mod auth;
pub mod instalar;
pub mod panel_ajustes;
pub mod ui;

#[cfg(feature = "ssr")]
pub mod changes;
#[cfg(feature = "ssr")]
pub mod cli;
#[cfg(feature = "ssr")]
pub mod config;
#[cfg(feature = "ssr")]
pub mod crypto;
#[cfg(feature = "ssr")]
pub mod db;
#[cfg(feature = "ssr")]
pub mod fotos;
#[cfg(feature = "ssr")]
pub mod instancia;
#[cfg(feature = "ssr")]
pub mod poller;
#[cfg(feature = "ssr")]
pub mod security;
#[cfg(feature = "ssr")]
pub mod series;
#[cfg(feature = "ssr")]
pub mod state;

#[cfg(feature = "ssr")]
pub use server::build_router;

#[cfg(feature = "ssr")]
mod server {
    use crate::{
        app::{App, shell},
        auth, security,
        state::AppState,
    };
    use axum::{Json, Router, extract::State, middleware, routing::get};
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use serde_json::{Value, json};

    /// Construye el router Axum completo: rutas Leptos (páginas con sesión
    /// obligatoria salvo `/login`; server functions en `/api/*`, que la
    /// comprueban ellas mismas), `/salud`, ficheros estáticos, freno de CSRF
    /// en `/api/*` y cabeceras de seguridad en todas las respuestas.
    pub fn build_router(state: AppState) -> Router {
        let routes = generate_route_list(App);
        Router::new()
            .leptos_routes(&state, routes, {
                let options = state.leptos.clone();
                move || shell(options.clone())
            })
            // Solo afecta a las rutas ya registradas (las de Leptos): `/salud`
            // y los ficheros estáticos quedan públicos.
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                auth::require_page_session,
            ))
            .route("/salud", get(salud))
            .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                security::csrf,
            ))
            .layer(security::layer())
            .with_state(state)
    }

    /// Comprobación de vida para el actualizador (sin login). No consulta al
    /// router: `router` es `"ok"` si el sondeo tiene datos recientes y sin
    /// error, `"sin_configurar"` si el panel aún no tiene las credenciales
    /// del router (`ok` sigue siendo `true`: el panel funciona).
    async fn salud(State(state): State<AppState>) -> Json<Value> {
        let router = if !state.hgu.configurado() {
            "sin_configurar"
        } else if state.poller.healthy() {
            "ok"
        } else {
            "error"
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
