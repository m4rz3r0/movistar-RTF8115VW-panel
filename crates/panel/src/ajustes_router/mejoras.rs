//! Mejoras: el estado de los lotes ([`leer_mejoras`]), el contador de la barra y la preparación de un lote como un solo cambio.
//!
//! Igual que el resto de Ajustes, los tipos solo llevan ids, claves de
//! traducción y textos ya traducidos: ningún nombre de la API.

use super::GrupoVista;
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};

/// La página Mejoras entera: los lotes con el estado de cada paso, el
/// contador de la barra y los proveedores de DNS del selector.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MejorasVista {
    pub lotes: Vec<LoteVista>,
    /// Pasos pendientes sin repetir de todos los lotes; `None` si no se
    /// pudo leer nada.
    pub pendientes: Option<u32>,
    /// El panel solo deja consultar: sin «Revisar y aplicar».
    pub solo_lectura: bool,
    pub proveedores: Vec<ProveedorVista>,
}

/// Un lote con sus pasos y su estado.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoteVista {
    /// Id del lote: da `mejoras-lote-<id>` y su `-desc`.
    pub id: String,
    /// Glifo de su tarjeta.
    pub icono: String,
    pub estado: EstadoLote,
    pub pasos: Vec<PasoLoteVista>,
    /// Claves de traducción de los avisos (`mejoras-aviso-tr069`).
    pub avisos: Vec<String>,
    /// Claves de traducción de los consejos sin botón.
    pub consejos: Vec<String>,
}

/// Estado de un lote. Los pasos opcionales, los que no aplican y los que no
/// se pudieron leer no cuentan en `total`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoLote {
    Aplicado,
    Pendientes {
        pendientes: u32,
        total: u32,
    },
    /// No se pudo leer ningún paso: el motivo, traducido.
    SinLeer {
        motivo: String,
    },
}

/// Un paso de un lote, ya expandido (uno por red WiFi en `wps@red-24`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasoLoteVista {
    /// `wps@red-24`, o el id del paso.
    pub clave: String,
    /// Clave de traducción del paso (`mejoras-paso-<id>`).
    pub texto: String,
    /// Clave del título del ajuste del elemento (`ajuste-wifi-red_24-titulo`)
    /// en un paso repetido por elemento.
    pub detalle: Option<String>,
    pub estado: EstadoPaso,
    /// Clave del texto que sustituye al del estado («DNS propios» en el paso
    /// de los DNS hecho con unos que no son de la lista).
    #[serde(default)]
    pub nota: Option<String>,
    /// Id del proveedor de DNS que nombra la nota (`$proveedor`, con
    /// `mejoras-dns-<id>`), si la nota lo lleva.
    #[serde(default)]
    pub nota_proveedor: Option<String>,
    /// Sale desmarcado en la revisión y no cuenta como pendiente.
    pub opcional: bool,
}

/// Estado de un paso.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoPaso {
    Hecho,
    Pendiente,
    /// No se pudo leer: el motivo, traducido y sin nombres de la API.
    SinLeer {
        motivo: String,
    },
    /// Su condición no se cumple (la red de invitados está apagada).
    NoAplica,
}

/// Id de la opción del selector de DNS que no cambia los DNS de casa
/// («Mantener los actuales»): sale la primera, y marcada si el paso está
/// hecho con DNS propios.
pub const MANTENER_DNS: &str = "mantener";

/// Un proveedor de DNS del selector de la revisión: da `mejoras-dns-<id>` y
/// su `-desc`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProveedorVista {
    pub id: String,
    /// DNS principal y secundario.
    pub servidores: [String; 2],
    /// El que sale elegido.
    pub defecto: bool,
}

/// Lo que se elige al revisar un lote. Los pasos desmarcados van como
/// índices al confirmar ([`confirmar_ajuste`](super::confirmar_ajuste)).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpcionesLote {
    /// Id del proveedor de DNS, o [`MANTENER_DNS`]; sin él, el que ya está
    /// puesto (con DNS propios, se mantienen) o el de por defecto.
    #[serde(default)]
    pub proveedor_dns: Option<String>,
}

/// La página Mejoras: lee (de la caché corta si está) las pestañas que tocan
/// los lotes y da el estado de cada paso y de cada lote, y el contador. Una
/// lectura que falla deja sus pasos «sin leer» con su motivo. Exige sesión;
/// la respuesta va con `no-store`.
///
/// Con `fresca` («Volver a leer»), nada sale de la caché.
#[server(endpoint = "leer_mejoras", input = Json)]
pub async fn leer_mejoras(#[server(default)] fresca: bool) -> Result<MejorasVista, ServerFnError> {
    super::servidor::leer_mejoras(fresca).await
}

/// El contador de la barra lateral: los pasos pendientes sin repetir de
/// todos los lotes, o `None` si no se pudo leer nada. Exige sesión.
#[server(endpoint = "contar_mejoras", input = Json)]
pub async fn contar_mejoras() -> Result<Option<u32>, ServerFnError> {
    super::servidor::contar_mejoras().await
}

/// Prepara el lote `id` como un único grupo del gestor, con un paso por
/// cada paso del lote que se pudo leer y aplica, agrupados por pestaña. Se
/// confirma con [`confirmar_ajuste`](super::confirmar_ajuste) y se deshace
/// entero. Errores con el mensaje traducido: 403 en solo lectura, 404 si
/// el lote no existe, 400 si el proveedor no existe y 409 si ya está
/// aplicado u otro cambio está en curso. Exige sesión.
#[server(endpoint = "preparar_lote", input = Json)]
pub async fn preparar_lote(
    id: String,
    opciones: OpcionesLote,
) -> Result<GrupoVista, ServerFnError> {
    super::servidor::preparar_lote(id, opciones).await
}
