//! Datos del explorador: tipos que viajan al navegador, validación de campos
//! (la misma en el navegador y en el servidor), enlaces y server functions.
//!
//! Los tipos de aquí no dependen de `catalogo` ni de `hgu` (que solo existen
//! en el servidor). Un valor secreto nunca viaja: va como [`Valor::Secreto`]
//! (lecturas y diff) o como [`MASCARA`] (formularios, que el gestor entiende
//! como "sin tocar"). Solo [`revelar`] devuelve un secreto, tras pedir otra
//! vez la contraseña del panel, y queda registrado en `reveals`.
//!
//! Toda escritura pasa por el gestor de cambios del servidor
//! (`AppState::cambios`): `preparar_cambio` y `confirmar_cambio` son sus
//! dos pasos, y `estado_cambio` sigue el resultado.

use crate::t;
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Lo que se muestra en lugar de un secreto (el mismo texto que usa el
/// gestor de cambios para "sin tocar").
pub const MASCARA: &str = "••••••";

/// ListaInstancias que se muestran como mucho de cada lista.
pub const MAX_INSTANCIAS: usize = 50;

/// Qué hace una función.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoFuncion {
    Lectura,
    Escritura,
    Accion,
}

impl TipoFuncion {
    pub fn etiqueta(self) -> String {
        match self {
            TipoFuncion::Lectura => t!("biblioteca-tipo-lectura"),
            TipoFuncion::Escritura => t!("biblioteca-tipo-escritura"),
            TipoFuncion::Accion => t!("biblioteca-tipo-accion"),
        }
    }

    /// Sufijo de la clase CSS `chip-…` (no depende del idioma).
    pub fn clase(self) -> &'static str {
        match self {
            TipoFuncion::Lectura => "lectura",
            TipoFuncion::Escritura => "escritura",
            TipoFuncion::Accion => "accion",
        }
    }
}

/// De qué parte de la web del router sale una función.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrigenFuncion {
    Basica,
    Avanzada,
    Oculta,
    Incompleta,
}

impl OrigenFuncion {
    pub fn etiqueta(self) -> String {
        match self {
            OrigenFuncion::Basica => t!("biblioteca-origen-basica"),
            OrigenFuncion::Avanzada => t!("biblioteca-origen-avanzada"),
            OrigenFuncion::Oculta => t!("biblioteca-origen-oculta"),
            OrigenFuncion::Incompleta => t!("biblioteca-origen-incompleta"),
        }
    }

    /// Explicación para el `title` de la etiqueta.
    pub fn ayuda(self) -> String {
        match self {
            OrigenFuncion::Basica => t!("biblioteca-origen-basica-ayuda"),
            OrigenFuncion::Avanzada => t!("biblioteca-origen-avanzada-ayuda"),
            OrigenFuncion::Oculta => t!("biblioteca-origen-oculta-ayuda"),
            OrigenFuncion::Incompleta => t!("biblioteca-origen-incompleta-ayuda"),
        }
    }
}

/// Riesgo de un cambio (determina la vigilancia).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiesgoCambio {
    Bajo,
    Medio,
    Alto,
}

impl RiesgoCambio {
    pub fn etiqueta(self) -> String {
        match self {
            RiesgoCambio::Bajo => t!("biblioteca-riesgo-bajo"),
            RiesgoCambio::Medio => t!("biblioteca-riesgo-medio"),
            RiesgoCambio::Alto => t!("biblioteca-riesgo-alto"),
        }
    }

    pub fn estado(self) -> crate::ui::Estado {
        match self {
            RiesgoCambio::Bajo => crate::ui::Estado::Ok,
            RiesgoCambio::Medio => crate::ui::Estado::Aviso,
            RiesgoCambio::Alto => crate::ui::Estado::Error,
        }
    }
}

/// Tipo de un campo de formulario.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoCampo {
    Ip,
    Mac,
    Bool,
    Enum,
    Int,
    Text,
    Secret,
}

impl TipoCampo {
    pub fn etiqueta(self) -> String {
        match self {
            TipoCampo::Ip => "IPv4".to_owned(),
            TipoCampo::Mac => "MAC".to_owned(),
            TipoCampo::Bool => t!("biblioteca-campo-bool"),
            TipoCampo::Enum => t!("biblioteca-campo-enum"),
            TipoCampo::Int => t!("biblioteca-campo-int"),
            TipoCampo::Text => t!("biblioteca-campo-texto"),
            TipoCampo::Secret => t!("biblioteca-campo-secreto"),
        }
    }
}

/// Función en las listas del explorador.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuncionResumen {
    pub nombre: String,
    /// Identificador del área (`wan`, `lan_dhcp`…).
    pub area: String,
    pub descripcion: String,
    pub tipo: TipoFuncion,
    pub origen: OrigenFuncion,
    pub riesgo: RiesgoCambio,
    /// Motivo por el que el panel no la usa (catálogo o bloqueos de la v1).
    pub bloqueada: Option<String>,
}

/// Área del lateral con sus funciones.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaVista {
    pub slug: String,
    pub etiqueta: String,
    pub funciones: Vec<FuncionResumen>,
}

/// Parámetro de una función.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamVista {
    pub nombre: String,
    pub etiqueta: String,
    pub tipo: TipoCampo,
    /// `(valor, etiqueta)` de un `Enum`.
    pub opciones: Vec<(String, String)>,
    /// Valor con el que se rellena al leer (`ALL`, `0` en ids…).
    pub inicial: String,
}

/// Lista de instancias de una función.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListaVista {
    pub id_param: String,
    /// Admite borrar instancias.
    pub borrar: bool,
}

/// Todo lo que hace falta para pintar la página de una función.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuncionDetalle {
    pub resumen: FuncionResumen,
    pub etiqueta_area: String,
    pub pagina: Option<String>,
    pub parametros: Vec<ParamVista>,
    /// Lectura que devuelve lo que escribe (nombre y área, para enlazarla).
    pub lectura_pareada: Option<(String, String)>,
    pub lista: Option<ListaVista>,
    /// Parámetros que identifican la instancia (sin el id de la lista): hay
    /// que darlos antes de leer los valores actuales.
    pub claves: Vec<ParamVista>,
    /// Segundos de vigilancia tras un cambio.
    pub vigilancia_s: u64,
    /// La lectura se puede ejecutar desde el explorador (si no, el motivo).
    pub lectura: Result<(), String>,
}

/// Valor de una lectura.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Valor {
    Texto(String),
    /// Secreto: no viaja.
    Secreto,
    /// La instancia no tiene ese campo.
    Ausente,
    /// Valor de una opción (o sí/no) con su etiqueta; `None` si no es una
    /// de las opciones que conoce el catálogo.
    Opcion {
        valor: String,
        etiqueta: Option<String>,
    },
}

impl Valor {
    /// Texto para mostrar una opción: «Activado (1)» o «(valor
    /// desconocido: 7)».
    pub fn texto_opcion(valor: &str, etiqueta: Option<&str>) -> String {
        match etiqueta {
            Some(e) => format!("{e} ({valor})"),
            None => t!("biblioteca-valor-desconocido", valor = valor),
        }
    }
}

/// Campo suelto de una lectura.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Par {
    pub nombre: String,
    pub valor: Valor,
}

/// Fila de una tabla de instancias.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fila {
    /// Identificador de la instancia (su id, o `#n` por posición).
    pub id: String,
    pub valores: Vec<Valor>,
}

/// ListaInstancias de un mismo elemento de la respuesta.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grupo {
    pub nombre: String,
    pub columnas: Vec<String>,
    pub filas: Vec<Fila>,
    /// ListaInstancias en la respuesta (puede haber más que filas).
    pub total: usize,
}

/// Resultado de una lectura, enmascarado.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vista {
    pub pares: Vec<Par>,
    pub grupos: Vec<Grupo>,
    /// XML de la respuesta, enmascarado.
    pub crudo: Option<String>,
}

/// Qué hace un formulario.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operacion {
    Editar,
    Crear,
    Accion,
}

/// Campo de un formulario.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampoForm {
    pub param: ParamVista,
    /// Valor actual; [`MASCARA`] en los secretos (enviarlo tal cual = sin
    /// tocar).
    pub valor: String,
}

/// Formulario generado del catálogo y rellenado con la lectura pareada.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormVista {
    pub funcion: String,
    pub operacion: Operacion,
    pub instancia: Option<String>,
    /// Parámetros que identifican la instancia (se envían sin cambios).
    pub claves: Vec<(String, String)>,
    pub campos: Vec<CampoForm>,
    /// Parámetros del catálogo que no se pueden editar porque la lectura
    /// pareada no los muestra (sin valor anterior no habría deshacer).
    pub omitidos: Vec<String>,
}

/// ListaInstancias de una lista con sus valores, para editar o borrar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListaInstancias {
    pub columnas: Vec<String>,
    pub filas: Vec<Fila>,
    pub total: usize,
}

/// Petición de cambio desde el navegador. Los IP/MAC van unidos.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peticion {
    pub funcion: String,
    /// Instancia que se edita o borra; `None` crea una nueva.
    pub instancia: Option<String>,
    pub valores: Vec<(String, String)>,
    pub borrar: bool,
}

/// Campo del diff.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampoCambio {
    pub nombre: String,
    pub etiqueta: String,
    pub antes: Valor,
    pub despues: Valor,
}

/// Cambio preparado: diff, riesgo y token para confirmarlo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparadoVista {
    pub token: String,
    pub funcion: String,
    pub instancia: Option<String>,
    pub borrar: bool,
    pub campos: Vec<CampoCambio>,
    pub riesgo: RiesgoCambio,
    pub vigilancia_s: u64,
    /// Palabra que hay que escribir para confirmar (acciones).
    pub palabra: Option<String>,
}

/// Estado de un cambio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoDeCambio {
    Preparado,
    Aplicando,
    Vigilando,
    Aplicado,
    Revertido,
    Fallido,
    /// Un grupo que falló y no se pudo revertir entero.
    RevertidoParcial,
}

impl EstadoDeCambio {
    pub fn etiqueta(self) -> String {
        match self {
            EstadoDeCambio::Preparado => t!("biblioteca-cambio-preparado"),
            EstadoDeCambio::Aplicando => t!("biblioteca-cambio-aplicando"),
            EstadoDeCambio::Vigilando => t!("biblioteca-cambio-vigilando"),
            EstadoDeCambio::Aplicado => t!("biblioteca-cambio-aplicado"),
            EstadoDeCambio::Revertido => t!("biblioteca-cambio-revertido"),
            EstadoDeCambio::Fallido => t!("biblioteca-cambio-fallido"),
            EstadoDeCambio::RevertidoParcial => t!("biblioteca-cambio-revertido-parcial"),
        }
    }

    /// Ya no va a cambiar.
    pub fn final_(self) -> bool {
        matches!(
            self,
            EstadoDeCambio::Aplicado
                | EstadoDeCambio::Revertido
                | EstadoDeCambio::Fallido
                | EstadoDeCambio::RevertidoParcial
        )
    }

    pub fn estado(self) -> crate::ui::Estado {
        match self {
            EstadoDeCambio::Aplicado => crate::ui::Estado::Ok,
            EstadoDeCambio::Revertido => crate::ui::Estado::Aviso,
            EstadoDeCambio::Fallido | EstadoDeCambio::RevertidoParcial => crate::ui::Estado::Error,
            _ => crate::ui::Estado::Neutro,
        }
    }
}

/// Seguimiento de un cambio.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CambioVista {
    pub id: i64,
    pub estado: EstadoDeCambio,
    pub error: Option<String>,
    pub nota: Option<String>,
    /// Segundos que quedan de vigilancia (solo `vigilando`).
    pub restante_s: Option<i64>,
}

// ------------------------------------------------------------ validación

/// Comprueba un valor nuevo según su tipo (navegador y servidor). El
/// mensaje nunca repite un secreto.
pub fn validar_campo(
    tipo: TipoCampo,
    opciones: &[(String, String)],
    v: &str,
) -> Result<(), String> {
    let t = v.trim();
    match tipo {
        TipoCampo::Ip if t.parse::<Ipv4Addr>().is_err() => Err(t!("biblioteca-validar-ip")),
        TipoCampo::Mac if !es_mac(t) => Err(t!("biblioteca-validar-mac")),
        TipoCampo::Bool if !matches!(t, "0" | "1") => Err(t!("biblioteca-validar-bool")),
        TipoCampo::Enum if !opciones.iter().any(|(o, _)| o == t) => {
            Err(t!("biblioteca-validar-enum"))
        }
        TipoCampo::Int if t.parse::<i64>().is_err() => Err(t!("biblioteca-validar-int")),
        TipoCampo::Text | TipoCampo::Secret if v.chars().any(char::is_control) => {
            Err(t!("biblioteca-validar-control"))
        }
        _ => Ok(()),
    }
}

fn es_mac(s: &str) -> bool {
    let partes: Vec<&str> = s.split(':').collect();
    partes.len() == 6
        && partes
            .iter()
            .all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

// ------------------------------------------------------------ enlaces

/// Codifica un componente de ruta como `encodeURIComponent` (los nombres de
/// función llevan `.`, `?`, `=`, `/`…).
pub fn codificar(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Ruta de la página de una función.
pub fn ruta_funcion(area: &str, nombre: &str) -> String {
    format!("/explorador/{area}/{}", codificar(nombre))
}

// ------------------------------------------------------------ server functions

/// Áreas del explorador con sus funciones (para el lateral y la búsqueda).
#[server(endpoint = "explorador_catalogo", input = Json)]
pub async fn catalogo_explorador() -> Result<Vec<AreaVista>, ServerFnError> {
    crate::auth::require_user().await?;
    let state = super::servidor::app_state()?;
    Ok(super::servidor::areas(&state))
}

/// Ficha de una función.
#[server(endpoint = "explorador_funcion", input = Json)]
pub async fn detalle_funcion(nombre: String) -> Result<FuncionDetalle, ServerFnError> {
    crate::auth::require_user().await?;
    let state = super::servidor::app_state()?;
    super::servidor::detalle(&state, &nombre)
}

/// Ejecuta la lectura `funcion` con `params` y la devuelve enmascarada.
#[server(endpoint = "leer", input = Json)]
pub async fn leer(funcion: String, params: Vec<(String, String)>) -> Result<Vista, ServerFnError> {
    crate::auth::require_user().await?;
    let state = super::servidor::app_state()?;
    super::servidor::leer(&state, &funcion, &params).await
}

/// Formulario de `funcion` para la instancia (`None` en una lista: crear),
/// rellenado con los valores actuales.
#[server(endpoint = "formulario", input = Json)]
pub async fn formulario(
    funcion: String,
    instancia: Option<String>,
    claves: Vec<(String, String)>,
) -> Result<FormVista, ServerFnError> {
    crate::auth::require_user().await?;
    let state = super::servidor::app_state()?;
    super::servidor::formulario(&state, &funcion, instancia, claves).await
}

/// ListaInstancias de una función con lista, para editarlas o borrarlas.
#[server(endpoint = "instancias", input = Json)]
pub async fn instancias(
    funcion: String,
    claves: Vec<(String, String)>,
) -> Result<ListaInstancias, ServerFnError> {
    crate::auth::require_user().await?;
    let state = super::servidor::app_state()?;
    super::servidor::instancias(&state, &funcion, claves).await
}

/// Paso 1 del gestor de cambios: diff y token.
#[server(endpoint = "preparar_cambio", input = Json)]
pub async fn preparar_cambio(peticion: Peticion) -> Result<PreparadoVista, ServerFnError> {
    crate::auth::require_user().await?;
    let state = super::servidor::app_state()?;
    super::servidor::preparar(&state, peticion).await
}

/// Paso 2: aplica el cambio preparado. Devuelve su id para seguirlo.
#[server(endpoint = "confirmar_cambio", input = Json)]
pub async fn confirmar_cambio(
    token: String,
    palabra: Option<String>,
) -> Result<i64, ServerFnError> {
    let user = crate::auth::require_user().await?;
    let state = super::servidor::app_state()?;
    super::servidor::confirmar(&state, &token, palabra.as_deref(), &user.username).await
}

/// Estado de un cambio.
#[server(endpoint = "estado_cambio", input = Json)]
pub async fn estado_cambio(id: i64) -> Result<CambioVista, ServerFnError> {
    crate::auth::require_user().await?;
    let state = super::servidor::app_state()?;
    super::servidor::estado_cambio(&state, id).await
}

/// Valor de un secreto de la lectura `funcion` (con `params`): campo `campo`
/// de la instancia `instancia` (id de la fila) o suelto. Exige la contraseña
/// del panel, con el mismo freno que el login, y queda registrado (sin el
/// valor) en `reveals`.
#[server(endpoint = "revelar", input = Json)]
pub async fn revelar(
    funcion: String,
    campo: String,
    instancia: Option<String>,
    params: Vec<(String, String)>,
    contrasena: String,
) -> Result<String, ServerFnError> {
    let user = crate::auth::require_user().await?;
    let state = super::servidor::app_state()?;
    super::servidor::revelar(
        &state,
        &user,
        &funcion,
        &campo,
        instancia.as_deref(),
        &params,
        &contrasena,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valida_cada_tipo() {
        let op = vec![("1".to_owned(), "Sí".to_owned())];
        assert!(validar_campo(TipoCampo::Ip, &[], " 192.168.1.5 ").is_ok());
        assert!(validar_campo(TipoCampo::Ip, &[], "192.168.1").is_err());
        assert!(validar_campo(TipoCampo::Mac, &[], "02:00:00:00:00:aB").is_ok());
        assert!(validar_campo(TipoCampo::Mac, &[], "02:00:00:00:00").is_err());
        assert!(validar_campo(TipoCampo::Mac, &[], "02:00:00:00:00:zz").is_err());
        assert!(validar_campo(TipoCampo::Bool, &[], "1").is_ok());
        assert!(validar_campo(TipoCampo::Bool, &[], "sí").is_err());
        assert!(validar_campo(TipoCampo::Enum, &op, "1").is_ok());
        assert!(validar_campo(TipoCampo::Enum, &op, "2").is_err());
        assert!(validar_campo(TipoCampo::Int, &[], "-3").is_ok());
        assert!(validar_campo(TipoCampo::Int, &[], "3.5").is_err());
        assert!(validar_campo(TipoCampo::Text, &[], "a\nb").is_err());
        let e = validar_campo(TipoCampo::Secret, &[], "cl\0ve").unwrap_err();
        assert!(!e.contains("cl"), "{e}");
    }

    #[test]
    fn codifica_nombres_de_funcion() {
        assert_eq!(codificar("cbGetWifi"), "cbGetWifi");
        assert_eq!(
            codificar("te_puertos.cmd?action=add"),
            "te_puertos.cmd%3Faction%3Dadd"
        );
        assert_eq!(codificar("upload/cfgupload.asp"), "upload%2Fcfgupload.asp");
        assert_eq!(codificar("ñ"), "%C3%B1");
        assert_eq!(
            ruta_funcion("nat", "te_ppp_pm.cmd?action=create"),
            "/explorador/nat/te_ppp_pm.cmd%3Faction%3Dcreate"
        );
    }
}
