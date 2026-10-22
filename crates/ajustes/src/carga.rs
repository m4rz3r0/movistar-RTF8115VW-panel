//! Carga de `ajustes/*.toml`, `ajustes/lotes.toml` y `excluidas.toml`.
//!
//! Los ficheros se compilan dentro del binario. Cargar solo comprueba el
//! formato; las reglas contra el catálogo de funciones están en
//! [`crate::comprobar`].

use serde::Deserialize;
use thiserror::Error;

use crate::lotes::{Eleccion, Lote, PasoLote};
use crate::modelo::{Accion, Ajuste, Elemento, Excluida, Tema};

/// Registra los ficheros de `ajustes/`: un tema nuevo se añade solo aquí.
macro_rules! ficheros {
    ($($nombre:literal),* $(,)?) => {
        /// `(nombre, texto)` de cada `ajustes/<nombre>.toml`.
        const FICHEROS: &[(&str, &str)] =
            &[$(($nombre, include_str!(concat!("../ajustes/", $nombre, ".toml")))),*];
    };
}

ficheros!(
    "internet",
    "wifi",
    "red_local",
    "puertos",
    "seguridad",
    "telefono",
    "sistema"
);

/// Texto de `excluidas.toml`.
const EXCLUIDAS: &str = include_str!("../excluidas.toml");

/// Texto de `ajustes/lotes.toml`.
const LOTES: &str = include_str!("../ajustes/lotes.toml");

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
    #[serde(default)]
    elemento: Vec<Elemento>,
    #[serde(default)]
    accion: Vec<Accion>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FicheroLotes {
    #[serde(default)]
    lote: Vec<Lote>,
    #[serde(default)]
    paso: Vec<PasoLote>,
    #[serde(default)]
    eleccion: Vec<Eleccion>,
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
    elementos: Vec<Elemento>,
    acciones: Vec<Accion>,
    excluidas: Vec<Excluida>,
    lotes: Vec<Lote>,
    pasos: Vec<PasoLote>,
    elecciones: Vec<Eleccion>,
}

impl CatalogoAjustes {
    /// Carga los ficheros compilados en el binario, lotes incluidos.
    pub fn load() -> Result<CatalogoAjustes, ErrorAjustes> {
        Self::from_toml(FICHEROS, EXCLUIDAS)?.con_lotes(LOTES)
    }

    /// Carga desde texto: `(nombre, texto)` de cada fichero de ajustes y el
    /// texto de `excluidas.toml`. Queda sin lotes (ver [`Self::con_lotes`]).
    pub fn from_toml(
        ficheros: &[(&str, &str)],
        excluidas: &str,
    ) -> Result<CatalogoAjustes, ErrorAjustes> {
        let error = |fichero: &str, e: toml::de::Error| ErrorAjustes::Toml {
            fichero: fichero.to_owned(),
            mensaje: e.to_string(),
        };
        let (mut ajustes, mut elementos, mut acciones) = (Vec::new(), Vec::new(), Vec::new());
        for (nombre, texto) in ficheros {
            let f: FicheroAjustes = toml::from_str(texto).map_err(|e| error(nombre, e))?;
            ajustes.extend(f.ajuste);
            elementos.extend(f.elemento);
            acciones.extend(f.accion);
        }
        let ex: FicheroExcluidas = toml::from_str(excluidas).map_err(|e| error("excluidas", e))?;
        Ok(CatalogoAjustes {
            ajustes,
            elementos,
            acciones,
            excluidas: ex.excluida,
            lotes: Vec::new(),
            pasos: Vec::new(),
            elecciones: Vec::new(),
        })
    }

    /// El catálogo con los lotes, pasos y elecciones del texto de
    /// `lotes.toml` (en lugar de los que tuviera).
    pub fn con_lotes(mut self, texto: &str) -> Result<CatalogoAjustes, ErrorAjustes> {
        let f: FicheroLotes = toml::from_str(texto).map_err(|e| ErrorAjustes::Toml {
            fichero: "lotes".to_owned(),
            mensaje: e.to_string(),
        })?;
        self.lotes = f.lote;
        self.pasos = f.paso;
        self.elecciones = f.eleccion;
        Ok(self)
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

    /// Todos los elementos, en el orden de los ficheros.
    pub fn elementos(&self) -> &[Elemento] {
        &self.elementos
    }

    /// Elementos de un tema, en el orden de los ficheros.
    pub fn elementos_de(&self, tema: Tema) -> impl Iterator<Item = &Elemento> {
        self.elementos.iter().filter(move |e| e.tema == tema)
    }

    /// Todas las acciones, en el orden de los ficheros.
    pub fn acciones(&self) -> &[Accion] {
        &self.acciones
    }

    /// Acciones de un tema, en el orden de los ficheros.
    pub fn acciones_de(&self, tema: Tema) -> impl Iterator<Item = &Accion> {
        self.acciones.iter().filter(move |a| a.tema == tema)
    }

    /// Funciones de escritura que ningún ajuste cubre, con su motivo.
    pub fn excluidas(&self) -> &[Excluida] {
        &self.excluidas
    }

    /// Lotes, en el orden del fichero.
    pub fn lotes(&self) -> &[Lote] {
        &self.lotes
    }

    /// Lote por id.
    pub fn lote(&self, id: &str) -> Option<&Lote> {
        self.lotes.iter().find(|l| l.id == id)
    }

    /// Pasos de los lotes, en el orden del fichero.
    pub fn pasos(&self) -> &[PasoLote] {
        &self.pasos
    }

    /// Paso de un lote por id.
    pub fn paso(&self, id: &str) -> Option<&PasoLote> {
        self.pasos.iter().find(|p| p.id == id)
    }

    /// Elecciones de los pasos, en el orden del fichero.
    pub fn elecciones(&self) -> &[Eleccion] {
        &self.elecciones
    }

    /// Elección por id.
    pub fn eleccion(&self, id: &str) -> Option<&Eleccion> {
        self.elecciones.iter().find(|e| e.id == id)
    }
}
