//! Carga de `ajustes/*.toml` y `excluidas.toml`.
//!
//! Los ficheros se compilan dentro del binario. Cargar solo comprueba el
//! formato; las reglas contra el catálogo de funciones están en
//! [`crate::comprobar`].

use serde::Deserialize;
use thiserror::Error;

use crate::modelo::{Ajuste, Excluida, Tema};

/// Registra los ficheros de `ajustes/`: un tema nuevo se añade solo aquí.
macro_rules! ficheros {
    ($($nombre:literal),* $(,)?) => {
        /// `(nombre, texto)` de cada `ajustes/<nombre>.toml`.
        const FICHEROS: &[(&str, &str)] =
            &[$(($nombre, include_str!(concat!("../ajustes/", $nombre, ".toml")))),*];
    };
}

ficheros!("internet", "wifi", "red_local", "puertos");

/// Texto de `excluidas.toml`.
const EXCLUIDAS: &str = include_str!("../excluidas.toml");

/// Errores al cargar los ficheros del catálogo de ajustes.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErrorAjustes {
    /// Un fichero no se puede interpretar.
    #[error("{fichero}.toml no válido: {mensaje}")]
    Toml { fichero: String, mensaje: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FicheroAjustes {
    #[serde(default)]
    ajuste: Vec<Ajuste>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FicheroExcluidas {
    #[serde(default)]
    excluida: Vec<Excluida>,
}

/// Catálogo de ajustes cargado. Antes de usarlo, el panel lo pasa por
/// [`crate::comprobar`].
#[derive(Debug, Clone)]
pub struct CatalogoAjustes {
    ajustes: Vec<Ajuste>,
    excluidas: Vec<Excluida>,
}

impl CatalogoAjustes {
    /// Carga los ficheros compilados en el binario.
    pub fn load() -> Result<CatalogoAjustes, ErrorAjustes> {
        Self::from_toml(FICHEROS, EXCLUIDAS)
    }

    /// Carga desde texto: `(nombre, texto)` de cada fichero de ajustes y el
    /// texto de `excluidas.toml`.
    pub fn from_toml(
        ficheros: &[(&str, &str)],
        excluidas: &str,
    ) -> Result<CatalogoAjustes, ErrorAjustes> {
        let error = |fichero: &str, e: toml::de::Error| ErrorAjustes::Toml {
            fichero: fichero.to_owned(),
            mensaje: e.to_string(),
        };
        let mut ajustes = Vec::new();
        for (nombre, texto) in ficheros {
            let f: FicheroAjustes = toml::from_str(texto).map_err(|e| error(nombre, e))?;
            ajustes.extend(f.ajuste);
        }
        let ex: FicheroExcluidas = toml::from_str(excluidas).map_err(|e| error("excluidas", e))?;
        Ok(CatalogoAjustes {
            ajustes,
            excluidas: ex.excluida,
        })
    }

    /// Todos los ajustes, en el orden de los ficheros.
    pub fn todos(&self) -> &[Ajuste] {
        &self.ajustes
    }

    /// Ajustes de un tema, en el orden de los ficheros.
    pub fn por_tema(&self, tema: Tema) -> impl Iterator<Item = &Ajuste> {
        self.ajustes.iter().filter(move |a| a.tema == tema)
    }

    /// Ajuste por id.
    pub fn ajuste(&self, id: &str) -> Option<&Ajuste> {
        self.ajustes.iter().find(|a| a.id == id)
    }

    /// Funciones de escritura que ningún ajuste cubre, con su motivo.
    pub fn excluidas(&self) -> &[Excluida] {
        &self.excluidas
    }
}
