//! Arnés común de las pruebas HTTP: panel en proceso contra `router-sim`.

// Cada binario de pruebas usa solo parte del arnés.
#![allow(dead_code, unused_imports)]

mod cambios;
mod esperar;
mod formularios;
mod http;
mod panel;
mod registro;
mod sesion;

pub use cambios::*;
pub use esperar::*;
pub use formularios::*;
pub use http::*;
pub use panel::*;
pub use registro::*;
pub use sesion::*;

pub const USER: &str = "1234";
pub const PASS: &str = "clave-de-prueba";
/// Origen público configurado en el panel de pruebas.
pub const ORIGIN: &str = "http://localhost:3000";
/// Usuario y contraseña del panel que crean las pruebas.
pub const PANEL_USER: &str = "admin";
pub const PANEL_PASS: &str = "una contraseña larga";
