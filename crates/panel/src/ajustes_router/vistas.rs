//! Tipos de Ajustes que comparten el servidor y el navegador.

use crate::ui::explorador::RiesgoCambio;
use ajustes::{Nivel, Requiere, TipoCampo, ValorLegible};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Los ajustes de un tema, leídos del router.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemaVista {
    /// Identificador corto del tema (`wifi`).
    pub tema: String,
    /// El panel está en modo solo lectura (firmware sin confirmar): los
    /// ajustes se ven pero no se cambian.
    pub solo_lectura: bool,
    /// En el orden del catálogo.
    pub ajustes: Vec<AjusteVista>,
}

/// Un ajuste con su valor actual.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AjusteVista {
    /// `<tema>.<nombre>` (`wifi.red_24`).
    pub id: String,
    /// Prefijo de sus claves de traducción (`ajuste-wifi-red_24`): con
    /// `-titulo`, `-desc` y `-ayuda`.
    pub clave: String,
    pub nivel: Nivel,
    /// El mayor de las funciones con que se escribe (bajo si no escribe).
    pub riesgo: RiesgoCambio,
    /// Clave de traducción del efecto al aplicarlo (`efecto-wifi_red`).
    pub efecto: Option<String>,
    /// Es una lista: sus valores están en [`AjusteVista::instancias`] y
    /// [`AjusteVista::campos`] es la plantilla de una instancia nueva.
    pub lista: bool,
    /// Componente propio de la interfaz (`reservas`), si no usa la tarjeta
    /// genérica.
    pub componente: Option<String>,
    pub estado: EstadoLectura,
    /// Campos con su valor. En una lista, la plantilla de una instancia
    /// nueva (valor vacío). Vacío si no se pudo leer.
    pub campos: Vec<CampoVista>,
    /// Instancias de una lista, en el orden del router.
    pub instancias: Vec<InstanciaVista>,
    pub tecnico: DetallesTecnicos,
}

/// Si se pudo leer el valor actual de un ajuste.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoLectura {
    Leido,
    /// No se pudo leer: el motivo, traducido y sin secretos.
    SinLeer(String),
}

/// Un campo con su valor actual y lo que la página necesita para pintarlo y
/// validarlo sin el catálogo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampoVista {
    pub id: String,
    /// Prefijo de sus claves de traducción (`ajuste-wifi-red_24-seguridad`):
    /// sola es la etiqueta; con `-desc`, su descripción; con `-<opción>` y
    /// `-<opción>-desc`, las de cada opción.
    pub clave: String,
    pub tipo: TipoCampo,
    pub valor: ValorLegible,
    /// Opciones de un campo `opcion`, en el orden del catálogo.
    pub opciones: Vec<OpcionVista>,
    /// Límites: en bytes UTF-8 para textos y secretos, en valor para números.
    pub min: Option<i64>,
    pub max: Option<i64>,
    /// Solo admite ASCII imprimible (la contraseña de una red WiFi).
    pub ascii: bool,
    /// Se puede dejar vacío.
    pub opcional: bool,
    /// Clave de traducción del motivo por el que no se puede editar.
    pub solo_lectura: Option<String>,
    /// Secreto que el router da tapado al leer: sale vacío, y guardar
    /// cualquier campo que escriba con él exige escribirlo.
    #[serde(default)]
    pub tapado: bool,
    /// El valor leído de un secreto, para su campo censurado con el ojo
    /// (decisión del usuario del 2026-10-18: viaja con la página, que va con
    /// `no-store`). `None` si no es un secreto o el router no deja leerlo
    /// (lo da enmascarado, o vacío uno marcado como tapado al leer).
    #[serde(default)]
    pub valor_secreto: Option<String>,
    /// Valor legible recomendado: la clave de la opción recomendada en un
    /// campo `opcion` (`wpa2`), `true`/`false` en un booleano o el texto.
    pub recomendado: Option<String>,
    /// Clave de traducción del porqué de la recomendación (para quién es, a
    /// favor y en contra): `<clave>-recomendado`. Solo si hay recomendación.
    pub motivo_recomendado: Option<String>,
    /// Otro campo del ajuste que hace falta cuando este toma un valor (la
    /// toma del espejo al encenderlo): la página lo pide junto a ese campo.
    pub requiere: Option<Requiere>,
    /// El campo `opcion` tiene un valor «sin elegir» ([`ajustes::SIN_ELECCION`],
    /// la toma del espejo apagado): se ve como una opción más, pero no se
    /// escribe ni cuenta como elegida.
    pub sin_eleccion: bool,
}

/// Una opción de un campo `opcion`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpcionVista {
    /// Clave legible (`wpa2`): el valor que se envía al guardar y el final de
    /// sus claves de traducción.
    pub clave: String,
    pub recomendada: bool,
    /// Clave de traducción del aviso al elegirla (`aviso-wifi_abierta`).
    pub aviso: Option<String>,
}

/// Una instancia de una lista (una regla, una reserva…).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanciaVista {
    /// Id de la instancia en el router (su posición si el router no da uno).
    pub id: String,
    pub campos: Vec<CampoVista>,
}

/// Funciones y parámetros del router detrás de un ajuste. Lo único con
/// nombres de la API: solo se pinta dentro de los detalles técnicos plegados.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetallesTecnicos {
    /// Función de lectura con sus parámetros.
    pub lectura: String,
    /// Funciones de escritura, sin repetir.
    pub escrituras: Vec<String>,
    /// Pares campo → parámetro (el que escribe o, si solo se lee, el
    /// elemento de la lectura; después, los que fija cada opción).
    pub parametros: Vec<(String, String)>,
}

/// Lo que la página pide guardar de un ajuste. Su `Debug` no enseña los
/// valores (pueden ser contraseñas), solo los ids de los campos.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeticionAjuste {
    /// Id del ajuste (`wifi.red_24`).
    pub ajuste: String,
    /// Id de la instancia que se edita o borra en una lista; `None` crea
    /// una nueva. No cuenta en un ajuste que no es lista.
    pub instancia: Option<String>,
    /// Borra la instancia (solo en una lista).
    pub borrar: bool,
    /// Campos que se cambian: id del campo → valor legible (la clave de una
    /// opción, `true`/`false`, el texto). Un secreto sin tocar no va.
    pub valores: BTreeMap<String, String>,
    /// Lo que la página espera que tenga la instancia que edita o borra
    /// (id del campo → valor legible): si el router ya tiene otra cosa
    /// (otra sesión la cambió), 409. Por ahora lo comprueban las reservas,
    /// que al editar o borrar exigen su MAC (sin ella, 400).
    #[serde(default)]
    pub esperados: BTreeMap<String, String>,
}

impl std::fmt::Debug for PeticionAjuste {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeticionAjuste")
            .field("ajuste", &self.ajuste)
            .field("instancia", &self.instancia)
            .field("borrar", &self.borrar)
            .field("valores", &self.valores.keys().collect::<Vec<_>>())
            .field("esperados", &self.esperados.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Resumen de un ajuste preparado para guardarlo: lo que cambia en cada paso
/// y lo que hace falta para confirmarlo. Sin nombres de la API y con los
/// secretos como [`MASCARA`](crate::ui::explorador::MASCARA).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrupoVista {
    /// Token para [`confirmar_ajuste`](super::confirmar_ajuste) (un solo uso, 10 min).
    pub token: String,
    /// Clave de traducción del título del ajuste (`ajuste-wifi-red_24-titulo`).
    pub titulo: String,
    /// Un paso por función de escritura, en orden.
    pub pasos: Vec<PasoVista>,
    /// El mayor de los pasos con cambios: marca la vigilancia.
    pub riesgo: RiesgoCambio,
    /// Clave de traducción del efecto al aplicarlo (`efecto-wifi_red`).
    pub efecto: Option<String>,
    /// Segundos que se vigila el router tras aplicarlo.
    pub vigilancia_s: u64,
    /// A qué se refiere, si el título no basta (el dispositivo de una IP fija).
    pub detalle: Option<String>,
    /// Un aviso para quien confirma, ya traducido (al deshacer: la clave que
    /// el router no deja leer se queda como se escribió).
    #[serde(default)]
    pub aviso: Option<String>,
}

/// Un paso del grupo: los campos de una misma escritura del router.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasoVista {
    /// Posición en el grupo (desde 0): la que se desmarca al confirmar.
    pub indice: usize,
    /// El router ya está así: no se escribirá.
    pub sin_cambios: bool,
    pub campos: Vec<CambioLegible>,
    /// Clave de traducción de su título («TCP»); sin ella, «Paso N».
    #[serde(default)]
    pub titulo: Option<String>,
    /// Clave de traducción de una nota bajo el título (lo que no se repite).
    #[serde(default)]
    pub nota: Option<String>,
}

/// Un campo que cambia, con sus valores legibles. Si es `secreto`, `antes`
/// y `despues` valen [`MASCARA`](crate::ui::explorador::MASCARA).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CambioLegible {
    /// Clave de traducción del campo (`ajuste-wifi-red_24-nombre`).
    pub campo: String,
    /// Valor legible: la clave de una opción (con `campo` da su texto,
    /// `<campo>-<opción>`), `true`/`false` en un sí/no o el texto tal cual.
    /// Un valor que el panel no reconoce va tal cual lo da el router.
    pub antes: String,
    pub despues: String,
    pub secreto: bool,
    /// Tipo del campo: dice cómo se pintan `antes` y `despues`.
    pub tipo: TipoCampo,
}

/// Un dispositivo que el router conoce en la red de casa.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispositivoVista {
    /// Nombre que da el dispositivo; vacío si no da ninguno.
    pub nombre: String,
    pub ip: String,
    pub mac: String,
    pub conexion: Conexion,
    /// Conectado ahora.
    pub activo: bool,
    /// Id de su reserva (IP fija), si tiene.
    pub reserva: Option<String>,
    /// La IP de su reserva (puede no ser la que tiene ahora).
    pub ip_fija: Option<String>,
}

/// Por dónde está conectado un dispositivo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Conexion {
    /// Por cable: la interfaz del router tal como la da (`tipo:id`). Qué
    /// toma física es cada id está sin comprobar en el router real, así que
    /// la página solo dice «por cable».
    Cable(String),
    Wifi24,
    Wifi5,
    /// Asociado a una red de invitados.
    Invitados,
    /// No se sabe: no está conectado ahora o no se pudo leer la WiFi.
    Desconocida,
}
