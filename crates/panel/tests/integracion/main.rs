//! Pruebas de integración del panel (servidor, `ssr`) contra `router-sim`.
//!
//! Un solo binario con un módulo por área: se enlaza una vez, no una por
//! fichero. Para un área, el filtro de su módulo: `cargo test --test
//! integracion ajustes::` o `cargo nextest run -E 'test(/^ajustes::/)'`.

mod comun;

mod ajustes;
mod ajustes_catalogo;
mod ajustes_ida_y_vuelta;
mod ajustes_listas;
mod ajustes_secretos;
mod auth;
mod changes;
mod contrasena_router;
mod estado;
mod estaticos;
mod explorador;
mod grupos;
mod historial;
mod http;
mod idioma;
mod instalar;
mod instancia;
mod lecturas;
mod menu;
mod panel_ajustes;
mod poller;
mod red;
mod vista;
