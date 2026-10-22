//! Formato de los elementos y las acciones de una pestaña (`[[elemento]]` y `[[accion]]`).

use super::Tema;
use serde::Deserialize;

/// Un elemento de una pestaña: lo que la persona ve como una cosa (una
/// red WiFi, una radio) aunque junte varios ajustes. Las referencias a
/// campos son `<ajuste>.<campo>` (`wifi.red_24.activa`) y las de datos
/// calculados, `derivado:<nombre>`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Elemento {
    /// Id en la URL (`red-24`): minúsculas, cifras y `-`.
    pub id: String,
    pub tema: Tema,
    /// Ids de sus ajustes; cada ajuste va en un solo elemento.
    pub ajustes: Vec<String>,
    /// Papel (`principal`, `invitados`, `radio`): da `ajustes-papel-<papel>`.
    #[serde(default)]
    pub papel: Option<String>,
    /// Campo booleano escribible que lo enciende y lo apaga.
    #[serde(default)]
    pub interruptor: Option<String>,
    /// Hasta seis datos de la tarjeta: campos o derivados.
    #[serde(default)]
    pub datos: Vec<String>,
    /// Regla de estado (`derivado:estado_red`).
    #[serde(default)]
    pub estado: Option<String>,
    /// Glifo de su tarjeta (`reloj`) en vez de los arcos de emisión de
    /// la WiFi; ver [`ICONOS`](crate::ICONOS).
    #[serde(default)]
    pub icono: Option<String>,
}

/// Una acción que se aplica a varios elementos de una pestaña a la vez
/// (`quitar_wps`): pone el campo `campo` de cada uno a `valor` (legible).
/// En una lista (`lista`), cada instancia es un elemento: pone su campo
/// o, con `borrar`, la borra (sin `campo` ni `valor`, y con `peligro`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accion {
    /// Id (`quitar_wps`): da `ajustes-accion-<id>`.
    pub id: String,
    pub tema: Tema,
    /// Id del campo, en el ajuste de cada elemento que lo tenga.
    #[serde(default)]
    pub campo: Option<String>,
    /// Valor legible que se escribe (`false`, la clave de una opción…).
    #[serde(default)]
    pub valor: Option<String>,
    /// Lleva el botón de peligro y confirmación.
    #[serde(default)]
    pub peligro: bool,
    /// Id del ajuste de tipo lista cuyas instancias son sus elementos.
    #[serde(default)]
    pub lista: Option<String>,
    /// Borra las instancias marcadas de `lista`.
    #[serde(default)]
    pub borrar: bool,
}
