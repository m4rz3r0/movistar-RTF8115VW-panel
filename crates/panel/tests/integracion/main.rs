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
mod barra_temas;
mod binario_e2e;
mod changes;
mod cola_qos;
mod conexion;
mod contrasena_router;
mod contrasena_router_plazo;
mod contraste;
mod cortafuegos;
mod dmz;
mod estado;
mod estaticos;
mod explorador;
mod fuentes;
mod grupos;
mod historial;
mod historial_idioma;
mod hora_local;
mod http;
mod idioma;
mod instalar;
mod instancia;
mod lecturas;
mod lista;
mod lista_blanca;
mod mejoras;
mod menu;
mod nat_alg;
mod opciones_dhcp;
mod pagina_tema;
mod pagina_tema_lotes;
mod pagina_temas;
mod panel_ajustes;
mod poller;
mod pool_casillas;
mod pool_por_id;
mod pool_reservadas;
mod qos_dependientes;
mod red;
mod registro_router;
mod registro_y_dns;
mod seleccion;
mod sensibles;
mod tema;
mod token_usuario;
mod usuarios_router;
mod usuarios_router_deshacer;
mod usuarios_router_guardas;
mod vista;
mod voz;
mod wifi_avanzado;
