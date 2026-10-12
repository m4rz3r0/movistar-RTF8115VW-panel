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
//! añade la carga de `ajustes/*.toml` y `excluidas.toml`, su comprobación
//! contra `funciones.toml`/`secretos.toml` y la traducción de valores entre el
//! formato del router y el legible.

mod modelo;
pub use modelo::{Nivel, Tema, TipoCampo, ValorLegible};

#[cfg(feature = "catalogo")]
mod carga;
#[cfg(feature = "catalogo")]
mod validar;
#[cfg(feature = "catalogo")]
mod valores;

#[cfg(feature = "catalogo")]
pub use carga::{CatalogoAjustes, ErrorAjustes};
#[cfg(feature = "catalogo")]
pub use modelo::{
    Ajuste, Booleano, Campo, Componente, Escritura, Excluida, Lectura, Opcion, Validar,
};
#[cfg(feature = "catalogo")]
pub use validar::{claves_requeridas, comprobar, prefijo_i18n, riesgo};
#[cfg(feature = "catalogo")]
pub use valores::{ErrorValor, a_legible, a_router};
