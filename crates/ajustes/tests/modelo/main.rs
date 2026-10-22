//! Carga, claves de traducción y reglas de `comprobar` sobre un catálogo de
//! funciones y unos ajustes sintéticos.

use ajustes::{
    CatalogoAjustes, Nivel, Tema, TipoCampo, claves_de_elementos, claves_requeridas, comprobar,
    prefijo_i18n, riesgo,
};
use catalogo::{Catalogo, Riesgo, mask::Mascara};

mod comun;
use comun::*;

mod campos;
mod carga;
mod elementos;
mod sensibles;
mod validar;
