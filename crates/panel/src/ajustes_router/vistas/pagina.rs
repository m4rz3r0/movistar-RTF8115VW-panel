//! La página de un tema tal como la pinta el navegador: resumen y elementos.

use super::{AjusteVista, EstadoLectura};
use crate::ui::comun::EstadoVisual;
use ajustes::ValorLegible;
use serde::{Deserialize, Serialize};

/// Todo lo que pinta la página de un tema, de una sola lectura: la franja
/// de resumen, los elementos (cada uno con el detalle completo de sus
/// ajustes) y las acciones sobre varios elementos.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginaTema {
    /// Identificador corto del tema (`wifi`).
    pub tema: String,
    /// El panel está en modo solo lectura: se ve todo, no se cambia nada.
    pub solo_lectura: bool,
    /// Hay foto de partida: la cabecera enlaza a su comparación.
    pub foto_de_partida: bool,
    /// Franja de resumen (vacía en los temas que aún no la tienen).
    pub resumen: Vec<DatoResumen>,
    /// En el orden del catálogo. Un ajuste que no está en ningún elemento
    /// declarado es un elemento por sí solo.
    pub elementos: Vec<ElementoVista>,
    /// Acciones sobre varios elementos del tema.
    pub acciones: Vec<AccionVista>,
}

/// Un dato de la franja de resumen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatoResumen {
    /// Clave de traducción de su nombre (`ajustes-resumen-wifi-redes`).
    pub clave: String,
    /// La cifra, ya traducida («0 de 3»).
    pub cifra: String,
    /// Explicación breve, ya traducida.
    pub detalle: String,
    /// Va en ámbar (hay recomendaciones pendientes).
    pub atencion: bool,
}

/// Un elemento de la página: una tarjeta (una red, una radio, un ajuste).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementoVista {
    /// Id en la URL (`red-24`).
    pub id: String,
    /// Prefijo de las claves de su primer ajuste (`ajuste-wifi-red_24`):
    /// con `-titulo`, su nombre.
    pub clave: String,
    /// Clave de traducción de su papel (`ajustes-papel-principal`).
    pub papel: Option<String>,
    /// Nombre de la red WiFi, si es una.
    pub ssid: Option<String>,
    pub estado: EstadoVisual,
    /// Texto de la insignia, ya traducido («Apagada»).
    pub texto_estado: String,
    pub interruptor: Option<InterruptorVista>,
    /// Como mucho seis, en el orden del catálogo.
    pub datos: Vec<DatoVista>,
    /// La recomendación pendiente más importante.
    pub recomendacion: Option<RecomendacionVista>,
    pub pie: PieVista,
    /// Detalle completo: sus ajustes con sus campos.
    pub ajustes: Vec<AjusteVista>,
    /// [`EstadoLectura::SinLeer`] si alguno de sus ajustes no se pudo leer.
    pub lectura: EstadoLectura,
    /// Ids de las acciones de [`PaginaTema::acciones`] que se le pueden
    /// aplicar.
    pub acciones: Vec<String>,
}

/// El interruptor de la cabecera de un elemento.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterruptorVista {
    /// Id del ajuste y del campo que escribe.
    pub ajuste: String,
    pub campo: String,
    pub encendido: bool,
    /// Clave de traducción del efecto al aplicarlo (`efecto-wifi_red`).
    pub efecto: Option<String>,
}

/// Un dato de la tarjeta.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatoVista {
    /// Clave de traducción de su nombre: la del campo
    /// (`ajuste-wifi-red_24-seguridad`) o la del derivado
    /// (`ajustes-dato-conectados`).
    pub clave: String,
    pub valor: ValorLegible,
    /// No tiene el valor recomendado.
    pub atencion: bool,
}

/// La recomendación principal de un elemento y el cambio que la aplica.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecomendacionVista {
    /// El porqué («Recomendado si…»), ya traducido.
    pub texto: String,
    /// Es un riesgo de seguridad: fondo ámbar.
    pub riesgo: bool,
    /// Texto del botón, ya traducido («Quitar WPS»).
    pub boton: String,
    /// Lo que prepara el botón: ajuste, campo y valor legible.
    pub ajuste: String,
    pub campo: String,
    pub valor: String,
}

/// El pie de la tarjeta: información viva.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieVista {
    /// Nombres de los dispositivos conectados.
    pub dispositivos: Vec<String>,
    /// Tráfico reciente (bytes por intervalo).
    pub trafico: Vec<u64>,
    /// Una nota ya traducida (lo que no se ve en sus redes, «apagada desde…»).
    pub nota: Option<String>,
}

/// Una acción sobre varios elementos.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccionVista {
    pub id: String,
    /// Clave de traducción del botón (`ajustes-accion-quitar_wps`).
    pub clave: String,
    /// Botón de peligro, con confirmación.
    pub peligro: bool,
}

impl PaginaTema {
    /// Los ajustes de todos sus elementos, en orden.
    pub fn ajustes(&self) -> impl Iterator<Item = &AjusteVista> {
        self.elementos.iter().flat_map(|e| e.ajustes.iter())
    }
}
