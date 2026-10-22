//! Catálogo de ajustes del panel.
//!
//! Un **ajuste** es una unidad humana («Red WiFi 5 GHz») con uno o más
//! **campos** («Nombre», «Contraseña», «Visible»). Cada campo apunta a un
//! parámetro de una función de escritura del catálogo de funciones, y el
//! ajuste se lee con una función de lectura. Así un ajuste puede leer y
//! escribir con varias funciones de la API sin que la interfaz las nombre:
//! la interfaz solo ve ids y claves de traducción (`ajuste-<tema>-<nombre>-…`).
//!
//! Sin features, la crate solo trae los tipos que usan las vistas del panel,
//! que también se compilan para el navegador. La feature `catalogo` (servidor)
//! añade la carga de `ajustes/*.toml` (con los lotes de mejoras de
//! `ajustes/lotes.toml`) y `excluidas.toml`, su comprobación
//! contra `funciones.toml`/`secretos.toml` y la traducción de valores entre el
//! formato del router y el legible.

mod legible;
mod modelo;
pub use legible::{ErrorValor, Limites, mac as normalizar_mac, validar_legible};
pub use modelo::{
    Nivel, Requiere, SIN_ELECCION, Tema, TipoCampo, ValorLegible, prefijo_de_ajuste, valor_aceptado,
};

/// Glifos que puede llevar la tarjeta de un elemento (`icono` en el
/// catálogo) o de un lote de mejoras; el panel los dibuja en
/// `ui::comun::Glifo`.
pub const ICONOS: &[&str] = &[
    "globo", "escudo", "llave", "personas", "telefono", "fax", "reloj", "lupa", "puerta", "casa",
    "ojo",
];

/// Columnas que puede declarar la lista de los dispositivos (no es una
/// lista del router: sus datos salen de la lectura de la red de casa).
pub const COLUMNAS_DISPOSITIVOS: &[&str] = &["ip", "mac", "conexion", "ip_fija"];

#[cfg(feature = "catalogo")]
mod carga;
#[cfg(feature = "catalogo")]
mod elementos;
#[cfg(feature = "catalogo")]
mod lotes;
#[cfg(feature = "catalogo")]
mod validar;
#[cfg(feature = "catalogo")]
mod valores;

#[cfg(feature = "catalogo")]
pub use carga::{CatalogoAjustes, ErrorAjustes};
#[cfg(feature = "catalogo")]
pub use elementos::{
    DERIVADOS_DATO, DERIVADOS_ESTADO, MAX_DATOS, PREFIJO_DERIVADO, clave_accion, clave_papel,
    claves_de_elementos, partir_referencia, titulo_accion,
};
#[cfg(feature = "catalogo")]
pub use lotes::{
    Condicion, Destino, Eleccion, Lote, OpcionEleccion, ParaCada, PasoExpandido, PasoLote,
    clave_lote, clave_paso, claves_de_lotes, expandir, lotes_del_campo, pasos_del_lote,
    titulo_lote,
};
#[cfg(feature = "catalogo")]
pub use modelo::{
    Accion, Adicional, Ajuste, Booleano, Campo, Componente, Elemento, Escritura, Excluida, Lectura,
    Opcion, Validar,
};
#[cfg(feature = "catalogo")]
pub use validar::{claves_requeridas, comprobar, prefijo_i18n, riesgo};
#[cfg(feature = "catalogo")]
pub use valores::{a_legible, a_router, falta_requerido};
