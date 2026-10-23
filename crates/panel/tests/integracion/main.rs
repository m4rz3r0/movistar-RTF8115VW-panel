//! Pruebas de integración del panel (servidor, `ssr`) contra `router-sim`.
//!
//! Un solo binario con un módulo por área: se enlaza una vez, no una por
//! fichero. Para un área, el filtro de su módulo: `cargo test --test
//! integracion ajustes::` o `cargo nextest run -E 'test(/^ajustes::/)'`.

mod comun;

mod acciones;
mod ajustes;
mod ajustes_catalogo;
mod ajustes_foto;
mod ajustes_ida_y_vuelta;
mod ajustes_listas;
mod ajustes_secretos;
mod auth;
mod binario_e2e;
mod changes;
mod contrasena_router;
mod contrasena_router_plazo;
mod contraste;
mod cortafuegos;
mod estado;
mod estaticos;
mod explorador;
mod grupos;
mod historial;
mod historial_idioma;
mod http;
mod idioma;
mod instalar;
mod instancia;
mod lecturas;
mod lista;
mod mejoras;
mod menu;
mod pagina_tema;
mod pagina_tema_lotes;
mod pagina_temas;
mod panel_ajustes;
mod poller;
mod pool_por_id;
mod red;
mod registro_y_dns;
mod sensibles;
mod tema;
mod usuarios_router;
mod vista;
