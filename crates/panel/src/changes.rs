//! Gestor de cambios: toda escritura al router pasa por aquí.
//!
//! 1. [`ChangeManager::preparar`]: lee la lectura pareada, calcula el diff
//!    (secretos enmascarados) y devuelve un token de un solo uso (10 min).
//!    Rechaza funciones bloqueadas y peticiones que no cambian nada.
//! 2. [`ChangeManager::confirmar`]: con el sondeo en pausa, vuelve a leer,
//!    guarda el cambio (`aplicando`) con los valores anteriores (los secretos,
//!    cifrados), escribe la instancia completa y relee para verificar. Si el
//!    router no muestra lo escrito, `fallido` (no se revierte lo que el
//!    router no aceptó).
//! 3. Vigilancia (`vigilando`), ya sin pausar el sondeo: cada `intervalo`, el
//!    router deja iniciar sesión y la WAN `ip2` sigue arriba. Tras
//!    `fallos_para_revertir` fallos seguidos se escriben los valores
//!    anteriores, se verifica y queda `revertido`; si no, `aplicado`.
//!    `cbReboot` (palabra `REINICIAR`) espera la vuelta del router sin
//!    revertir.
//!
//! Un único cambio activo a la vez: otro `confirmar` mientras tanto recibe
//! [`CambioError::EnCurso`]. Al arrancar, [`ChangeManager::reanudar_pendientes`]
//! retoma los cambios que quedaron en `aplicando`/`vigilando`.
//!
//! Un valor secreto que llega como [`MASCARA`] o vacío significa "sin tocar":
//! se reenvía el valor actual leído del router y no cuenta como cambio. Los
//! registros (`tracing`) y el historial solo muestran secretos enmascarados.

use crate::{series, state::AppState};
use catalogo::{
    Funcion, Parametro, Riesgo, Tipo, TipoParam,
    mask::{MASCARA, Mascara},
};
use hgu::{
    fmt::{mess_userpass, page_vars, split_mac},
    read,
    xml::XmlNode,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    net::Ipv4Addr,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};
use tokio::{task::AbortHandle, time::Instant};

/// Palabra que hay que escribir para reiniciar el router.
pub const PALABRA_REINICIO: &str = "REINICIAR";
/// Palabra para las demás acciones.
pub const PALABRA_ACCION: &str = "CONFIRMAR";
/// Vida de un token de [`ChangeManager::preparar`].
const TOKEN_TTL: Duration = Duration::from_secs(10 * 60);
/// Interfaz de la conexión PPP principal.
const WAN: &str = "ip2";
const REBOOT: &str = "cbReboot";

/// Tiempos del gestor.
#[derive(Clone, Debug)]
pub struct Tiempos {
    /// Vigilancia de un cambio de riesgo bajo (30 s).
    pub bajo: Duration,
    /// Riesgo medio (2 min).
    pub medio: Duration,
    /// Riesgo alto (5 min); también la espera máxima de la vuelta tras un
    /// reinicio y el plazo para conseguir revertir.
    pub alto: Duration,
    /// Entre comprobaciones de la vigilancia (5 s).
    pub intervalo: Duration,
    /// Fallos seguidos que provocan la reversión (3).
    pub fallos_para_revertir: u32,
    /// Espera máxima a que el sondeo se detenga antes de escribir (30 s).
    pub pausa_max: Duration,
}

impl Default for Tiempos {
    fn default() -> Self {
        Tiempos {
            bajo: Duration::from_secs(30),
            medio: Duration::from_secs(120),
            alto: Duration::from_secs(300),
            intervalo: Duration::from_secs(5),
            fallos_para_revertir: 3,
            pausa_max: Duration::from_secs(30),
        }
    }
}

impl Tiempos {
    /// Duración de la vigilancia según el riesgo.
    pub fn vigilancia(&self, riesgo: Riesgo) -> Duration {
        match riesgo {
            Riesgo::Bajo => self.bajo,
            Riesgo::Medio => self.medio,
            Riesgo::Alto => self.alto,
        }
    }
}

/// Petición de cambio.
///
/// En las funciones con `lista`, `instancia` es el id de la instancia que se
/// edita o borra (`borrar`); `None` crea una nueva (id `-1`). Los campos
/// IP/MAC partidos llegan unidos con su nombre base.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeRequest {
    pub funcion: String,
    pub instancia: Option<String>,
    pub valores: BTreeMap<String, String>,
    pub borrar: bool,
}

/// Un campo del diff. Si `secreto`, `antes` y `despues` valen [`MASCARA`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampoDiff {
    pub nombre: String,
    pub antes: String,
    pub despues: String,
    pub secreto: bool,
}

/// Diferencias entre el router y la petición.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diff {
    pub campos: Vec<CampoDiff>,
}

/// Resultado de [`ChangeManager::preparar`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preparado {
    /// Token para [`ChangeManager::confirmar`] (un solo uso, 10 min).
    pub token: String,
    pub diff: Diff,
    pub riesgo: Riesgo,
    /// Palabra que hay que escribir para confirmar (acciones).
    pub palabra: Option<String>,
}

/// Errores del gestor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CambioError {
    /// La función está bloqueada (motivo).
    Bloqueada(String),
    /// La petición no cambia nada.
    SinCambios,
    /// Hay otro cambio en curso.
    EnCurso,
    /// El router (o el panel) no pudo hacer el cambio.
    Router(String),
    /// El router no muestra lo que se escribió.
    Verificacion(Vec<String>),
    /// Falta la palabra de confirmación o no coincide.
    PalabraIncorrecta,
    /// Token desconocido, caducado o ya usado.
    TokenInvalido,
    /// Petición no válida (función, parámetro o valor).
    Invalida(String),
}

impl std::fmt::Display for CambioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CambioError::Bloqueada(m) => write!(f, "función bloqueada: {m}"),
            CambioError::SinCambios => f.write_str("no hay nada que cambiar"),
            CambioError::EnCurso => f.write_str("hay otro cambio en curso"),
            CambioError::Router(e) => write!(f, "{e}"),
            CambioError::Verificacion(v) => {
                write!(f, "el router no muestra el cambio: {}", v.join("; "))
            }
            CambioError::PalabraIncorrecta => f.write_str("la palabra de confirmación no coincide"),
            CambioError::TokenInvalido => {
                f.write_str("el cambio preparado ha caducado o ya se usó; revísalo de nuevo")
            }
            CambioError::Invalida(e) => write!(f, "petición no válida: {e}"),
        }
    }
}

impl std::error::Error for CambioError {}

/// Estado de un cambio en el historial.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Estado {
    Preparado,
    Aplicando,
    Vigilando,
    Aplicado,
    Revertido,
    Fallido,
}

impl Estado {
    /// Nombre en la base de datos.
    pub fn as_str(self) -> &'static str {
        match self {
            Estado::Preparado => "preparado",
            Estado::Aplicando => "aplicando",
            Estado::Vigilando => "vigilando",
            Estado::Aplicado => "aplicado",
            Estado::Revertido => "revertido",
            Estado::Fallido => "fallido",
        }
    }

    fn parse(s: &str) -> Option<Estado> {
        [
            Estado::Preparado,
            Estado::Aplicando,
            Estado::Vigilando,
            Estado::Aplicado,
            Estado::Revertido,
            Estado::Fallido,
        ]
        .into_iter()
        .find(|e| e.as_str() == s)
    }
}

/// Un cambio del historial (secretos enmascarados).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cambio {
    pub id: i64,
    /// Creación y última actualización (segundos Unix).
    pub ts: i64,
    pub updated: i64,
    pub funcion: String,
    pub instancia: Option<String>,
    pub riesgo: Riesgo,
    pub estado: Estado,
    pub diff: Diff,
    pub error: Option<String>,
    pub nota: Option<String>,
    pub usuario: Option<String>,
}

// ------------------------------------------------------------- planes

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operacion {
    Editar,
    Crear,
    Borrar,
    Accion,
}

impl Operacion {
    fn as_str(self) -> &'static str {
        match self {
            Operacion::Editar => "editar",
            Operacion::Crear => "crear",
            Operacion::Borrar => "borrar",
            Operacion::Accion => "accion",
        }
    }

    fn parse(s: &str) -> Option<Operacion> {
        [
            Operacion::Editar,
            Operacion::Crear,
            Operacion::Borrar,
            Operacion::Accion,
        ]
        .into_iter()
        .find(|o| o.as_str() == s)
    }
}

/// Dónde leer lo que escribe una función.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Lectura {
    /// Llamada `cb*` de la UI avanzada. `clave_id`: parámetro de la lectura
    /// que lleva el id de la instancia (se quita para listarlas todas).
    Xml {
        funcion: String,
        params: Vec<(String, String)>,
        clave_id: Option<String>,
    },
    /// Variables `var x='…'` de una página de la UI básica.
    Pagina(String),
    /// Sin lectura comprobable.
    Ninguna,
}

/// Un campo que cambia, con los valores en claro (solo en memoria).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Campo {
    nombre: String,
    antes: String,
    despues: String,
    secreto: bool,
    /// El router mostraba el campo: `antes` es su valor real.
    leido: bool,
}

/// Cambio listo para escribir y comprobar.
#[derive(Clone, Debug)]
struct Plan {
    funcion: Funcion,
    operacion: Operacion,
    instancia: Option<String>,
    /// Valores de la petición, con los secretos sin tocar sustituidos por el
    /// valor actual (o quitados si no se conoce) y el id de la instancia.
    valores: BTreeMap<String, String>,
    /// Lo que se envía (sin `sessionKey`).
    payload: Vec<(String, String)>,
    campos: Vec<Campo>,
    lectura: Lectura,
}

/// Valor de `nombre` en `m`: primero exacto, luego sin distinguir mayúsculas.
fn buscar<'a>(m: &'a BTreeMap<String, String>, nombre: &str) -> Option<&'a str> {
    m.get(nombre)
        .or_else(|| {
            m.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nombre))
                .map(|(_, v)| v)
        })
        .map(String::as_str)
}

/// Iguales una vez normalizados: sin espacios en los extremos; IP y MAC sin
/// distinguir mayúsculas.
fn iguales(p: Option<&Parametro>, a: &str, b: &str) -> bool {
    let (a, b) = (a.trim(), b.trim());
    match p.map(|p| p.tipo) {
        Some(TipoParam::Ip | TipoParam::Mac) => a.eq_ignore_ascii_case(b),
        _ => a == b,
    }
}

fn es_secreto_param(f: &Funcion, m: &Mascara, nombre: &str) -> bool {
    f.parametro(nombre)
        .is_some_and(|p| p.tipo == TipoParam::Secret)
        || m.es_secreto(nombre)
}

/// Comprueba el valor nuevo de un parámetro.
fn validar(p: &Parametro, v: &str) -> Result<(), String> {
    let n = &p.nombre;
    let ok = match p.tipo {
        TipoParam::Ip => v.trim().parse::<Ipv4Addr>().is_ok(),
        TipoParam::Mac => split_mac(v.trim()).is_some_and(|parts| {
            parts
                .iter()
                .all(|x| x.len() == 2 && x.bytes().all(|b| b.is_ascii_hexdigit()))
        }),
        TipoParam::Bool => matches!(v.trim(), "0" | "1"),
        TipoParam::Enum => p.opciones.iter().any(|(o, _)| o == v.trim()),
        TipoParam::Int => v.trim().parse::<i64>().is_ok(),
        TipoParam::Text | TipoParam::Secret => !v.chars().any(char::is_control),
    };
    if ok {
        Ok(())
    } else if p.tipo == TipoParam::Secret {
        Err(format!("{n}: contiene caracteres no válidos"))
    } else {
        Err(format!("{n}: «{v}» no es un valor válido"))
    }
}

/// Parámetros que identifican la instancia: los que usa la lectura pareada
/// (`{param}`) y el id de la lista.
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

/// Lectura pareada con los `{param}` sustituidos por `valores` (o la
/// instancia, para el id de la lista).
fn lectura_de(
    f: &Funcion,
    valores: &BTreeMap<String, String>,
    instancia: Option<&str>,
) -> Result<Lectura, CambioError> {
    let Some(lp) = &f.lectura_pareada else {
        return Ok(Lectura::Ninguna);
    };
    if lp.funcion.ends_with(".asp") {
        return Ok(Lectura::Pagina(lp.funcion.clone()));
    }
    if !lp.funcion.starts_with("cb") {
        // Lecturas `.cmd` de la UI básica (texto sin formato conocido).
        return Ok(Lectura::Ninguna);
    }
    let id_param = f.lista.as_ref().map(|l| l.id_param.as_str());
    let mut params = Vec::new();
    let mut clave_id = None;
    for (k, v) in &lp.params {
        let valor = match v.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
            Some(p) => {
                if Some(p) == id_param {
                    clave_id = Some(k.clone());
                }
                valores
                    .get(p)
                    .map(String::as_str)
                    .or(if Some(p) == id_param { instancia } else { None })
                    .ok_or_else(|| CambioError::Invalida(format!("falta el parámetro {p}")))?
                    .to_owned()
            }
            None if Some(k.as_str()) == id_param => {
                clave_id = Some(k.clone());
                instancia.unwrap_or(v).to_owned()
            }
            None => v.clone(),
        };
        params.push((k.clone(), valor));
    }
    Ok(Lectura::Xml {
        funcion: lp.funcion.clone(),
        params,
        clave_id,
    })
}

/// Campos hoja de un elemento.
fn hojas(n: &XmlNode) -> BTreeMap<String, String> {
    n.children
        .iter()
        .filter(|c| c.children.is_empty() && c.tag != "Result" && c.tag != "ErrorString")
        .map(|c| (c.tag.clone(), c.text.clone()))
        .collect()
}

/// Elementos (hijos con hijos) de una respuesta; si no hay, la propia
/// respuesta.
fn elementos(n: &XmlNode) -> Vec<BTreeMap<String, String>> {
    let v: Vec<_> = n
        .children
        .iter()
        .filter(|c| !c.children.is_empty())
        .map(hojas)
        .collect();
    if v.is_empty() { vec![hojas(n)] } else { v }
}

/// Resultado de [`componer`].
struct Compuesto {
    /// Valores de la petición con los secretos sin tocar resueltos.
    efectivos: BTreeMap<String, String>,
    /// Carga a enviar.
    payload: Vec<(String, String)>,
    /// Campos que cambian.
    campos: Vec<Campo>,
}

/// Une valores de la petición y valores actuales. No hace peticiones.
fn componer(
    f: &Funcion,
    m: &Mascara,
    op: Operacion,
    actual: &BTreeMap<String, String>,
    valores: &BTreeMap<String, String>,
) -> Compuesto {
    let ident = identidad(f);
    // Sin tocar (un secreto vacío o cualquier campo que llega enmascarado):
    // valor actual, o fuera si no se conoce. `MASCARA` nunca se escribe.
    let mut efectivos = BTreeMap::new();
    for (k, v) in valores {
        let p = f.parametro(k);
        if v == MASCARA || (es_secreto_param(f, m, k) && v.trim().is_empty()) {
            if let Some(cur) = buscar(actual, k) {
                efectivos.insert(k.clone(), cur.to_owned());
            }
            continue;
        }
        let v = match p.map(|p| p.tipo) {
            Some(TipoParam::Text | TipoParam::Secret) | None => v.clone(),
            Some(_) => v.trim().to_owned(),
        };
        efectivos.insert(k.clone(), v);
    }
    let mut payload = Vec::new();
    for p in &f.parametros {
        if let Some(v) = efectivos.get(&p.nombre) {
            payload.push((p.nombre.clone(), v.clone()));
        } else if op == Operacion::Editar
            && let Some(cur) = buscar(actual, &p.nombre)
        {
            payload.push((p.nombre.clone(), cur.to_owned()));
        }
    }
    let secreto =
        |k: &str, a: &str, b: &str| es_secreto_param(f, m, k) || m.oculta(k, a) || m.oculta(k, b);
    let mut campos = Vec::new();
    if op == Operacion::Borrar {
        payload.push(("del".into(), "DEL".into()));
        for p in &f.parametros {
            if ident.contains(&p.nombre) || efectivos.contains_key(&p.nombre) {
                continue;
            }
            if let Some(cur) = buscar(actual, &p.nombre) {
                campos.push(Campo {
                    nombre: p.nombre.clone(),
                    antes: cur.to_owned(),
                    despues: String::new(),
                    secreto: secreto(&p.nombre, cur, ""),
                    leido: true,
                });
            }
        }
    } else {
        for p in &f.parametros {
            let Some(v) = efectivos.get(&p.nombre) else {
                continue;
            };
            if ident.contains(&p.nombre) && op != Operacion::Accion {
                continue;
            }
            let cur = buscar(actual, &p.nombre);
            let antes = cur.unwrap_or_default();
            if op != Operacion::Accion && iguales(Some(p), antes, v) {
                continue;
            }
            campos.push(Campo {
                nombre: p.nombre.clone(),
                antes: antes.to_owned(),
                despues: v.clone(),
                secreto: secreto(&p.nombre, antes, v),
                leido: cur.is_some(),
            });
        }
    }
    Compuesto {
        efectivos,
        payload,
        campos,
    }
}

/// Petición que deshace `plan` (`None` si no se puede: acciones, o sin
/// valores anteriores conocidos).
fn inversa(plan: &Plan) -> Option<ChangeRequest> {
    let f = &plan.funcion;
    let ids = identidad(f);
    let id_param = f.lista.as_ref().map(|l| l.id_param.as_str());
    // Lo que identifica la instancia (el id de la lista va en `instancia`).
    let mut valores: BTreeMap<String, String> = plan
        .valores
        .iter()
        .filter(|(k, _)| ids.contains(*k) && Some(k.as_str()) != id_param)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let anteriores: Vec<(String, String)> = plan
        .campos
        .iter()
        .filter(|c| c.leido)
        .map(|c| (c.nombre.clone(), c.antes.clone()))
        .collect();
    let (instancia, borrar) = match plan.operacion {
        Operacion::Accion => return None,
        Operacion::Editar if anteriores.is_empty() => return None,
        Operacion::Editar => {
            valores.extend(anteriores);
            (plan.instancia.clone(), false)
        }
        Operacion::Crear => (Some(plan.instancia.clone()?), true),
        Operacion::Borrar => {
            valores.extend(anteriores);
            (None, false)
        }
    };
    Some(ChangeRequest {
        funcion: f.name.clone(),
        instancia,
        valores,
        borrar,
    })
}

/// Qué muestra el router respecto a un plan.
#[derive(Debug, PartialEq, Eq)]
enum Comprobacion {
    /// Coincide; con la instancia nueva si se creó una.
    Coincide(Option<String>),
    /// No hay forma de comprobarlo (sin lectura o sin campos comparables).
    Limitada,
    /// No coincide (motivos, secretos enmascarados).
    Difiere(Vec<String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lado {
    Nuevo,
    Anterior,
}

/// Referencia para la vigilancia, tomada antes de escribir.
#[derive(Clone, Copy, Debug)]
enum Base {
    /// Si la WAN estaba arriba.
    Wan(bool),
    /// Tiempo encendido antes de `cbReboot`.
    Uptime(u64),
}

// ------------------------------------------------------------- gestor

struct Pendiente {
    req: ChangeRequest,
    caduca: Instant,
}

/// Mientras viva, el gestor está ocupado con un cambio.
struct Reserva {
    m: Arc<ChangeManager>,
}

impl Reserva {
    fn fijar(&self, id: i64) {
        *lock(&self.m.activo) = Some(Some(id));
    }
}

impl Drop for Reserva {
    fn drop(&mut self) {
        *lock(&self.m.activo) = None;
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Gestor de cambios. Uno por proceso.
pub struct ChangeManager {
    state: AppState,
    tiempos: Tiempos,
    pendientes: Mutex<HashMap<String, Pendiente>>,
    /// Cambio activo: `Some(None)` reservado y aún sin id.
    activo: Mutex<Option<Option<i64>>>,
    /// Vigilancia en segundo plano.
    tarea: Mutex<Option<AbortHandle>>,
}

impl ChangeManager {
    /// Crea el gestor.
    pub fn new(state: AppState, tiempos: Tiempos) -> Arc<ChangeManager> {
        Arc::new(ChangeManager {
            state,
            tiempos,
            pendientes: Mutex::new(HashMap::new()),
            activo: Mutex::new(None),
            tarea: Mutex::new(None),
        })
    }

    /// Id del cambio en curso (aplicándose o vigilándose).
    pub fn activo(&self) -> Option<i64> {
        lock(&self.activo).flatten()
    }

    /// Detiene la vigilancia en curso sin cerrar el cambio (queda como al
    /// cortarse el proceso; [`ChangeManager::reanudar_pendientes`] lo retoma).
    pub fn detener(&self) {
        if let Some(t) = lock(&self.tarea).take() {
            t.abort();
        }
    }

    /// Lee el router y calcula el diff de `req`. Devuelve un token para
    /// [`ChangeManager::confirmar`].
    pub async fn preparar(&self, req: ChangeRequest) -> Result<Preparado, CambioError> {
        let plan = self.planificar(&req).await?;
        let token = nuevo_token();
        let palabra = palabra_requerida(&plan.funcion).map(str::to_owned);
        let mut p = lock(&self.pendientes);
        let ahora = Instant::now();
        p.retain(|_, v| v.caduca > ahora);
        p.insert(
            token.clone(),
            Pendiente {
                req,
                caduca: ahora + TOKEN_TTL,
            },
        );
        Ok(Preparado {
            token,
            diff: visible(&plan.campos),
            riesgo: plan.funcion.riesgo,
            palabra,
        })
    }

    /// Aplica el cambio preparado con `token`: escribe, verifica y deja la
    /// vigilancia en marcha. Devuelve el id del cambio (en `vigilando`).
    pub async fn confirmar(
        self: &Arc<Self>,
        token: &str,
        palabra: Option<&str>,
        usuario: &str,
    ) -> Result<i64, CambioError> {
        let req = {
            let p = lock(&self.pendientes);
            match p.get(token) {
                Some(v) if v.caduca > Instant::now() => v.req.clone(),
                _ => return Err(CambioError::TokenInvalido),
            }
        };
        let f = self.funcion_escribible(&req.funcion)?.clone();
        if let Some(esperada) = palabra_requerida(&f)
            && palabra != Some(esperada)
        {
            return Err(CambioError::PalabraIncorrecta);
        }
        let reserva = self.reservar()?;
        if lock(&self.pendientes).remove(token).is_none() {
            return Err(CambioError::TokenInvalido);
        }
        let pausa = tokio::time::timeout(self.tiempos.pausa_max, self.state.poller.pause())
            .await
            .map_err(|_| {
                CambioError::Router("el sondeo no se detuvo a tiempo; no se ha escrito nada".into())
            })?;
        let plan = self.planificar(&req).await?;
        let c = &self.state.hgu;
        let no_responde =
            |e: hgu::Error| CambioError::Router(format!("el router no responde: {e}"));
        let base = if f.name == REBOOT {
            Base::Uptime(read::summary(c).await.map_err(no_responde)?.uptime_s)
        } else {
            Base::Wan(wan_arriba(&read::wan(c).await.map_err(no_responde)?))
        };
        let id = self
            .insertar(&plan, usuario)
            .await
            .map_err(|e| CambioError::Router(format!("no se puede guardar el cambio: {e}")))?;
        reserva.fijar(id);
        tracing::info!(
            cambio = id,
            funcion = %f.name,
            instancia = plan.instancia.as_deref().unwrap_or("-"),
            usuario,
            diff = %resumen(&plan.campos),
            "cambio aplicándose"
        );
        let mut nota = None;
        if let Err(e) = self.enviar(&plan).await {
            let sin_respuesta = f.name == REBOOT && !matches!(e, hgu::Error::CallFailed { .. });
            if !sin_respuesta {
                let e = e.to_string();
                self.marcar(id, Estado::Fallido, Some(&e), None).await;
                return Err(CambioError::Router(e));
            }
            nota = Some(format!(
                "el router no respondió a la orden de reinicio ({e})"
            ));
        }
        match self.comprobar(&plan, Lado::Nuevo).await {
            Comprobacion::Coincide(nueva) => {
                if let Some(n) = nueva {
                    self.fijar_instancia(id, &n).await;
                }
            }
            Comprobacion::Limitada => {
                nota =
                    Some("verificación limitada: el router no muestra los campos escritos".into());
            }
            Comprobacion::Difiere(v) => {
                self.marcar(id, Estado::Fallido, Some(&v.join("; ")), None)
                    .await;
                return Err(CambioError::Verificacion(v));
            }
        }
        self.marcar(id, Estado::Vigilando, None, nota.as_deref())
            .await;
        drop(pausa);
        self.vigilar_en_segundo_plano(id, plan, base, nota, reserva);
        Ok(id)
    }

    /// Prepara el cambio inverso de uno `aplicado` (con los valores
    /// anteriores, secretos incluidos).
    pub async fn deshacer(&self, change_id: i64) -> Result<Preparado, CambioError> {
        let fila = self
            .fila(change_id)
            .await
            .ok_or_else(|| CambioError::Invalida("el cambio no existe".into()))?;
        let estado: String = fila.get("estado");
        if estado != Estado::Aplicado.as_str() {
            return Err(CambioError::Invalida(
                "solo se puede deshacer un cambio aplicado".into(),
            ));
        }
        let plan = self.reconstruir(&fila).map_err(CambioError::Invalida)?;
        let req = inversa(&plan).ok_or_else(|| {
            CambioError::Invalida(
                "este cambio no se puede deshacer (acción o valores anteriores desconocidos)"
                    .into(),
            )
        })?;
        self.preparar(req).await
    }

    /// Retoma los cambios que quedaron en `aplicando`/`vigilando` (llamar al
    /// arrancar, con el sondeo ya en marcha). Si el router muestra los
    /// valores nuevos, la vigilancia empieza de cero; si muestra los
    /// anteriores, `fallido` ("interrumpido antes de aplicar"); si no se
    /// puede saber, `fallido` con el motivo.
    pub async fn reanudar_pendientes(self: &Arc<Self>) {
        let filas = match sqlx::query(
            "SELECT * FROM changes WHERE estado IN ('aplicando', 'vigilando') ORDER BY id",
        )
        .fetch_all(&self.state.db)
        .await
        {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("no se pueden leer los cambios pendientes: {e}");
                return;
            }
        };
        for fila in filas {
            let id: i64 = fila.get("id");
            match self.reconstruir(&fila) {
                Ok(plan) => {
                    let aplicando = fila.get::<String, _>("estado") == Estado::Aplicando.as_str();
                    self.reanudar(id, plan, aplicando).await
                }
                Err(e) => self.marcar(id, Estado::Fallido, Some(&e), None).await,
            }
        }
    }

    /// Retoma un cambio; `aplicando`: se cortó antes de terminar de
    /// escribir y verificar.
    async fn reanudar(self: &Arc<Self>, id: i64, plan: Plan, aplicando: bool) {
        const RETOMADA: &str = "vigilancia retomada tras un reinicio del panel";
        let c = &self.state.hgu;
        if plan.funcion.name == REBOOT {
            match read::summary(c).await {
                // No se sabe si la orden llegó al router.
                Ok(_) if aplicando => {
                    let e = "interrumpido: no se sabe si el router llegó a reiniciarse";
                    self.marcar(id, Estado::Fallido, Some(e), None).await;
                }
                Ok(_) => {
                    self.marcar(id, Estado::Aplicado, None, Some(RETOMADA))
                        .await
                }
                Err(e) => {
                    let e = format!("el router no responde tras reiniciar el panel: {e}");
                    self.marcar(id, Estado::Fallido, Some(&e), None).await;
                }
            }
            return;
        }
        let nota = match self.comprobar(&plan, Lado::Nuevo).await {
            Comprobacion::Coincide(_) => RETOMADA.to_owned(),
            Comprobacion::Limitada => format!("{RETOMADA}; verificación limitada"),
            Comprobacion::Difiere(v) => {
                let anterior = self.comprobar(&plan, Lado::Anterior).await;
                let e = if anterior == Comprobacion::Coincide(None) {
                    "interrumpido antes de aplicar: el router tiene los valores anteriores"
                        .to_owned()
                } else {
                    format!(
                        "interrumpido; el router no muestra el cambio: {}",
                        v.join("; ")
                    )
                };
                self.marcar(id, Estado::Fallido, Some(&e), None).await;
                return;
            }
        };
        let Ok(reserva) = self.reservar() else {
            let e = "interrumpido; había otro cambio en curso al retomarlo";
            self.marcar(id, Estado::Fallido, Some(e), None).await;
            return;
        };
        reserva.fijar(id);
        // Sin saber cómo estaba la WAN antes, se exige que esté arriba.
        let base = Base::Wan(read::wan(c).await.map(|w| wan_arriba(&w)).unwrap_or(true));
        self.marcar(id, Estado::Vigilando, None, Some(&nota)).await;
        self.vigilar_en_segundo_plano(id, plan, base, Some(nota), reserva);
    }

    /// Un cambio del historial.
    pub async fn cambio(&self, id: i64) -> Option<Cambio> {
        self.fila(id).await.and_then(|f| a_cambio(&f))
    }

    /// Los `limite` cambios más recientes, del último al primero.
    pub async fn historial(&self, limite: u32) -> Vec<Cambio> {
        match sqlx::query("SELECT * FROM changes ORDER BY id DESC LIMIT ?")
            .bind(limite)
            .fetch_all(&self.state.db)
            .await
        {
            Ok(filas) => filas.iter().filter_map(a_cambio).collect(),
            Err(e) => {
                tracing::error!("no se puede leer el historial: {e}");
                Vec::new()
            }
        }
    }

    // ---- planificación

    fn funcion_escribible(&self, nombre: &str) -> Result<&Funcion, CambioError> {
        let f = self
            .state
            .catalogo
            .funcion(nombre)
            .ok_or_else(|| CambioError::Invalida(format!("función desconocida: {nombre}")))?;
        if let Some(m) = &f.bloqueada {
            return Err(CambioError::Bloqueada(m.clone()));
        }
        if f.tipo == Tipo::Lectura {
            return Err(CambioError::Invalida(format!("{nombre} es una lectura")));
        }
        if f.pagina.is_none() {
            return Err(CambioError::Invalida(format!("{nombre}: sin página")));
        }
        Ok(f)
    }

    async fn planificar(&self, req: &ChangeRequest) -> Result<Plan, CambioError> {
        let f = self.funcion_escribible(&req.funcion)?;
        for k in req.valores.keys() {
            if f.parametro(k).is_none() {
                return Err(CambioError::Invalida(format!(
                    "{}: parámetro desconocido {k}",
                    f.name
                )));
            }
        }
        let op = match (f.tipo, &f.lista, req.borrar, &req.instancia) {
            (Tipo::Accion, _, false, _) => Operacion::Accion,
            (Tipo::Escritura, Some(l), true, Some(_)) if l.del => Operacion::Borrar,
            (_, _, true, _) => {
                return Err(CambioError::Invalida(format!(
                    "{}: no se pueden borrar instancias así",
                    f.name
                )));
            }
            (_, Some(_), false, None) => Operacion::Crear,
            _ => Operacion::Editar,
        };
        let mut valores = req.valores.clone();
        if let Some(l) = &f.lista
            && op != Operacion::Accion
            && f.parametro(&l.id_param).is_some()
        {
            let id = req.instancia.as_deref().unwrap_or("-1");
            if valores.get(&l.id_param).is_some_and(|v| v != id) {
                return Err(CambioError::Invalida(format!(
                    "{} no coincide con la instancia",
                    l.id_param
                )));
            }
            valores.insert(l.id_param.clone(), id.to_owned());
        }
        let lectura = lectura_de(f, &valores, req.instancia.as_deref())?;
        let actual = match op {
            Operacion::Crear | Operacion::Accion => BTreeMap::new(),
            Operacion::Editar | Operacion::Borrar => self.leer(&lectura).await.map_err(|e| {
                CambioError::Router(format!("no se puede leer el estado actual: {e}"))
            })?,
        };
        let Compuesto {
            efectivos,
            payload,
            campos,
        } = componer(f, &self.state.mascara, op, &actual, &valores);
        if op != Operacion::Borrar {
            for c in &campos {
                if let Some(p) = f.parametro(&c.nombre) {
                    validar(p, &c.despues).map_err(CambioError::Invalida)?;
                }
            }
        }
        if campos.is_empty() && matches!(op, Operacion::Editar | Operacion::Crear) {
            return Err(CambioError::SinCambios);
        }
        if payload.iter().any(|(_, v)| v == MASCARA) {
            return Err(CambioError::Invalida(
                "un valor enmascarado no se puede escribir en el router".into(),
            ));
        }
        Ok(Plan {
            funcion: f.clone(),
            operacion: op,
            instancia: req.instancia.clone(),
            valores: efectivos,
            payload,
            campos,
            lectura,
        })
    }

    /// Valores de la instancia que muestra `l` (vacío si no hay lectura).
    async fn leer(&self, l: &Lectura) -> Result<BTreeMap<String, String>, hgu::Error> {
        let c = &self.state.hgu;
        match l {
            Lectura::Xml {
                funcion, params, ..
            } => {
                let p: Vec<(&str, &str)> = params
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                let n = c.call(funcion, &p).await?;
                Ok(elementos(&n).into_iter().next().unwrap_or_default())
            }
            Lectura::Pagina(page) => {
                let mut vars = page_vars(&c.basic_page(page).await?);
                vars.remove("sessionKey");
                Ok(vars)
            }
            Lectura::Ninguna => Ok(BTreeMap::new()),
        }
    }

    /// Todas las instancias de la lista (lectura sin el id, con `all=ALL`).
    async fn listar(&self, l: &Lectura) -> Result<Vec<BTreeMap<String, String>>, hgu::Error> {
        let Lectura::Xml {
            funcion,
            params,
            clave_id,
        } = l
        else {
            return Ok(Vec::new());
        };
        let mut p: Vec<(&str, &str)> = params
            .iter()
            .filter(|(k, _)| Some(k) != clave_id.as_ref())
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        if !p.iter().any(|(k, _)| *k == "all") {
            p.push(("all", "ALL"));
        }
        let n = self.state.hgu.call(funcion, &p).await?;
        Ok(n.children
            .iter()
            .filter(|c| !c.children.is_empty())
            .map(hojas)
            .collect())
    }

    /// Envía la carga del plan (las claves de la UI básica, ofuscadas con
    /// XOR 0x1f como hace la página).
    async fn enviar(&self, plan: &Plan) -> Result<(), hgu::Error> {
        let f = &plan.funcion;
        let pagina = f.pagina.as_deref().unwrap_or_default();
        let basica = !f.name.starts_with("cb");
        let m = &self.state.mascara;
        let params: Vec<(String, String)> = plan
            .payload
            .iter()
            .map(|(k, v)| {
                if basica && es_secreto_param(f, m, k) {
                    (k.clone(), mess_userpass(v))
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect();
        let p: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let c = &self.state.hgu;
        if basica {
            c.basic_write(pagina, &f.name, &p).await.map(drop)
        } else {
            c.write(pagina, &f.name, &p).await.map(drop)
        }
    }

    /// ¿Muestra el router el lado `lado` del plan?
    async fn comprobar(&self, plan: &Plan, lado: Lado) -> Comprobacion {
        let f = &plan.funcion;
        let no_leido = |e: hgu::Error| {
            Comprobacion::Difiere(vec![format!("no se puede releer el estado: {e}")])
        };
        match (plan.operacion, &plan.lectura) {
            (Operacion::Accion, _) => Comprobacion::Coincide(None),
            (_, Lectura::Ninguna) => Comprobacion::Limitada,
            (Operacion::Editar, l) => match self.leer(l).await {
                Ok(actual) => comparar(f, &plan.campos, &actual, lado),
                Err(e) => no_leido(e),
            },
            (Operacion::Crear, l @ Lectura::Xml { .. }) => {
                let lista = match self.listar(l).await {
                    Ok(v) => v,
                    Err(e) => return no_leido(e),
                };
                let nueva = lista.iter().find(|e| {
                    comparar(f, &plan.campos, e, Lado::Nuevo) == Comprobacion::Coincide(None)
                });
                match (lado, nueva) {
                    (Lado::Nuevo, Some(e)) => {
                        let id_param = f.lista.as_ref().map(|l| l.id_param.as_str());
                        Comprobacion::Coincide(
                            id_param.and_then(|p| buscar(e, p)).map(str::to_owned),
                        )
                    }
                    (Lado::Nuevo, None) => {
                        Comprobacion::Difiere(vec!["no aparece la instancia nueva".into()])
                    }
                    (Lado::Anterior, None) => Comprobacion::Coincide(None),
                    (Lado::Anterior, Some(_)) => {
                        Comprobacion::Difiere(vec!["la instancia nueva existe".into()])
                    }
                }
            }
            (Operacion::Crear, _) => Comprobacion::Limitada,
            (Operacion::Borrar, l) => {
                let existe = match self.leer(l).await {
                    Ok(a) if a.is_empty() => false,
                    Ok(a) => {
                        comparar(f, &plan.campos, &a, Lado::Anterior)
                            == Comprobacion::Coincide(None)
                    }
                    Err(hgu::Error::CallFailed { .. }) => false,
                    Err(e) => return no_leido(e),
                };
                match (lado, existe) {
                    (Lado::Nuevo, false) | (Lado::Anterior, true) => Comprobacion::Coincide(None),
                    (Lado::Nuevo, true) => {
                        Comprobacion::Difiere(vec!["la instancia sigue existiendo".into()])
                    }
                    (Lado::Anterior, false) => {
                        Comprobacion::Difiere(vec!["la instancia ya no existe".into()])
                    }
                }
            }
        }
    }

    fn reservar(self: &Arc<Self>) -> Result<Reserva, CambioError> {
        let mut a = lock(&self.activo);
        if a.is_some() {
            return Err(CambioError::EnCurso);
        }
        *a = Some(None);
        Ok(Reserva { m: self.clone() })
    }

    // ---- vigilancia

    fn vigilar_en_segundo_plano(
        self: &Arc<Self>,
        id: i64,
        plan: Plan,
        base: Base,
        nota: Option<String>,
        reserva: Reserva,
    ) {
        let m = self.clone();
        let h = tokio::spawn(async move {
            m.seguir(id, plan, base, nota).await;
            drop(reserva);
        });
        *lock(&self.tarea) = Some(h.abort_handle());
    }

    async fn seguir(&self, id: i64, plan: Plan, base: Base, nota: Option<String>) {
        match base {
            Base::Uptime(antes) => match self.esperar_vuelta(antes).await {
                Ok(()) => {
                    self.marcar(id, Estado::Aplicado, None, nota.as_deref())
                        .await
                }
                Err(e) => self.marcar(id, Estado::Fallido, Some(&e), None).await,
            },
            Base::Wan(wan) => {
                let nota = if wan {
                    nota
                } else {
                    let extra = "la WAN ya estaba caída antes del cambio: \
                                 solo se vigila el acceso al router";
                    Some(match nota {
                        Some(n) => format!("{n}; {extra}"),
                        None => extra.to_owned(),
                    })
                };
                let dur = self.tiempos.vigilancia(plan.funcion.riesgo);
                match self.vigilar(dur, wan).await {
                    Ok(()) => {
                        self.marcar(id, Estado::Aplicado, None, nota.as_deref())
                            .await
                    }
                    Err(motivo) if plan.operacion == Operacion::Accion => {
                        let e = format!("{motivo}; una acción no se puede revertir");
                        self.marcar(id, Estado::Fallido, Some(&e), nota.as_deref())
                            .await;
                    }
                    Err(motivo) => self.revertir(id, &plan, &motivo, nota.as_deref()).await,
                }
            }
        }
    }

    /// Comprueba el router cada `intervalo` durante `dur`. Error con el
    /// motivo tras `fallos_para_revertir` fallos seguidos. Si el plazo acaba
    /// con fallos pendientes, sigue hasta un acierto o la reversión.
    async fn vigilar(&self, dur: Duration, wan: bool) -> Result<(), String> {
        let fin = Instant::now() + dur;
        let mut seguidos = 0;
        loop {
            tokio::time::sleep(self.tiempos.intervalo).await;
            match self.salud(wan).await {
                Ok(()) => seguidos = 0,
                Err(e) => {
                    seguidos += 1;
                    tracing::warn!("vigilancia: fallo {seguidos}: {e}");
                    if seguidos >= self.tiempos.fallos_para_revertir {
                        return Err(format!("vigilancia: {seguidos} fallos seguidos ({e})"));
                    }
                }
            }
            if seguidos == 0 && Instant::now() >= fin {
                return Ok(());
            }
        }
    }

    /// El router deja iniciar sesión y (si `wan`) la WAN `ip2` está arriba.
    async fn salud(&self, wan: bool) -> Result<(), String> {
        let c = &self.state.hgu;
        let no_responde = |e: hgu::Error| format!("el router no responde: {e}");
        read::summary(c).await.map_err(no_responde)?;
        if wan && !wan_arriba(&read::wan(c).await.map_err(no_responde)?) {
            return Err(format!("la WAN ({WAN}) está caída"));
        }
        Ok(())
    }

    /// Tras `cbReboot`: espera hasta `alto` a que el router vuelva (responde
    /// y su tiempo encendido es menor que antes, o se le vio caído).
    async fn esperar_vuelta(&self, uptime_antes: u64) -> Result<(), String> {
        let fin = Instant::now() + self.tiempos.alto;
        let mut caido = false;
        while Instant::now() < fin {
            tokio::time::sleep(self.tiempos.intervalo).await;
            match read::summary(&self.state.hgu).await {
                Ok(s) if caido || s.uptime_s < uptime_antes => return Ok(()),
                Ok(_) => {}
                Err(_) => caido = true,
            }
        }
        Err(format!(
            "el router no volvió tras el reinicio en {} s",
            self.tiempos.alto.as_secs_f32()
        ))
    }

    /// Escribe los valores anteriores y verifica; si el router no responde,
    /// reintenta durante `alto`.
    async fn revertir(&self, id: i64, plan: &Plan, motivo: &str, nota: Option<&str>) {
        let Some(req) = inversa(plan) else {
            let e = format!("{motivo}; no se puede revertir: valores anteriores desconocidos");
            self.marcar(id, Estado::Fallido, Some(&e), nota).await;
            return;
        };
        tracing::warn!(cambio = id, "{motivo}; revirtiendo");
        let fin = Instant::now() + self.tiempos.alto;
        let ultimo = loop {
            let error = match self.revertir_una_vez(&req).await {
                Ok(()) => {
                    let e = format!("{motivo}; revertido a los valores anteriores");
                    self.marcar(id, Estado::Revertido, Some(&e), nota).await;
                    return;
                }
                Err(CambioError::Verificacion(v)) => {
                    let e = format!("{motivo}; reversión no verificada: {}", v.join("; "));
                    self.marcar(id, Estado::Fallido, Some(&e), nota).await;
                    return;
                }
                Err(e) => e.to_string(),
            };
            if Instant::now() >= fin {
                break error;
            }
            tokio::time::sleep(self.tiempos.intervalo).await;
        };
        let e = format!("{motivo}; no se pudo revertir: {ultimo}");
        self.marcar(id, Estado::Fallido, Some(&e), nota).await;
    }

    async fn revertir_una_vez(&self, req: &ChangeRequest) -> Result<(), CambioError> {
        // El sondeo se pausa si se puede; si no, se revierte igualmente (las
        // peticiones al router van serializadas de todos modos).
        let _pausa = tokio::time::timeout(self.tiempos.pausa_max, self.state.poller.pause())
            .await
            .ok();
        let plan = match self.planificar(req).await {
            Ok(p) => p,
            Err(CambioError::SinCambios) => return Ok(()),
            Err(e) => return Err(e),
        };
        self.enviar(&plan)
            .await
            .map_err(|e| CambioError::Router(e.to_string()))?;
        match self.comprobar(&plan, Lado::Nuevo).await {
            Comprobacion::Difiere(v) => Err(CambioError::Verificacion(v)),
            Comprobacion::Coincide(_) | Comprobacion::Limitada => Ok(()),
        }
    }

    // ---- base de datos

    async fn insertar(&self, plan: &Plan, usuario: &str) -> Result<i64, sqlx::Error> {
        let clave = &self.state.history_key;
        // JSON de los valores no secretos y, aparte, los secretos cifrados.
        let partir = |pares: Vec<(&str, &str, bool)>| {
            let mut publico = BTreeMap::new();
            let mut secreto = BTreeMap::new();
            for (k, v, s) in pares {
                if s { &mut secreto } else { &mut publico }.insert(k, v);
            }
            let cifrado = (!secreto.is_empty()).then(|| clave.seal(json(&secreto).as_bytes()));
            (json(&publico), cifrado)
        };
        let (anteriores, anteriores_cifrados) = partir(
            plan.campos
                .iter()
                .filter(|c| c.leido)
                .map(|c| (c.nombre.as_str(), c.antes.as_str(), c.secreto))
                .collect(),
        );
        let (nuevos, nuevos_cifrados) = partir(
            plan.campos
                .iter()
                .map(|c| (c.nombre.as_str(), c.despues.as_str(), c.secreto))
                .collect(),
        );
        let m = &self.state.mascara;
        let f = &plan.funcion;
        let solicitud: BTreeMap<&str, &str> = plan
            .valores
            .iter()
            .filter(|(k, v)| {
                !es_secreto_param(f, m, k)
                    && !m.oculta(k, v)
                    && !plan.campos.iter().any(|c| c.secreto && &c.nombre == *k)
            })
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let now = series::now();
        sqlx::query_scalar(
            "INSERT INTO changes (ts, updated, user_id, usuario, funcion, instancia, riesgo, \
             estado, operacion, diff, anteriores, anteriores_cifrados, solicitud, nuevos, \
             nuevos_cifrados) VALUES (?, ?, (SELECT id FROM users WHERE username = ?), ?, ?, ?, \
             ?, 'aplicando', ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(now)
        .bind(now)
        .bind(usuario)
        .bind(usuario)
        .bind(&f.name)
        .bind(&plan.instancia)
        .bind(riesgo_str(f.riesgo))
        .bind(plan.operacion.as_str())
        .bind(json(&visible(&plan.campos).campos))
        .bind(anteriores)
        .bind(anteriores_cifrados)
        .bind(json(&solicitud))
        .bind(nuevos)
        .bind(nuevos_cifrados)
        .fetch_one(&self.state.db)
        .await
    }

    /// Cambia el estado; `error` y `nota` solo se escriben si son `Some`.
    async fn marcar(&self, id: i64, estado: Estado, error: Option<&str>, nota: Option<&str>) {
        let r = sqlx::query(
            "UPDATE changes SET estado = ?, updated = ?, error = COALESCE(?, error), \
             nota = COALESCE(?, nota) WHERE id = ?",
        )
        .bind(estado.as_str())
        .bind(series::now())
        .bind(error)
        .bind(nota)
        .bind(id)
        .execute(&self.state.db)
        .await;
        if let Err(e) = r {
            tracing::error!(cambio = id, "no se puede guardar el estado {estado:?}: {e}");
        }
        match error {
            Some(e) => tracing::warn!(cambio = id, estado = estado.as_str(), "{e}"),
            None => tracing::info!(
                cambio = id,
                estado = estado.as_str(),
                nota = nota.unwrap_or(""),
                "cambio actualizado"
            ),
        }
    }

    async fn fijar_instancia(&self, id: i64, instancia: &str) {
        let r = sqlx::query("UPDATE changes SET instancia = ? WHERE id = ?")
            .bind(instancia)
            .bind(id)
            .execute(&self.state.db)
            .await;
        if let Err(e) = r {
            tracing::error!(cambio = id, "no se puede guardar la instancia nueva: {e}");
        }
    }

    async fn fila(&self, id: i64) -> Option<sqlx::sqlite::SqliteRow> {
        sqlx::query("SELECT * FROM changes WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.state.db)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(cambio = id, "no se puede leer el cambio: {e}");
                None
            })
    }

    /// Plan de un cambio guardado (sin carga: solo para comprobar, revertir
    /// o deshacer). Los secretos se descifran con la clave del historial.
    fn reconstruir(&self, fila: &sqlx::sqlite::SqliteRow) -> Result<Plan, String> {
        let nombre: String = fila.get("funcion");
        let f = self
            .state
            .catalogo
            .funcion(&nombre)
            .ok_or_else(|| format!("la función {nombre} ya no está en el catálogo"))?
            .clone();
        let op =
            Operacion::parse(&fila.get::<String, _>("operacion")).ok_or("operación desconocida")?;
        let instancia: Option<String> = fila.get("instancia");
        let mapa = |col: &str| -> Result<BTreeMap<String, String>, String> {
            serde_json::from_str(&fila.get::<String, _>(col))
                .map_err(|e| format!("{col} no válido: {e}"))
        };
        let abrir = |col: &str, m: &mut BTreeMap<String, String>| -> Result<(), String> {
            if let Some(blob) = fila.get::<Option<Vec<u8>>, _>(col) {
                let plano = self
                    .state
                    .history_key
                    .open(&blob)
                    .ok_or_else(|| format!("no se puede descifrar {col} (¿otra clave?)"))?;
                let s: BTreeMap<String, String> =
                    serde_json::from_slice(&plano).map_err(|e| format!("{col}: {e}"))?;
                m.extend(s);
            }
            Ok(())
        };
        let mut anteriores = mapa("anteriores")?;
        abrir("anteriores_cifrados", &mut anteriores)?;
        let mut nuevos = mapa("nuevos")?;
        abrir("nuevos_cifrados", &mut nuevos)?;
        let diff: Vec<CampoDiff> = serde_json::from_str(&fila.get::<String, _>("diff"))
            .map_err(|e| format!("diff no válido: {e}"))?;
        let campos: Vec<Campo> = diff
            .into_iter()
            .map(|d| Campo {
                antes: anteriores.get(&d.nombre).cloned().unwrap_or_default(),
                despues: nuevos.get(&d.nombre).cloned().unwrap_or_default(),
                leido: anteriores.contains_key(&d.nombre),
                secreto: d.secreto,
                nombre: d.nombre,
            })
            .collect();
        let mut valores = mapa("solicitud")?;
        if op != Operacion::Borrar {
            valores.extend(campos.iter().map(|c| (c.nombre.clone(), c.despues.clone())));
        }
        let lectura = lectura_de(&f, &valores, instancia.as_deref()).map_err(|e| e.to_string())?;
        Ok(Plan {
            funcion: f,
            operacion: op,
            instancia,
            valores,
            payload: Vec::new(),
            campos,
            lectura,
        })
    }
}

/// Compara los campos del plan (lado `lado`) con lo que muestra el router.
/// `Limitada` si el router no muestra ninguno.
fn comparar(
    f: &Funcion,
    campos: &[Campo],
    actual: &BTreeMap<String, String>,
    lado: Lado,
) -> Comprobacion {
    let mut comparados = 0;
    let mut fallos = Vec::new();
    for c in campos {
        if lado == Lado::Anterior && !c.leido {
            continue;
        }
        let Some(leido) = buscar(actual, &c.nombre) else {
            continue;
        };
        comparados += 1;
        let esperado = match lado {
            Lado::Nuevo => &c.despues,
            Lado::Anterior => &c.antes,
        };
        if !iguales(f.parametro(&c.nombre), leido, esperado) {
            let ver = |v: &str| {
                if c.secreto {
                    MASCARA.to_owned()
                } else {
                    format!("«{v}»")
                }
            };
            fallos.push(format!(
                "{}: se esperaba {}, el router tiene {}",
                c.nombre,
                ver(esperado),
                ver(leido)
            ));
        }
    }
    if comparados == 0 {
        Comprobacion::Limitada
    } else if fallos.is_empty() {
        Comprobacion::Coincide(None)
    } else {
        Comprobacion::Difiere(fallos)
    }
}

fn wan_arriba(w: &[hgu::models::WanConn]) -> bool {
    w.iter().any(|c| c.name == WAN && c.up)
}

fn palabra_requerida(f: &Funcion) -> Option<&'static str> {
    match f.tipo {
        Tipo::Accion if f.name == REBOOT => Some(PALABRA_REINICIO),
        Tipo::Accion => Some(PALABRA_ACCION),
        _ => None,
    }
}

/// Diff para mostrar y guardar: secretos enmascarados.
fn visible(campos: &[Campo]) -> Diff {
    let ver = |c: &Campo, v: &str| {
        if c.secreto {
            MASCARA.to_owned()
        } else {
            v.to_owned()
        }
    };
    Diff {
        campos: campos
            .iter()
            .map(|c| CampoDiff {
                nombre: c.nombre.clone(),
                antes: ver(c, &c.antes),
                despues: ver(c, &c.despues),
                secreto: c.secreto,
            })
            .collect(),
    }
}

/// Resumen del diff para los registros (secretos enmascarados).
fn resumen(campos: &[Campo]) -> String {
    visible(campos)
        .campos
        .iter()
        .map(|c| format!("{}: «{}» → «{}»", c.nombre, c.antes, c.despues))
        .collect::<Vec<_>>()
        .join(", ")
}

fn riesgo_str(r: Riesgo) -> &'static str {
    match r {
        Riesgo::Bajo => "bajo",
        Riesgo::Medio => "medio",
        Riesgo::Alto => "alto",
    }
}

fn json<T: Serialize + ?Sized>(v: &T) -> String {
    serde_json::to_string(v).expect("mapas de cadenas serializables")
}

fn nuevo_token() -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("generador aleatorio del sistema");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn a_cambio(f: &sqlx::sqlite::SqliteRow) -> Option<Cambio> {
    let riesgo = match f.get::<String, _>("riesgo").as_str() {
        "bajo" => Riesgo::Bajo,
        "medio" => Riesgo::Medio,
        _ => Riesgo::Alto,
    };
    Some(Cambio {
        id: f.get("id"),
        ts: f.get("ts"),
        updated: f.get("updated"),
        funcion: f.get("funcion"),
        instancia: f.get("instancia"),
        riesgo,
        estado: Estado::parse(&f.get::<String, _>("estado"))?,
        diff: Diff {
            campos: serde_json::from_str(&f.get::<String, _>("diff")).unwrap_or_default(),
        },
        error: f.get("error"),
        nota: f.get("nota"),
        usuario: f.get("usuario"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalogo::Catalogo;

    fn cat() -> Catalogo {
        Catalogo::load().unwrap()
    }

    fn mapa(pares: &[(&str, &str)]) -> BTreeMap<String, String> {
        pares
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn carga_con_la_instancia_completa_y_secretos_sin_tocar() {
        let c = cat();
        let m = Mascara::load().unwrap();
        let f = c.funcion("cbSetWifiAccPt").unwrap();
        let actual = mapa(&[
            ("dpm_cfg_wifi_acc_pt_id", "1"),
            ("ssid", "Casa"),
            ("key_phrase", "clave-actual"),
            ("WMM", "1"),
            ("bssid", "02:00:00:00:00:01"),
        ]);
        let valores = mapa(&[
            ("dpm_cfg_wifi_acc_pt_id", "1"),
            ("ssid", "Otra"),
            ("key_phrase", MASCARA),
            ("wep_key1_data", ""),
        ]);
        let Compuesto {
            efectivos,
            payload,
            campos,
        } = componer(f, &m, Operacion::Editar, &actual, &valores);
        let p: BTreeMap<_, _> = payload.into_iter().collect();
        assert_eq!(p["ssid"], "Otra");
        assert_eq!(p["key_phrase"], "clave-actual", "secreto sin tocar");
        // Sin distinguir mayúsculas cuando no hay coincidencia exacta.
        assert_eq!(p["wmm"], "1");
        assert_eq!(p["dpm_cfg_wifi_acc_pt_id"], "1");
        // Ni lo que no es parámetro ni lo que no tiene valor.
        assert!(!p.contains_key("bssid"));
        assert!(!p.contains_key("wep_key1_data"));
        assert!(!p.contains_key("validation_only"));
        assert!(p.values().all(|v| v != MASCARA));
        assert_eq!(efectivos["key_phrase"], "clave-actual");
        assert_eq!(campos.len(), 1);
        assert_eq!(campos[0].nombre, "ssid");

        // Un campo que no es secreto pero llegó enmascarado (p. ej. por la
        // regla de sufijo numérico) tampoco se escribe enmascarado.
        let actual = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("wep_key_idx", "2")]);
        let valores = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("wep_key_idx", MASCARA)]);
        let r = componer(f, &m, Operacion::Editar, &actual, &valores);
        assert!(r.campos.is_empty());
        assert!(r.payload.contains(&("wep_key_idx".into(), "2".into())));
        let r = componer(f, &m, Operacion::Editar, &mapa(&[]), &valores);
        assert!(r.payload.iter().all(|(_, v)| v != MASCARA));
    }

    #[test]
    fn secreto_cambiado_se_marca() {
        let c = cat();
        let m = Mascara::load().unwrap();
        let f = c.funcion("cbSetWifiAccPt").unwrap();
        let actual = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "vieja")]);
        let valores = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "nueva")]);
        let campos = componer(f, &m, Operacion::Editar, &actual, &valores).campos;
        assert_eq!(campos.len(), 1);
        assert!(campos[0].secreto && campos[0].leido);
        let d = visible(&campos);
        assert_eq!(
            (d.campos[0].antes.as_str(), d.campos[0].despues.as_str()),
            (MASCARA, MASCARA)
        );
        assert!(!resumen(&campos).contains("nueva"));
        assert!(!resumen(&campos).contains("vieja"));
    }

    #[test]
    fn crear_y_borrar() {
        let c = cat();
        let m = Mascara::load().unwrap();
        let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
        let valores = mapa(&[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("rtm_dhcpv4_svr_pool_static_id", "-1"),
            ("mac_addr", " 02:00:00:00:00:AA "),
            ("ipv4_addr", "192.168.1.60"),
        ]);
        let Compuesto {
            payload, campos, ..
        } = componer(f, &m, Operacion::Crear, &BTreeMap::new(), &valores);
        let p: BTreeMap<_, _> = payload.into_iter().collect();
        assert_eq!(p["rtm_dhcpv4_svr_pool_static_id"], "-1");
        assert_eq!(p["mac_addr"], "02:00:00:00:00:AA", "IP/MAC sin espacios");
        let nombres: Vec<_> = campos.iter().map(|c| c.nombre.as_str()).collect();
        assert_eq!(nombres, ["ipv4_addr", "mac_addr"], "sin los ids");

        let actual = mapa(&[
            ("rtm_dhcpv4_svr_pool_static_id", "3"),
            ("adm_state", "1"),
            ("mac_addr", "02:00:00:00:00:aa"),
            ("ipv4_addr", "192.168.1.60"),
        ]);
        let valores = mapa(&[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("rtm_dhcpv4_svr_pool_static_id", "3"),
        ]);
        let Compuesto {
            payload, campos, ..
        } = componer(f, &m, Operacion::Borrar, &actual, &valores);
        assert_eq!(
            payload,
            vec![
                ("rtm_dhcpv4_svr_pool_static_id".to_owned(), "3".to_owned()),
                ("rtm_cfg_dhcpv4_svr_pool_id".to_owned(), "0".to_owned()),
                ("del".to_owned(), "DEL".to_owned()),
            ]
        );
        assert_eq!(campos.len(), 3);
        assert!(campos.iter().all(|c| c.despues.is_empty() && c.leido));
    }

    #[test]
    fn inversa_de_cada_operacion() {
        let c = cat();
        let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap().clone();
        let campo = |n: &str, a: &str, d: &str, leido| Campo {
            nombre: n.into(),
            antes: a.into(),
            despues: d.into(),
            secreto: false,
            leido,
        };
        let mut plan = Plan {
            funcion: f,
            operacion: Operacion::Editar,
            instancia: Some("0".into()),
            valores: mapa(&[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("ipv4_addr", "192.168.1.84"),
            ]),
            payload: Vec::new(),
            campos: vec![campo("ipv4_addr", "192.168.1.68", "192.168.1.84", true)],
            lectura: Lectura::Ninguna,
        };
        let r = inversa(&plan).unwrap();
        assert_eq!(r.instancia.as_deref(), Some("0"));
        assert!(!r.borrar);
        assert_eq!(
            r.valores,
            mapa(&[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("ipv4_addr", "192.168.1.68")
            ])
        );
        // Sin valores anteriores conocidos no hay inversa.
        plan.campos = vec![campo("ipv4_addr", "", "192.168.1.84", false)];
        assert_eq!(inversa(&plan), None);

        plan.operacion = Operacion::Crear;
        plan.instancia = None;
        assert_eq!(inversa(&plan), None, "sin el id de la instancia creada");
        plan.instancia = Some("5".into());
        let r = inversa(&plan).unwrap();
        assert!(r.borrar);
        assert_eq!(r.instancia.as_deref(), Some("5"));
        assert_eq!(r.valores, mapa(&[("rtm_cfg_dhcpv4_svr_pool_id", "0")]));

        plan.operacion = Operacion::Borrar;
        plan.campos = vec![campo("ipv4_addr", "192.168.1.60", "", true)];
        let r = inversa(&plan).unwrap();
        assert!(!r.borrar);
        assert_eq!(r.instancia, None);
        assert_eq!(r.valores["ipv4_addr"], "192.168.1.60");

        plan.operacion = Operacion::Accion;
        assert_eq!(inversa(&plan), None);
    }

    #[test]
    fn lectura_pareada_sustituye_parametros() {
        let c = cat();
        let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
        let v = mapa(&[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("rtm_dhcpv4_svr_pool_static_id", "4"),
        ]);
        let l = lectura_de(f, &v, Some("4")).unwrap();
        assert_eq!(
            l,
            Lectura::Xml {
                funcion: "cbDhcpV4SvrPoolGetStatic".into(),
                params: vec![
                    ("rtm_cfg_dhcpv4_svr_pool_id".into(), "0".into()),
                    ("rtm_dhcpv4_svr_pool_static_id".into(), "4".into()),
                ],
                clave_id: Some("rtm_dhcpv4_svr_pool_static_id".into()),
            }
        );
        let falta = lectura_de(f, &mapa(&[]), Some("4"));
        assert!(matches!(falta, Err(CambioError::Invalida(_))), "{falta:?}");
        let wl = c.funcion("te_wifi.wl").unwrap();
        assert_eq!(
            lectura_de(wl, &mapa(&[]), None).unwrap(),
            Lectura::Pagina("te_wifi.asp".into())
        );
    }

    #[test]
    fn validacion_de_valores() {
        let c = cat();
        let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
        let p = |n| f.parametro(n).unwrap();
        assert!(validar(p("ipv4_addr"), "192.168.1.5").is_ok());
        assert!(validar(p("ipv4_addr"), "192.168.1.256").is_err());
        assert!(validar(p("mac_addr"), "02:00:00:00:00:aB").is_ok());
        assert!(validar(p("mac_addr"), "02:00:00:00:00").is_err());
        assert!(validar(p("mac_addr"), "02:00:00:00:00:zz").is_err());
        assert!(validar(p("adm_state"), "1").is_ok());
        assert!(validar(p("adm_state"), "2").is_err());
        assert!(validar(p("rtm_cfg_dhcpv4_svr_pool_id"), "x").is_err());
        let w = c.funcion("cbSetWifiAccPt").unwrap();
        assert!(validar(w.parametro("ssid").unwrap(), "a\nb").is_err());
        let e = validar(w.parametro("key_phrase").unwrap(), "a\0b").unwrap_err();
        assert!(!e.contains("a\0b"), "el error no muestra el secreto");
    }

    #[test]
    fn comparar_normaliza_y_enmascara() {
        let c = cat();
        let f = c.funcion("cbSetWifiAccPt").unwrap();
        let campos = vec![
            Campo {
                nombre: "key_phrase".into(),
                antes: "vieja".into(),
                despues: "nueva".into(),
                secreto: true,
                leido: true,
            },
            Campo {
                nombre: "ssid".into(),
                antes: "A".into(),
                despues: "B".into(),
                secreto: false,
                leido: true,
            },
        ];
        let actual = mapa(&[("key_phrase", "otra"), ("ssid", " B ")]);
        let Comprobacion::Difiere(v) = comparar(f, &campos, &actual, Lado::Nuevo) else {
            panic!()
        };
        assert_eq!(v.len(), 1);
        assert!(v[0].contains(MASCARA) && !v[0].contains("otra") && !v[0].contains("nueva"));
        assert_eq!(
            comparar(f, &campos, &mapa(&[("bssid", "x")]), Lado::Nuevo),
            Comprobacion::Limitada
        );
        let anterior = mapa(&[("key_phrase", "vieja"), ("SSID", "A")]);
        assert_eq!(
            comparar(f, &campos, &anterior, Lado::Anterior),
            Comprobacion::Coincide(None)
        );
        let s = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
        assert!(iguales(
            s.parametro("mac_addr"),
            "02:AA:00:00:00:00",
            " 02:aa:00:00:00:00"
        ));
        assert!(!iguales(None, "Casa", "casa"));
    }
}
