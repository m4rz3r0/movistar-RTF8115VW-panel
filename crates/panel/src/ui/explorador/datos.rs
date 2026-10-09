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
    pub fn etiqueta(self) -> &'static str {
        match self {
            TipoFuncion::Lectura => "Lectura",
            TipoFuncion::Escritura => "Escritura",
            TipoFuncion::Accion => "Acción",
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
    pub fn etiqueta(self) -> &'static str {
        match self {
            OrigenFuncion::Basica => "Básica",
            OrigenFuncion::Avanzada => "Avanzada",
            OrigenFuncion::Oculta => "Oculta",
            OrigenFuncion::Incompleta => "Incompleta",
        }
    }

    /// Explicación para el `title` de la etiqueta.
    pub fn ayuda(self) -> &'static str {
        match self {
            OrigenFuncion::Basica => "De la web básica del router",
            OrigenFuncion::Avanzada => "De la web avanzada, enlazada desde su menú",
            OrigenFuncion::Oculta => "Funciona, pero ningún menú del router la enlaza",
            OrigenFuncion::Incompleta => "Restos sin terminar: puede no responder",
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
    pub fn etiqueta(self) -> &'static str {
        match self {
            RiesgoCambio::Bajo => "Riesgo bajo",
            RiesgoCambio::Medio => "Riesgo medio",
            RiesgoCambio::Alto => "Riesgo alto",
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
    pub fn etiqueta(self) -> &'static str {
        match self {
            TipoCampo::Ip => "IPv4",
            TipoCampo::Mac => "MAC",
            TipoCampo::Bool => "Sí/no",
            TipoCampo::Enum => "Opción",
            TipoCampo::Int => "Entero",
            TipoCampo::Text => "Texto",
            TipoCampo::Secret => "Secreto",
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
}

impl EstadoDeCambio {
    pub fn etiqueta(self) -> &'static str {
        match self {
            EstadoDeCambio::Preparado => "preparado",
            EstadoDeCambio::Aplicando => "aplicando",
            EstadoDeCambio::Vigilando => "vigilando",
            EstadoDeCambio::Aplicado => "aplicado",
            EstadoDeCambio::Revertido => "revertido",
            EstadoDeCambio::Fallido => "fallido",
        }
    }

    /// Ya no va a cambiar.
    pub fn final_(self) -> bool {
        matches!(
            self,
            EstadoDeCambio::Aplicado | EstadoDeCambio::Revertido | EstadoDeCambio::Fallido
        )
    }

    pub fn estado(self) -> crate::ui::Estado {
        match self {
            EstadoDeCambio::Aplicado => crate::ui::Estado::Ok,
            EstadoDeCambio::Revertido => crate::ui::Estado::Aviso,
            EstadoDeCambio::Fallido => crate::ui::Estado::Error,
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
        TipoCampo::Ip if t.parse::<Ipv4Addr>().is_err() => {
            Err("no es una dirección IPv4 (p. ej. 192.168.1.60)".into())
        }
        TipoCampo::Mac if !es_mac(t) => {
            Err("no es una MAC (seis pares hexadecimales con «:»)".into())
        }
        TipoCampo::Bool if !matches!(t, "0" | "1") => Err("debe ser 0 o 1".into()),
        TipoCampo::Enum if !opciones.iter().any(|(o, _)| o == t) => {
            Err("no es una de las opciones".into())
        }
        TipoCampo::Int if t.parse::<i64>().is_err() => Err("debe ser un número entero".into()),
        TipoCampo::Text | TipoCampo::Secret if v.chars().any(char::is_control) => {
            Err("contiene caracteres no válidos".into())
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
    let state = servidor::app_state()?;
    Ok(servidor::areas(&state))
}

/// Ficha de una función.
#[server(endpoint = "explorador_funcion", input = Json)]
pub async fn detalle_funcion(nombre: String) -> Result<FuncionDetalle, ServerFnError> {
    crate::auth::require_user().await?;
    let state = servidor::app_state()?;
    servidor::detalle(&state, &nombre)
}

/// Ejecuta la lectura `funcion` con `params` y la devuelve enmascarada.
#[server(endpoint = "leer", input = Json)]
pub async fn leer(funcion: String, params: Vec<(String, String)>) -> Result<Vista, ServerFnError> {
    crate::auth::require_user().await?;
    let state = servidor::app_state()?;
    servidor::leer(&state, &funcion, &params).await
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
    let state = servidor::app_state()?;
    servidor::formulario(&state, &funcion, instancia, claves).await
}

/// ListaInstancias de una función con lista, para editarlas o borrarlas.
#[server(endpoint = "instancias", input = Json)]
pub async fn instancias(
    funcion: String,
    claves: Vec<(String, String)>,
) -> Result<ListaInstancias, ServerFnError> {
    crate::auth::require_user().await?;
    let state = servidor::app_state()?;
    servidor::instancias(&state, &funcion, claves).await
}

/// Paso 1 del gestor de cambios: diff y token.
#[server(endpoint = "preparar_cambio", input = Json)]
pub async fn preparar_cambio(peticion: Peticion) -> Result<PreparadoVista, ServerFnError> {
    crate::auth::require_user().await?;
    let state = servidor::app_state()?;
    servidor::preparar(&state, peticion).await
}

/// Paso 2: aplica el cambio preparado. Devuelve su id para seguirlo.
#[server(endpoint = "confirmar_cambio", input = Json)]
pub async fn confirmar_cambio(
    token: String,
    palabra: Option<String>,
) -> Result<i64, ServerFnError> {
    let user = crate::auth::require_user().await?;
    let state = servidor::app_state()?;
    servidor::confirmar(&state, &token, palabra.as_deref(), &user.username).await
}

/// Estado de un cambio.
#[server(endpoint = "estado_cambio", input = Json)]
pub async fn estado_cambio(id: i64) -> Result<CambioVista, ServerFnError> {
    crate::auth::require_user().await?;
    let state = servidor::app_state()?;
    servidor::estado_cambio(&state, id).await
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
    let state = servidor::app_state()?;
    servidor::revelar(
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

#[cfg(feature = "ssr")]
pub(crate) mod servidor {
    use super::*;
    use crate::{
        auth::{LoginError, User},
        changes::{CambioError, ChangeManager, ChangeRequest, Estado},
        state::AppState,
    };
    use axum::http::StatusCode;
    use catalogo::{Area, Funcion, Origen, Riesgo, Tipo, TipoParam};
    use hgu::xml::XmlNode;
    use std::{
        collections::{BTreeMap, BTreeSet, HashSet},
        fmt,
        sync::Arc,
    };

    /// Páginas que disparan acciones al cargarse: nunca se cargan, digan lo
    /// que digan el catálogo o la petición.
    const PAGINAS_PROHIBIDAS: &[&str] = &["syslog.asp", "backup_restore.asp"];

    pub fn app_state() -> Result<AppState, ServerFnError> {
        use_context::<AppState>().ok_or_else(|| ServerFnError::new("falta el estado del panel"))
    }

    fn con_estado(status: StatusCode, msg: impl Into<String>) -> ServerFnError {
        if let Some(resp) = use_context::<leptos_axum::ResponseOptions>() {
            resp.set_status(status);
        }
        ServerFnError::new(msg.into())
    }

    fn invalida(msg: impl Into<String>) -> ServerFnError {
        con_estado(StatusCode::BAD_REQUEST, msg)
    }

    /// Registra el error con un id y devuelve un mensaje genérico con ese id.
    fn interno(e: impl fmt::Display) -> ServerFnError {
        let mut id = [0u8; 4];
        if getrandom::fill(&mut id).is_err() {
            id = (crate::series::now_ms() as u32).to_be_bytes();
        }
        let id: String = id.iter().map(|b| format!("{b:02x}")).collect();
        tracing::error!(id, "error interno en el explorador: {e}");
        con_estado(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error interno del panel (id {id})"),
        )
    }

    fn gestor(state: &AppState) -> Result<Arc<ChangeManager>, ServerFnError> {
        state
            .cambios()
            .ok_or_else(|| interno("el gestor de cambios no está instalado"))
    }

    /// Error del gestor en español y con qué hacer.
    fn error_cambio(e: CambioError) -> ServerFnError {
        use CambioError as E;
        let (status, msg) = match e {
            E::Bloqueada(m) => (StatusCode::FORBIDDEN, format!("Función bloqueada: {m}.")),
            E::SinCambios => (
                StatusCode::BAD_REQUEST,
                "No hay nada que cambiar: los valores coinciden con los del router.".into(),
            ),
            E::EnCurso => (
                StatusCode::CONFLICT,
                "Hay otro cambio en curso; espera a que termine y vuelve a intentarlo.".into(),
            ),
            E::Router(m) => (
                StatusCode::BAD_GATEWAY,
                format!("{m}. Comprueba la conexión con el router y vuelve a intentarlo."),
            ),
            E::Verificacion(v) => (
                StatusCode::BAD_GATEWAY,
                format!(
                    "El router no muestra el cambio ({}). No se ha revertido nada: revisa los \
                     valores en su página.",
                    v.join("; ")
                ),
            ),
            E::PalabraIncorrecta => (
                StatusCode::BAD_REQUEST,
                "La palabra de confirmación no coincide: escríbela exactamente como se indica."
                    .into(),
            ),
            E::TokenInvalido => (
                StatusCode::BAD_REQUEST,
                "El cambio preparado ha caducado o ya se usó: pulsa «Revisar cambios» de nuevo."
                    .into(),
            ),
            E::Invalida(m) => (
                StatusCode::BAD_REQUEST,
                format!("Petición no válida: {m}. Corrige el valor y revisa de nuevo."),
            ),
        };
        con_estado(status, msg)
    }

    fn funcion<'a>(state: &'a AppState, nombre: &str) -> Result<&'a Funcion, ServerFnError> {
        state.catalogo.funcion(nombre).ok_or_else(|| {
            con_estado(
                StatusCode::NOT_FOUND,
                format!("No existe la función {nombre}"),
            )
        })
    }

    // ---- catálogo

    fn tipo(t: Tipo) -> TipoFuncion {
        match t {
            Tipo::Lectura => TipoFuncion::Lectura,
            Tipo::Escritura => TipoFuncion::Escritura,
            Tipo::Accion => TipoFuncion::Accion,
        }
    }

    fn origen(o: Origen) -> OrigenFuncion {
        match o {
            Origen::Basica => OrigenFuncion::Basica,
            Origen::Avanzada => OrigenFuncion::Avanzada,
            Origen::Oculta => OrigenFuncion::Oculta,
            Origen::Incompleta => OrigenFuncion::Incompleta,
        }
    }

    pub fn riesgo(r: Riesgo) -> RiesgoCambio {
        match r {
            Riesgo::Bajo => RiesgoCambio::Bajo,
            Riesgo::Medio => RiesgoCambio::Medio,
            Riesgo::Alto => RiesgoCambio::Alto,
        }
    }

    fn tipo_campo(t: TipoParam) -> TipoCampo {
        match t {
            TipoParam::Ip => TipoCampo::Ip,
            TipoParam::Mac => TipoCampo::Mac,
            TipoParam::Bool => TipoCampo::Bool,
            TipoParam::Enum => TipoCampo::Enum,
            TipoParam::Int => TipoCampo::Int,
            TipoParam::Text => TipoCampo::Text,
            TipoParam::Secret => TipoCampo::Secret,
        }
    }

    /// Motivo por el que el panel no escribe con `f` (catálogo o bloqueos de
    /// la v1 del gestor). Las lecturas solo por el catálogo.
    fn bloqueo(state: &AppState, f: &Funcion) -> Option<String> {
        if let Some(m) = &f.bloqueada {
            return Some(m.clone());
        }
        if f.tipo == Tipo::Lectura {
            return None;
        }
        match state.cambios().map(|g| g.escribible(&f.name)) {
            Some(Err(CambioError::Bloqueada(m))) => Some(m),
            _ => None,
        }
    }

    fn resumen(state: &AppState, f: &Funcion) -> FuncionResumen {
        FuncionResumen {
            nombre: f.name.clone(),
            area: f.area.slug().to_owned(),
            descripcion: f.descripcion.clone(),
            tipo: tipo(f.tipo),
            origen: origen(f.origen),
            riesgo: riesgo(f.riesgo),
            bloqueada: bloqueo(state, f),
        }
    }

    pub fn areas(state: &AppState) -> Vec<AreaVista> {
        Area::TODAS
            .iter()
            .map(|a| AreaVista {
                slug: a.slug().to_owned(),
                etiqueta: a.etiqueta().to_owned(),
                funciones: state
                    .catalogo
                    .por_area(*a)
                    .map(|f| resumen(state, f))
                    .collect(),
            })
            .collect()
    }

    /// Parámetros que identifican la instancia: los `{p}` de la lectura
    /// pareada y el id de la lista.
    fn identidad(f: &Funcion) -> BTreeSet<String> {
        let mut out: BTreeSet<String> = f
            .lectura_pareada
            .iter()
            .flat_map(|lp| &lp.params)
            .filter_map(|(_, v)| v.strip_prefix('{').and_then(|v| v.strip_suffix('}')))
            .map(str::to_owned)
            .collect();
        if let Some(l) = &f.lista {
            out.insert(l.id_param.clone());
        }
        out
    }

    /// Nombre sin el sufijo de id (`_id` o `_0i`), si lo tiene.
    fn base_de_id(nombre: &str) -> Option<&str> {
        nombre
            .strip_suffix("_id")
            .or_else(|| nombre.strip_suffix("_0i"))
    }

    /// Valor con el que se rellena un parámetro de lectura o una clave:
    /// `ALL` en `all` y `0` en los ids, salvo el de una lista que admite
    /// `all=ALL` (vacío para listarlas todas) y sus alternativas (`x_0i`
    /// frente a `x_id`: se rellena solo el id de la lista).
    fn inicial(f: &Funcion, nombre: &str) -> String {
        if nombre == "all" {
            return "ALL".into();
        }
        let Some(base) = base_de_id(nombre) else {
            return String::new();
        };
        match &f.lista {
            Some(l) if l.id_param == nombre && l.all => String::new(),
            Some(l) if l.id_param != nombre && base_de_id(&l.id_param) == Some(base) => {
                String::new()
            }
            _ => "0".into(),
        }
    }

    fn param_vista(f: &Funcion, p: &catalogo::Parametro) -> ParamVista {
        ParamVista {
            nombre: p.nombre.clone(),
            etiqueta: p.etiqueta.clone(),
            tipo: tipo_campo(p.tipo),
            opciones: p.opciones.clone(),
            inicial: inicial(f, &p.nombre),
        }
    }

    pub fn detalle(state: &AppState, nombre: &str) -> Result<FuncionDetalle, ServerFnError> {
        let f = funcion(state, nombre)?;
        let ident = identidad(f);
        let id_lista = f.lista.as_ref().map(|l| l.id_param.as_str());
        let vigilancia_s = state
            .cambios()
            .map_or(0, |g| g.tiempos().vigilancia(f.riesgo).as_secs());
        Ok(FuncionDetalle {
            resumen: resumen(state, f),
            etiqueta_area: f.area.etiqueta().to_owned(),
            pagina: f.pagina.clone(),
            parametros: f.parametros.iter().map(|p| param_vista(f, p)).collect(),
            lectura_pareada: f.lectura_pareada.as_ref().map(|lp| {
                let area = state
                    .catalogo
                    .funcion(&lp.funcion)
                    .map_or(f.area, |l| l.area);
                (lp.funcion.clone(), area.slug().to_owned())
            }),
            lista: f.lista.as_ref().map(|l| ListaVista {
                id_param: l.id_param.clone(),
                borrar: l.del,
            }),
            claves: f
                .parametros
                .iter()
                .filter(|p| ident.contains(&p.nombre) && Some(p.nombre.as_str()) != id_lista)
                .map(|p| param_vista(f, p))
                .collect(),
            vigilancia_s,
            lectura: modo_lectura(f).map(drop),
        })
    }

    // ---- enmascarado

    /// Nombres de parámetros secretos del catálogo (en minúsculas).
    fn secretos_catalogo(state: &AppState) -> HashSet<String> {
        state
            .catalogo
            .todas()
            .iter()
            .flat_map(|f| &f.parametros)
            .filter(|p| p.tipo == TipoParam::Secret)
            .map(|p| p.nombre.to_ascii_lowercase())
            .collect()
    }

    /// Reglas de enmascarado: `secretos.toml` más los secretos del catálogo.
    struct Ocultar {
        mascara: Arc<catalogo::mask::Mascara>,
        catalogo: HashSet<String>,
    }

    impl Ocultar {
        fn new(state: &AppState) -> Ocultar {
            Ocultar {
                mascara: state.mascara.clone(),
                catalogo: secretos_catalogo(state),
            }
        }

        fn oculto(&self, nombre: &str, valor: &str) -> bool {
            self.mascara.oculta(nombre, valor)
                || self.catalogo.contains(&nombre.to_ascii_lowercase())
        }

        fn valor(&self, nombre: &str, valor: &str) -> Valor {
            if self.oculto(nombre, valor) {
                Valor::Secreto
            } else {
                Valor::Texto(valor.to_owned())
            }
        }
    }

    // ---- lecturas

    enum ModoLectura {
        Xml,
        Pagina,
    }

    /// Única puerta de las lecturas: solo funciones de lectura del catálogo,
    /// no bloqueadas, y nunca las páginas que disparan acciones.
    fn modo_lectura(f: &Funcion) -> Result<ModoLectura, String> {
        if let Some(m) = &f.bloqueada {
            return Err(format!("Función bloqueada: {m}."));
        }
        if PAGINAS_PROHIBIDAS.contains(&f.name.as_str()) {
            return Err("Esta página dispara una acción al cargarse: el panel no la carga.".into());
        }
        if f.tipo != Tipo::Lectura {
            return Err(format!("{} no es una lectura.", f.name));
        }
        if f.name.starts_with("cb") {
            Ok(ModoLectura::Xml)
        } else if f.name.ends_with(".asp") {
            Ok(ModoLectura::Pagina)
        } else {
            Err(
                "Lectura de la web básica con una respuesta que el panel no sabe \
                 interpretar (ni, por tanto, enmascarar): no se ejecuta desde el explorador."
                    .into(),
            )
        }
    }

    /// Campos hoja de un elemento con su ruta (`a.b`).
    type Campos<'a> = Vec<(String, &'a XmlNode)>;

    /// Campos hoja de `n` con su ruta (`a.b`), en orden.
    fn aplanar<'a>(n: &'a XmlNode, prefijo: &str, out: &mut Campos<'a>) {
        let mut vistos: BTreeMap<&str, usize> = BTreeMap::new();
        for c in &n.children {
            if c.tag == "Result" || c.tag == "ErrorString" {
                continue;
            }
            let i = vistos.entry(c.tag.as_str()).or_default();
            let nombre = if *i == 0 {
                format!("{prefijo}{}", c.tag)
            } else {
                format!("{prefijo}{}[{i}]", c.tag)
            };
            *i += 1;
            if c.children.is_empty() {
                out.push((nombre, c));
            } else {
                aplanar(c, &format!("{nombre}."), out);
            }
        }
    }

    fn xml_enmascarado(o: &Ocultar, n: &XmlNode, nivel: usize, out: &mut String) {
        let sangria = "  ".repeat(nivel);
        let escapar = |s: &str| {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        };
        if n.children.is_empty() {
            let texto = if o.oculto(&n.tag, &n.text) {
                MASCARA.to_owned()
            } else {
                escapar(&n.text)
            };
            out.push_str(&format!("{sangria}<{0}>{texto}</{0}>\n", n.tag));
        } else {
            out.push_str(&format!("{sangria}<{}>\n", n.tag));
            for c in &n.children {
                xml_enmascarado(o, c, nivel + 1, out);
            }
            out.push_str(&format!("{sangria}</{}>\n", n.tag));
        }
    }

    /// Identificador de una fila: el valor de `id` si lo tiene, o `#n`.
    fn id_fila(campos: &[(String, &XmlNode)], id: Option<&str>, n: usize) -> String {
        id.and_then(|id| campos.iter().find(|(k, _)| k == id))
            .map(|(_, v)| v.text.clone())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| format!("#{n}"))
    }

    fn id_lista(f: &Funcion) -> Option<&str> {
        f.lista.as_ref().map(|l| l.id_param.as_str())
    }

    fn vista_xml(o: &Ocultar, f: &Funcion, n: &XmlNode) -> Vista {
        let mut pares = Vec::new();
        let mut grupos: Vec<(String, Vec<Campos<'_>>)> = Vec::new();
        for c in &n.children {
            if c.tag == "Result" || c.tag == "ErrorString" {
                continue;
            }
            if c.children.is_empty() {
                pares.push(Par {
                    nombre: c.tag.clone(),
                    valor: o.valor(&c.tag, &c.text),
                });
                continue;
            }
            let mut campos = Vec::new();
            aplanar(c, "", &mut campos);
            match grupos.iter_mut().find(|(t, _)| *t == c.tag) {
                Some((_, filas)) => filas.push(campos),
                None => grupos.push((c.tag.clone(), vec![campos])),
            }
        }
        let grupos = grupos
            .into_iter()
            .map(|(nombre, filas)| {
                let mut columnas: Vec<String> = Vec::new();
                for fila in &filas {
                    for (k, _) in fila {
                        if !columnas.contains(k) {
                            columnas.push(k.clone());
                        }
                    }
                }
                let total = filas.len();
                let filas = filas
                    .iter()
                    .take(MAX_INSTANCIAS)
                    .enumerate()
                    .map(|(i, fila)| Fila {
                        id: id_fila(fila, id_lista(f), i),
                        valores: columnas
                            .iter()
                            .map(|c| match fila.iter().find(|(k, _)| k == c) {
                                Some((_, v)) => o.valor(&v.tag, &v.text),
                                None => Valor::Ausente,
                            })
                            .collect(),
                    })
                    .collect();
                Grupo {
                    nombre,
                    columnas,
                    filas,
                    total,
                }
            })
            .collect();
        let mut crudo = String::new();
        xml_enmascarado(o, n, 0, &mut crudo);
        Vista {
            pares,
            grupos,
            crudo: Some(crudo),
        }
    }

    /// Respuesta en claro de una lectura (solo en memoria).
    enum Respuesta {
        Xml(XmlNode),
        Pagina(BTreeMap<String, String>),
    }

    async fn ejecutar(
        state: &AppState,
        f: &Funcion,
        params: &[(String, String)],
    ) -> Result<Respuesta, ServerFnError> {
        let modo = modo_lectura(f).map_err(|m| con_estado(StatusCode::FORBIDDEN, m))?;
        for (k, v) in params {
            if f.parametro(k).is_none() {
                return Err(invalida(format!("{}: parámetro desconocido {k}", f.name)));
            }
            if v.chars().any(char::is_control) {
                return Err(invalida(format!("{k}: contiene caracteres no válidos")));
            }
        }
        let c = &state.hgu;
        let sin_router = |e: hgu::Error| {
            con_estado(
                StatusCode::BAD_GATEWAY,
                format!("El router no ha respondido a la lectura: {e}"),
            )
        };
        match modo {
            ModoLectura::Xml => {
                let p: Vec<(&str, &str)> = params
                    .iter()
                    .filter(|(_, v)| !v.trim().is_empty())
                    .map(|(k, v)| (k.as_str(), v.trim()))
                    .collect();
                c.call(&f.name, &p)
                    .await
                    .map(Respuesta::Xml)
                    .map_err(sin_router)
            }
            ModoLectura::Pagina => {
                let mut vars =
                    hgu::fmt::page_vars(&c.basic_page(&f.name).await.map_err(sin_router)?);
                vars.remove("sessionKey");
                Ok(Respuesta::Pagina(vars))
            }
        }
    }

    pub async fn leer(
        state: &AppState,
        nombre: &str,
        params: &[(String, String)],
    ) -> Result<Vista, ServerFnError> {
        let f = funcion(state, nombre)?;
        let o = Ocultar::new(state);
        Ok(match ejecutar(state, f, params).await? {
            Respuesta::Xml(n) => vista_xml(&o, f, &n),
            Respuesta::Pagina(vars) => Vista {
                pares: vars
                    .iter()
                    .map(|(k, v)| Par {
                        nombre: k.clone(),
                        valor: o.valor(k, v),
                    })
                    .collect(),
                grupos: Vec::new(),
                crudo: None,
            },
        })
    }

    // ---- formularios

    fn mapa(pares: Vec<(String, String)>) -> BTreeMap<String, String> {
        pares.into_iter().collect()
    }

    /// Claves de instancia aceptables para `f` (sin el id de la lista).
    fn claves_validas(
        f: &Funcion,
        claves: Vec<(String, String)>,
    ) -> Result<BTreeMap<String, String>, ServerFnError> {
        let ident = identidad(f);
        let id = id_lista(f);
        for (k, v) in &claves {
            if !ident.contains(k) || Some(k.as_str()) == id {
                return Err(invalida(format!(
                    "{k} no identifica instancias de {}",
                    f.name
                )));
            }
            if let Some(p) = f.parametro(k) {
                validar_campo(tipo_campo(p.tipo), &p.opciones, v)
                    .map_err(|e| invalida(format!("{}: {e}", p.etiqueta)))?;
            }
        }
        Ok(mapa(claves))
    }

    /// Valor de `nombre` en `m`: exacto o sin distinguir mayúsculas (como el
    /// gestor).
    fn buscar<'a>(m: &'a BTreeMap<String, String>, nombre: &str) -> Option<&'a str> {
        m.get(nombre)
            .or_else(|| {
                m.iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(nombre))
                    .map(|(_, v)| v)
            })
            .map(String::as_str)
    }

    pub async fn formulario(
        state: &AppState,
        nombre: &str,
        instancia: Option<String>,
        claves: Vec<(String, String)>,
    ) -> Result<FormVista, ServerFnError> {
        let f = funcion(state, nombre)?;
        if f.tipo == Tipo::Lectura {
            return Err(invalida(format!(
                "{nombre} es una lectura: no tiene formulario"
            )));
        }
        let g = gestor(state)?;
        g.escribible(nombre).map_err(error_cambio)?;
        let claves = claves_validas(f, claves)?;
        let operacion = match (f.tipo, &f.lista, &instancia) {
            (Tipo::Accion, _, _) => Operacion::Accion,
            (_, Some(_), None) => Operacion::Crear,
            _ => Operacion::Editar,
        };
        let o = Ocultar::new(state);
        // Qué campos se pueden rellenar: en una edición, los que muestra la
        // lectura pareada (el gestor rechaza cambiar uno que no muestra); al
        // crear, los que muestran las instancias que ya hay.
        let actual = match operacion {
            Operacion::Editar => g
                .valores_actuales(nombre, instancia.as_deref(), &claves)
                .await
                .map_err(error_cambio)?
                .unwrap_or_default(),
            Operacion::Crear => {
                let filas = g.instancias(nombre, &claves).await.map_err(error_cambio)?;
                filas
                    .into_iter()
                    .flatten()
                    .map(|(k, _)| (k, String::new()))
                    .collect()
            }
            Operacion::Accion => BTreeMap::new(),
        };
        let ident = identidad(f);
        let mut campos = Vec::new();
        let mut omitidos = Vec::new();
        for p in &f.parametros {
            if ident.contains(&p.nombre) && operacion != Operacion::Accion {
                continue;
            }
            let cur = buscar(&actual, &p.nombre);
            let valor = match (operacion, cur) {
                (Operacion::Accion, _) => String::new(),
                (Operacion::Crear, Some(_)) => String::new(),
                (Operacion::Crear, None) if actual.is_empty() => String::new(),
                (Operacion::Editar, Some(v)) => {
                    if p.tipo == TipoParam::Secret || o.oculto(&p.nombre, v) {
                        MASCARA.to_owned()
                    } else {
                        v.to_owned()
                    }
                }
                (_, None) => {
                    omitidos.push(p.nombre.clone());
                    continue;
                }
            };
            campos.push(CampoForm {
                param: param_vista(f, p),
                valor,
            });
        }
        Ok(FormVista {
            funcion: f.name.clone(),
            operacion,
            instancia,
            claves: claves.into_iter().collect(),
            campos,
            omitidos,
        })
    }

    pub async fn instancias(
        state: &AppState,
        nombre: &str,
        claves: Vec<(String, String)>,
    ) -> Result<ListaInstancias, ServerFnError> {
        let f = funcion(state, nombre)?;
        let g = gestor(state)?;
        g.escribible(nombre).map_err(error_cambio)?;
        let claves = claves_validas(f, claves)?;
        let filas = g.instancias(nombre, &claves).await.map_err(error_cambio)?;
        let o = Ocultar::new(state);
        let mut columnas: Vec<String> = Vec::new();
        for fila in &filas {
            for k in fila.keys() {
                if !columnas.contains(k) {
                    columnas.push(k.clone());
                }
            }
        }
        // El id primero.
        if let Some(id) = id_lista(f)
            && let Some(i) = columnas.iter().position(|c| c == id)
        {
            let c = columnas.remove(i);
            columnas.insert(0, c);
        }
        let total = filas.len();
        Ok(ListaInstancias {
            filas: filas
                .iter()
                .take(MAX_INSTANCIAS)
                .enumerate()
                .map(|(i, fila)| Fila {
                    id: id_lista(f)
                        .and_then(|id| buscar(fila, id))
                        .filter(|v| !v.is_empty())
                        .map_or_else(|| format!("#{i}"), str::to_owned),
                    valores: columnas
                        .iter()
                        .map(|c| fila.get(c).map_or(Valor::Ausente, |v| o.valor(c, v)))
                        .collect(),
                })
                .collect(),
            columnas,
            total,
        })
    }

    // ---- cambios

    pub async fn preparar(state: &AppState, p: Peticion) -> Result<PreparadoVista, ServerFnError> {
        let f = funcion(state, &p.funcion)?;
        let g = gestor(state)?;
        // Validación de tipo de los valores nuevos (el gestor vuelve a validar
        // lo que cambia): un MASCARA o un secreto vacío es "sin tocar".
        for (k, v) in &p.valores {
            let Some(param) = f.parametro(k) else {
                return Err(invalida(format!("{}: parámetro desconocido {k}", f.name)));
            };
            let sin_tocar = v == MASCARA || (param.tipo == TipoParam::Secret && v.is_empty());
            if !sin_tocar && !v.trim().is_empty() {
                validar_campo(tipo_campo(param.tipo), &param.opciones, v)
                    .map_err(|e| invalida(format!("{}: {e}", param.etiqueta)))?;
            }
        }
        let req = ChangeRequest {
            funcion: p.funcion.clone(),
            instancia: p.instancia.clone(),
            valores: mapa(p.valores),
            borrar: p.borrar,
        };
        let prep = g.preparar(req).await.map_err(error_cambio)?;
        let etiqueta = |n: &str| {
            f.parametro(n)
                .map_or_else(|| n.to_owned(), |p| p.etiqueta.clone())
        };
        let valor = |secreto: bool, v: String| {
            if secreto {
                Valor::Secreto
            } else if v.is_empty() {
                Valor::Ausente
            } else {
                Valor::Texto(v)
            }
        };
        Ok(PreparadoVista {
            token: prep.token,
            funcion: f.name.clone(),
            instancia: p.instancia,
            borrar: p.borrar,
            campos: prep
                .diff
                .campos
                .into_iter()
                .map(|c| CampoCambio {
                    etiqueta: etiqueta(&c.nombre),
                    antes: valor(c.secreto, c.antes),
                    despues: valor(c.secreto, c.despues),
                    nombre: c.nombre,
                })
                .collect(),
            riesgo: riesgo(prep.riesgo),
            vigilancia_s: g.tiempos().vigilancia(prep.riesgo).as_secs(),
            palabra: prep.palabra,
        })
    }

    pub async fn confirmar(
        state: &AppState,
        token: &str,
        palabra: Option<&str>,
        usuario: &str,
    ) -> Result<i64, ServerFnError> {
        let g = gestor(state)?;
        let palabra = palabra.map(str::trim).filter(|p| !p.is_empty());
        g.confirmar(token, palabra, usuario)
            .await
            .map_err(error_cambio)
    }

    fn estado(e: Estado) -> EstadoDeCambio {
        match e {
            Estado::Preparado => EstadoDeCambio::Preparado,
            Estado::Aplicando => EstadoDeCambio::Aplicando,
            Estado::Vigilando => EstadoDeCambio::Vigilando,
            Estado::Aplicado => EstadoDeCambio::Aplicado,
            Estado::Revertido => EstadoDeCambio::Revertido,
            Estado::Fallido => EstadoDeCambio::Fallido,
        }
    }

    pub async fn estado_cambio(state: &AppState, id: i64) -> Result<CambioVista, ServerFnError> {
        let g = gestor(state)?;
        let c = g.cambio(id).await.ok_or_else(|| {
            con_estado(StatusCode::NOT_FOUND, format!("No existe el cambio {id}"))
        })?;
        let restante_s = (c.estado == Estado::Vigilando).then(|| {
            let fin = c.updated + g.tiempos().vigilancia(c.riesgo).as_secs() as i64;
            (fin - crate::series::now()).max(0)
        });
        Ok(CambioVista {
            id: c.id,
            estado: estado(c.estado),
            error: c.error,
            nota: c.nota,
            restante_s,
        })
    }

    // ---- revelar

    async fn ip_cliente(state: &AppState) -> Result<std::net::IpAddr, ServerFnError> {
        use axum::extract::ConnectInfo;
        use std::net::SocketAddr;
        let ConnectInfo(peer) = leptos_axum::extract::<ConnectInfo<SocketAddr>>().await?;
        let headers: axum::http::HeaderMap = leptos_axum::extract().await?;
        Ok(crate::auth::client_ip(
            peer.ip(),
            &headers,
            state.cfg.trusted_proxy,
        ))
    }

    /// Valor en claro del campo `campo` de la lectura (de la fila `instancia`
    /// si se da).
    fn valor_de(
        f: &Funcion,
        r: &Respuesta,
        campo: &str,
        instancia: Option<&str>,
    ) -> Option<String> {
        match (r, instancia) {
            (Respuesta::Pagina(vars), None) => vars.get(campo).cloned(),
            (Respuesta::Pagina(_), Some(_)) => None,
            (Respuesta::Xml(n), None) => n
                .children
                .iter()
                .find(|c| c.children.is_empty() && c.tag == campo)
                .map(|c| c.text.clone()),
            (Respuesta::Xml(n), Some(inst)) => {
                // Las posiciones `#n` cuentan dentro de cada elemento, como en
                // la vista.
                let mut pos: BTreeMap<&str, usize> = BTreeMap::new();
                for c in n.children.iter().filter(|c| !c.children.is_empty()) {
                    let i = pos.entry(c.tag.as_str()).or_default();
                    let mut campos = Vec::new();
                    aplanar(c, "", &mut campos);
                    let id = id_fila(&campos, id_lista(f), *i);
                    *i += 1;
                    if id == inst
                        && let Some((_, v)) = campos.iter().find(|(k, _)| k == campo)
                    {
                        return Some(v.text.clone());
                    }
                }
                None
            }
        }
    }

    pub async fn revelar(
        state: &AppState,
        user: &User,
        nombre: &str,
        campo: &str,
        instancia: Option<&str>,
        params: &[(String, String)],
        contrasena: &str,
    ) -> Result<String, ServerFnError> {
        let f = funcion(state, nombre)?;
        let ip = ip_cliente(state).await?;
        match crate::auth::reauthenticate(state, ip, user, contrasena).await {
            Ok(()) => {}
            Err(LoginError::Throttled(s)) => {
                return Err(con_estado(
                    StatusCode::TOO_MANY_REQUESTS,
                    format!("Demasiados intentos: espera {s} s y vuelve a probar."),
                ));
            }
            Err(LoginError::BadCredentials) => {
                return Err(con_estado(
                    StatusCode::FORBIDDEN,
                    "Contraseña del panel incorrecta.",
                ));
            }
            Err(LoginError::Internal(e)) => return Err(interno(e)),
        }
        let r = ejecutar(state, f, params).await?;
        let valor = valor_de(f, &r, campo, instancia).ok_or_else(|| {
            con_estado(
                StatusCode::NOT_FOUND,
                format!("La lectura ya no muestra el campo {campo}; vuelve a leer."),
            )
        })?;
        sqlx::query(
            "INSERT INTO reveals (ts, user_id, funcion, campo, instancia) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(crate::series::now())
        .bind(user.id)
        .bind(&f.name)
        .bind(campo)
        .bind(instancia)
        .execute(&state.db)
        .await
        .map_err(interno)?;
        tracing::info!(
            usuario = %user.username,
            funcion = %f.name,
            campo,
            instancia = instancia.unwrap_or("-"),
            "valor secreto revelado"
        );
        Ok(valor)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn lecturas_prohibidas_y_permitidas() {
            let c = catalogo::Catalogo::load().unwrap();
            for n in [
                "syslog.asp",
                "backup_restore.asp",
                "cbRestoreConfig",
                "cbReboot",
            ] {
                let f = c.funcion(n).unwrap();
                assert!(modo_lectura(f).is_err(), "{n}");
            }
            for n in ["te_ppp_pm.cmd?action=retrieve", "te_wifirefresh.wl"] {
                assert!(modo_lectura(c.funcion(n).unwrap()).is_err(), "{n}");
            }
            assert!(matches!(
                modo_lectura(c.funcion("te_wifi.asp").unwrap()),
                Ok(ModoLectura::Pagina)
            ));
            assert!(matches!(
                modo_lectura(c.funcion("cbGetSummary").unwrap()),
                Ok(ModoLectura::Xml)
            ));
            // Ninguna lectura del catálogo carga una página prohibida.
            for f in c.todas() {
                if PAGINAS_PROHIBIDAS.contains(&f.name.as_str()) {
                    assert!(modo_lectura(f).is_err());
                }
            }
        }

        #[test]
        fn valores_iniciales_de_lectura() {
            let c = catalogo::Catalogo::load().unwrap();
            let f = c.funcion("cbDhcpV4SvrPoolGetStatic").unwrap();
            assert_eq!(inicial(f, "all"), "ALL");
            assert_eq!(inicial(f, "rtm_cfg_dhcpv4_svr_pool_id"), "0");
            // El id de una lista con `all=ALL` se deja vacío para listarlas.
            assert_eq!(inicial(f, "rtm_dhcpv4_svr_pool_static_id"), "");
            let w = c.funcion("cbGetWanIPIntf").unwrap();
            assert_eq!(inicial(w, "ip_intf_0i"), "0");
            let n = c.funcion("cbGetNatPortMapping").unwrap();
            assert_eq!(inicial(n, "dmz"), "");
            // Dos formas de dar la misma instancia: solo el id de la lista.
            let a = c.funcion("cbGetWifiAccPt").unwrap();
            assert_eq!(inicial(a, "dpm_cfg_wifi_acc_pt_id"), "0");
            assert_eq!(inicial(a, "dpm_cfg_wifi_acc_pt_0i"), "");
        }

        #[test]
        fn aplana_y_enmascara_anidados() {
            let n = hgu::xml::parse_response(
                "cbX",
                "<Response><cbX><a>1</a><e><id>3</id><key_phrase>s3cr3t</key_phrase>\
                 <sub><password>p</password><x>y</x></sub><sub><x>z</x></sub></e>\
                 <Result>0</Result></cbX></Response>",
            )
            .unwrap();
            let mut campos = Vec::new();
            aplanar(n.get("e").unwrap(), "", &mut campos);
            let nombres: Vec<_> = campos.iter().map(|(k, _)| k.as_str()).collect();
            assert_eq!(
                nombres,
                ["id", "key_phrase", "sub.password", "sub.x", "sub[1].x"]
            );
        }
    }
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
