//! Lotes de mejoras (`lotes.toml`): varios ajustes de un clic con un solo deshacer.
//!
//! Los pasos se declaran una vez, en `[[paso]]`, y cada `[[lote]]` los lista
//! por id: un paso compartido por dos lotes no puede divergir y cuenta una
//! vez. Cada paso apunta a uno (y solo uno) de estos destinos:
//! - `accion`: una acción existente de su tema (su campo y su valor);
//! - `campo` y `valor`: `<ajuste>.<campo>` de cualquier pestaña o, con
//!   `para_cada = "red_wifi"`, el id del campo en cada red WiFi;
//! - `ajuste` y `elegir`: los campos que da la opción elegida de una
//!   `[[eleccion]]` (un proveedor de DNS, por ejemplo).
//!
//! Admite además `si = { campo, valor }` (solo aplica si se cumple) y
//! `opcional = true` (sale desmarcado en la revisión).
//!
//! ```toml
//! [[paso]]
//! id = "wps"
//! tema = "wifi"
//! accion = "quitar_wps"
//! para_cada = "red_wifi"
//!
//! [[lote]]
//! id = "seguridad"
//! icono = "escudo"
//! pasos = ["wps"]
//! avisos = ["tr069"]
//! ```

use serde::Deserialize;
use std::collections::BTreeMap;

use crate::modelo::Tema;

mod comprobar;
mod expandir;

pub(crate) use comprobar::comprobar_lotes;
pub use expandir::{claves_de_lotes, expandir, lotes_del_campo, pasos_del_lote};

/// Un lote: un botón que aplica varios pasos con un solo deshacer.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lote {
    /// Id: da `mejoras-lote-<id>`, su `-desc` y su `-titulo`.
    pub id: String,
    /// Glifo de su tarjeta; ver [`ICONOS`](crate::ICONOS).
    pub icono: String,
    /// Ids de sus pasos, en el orden de la revisión.
    pub pasos: Vec<String>,
    /// Avisos comunes (`tr069` → `mejoras-aviso-tr069`).
    #[serde(default)]
    pub avisos: Vec<String>,
    /// Consejos sin botón (`wpa3` → `mejoras-consejo-wpa3`).
    #[serde(default)]
    pub consejos: Vec<String>,
}

/// Un paso de uno o varios lotes: da `mejoras-paso-<id>` y su `-desc`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasoLote {
    pub id: String,
    pub tema: Tema,
    /// Id de una acción de su tema.
    #[serde(default)]
    pub accion: Option<String>,
    /// `<ajuste>.<campo>`, o el id del campo con `para_cada`.
    #[serde(default)]
    pub campo: Option<String>,
    /// Valor legible que se escribe en `campo`.
    #[serde(default)]
    pub valor: Option<String>,
    /// Ajuste cuyos campos da la opción elegida de `elegir`.
    #[serde(default)]
    pub ajuste: Option<String>,
    /// El paso se repite en cada elemento de este tipo.
    #[serde(default)]
    pub para_cada: Option<ParaCada>,
    /// Solo aplica si se cumple; si no, «no aplica».
    #[serde(default)]
    pub si: Option<Condicion>,
    /// Sale desmarcado en la revisión.
    #[serde(default)]
    pub opcional: bool,
    /// Id de la `[[eleccion]]` que da los valores.
    #[serde(default)]
    pub elegir: Option<String>,
}

/// En qué elementos se repite un paso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParaCada {
    /// Cada red WiFi de casa o de invitados (papel `principal` o
    /// `invitados`), no las radios.
    RedWifi,
}

/// Condición de un paso: `<ajuste>.<campo>` tiene el valor legible `valor`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condicion {
    pub campo: String,
    pub valor: String,
}

/// Una elección entre varias opciones que dan valor a los campos de un
/// ajuste (el proveedor de DNS).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eleccion {
    pub id: String,
    #[serde(rename = "opcion")]
    pub opciones: Vec<OpcionEleccion>,
}

/// Opción de una elección: el valor legible de cada campo del ajuste.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpcionEleccion {
    pub id: String,
    /// La que sale elegida (una por elección).
    #[serde(default)]
    pub defecto: bool,
    pub valores: BTreeMap<String, String>,
}

/// Un campo que cambia un paso y el valor legible que le pone (`None`: lo
/// da la opción elegida).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Destino {
    pub ajuste: String,
    pub campo: String,
    pub valor: Option<String>,
}

/// Un paso ya expandido: uno por red con `para_cada`, uno solo si no.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasoExpandido {
    /// Id del paso.
    pub paso: String,
    pub tema: Tema,
    /// Id del elemento (`red-24`) con `para_cada`.
    pub elemento: Option<String>,
    pub destinos: Vec<Destino>,
    pub si: Option<Condicion>,
    pub opcional: bool,
    pub elegir: Option<String>,
}

impl PasoExpandido {
    /// Clave única dentro de un lote: `wps@red-24`, o el id del paso
    /// (`upnp`) si no se repite por elemento.
    pub fn clave(&self) -> String {
        match &self.elemento {
            Some(e) => format!("{}@{e}", self.paso),
            None => self.paso.clone(),
        }
    }
}

/// Clave de traducción del nombre de un lote (su `-desc` es la
/// recomendación).
pub fn clave_lote(id: &str) -> String {
    format!("mejoras-lote-{id}")
}

/// Clave de traducción del título del grupo de cambios de un lote (en el
/// Historial).
pub fn titulo_lote(id: &str) -> String {
    format!("mejoras-lote-{id}-titulo")
}

/// Clave de traducción del nombre de un paso (su `-desc` es la
/// recomendación).
pub fn clave_paso(id: &str) -> String {
    format!("mejoras-paso-{id}")
}
