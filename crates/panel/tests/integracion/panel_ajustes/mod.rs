//! Pruebas de «Ajustes del panel» (`/panel`) y «Configuración del router»
//! (`/router/conexion`): credenciales del router, parámetros de la instancia
//! (también los que usarán las recomendaciones futuras), usuarios y
//! ventana de reautenticación.

use crate::comun::{
    PANEL_PASS, PANEL_USER, PASS, Panel, PanelPrueba, USER, body_text, esperar_hasta,
    get_con_cookie, llamar, login_body, ok, post_form, post_json, reautenticar, send, sesion,
    set_cookie,
};
use panel::instancia::{self, claves};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

mod conexion;
mod preferencias;
mod usuarios;
mod wan;

const NUEVA: &str = "nueva-clave-del-router";

fn router(p: &Panel, contrasena: &str) -> Value {
    json!({ "url": p.sim.base_url, "usuario": USER, "contrasena": contrasena })
}

async fn credenciales(p: &Panel) -> instancia::CredencialesRouter {
    instancia::leer_credenciales(&p.state.db, &p.state.history_key)
        .await
        .unwrap()
        .unwrap()
}

fn campos() -> Value {
    json!({
        "idioma": { "Fijo": "Es" },
        "zona_horaria": "UTC+01:00",
        "dns_preferidos": [],
        "dispositivos_confianza": [],
        "mantenimiento": null,
        "servidor_propio": null,
    })
}

// ---- Ronda de correcciones 1

/// Sondeo rápido sobre un cliente del router rápido (la página se rehace
/// con él).
fn sondeo_rapido(p: &mut Panel) {
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, USER, PASS);
    cfg.timeout = Duration::from_millis(300);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    p.app = panel::build_router(p.state.clone());
    panel::poller::Poller::spawn_with(
        p.state.clone(),
        panel::poller::PollerConfig {
            fast: Duration::from_millis(50),
            slow: Duration::from_millis(200),
            backoff_max: Duration::from_millis(200),
            ..panel::poller::PollerConfig::default()
        },
    );
}
