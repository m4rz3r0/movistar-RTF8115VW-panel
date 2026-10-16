//! Gestor de cambios: toda escritura al router pasa por aquí.
//!
//! Cada cambio es un grupo de pasos ([`ChangeManager::preparar_grupo`]; ver
//! `grupo.rs`): se aplican en orden, se revierten en orden inverso y se
//! vigilan y deshacen como una unidad. Un cambio de un paso
//! ([`ChangeManager::preparar`]/[`ChangeManager::confirmar`]) sigue estas
//! reglas:
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
//!    router deja iniciar sesión y la conexión a internet sigue arriba (la
//!    detectada o elegida a mano; sin ninguna, cualquier conexión de datos:
//!    [`plan::wan_arriba`]). Tras
//!    `fallos_para_revertir` fallos seguidos se escriben los valores
//!    anteriores, se verifica y queda `revertido`; si no, `aplicado`.
//!    `cbReboot` (palabra `REINICIAR`) espera la vuelta del router sin
//!    revertir. Si la WAN ya estaba caída al confirmar (se guarda en el
//!    cambio), la vigilancia solo exige que el router deje entrar: no se
//!    revierte un intento de arreglar una WAN caída (queda anotado).
//!
//! Un único cambio activo a la vez: otro `confirmar` mientras tanto recibe
//! [`CambioError::EnCurso`]. Al arrancar, [`ChangeManager::reanudar_pendientes`]
//! retoma los cambios que quedaron en `aplicando`/`vigilando`; hasta que
//! termina, `confirmar` espera. La aplicación corre en su propia tarea: si
//! quien llamó a `confirmar` se va, el cambio sigue hasta un estado final.
//!
//! Bloqueos de la v1 (antes de dar token): escrituras de la web básica con
//! parámetros secretos (falta validar su formato con el router real) y las
//! que cambian la IP LAN del router (cortarían la conexión del panel).
//! Una edición que cambia un campo que el router no muestra se rechaza: sin
//! valor anterior no se podría revertir ni deshacer.
//!
//! Un valor secreto que llega como [`MASCARA`] o vacío significa "sin tocar":
//! se reenvía el valor actual leído del router y no cuenta como cambio. Si
//! la lectura lo da vacío o tapado (asteriscos, la máscara), no se reenvía:
//! la escritura se bloquea ([`BLOQUEO_SECRETO_TAPADO`]), porque reenviarlo
//! podría borrar la clave. Los registros (`tracing`) y el historial solo
//! muestran secretos enmascarados; sus valores anteriores van cifrados.

mod grupo;
pub mod plan;

use grupo::{Ejecucion, PasoGuardado};
pub use grupo::{EstadoPaso, GrupoRequest, Paso, PasoCambio, PasoPreparado, PreparadoGrupo};
use plan::*;
pub use plan::{PALABRA_ACCION, PALABRA_REINICIO};
/// Reglas del gestor que comparte el explorador (qué identifica una
/// instancia y cómo se busca un campo en una lectura): una sola copia.
pub(crate) use plan::{buscar, identidad};

use crate::{
    crypto::HistoryKey,
    lecturas::CacheLecturas,
    poller::PollerHandle,
    series,
    state::{AppState, RedActual},
};
use catalogo::{Catalogo, mask::Mascara};
use catalogo::{Funcion, Riesgo, Tipo, mask::MASCARA};
use hgu::{
    fmt::{mess_userpass, page_vars},
    read,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::SqlitePool;
use std::sync::OnceLock;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};
use tokio::sync::{oneshot, watch};
use tokio::{task::AbortHandle, time::Instant};

/// Vida de un token de [`ChangeManager::preparar`].
const TOKEN_TTL: Duration = Duration::from_secs(10 * 60);

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

/// Resultado de releer tras una escritura sin respuesta.
enum SinRespuesta {
    /// Hay que seguir (vigilar), con esta comprobación y esta nota.
    Aplicada(Comprobacion, String),
    /// Fallido, con el motivo: el router sigue como antes (no se aplicó) o,
    /// al crear, no se sabe qué instancia es la del cambio.
    Fallido(String),
    /// El router no muestra ni lo de antes ni lo nuevo (pudo quedar a
    /// medias): fallido, y hay que revertir (motivo).
    OtrosValores(String),
    /// Al crear, el router no muestra lo de antes pero tampoco se sabe qué
    /// instancia es la del cambio: pudo crearse algo (motivo).
    CreacionIncierta(String),
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
    /// Lo que se va a escribir: función, instancia y si es un borrado (lo
    /// que hace falta para presentarlo sin tener la petición a mano, p. ej.
    /// al deshacer).
    pub funcion: String,
    pub instancia: Option<String>,
    pub borrar: bool,
    /// Los pasos del grupo (uno, salvo al deshacer un grupo). `diff` junta
    /// los de todos; `funcion`, `instancia` y `borrar` son los del primero
    /// que cambia algo.
    pub pasos: Vec<PasoPreparado>,
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
    /// Grupo que falló y cuya reversión no se pudo completar: el detalle
    /// (`error`) dice qué pasos siguen aplicados.
    RevertidoParcial,
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
            Estado::RevertidoParcial => "revertido_parcial",
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
            Estado::RevertidoParcial,
        ]
        .into_iter()
        .find(|e| e.as_str() == s)
    }
}

/// Un cambio del historial (secretos enmascarados): un grupo de pasos.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cambio {
    pub id: i64,
    /// Creación y última actualización (segundos Unix).
    pub ts: i64,
    pub updated: i64,
    /// Función e instancia del primer paso que se escribió.
    pub funcion: String,
    pub instancia: Option<String>,
    /// El mayor de los pasos.
    pub riesgo: Riesgo,
    pub estado: Estado,
    /// Los diffs de los pasos que no se omitieron, juntos.
    pub diff: Diff,
    pub error: Option<String>,
    pub nota: Option<String>,
    pub usuario: Option<String>,
    /// Clave i18n del resumen del grupo.
    pub titulo: Option<String>,
    pub pasos: Vec<PasoCambio>,
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
    pasos: Vec<Paso>,
    /// Pasos que no cambiaban nada al preparar: se omiten.
    sin_cambios: Vec<bool>,
    titulo: Option<String>,
    caduca: Instant,
    /// Valores exactos (deshacer): sin "secreto vacío = sin tocar" ni
    /// validación de lo que vino del router.
    explicito: bool,
    /// Cambio que deshace, si es un deshacer.
    deshace: Option<i64>,
}

/// Lo que el gestor usa del estado del servidor (sin el propio gestor, para
/// que `AppState` pueda guardarlo sin ciclos).
struct Recursos {
    db: SqlitePool,
    hgu: hgu::Client,
    catalogo: Arc<Catalogo>,
    mascara: Arc<Mascara>,
    poller: PollerHandle,
    history_key: Arc<HistoryKey>,
    /// Conexión a internet que vigila la red de seguridad WAN.
    red: RedActual,
    /// Caché de lecturas para mostrar: se invalida al verificar y al
    /// terminar cada cambio (las escrituras ya la invalidan solas).
    lecturas: Arc<CacheLecturas>,
}

impl Recursos {
    /// ¿Hay internet? (ver [`plan::wan_arriba`]): la conexión que vigila
    /// ([`RedActual::wan_en`]) y si está arriba. Error si el router no
    /// responde.
    async fn wan_vigilada(&self) -> Result<(Option<String>, bool), hgu::Error> {
        let conns = read::wan(&self.hgu).await?;
        let wan = self.red.wan_en(&conns);
        let arriba = wan_arriba(&conns, wan.as_deref());
        Ok((wan, arriba))
    }
}

/// Fallo de un intento de reversión.
enum FalloReversion {
    /// Transitorio (el router no responde o no se pudo releer): se reintenta.
    Reintentar(String),
    /// Determinista: no tiene sentido reintentar.
    Definitivo(String),
}

/// Funciones que cambian la IP LAN del router (bloqueadas en v1).
const CAMBIAN_IP_LAN: &[&str] = &[
    "cbSetLanIPv4Addr",
    "cbSetLanIPv4Intf",
    "te_red_local.cgi",
    "te_mapa_red_local.cgi",
];
/// Motivo de [`CambioError::Bloqueada`] con el panel en solo lectura
/// (firmware no probado sin confirmar). Es la clave i18n del texto: la
/// interfaz lo traduce.
pub const BLOQUEO_SOLO_LECTURA: &str = "panel-solo-lectura";
const BLOQUEO_IP_LAN: &str = "bloqueada en v1: cambiar la IP del router corta la conexión del \
                              propio panel; hazlo desde la web del router";
/// Motivo de [`CambioError::Bloqueada`] cuando la escritura reenviaría un
/// secreto que el router da vacío o tapado (al completar la carga con lo
/// leído): podría borrar esa clave. Es la clave i18n del texto. Se quitará
/// cuando la fase 3 compruebe qué hace el router real con ese reenvío.
pub const BLOQUEO_SECRETO_TAPADO: &str = "cambio-error-secreto-tapado";
const BLOQUEO_SECRETOS_BASICA: &str = "bloqueada en v1: falta validar con el router real el \
                                       formato de los secretos de la web básica";

/// Mientras viva, el gestor está ocupado con un cambio (o lo tiene apartado
/// con [`ChangeManager::excluir_cambios`]): no empieza ninguno.
pub struct Reserva {
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
        // Lo leído para mostrar durante el cambio pudo ser de antes de que
        // el router lo aplicase.
        self.m.state.lecturas.invalidar();
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Gestor de cambios. Uno por proceso: el del servidor vive en
/// [`AppState::cambios`] ([`ChangeManager::instalar`]).
pub struct ChangeManager {
    state: Recursos,
    tiempos: Tiempos,
    pendientes: Mutex<HashMap<String, Pendiente>>,
    /// Cambio activo: `Some(None)` reservado y aún sin id.
    activo: Mutex<Option<Option<i64>>>,
    /// Tarea del cambio en curso (aplicación y vigilancia).
    tarea: Mutex<Option<AbortHandle>>,
    /// `true` cuando [`ChangeManager::reanudar_pendientes`] ha terminado.
    listo: watch::Sender<bool>,
    /// `reanudar_pendientes` ya empezó (solo se hace una vez).
    reanudando: std::sync::atomic::AtomicBool,
}

/// Hueco de `AppState` para el gestor del servidor.
pub type Ranura = Arc<OnceLock<Arc<ChangeManager>>>;

impl ChangeManager {
    /// Crea un gestor. `confirmar` no acepta cambios hasta que se haya
    /// llamado a [`ChangeManager::reanudar_pendientes`].
    pub fn new(state: AppState, tiempos: Tiempos) -> Arc<ChangeManager> {
        let red = state.red_actual();
        Arc::new(ChangeManager {
            state: Recursos {
                red,
                db: state.db,
                hgu: state.hgu,
                catalogo: state.catalogo,
                mascara: state.mascara,
                poller: state.poller,
                history_key: state.history_key,
                lecturas: state.lecturas,
            },
            tiempos,
            pendientes: Mutex::new(HashMap::new()),
            activo: Mutex::new(None),
            tarea: Mutex::new(None),
            listo: watch::Sender::new(false),
            reanudando: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// El gestor del servidor: lo crea y lo guarda en `state` la primera vez;
    /// después devuelve siempre el mismo.
    pub fn instalar(state: &AppState, tiempos: Tiempos) -> Arc<ChangeManager> {
        state
            .cambios
            .get_or_init(|| ChangeManager::new(state.clone(), tiempos))
            .clone()
    }

    /// Id del cambio en curso (aplicándose o vigilándose).
    pub fn activo(&self) -> Option<i64> {
        lock(&self.activo).flatten()
    }

    /// ¿Hay un cambio aplicándose o vigilándose, o se están retomando los
    /// del arranque (ver [`ChangeManager::reanudar_pendientes`])? Mientras,
    /// la red de seguridad no debe cambiar de conexión a internet.
    pub fn ocupado(&self) -> bool {
        let reanudando = self.reanudando.load(std::sync::atomic::Ordering::SeqCst);
        self.activo().is_some() || (reanudando && !*self.listo.borrow())
    }

    /// Aparta el gestor para algo que no puede coincidir con un cambio
    /// (sustituir las credenciales del router): [`CambioError::EnCurso`] si
    /// hay uno aplicándose o vigilándose; mientras viva lo devuelto, no
    /// empieza ninguno.
    ///
    /// Mientras se retoman los cambios pendientes del arranque
    /// ([`ChangeManager::reanudar_pendientes`]), también `EnCurso`: retomar
    /// puede tener que revertir con las credenciales de entonces.
    pub fn excluir_cambios(self: &Arc<Self>) -> Result<Reserva, CambioError> {
        let reanudando = self.reanudando.load(std::sync::atomic::Ordering::SeqCst);
        if reanudando && !*self.listo.borrow() {
            return Err(CambioError::EnCurso);
        }
        self.reservar()
    }

    /// Espera a que el hueco quede libre y lo reserva (al retomar: lo que
    /// lo ocupe, p. ej. un cambio de credenciales, termina pronto).
    /// `id`: el cambio que se va a retomar (para el registro).
    async fn esperar_hueco(self: &Arc<Self>, id: i64) -> Reserva {
        let mut avisado = false;
        loop {
            if let Ok(r) = self.reservar() {
                return r;
            }
            if !avisado {
                tracing::info!(
                    "retomar el cambio {id}: espera a que quede libre el gestor de cambios \
                     (p. ej. un cambio de credenciales del router en curso)"
                );
                avisado = true;
            }
            tokio::time::sleep(self.tiempos.intervalo).await;
        }
    }

    /// Detiene la vigilancia en curso sin cerrar el cambio (queda como al
    /// cortarse el proceso; [`ChangeManager::reanudar_pendientes`] lo retoma).
    pub fn detener(&self) {
        if let Some(t) = lock(&self.tarea).take() {
            t.abort();
        }
    }

    /// Lee el router y calcula el diff de `req`. Devuelve un token para
    /// [`ChangeManager::confirmar`]. Es un grupo de un paso
    /// ([`ChangeManager::preparar_grupo`]). Con el panel en solo lectura
    /// ([`crate::instancia::solo_lectura`]) no prepara nada:
    /// `Bloqueada(`[`BLOQUEO_SOLO_LECTURA`]`)`.
    pub async fn preparar(&self, req: ChangeRequest) -> Result<Preparado, CambioError> {
        self.preparar_con(vec![req], false, None).await
    }

    /// Prepara el grupo de `reqs` y lo presenta como un [`Preparado`].
    async fn preparar_con(
        &self,
        reqs: Vec<ChangeRequest>,
        explicito: bool,
        deshace: Option<i64>,
    ) -> Result<Preparado, CambioError> {
        let g = GrupoRequest {
            pasos: reqs
                .iter()
                .map(|req| Paso {
                    req: req.clone(),
                    etiqueta: None,
                })
                .collect(),
            titulo: None,
        };
        let g = self.preparar_grupo_con(g, explicito, deshace).await?;
        let primero = g.pasos.iter().position(|p| !p.sin_cambios).unwrap_or(0);
        let req = &reqs[primero];
        let palabra = reqs
            .iter()
            .filter_map(|r| self.state.catalogo.funcion(&r.funcion))
            .find_map(palabra_requerida)
            .map(str::to_owned);
        Ok(Preparado {
            token: g.token,
            diff: Diff {
                campos: g
                    .pasos
                    .iter()
                    .flat_map(|p| p.diff.campos.iter().cloned())
                    .collect(),
            },
            riesgo: g.riesgo,
            palabra,
            funcion: req.funcion.clone(),
            instancia: req.instancia.clone(),
            borrar: req.borrar,
            pasos: g.pasos,
        })
    }

    /// Aplica el cambio preparado con `token` (todos sus pasos): escribe,
    /// verifica y deja la vigilancia en marcha. Devuelve el id del cambio
    /// (en `vigilando`). Las acciones exigen `palabra`.
    ///
    /// Desde la reserva del hueco, el trabajo corre en una tarea propia: si
    /// este futuro se abandona, el cambio sigue hasta un estado final.
    pub async fn confirmar(
        self: &Arc<Self>,
        token: &str,
        palabra: Option<&str>,
        usuario: &str,
    ) -> Result<i64, CambioError> {
        self.confirmar_con(token, palabra, &[], usuario).await
    }

    /// Tras una escritura sin respuesta (error que no es `CallFailed`):
    /// relee el router, con reintentos durante la vigilancia del riesgo, para
    /// saber si la escritura llegó. Si el primer intento no consigue leer, se
    /// suelta la pausa del sondeo (`pausa`): el estado del panel refleja el
    /// router mientras se reintenta.
    ///
    /// - Muestra lo nuevo, no se puede comprobar o no se consigue leer ⇒
    ///   `Aplicada`: se vigila como cualquier cambio (si el router sigue sin
    ///   responder, la vigilancia revierte) y deshacer queda disponible. La
    ///   comprobación que se devuelve es `Coincide` o `Limitada`.
    /// - Muestra otros valores: si son los de antes ⇒ `Fallido` (no llegó
    ///   nada); si no son ni los de antes ni los nuevos (a medias),
    ///   `OtrosValores`: no se da por aplicada y se revierte; al crear,
    ///   `Fallido` (sin la instancia no hay nada que revertir).
    async fn tras_escritura_sin_respuesta(
        &self,
        plan: &Plan,
        e: &hgu::Error,
        pausa: &mut Option<crate::poller::PauseGuard>,
    ) -> SinRespuesta {
        let sin_confirmar = format!("el router no confirmó la escritura ({e})");
        let fin = Instant::now() + self.tiempos.vigilancia(plan.funcion.riesgo);
        loop {
            match self.comprobar(plan, Lado::Nuevo).await {
                c @ Comprobacion::Coincide(_) => {
                    return SinRespuesta::Aplicada(
                        c,
                        format!("{sin_confirmar}; releído: aplicada"),
                    );
                }
                Comprobacion::Limitada => {
                    return SinRespuesta::Aplicada(
                        Comprobacion::Limitada,
                        format!("{sin_confirmar}; el router no muestra los campos escritos"),
                    );
                }
                Comprobacion::Difiere(v) => {
                    return match self.comprobar(plan, Lado::Anterior).await {
                        Comprobacion::Coincide(_) => SinRespuesta::Fallido(format!(
                            "{sin_confirmar}; el router sigue con los valores de antes: no se aplicó"
                        )),
                        // Al crear, sin saber cuál es la instancia nueva no
                        // se podría revertir ni deshacer: no se toca nada.
                        _ if plan.operacion == Operacion::Crear => SinRespuesta::CreacionIncierta(
                            format!("{sin_confirmar}; {}", v.join("; ")),
                        ),
                        _ => SinRespuesta::OtrosValores(format!(
                            "{sin_confirmar}; el router muestra otros valores: {}",
                            v.join("; ")
                        )),
                    };
                }
                Comprobacion::SinLectura(l) => {
                    if Instant::now() >= fin {
                        return SinRespuesta::Aplicada(
                            Comprobacion::Limitada,
                            format!("{sin_confirmar}; tampoco se pudo releer ({l})"),
                        );
                    }
                    // No congelar el sondeo durante los reintentos.
                    drop(pausa.take());
                    tokio::time::sleep(self.tiempos.intervalo).await;
                }
            }
        }
    }

    /// Motivo por el que `original` ya no se puede deshacer: lo deshizo otro
    /// cambio que está en curso o aplicado.
    /// Si no se puede consultar, también se rechaza (mejor no deshacer que
    /// deshacer dos veces).
    async fn ya_deshecho(&self, original: i64) -> Option<String> {
        let r: Option<(i64, String)> = match sqlx::query_as(
            "SELECT d.id, d.estado FROM changes o JOIN changes d ON d.id = o.deshecho_por \
             WHERE o.id = ?",
        )
        .bind(original)
        .fetch_optional(&self.state.db)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(cambio = original, "no se puede leer deshecho_por: {e}");
                return Some(format!(
                    "no se puede comprobar si el cambio {original} ya se deshizo: {e}"
                ));
            }
        };
        let (d, estado) = r?;
        // Un deshacer que quedó revertido en parte también cuenta: parte de
        // él sigue aplicada y no se deshace dos veces.
        let vigente = [
            Estado::Aplicando,
            Estado::Vigilando,
            Estado::Aplicado,
            Estado::RevertidoParcial,
        ]
        .iter()
        .any(|e| e.as_str() == estado);
        vigente.then(|| format!("el cambio {original} ya se ha deshecho (cambio {d})"))
    }

    /// Peticiones inversas de un cambio `aplicado` que nadie ha deshecho, con
    /// los valores anteriores (secretos incluidos), en orden inverso al de
    /// sus pasos. Es la única comprobación de qué se puede deshacer: la usan
    /// [`ChangeManager::deshacer`] y [`ChangeManager::se_puede_deshacer`].
    async fn inversa_de(&self, change_id: i64) -> Result<Vec<ChangeRequest>, CambioError> {
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
        if let Some(e) = self.ya_deshecho(change_id).await {
            return Err(CambioError::Invalida(e));
        }
        let no_se_puede = || {
            CambioError::Invalida(
                "este cambio no se puede deshacer (acción o valores anteriores desconocidos)"
                    .into(),
            )
        };
        let mut reqs = Vec::new();
        for paso in self.filas_pasos(change_id).await.iter().rev() {
            if paso.get::<String, _>("estado") != EstadoPaso::Aplicado.as_str() {
                continue;
            }
            let plan = self.reconstruir(paso).map_err(CambioError::Invalida)?;
            reqs.push(inversa(&plan).ok_or_else(no_se_puede)?);
        }
        if reqs.is_empty() {
            return Err(no_se_puede());
        }
        Ok(reqs)
    }

    /// Prepara el cambio inverso de uno `aplicado` (con los valores
    /// anteriores, secretos incluidos): en un grupo, los pasos inversos en
    /// orden inverso, que se confirman juntos con [`ChangeManager::confirmar`].
    pub async fn deshacer(&self, change_id: i64) -> Result<Preparado, CambioError> {
        let reqs = self.inversa_de(change_id).await?;
        self.preparar_con(reqs, true, Some(change_id)).await
    }

    /// ¿Aceptaría [`ChangeManager::deshacer`] este cambio? Solo mira el
    /// historial (no el router): para mostrar o no el botón.
    pub async fn se_puede_deshacer(&self, change_id: i64) -> bool {
        self.inversa_de(change_id).await.is_ok()
    }

    /// Enlaces de deshacer de un cambio: el que lo deshizo y el que él
    /// deshace. Solo guardan el vínculo de la última vez que se deshizo un
    /// cambio.
    pub async fn enlaces_deshacer(&self, id: i64) -> (Option<i64>, Option<i64>) {
        let consulta = |sql: &'static str| async move {
            sqlx::query_scalar::<_, i64>(sql)
                .bind(id)
                .fetch_optional(&self.state.db)
                .await
                .unwrap_or_else(|e| {
                    tracing::error!(cambio = id, "no se pueden leer los enlaces: {e}");
                    None
                })
        };
        (
            consulta("SELECT deshecho_por FROM changes WHERE id = ? AND deshecho_por IS NOT NULL")
                .await,
            consulta("SELECT id FROM changes WHERE deshecho_por = ? ORDER BY id DESC LIMIT 1")
                .await,
        )
    }

    /// Retoma los cambios que quedaron en `aplicando`/`vigilando` (llamar al
    /// arrancar, con el sondeo ya en marcha). Si el router muestra los
    /// valores nuevos, la vigilancia empieza de cero; si muestra los
    /// anteriores, `fallido` ("interrumpido antes de aplicar"); si no se
    /// puede saber, `fallido` con el motivo.
    ///
    /// Solo actúa la primera vez; hasta entonces `confirmar` espera.
    pub async fn reanudar_pendientes(self: &Arc<Self>) {
        if self
            .reanudando
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return;
        }
        // En su propia tarea: si entra en pánico se registra, y `listo` se
        // marca al salir pase lo que pase (también si se abandona este
        // futuro: la tarea sigue y lo marca al terminar).
        struct ListoAlSalir(Arc<ChangeManager>);
        impl Drop for ListoAlSalir {
            fn drop(&mut self) {
                self.0.listo.send_replace(true);
            }
        }
        let m = self.clone();
        let h = tokio::spawn(async move {
            let _listo = ListoAlSalir(m.clone());
            m.reanudar_filas().await;
        });
        if let Err(e) = h.await {
            tracing::error!("error interno al retomar los cambios pendientes: {e}");
        }
    }

    async fn reanudar_filas(self: &Arc<Self>) {
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
            // El hueco, antes de leer o tocar nada: lo que lo ocupe (un
            // cambio de credenciales que empezó antes, o el cambio anterior
            // de esta lista) termina antes de seguir. Así ningún grupo se da
            // por cerrado sin intentar revertirlo.
            let reserva = self.esperar_hueco(id).await;
            reserva.fijar(id);
            let aplicando = fila.get::<String, _>("estado") == Estado::Aplicando.as_str();
            // La WAN como estaba antes del cambio (nunca la de ahora).
            let wan_antes = fila.get::<Option<i64>, _>("wan_antes") != Some(0);
            let pasos: Vec<PasoGuardado> = self
                .filas_pasos(id)
                .await
                .iter()
                .map(|f| {
                    let estado = EstadoPaso::parse(&f.get::<String, _>("estado"))
                        .unwrap_or(EstadoPaso::Pendiente);
                    let escrito = matches!(estado, EstadoPaso::Aplicando | EstadoPaso::Aplicado);
                    PasoGuardado {
                        orden: f.get::<i64, _>("orden") as usize,
                        funcion: f.get("funcion"),
                        estado,
                        con_error: f.get::<Option<String>, _>("error").is_some(),
                        plan: if escrito {
                            self.reconstruir(f)
                        } else {
                            Err("sin escribir".into())
                        },
                    }
                })
                .collect();
            let activos: Vec<&PasoGuardado> = pasos
                .iter()
                .filter(|p| p.estado != EstadoPaso::Omitido)
                .collect();
            match activos.as_slice() {
                [] => {
                    let e = "interrumpido; el cambio no tiene pasos";
                    self.marcar(id, Estado::Fallido, Some(e), None).await;
                }
                [_] => {
                    let p = pasos.into_iter().find(|p| p.estado != EstadoPaso::Omitido);
                    let Some(p) = p else { continue };
                    match (p.estado, p.plan) {
                        (EstadoPaso::Aplicando | EstadoPaso::Aplicado, Ok(plan)) => {
                            self.reanudar(id, p.orden, plan, aplicando, wan_antes, reserva)
                                .await
                        }
                        (EstadoPaso::Aplicando | EstadoPaso::Aplicado, Err(e)) => {
                            self.marcar_paso(id, p.orden, EstadoPaso::Fallido, Some(&e))
                                .await;
                            self.marcar(id, Estado::Fallido, Some(&e), None).await;
                        }
                        // Sin escribir, fallido o ya revertido: nada aplicado
                        // que retomar.
                        _ => {
                            let e = "interrumpido: el cambio no quedó aplicado";
                            self.marcar(id, Estado::Fallido, Some(e), None).await;
                        }
                    }
                }
                _ => {
                    self.reanudar_grupo(id, pasos, aplicando, wan_antes, reserva)
                        .await
                }
            }
        }
    }

    /// Retoma un cambio de un paso (`orden`); `aplicando`: se cortó antes de
    /// terminar de escribir y verificar. `reserva`: el hueco, ya fijado.
    async fn reanudar(
        self: &Arc<Self>,
        id: i64,
        orden: usize,
        mut plan: Plan,
        aplicando: bool,
        wan_antes: bool,
        reserva: Reserva,
    ) {
        const RETOMADA: &str = "vigilancia retomada tras un reinicio del panel";
        let fallido = |e: String| async move {
            self.marcar_paso(id, orden, EstadoPaso::Fallido, Some(&e))
                .await;
            self.marcar(id, Estado::Fallido, Some(&e), None).await;
        };
        let c = &self.state.hgu;
        if plan.funcion.name == REBOOT {
            match read::summary(c).await {
                // No se sabe si la orden llegó al router.
                Ok(_) if aplicando => {
                    fallido("interrumpido: no se sabe si el router llegó a reiniciarse".into())
                        .await;
                }
                Ok(_) => {
                    self.marcar(id, Estado::Aplicado, None, Some(RETOMADA))
                        .await
                }
                Err(e) => {
                    fallido(format!(
                        "el router no responde tras reiniciar el panel: {e}"
                    ))
                    .await
                }
            }
            return;
        }
        let nota = match self.comprobar(&plan, Lado::Nuevo).await {
            Comprobacion::Coincide(nueva) => {
                if let Some(n) = nueva
                    && plan.instancia.is_none()
                {
                    self.fijar_instancia(id, orden, &n).await;
                    plan.instancia = Some(n);
                }
                RETOMADA.to_owned()
            }
            Comprobacion::SinLectura(e) => {
                fallido(format!(
                    "interrumpido; no se puede comprobar el estado: {e}"
                ))
                .await;
                return;
            }
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
                fallido(e).await;
                return;
            }
        };
        self.marcar_paso(id, orden, EstadoPaso::Aplicado, None)
            .await;
        self.marcar(id, Estado::Vigilando, None, Some(&nota)).await;
        let ej = Ejecucion {
            id,
            riesgo: plan.funcion.riesgo,
            pasos: vec![grupo::PasoEscrito {
                orden,
                plan,
                verificado: true,
            }],
            base: Base::Wan(wan_antes),
            nota: Some(nota),
            multi: false,
        };
        self.vigilar_en_segundo_plano(ej, reserva);
    }

    /// Tiempos con los que vigila este gestor.
    pub fn tiempos(&self) -> &Tiempos {
        &self.tiempos
    }

    /// ¿Se puede escribir `nombre`? Mismas reglas que [`ChangeManager::preparar`]
    /// (catálogo y bloqueos de la v1), sin leer el router.
    pub fn escribible(&self, nombre: &str) -> Result<(), CambioError> {
        self.funcion_escribible(nombre).map(drop)
    }

    /// Valores que muestra ahora la lectura pareada de `funcion` para la
    /// instancia (`valores` lleva los parámetros que la identifican). Son los
    /// mismos con los que [`ChangeManager::preparar`] calcula el diff. `None`
    /// si la función no tiene lectura comprobable.
    pub async fn valores_actuales(
        &self,
        funcion: &str,
        instancia: Option<&str>,
        valores: &BTreeMap<String, String>,
    ) -> Result<Option<BTreeMap<String, String>>, CambioError> {
        let f = self.funcion_escribible(funcion)?;
        match lectura_de(f, valores, instancia)? {
            Lectura::Ninguna => Ok(None),
            l => self.leer(&l).await.map(Some).map_err(|e| {
                CambioError::Router(format!("no se puede leer el estado actual: {e}"))
            }),
        }
    }

    /// Instancias de la lista de `funcion` según su lectura pareada (sin el
    /// id, con `all=ALL`), como las ve el gestor al verificar una creación.
    pub async fn instancias(
        &self,
        funcion: &str,
        valores: &BTreeMap<String, String>,
    ) -> Result<Vec<BTreeMap<String, String>>, CambioError> {
        let f = self.funcion_escribible(funcion)?;
        if f.lista.is_none() {
            return Err(CambioError::Invalida(format!("{funcion} no es una lista")));
        }
        // El id de la lectura se quita al listar: cualquier valor sirve.
        let l = lectura_de(f, valores, Some("-1"))?;
        self.listar(&l)
            .await
            .map_err(|e| CambioError::Router(format!("no se pueden listar las instancias: {e}")))
    }

    /// Un cambio del historial.
    pub async fn cambio(&self, id: i64) -> Option<Cambio> {
        let f = self.fila(id).await?;
        a_cambio(&f, &self.filas_pasos(id).await)
    }

    /// Valores anteriores secretos de cada paso del cambio `id`, en claro:
    /// `(orden, función, parámetro → valor)`. Son los que se guardaron
    /// cifrados: los secretos que cambiaron y se leyeron antes de escribir.
    /// Solo para «Mostrar el valor anterior» del Historial; un paso que no
    /// se puede descifrar (otra clave) queda fuera y se anota, sin valores.
    pub async fn anteriores_secretos(
        &self,
        id: i64,
    ) -> Vec<(i64, String, BTreeMap<String, String>)> {
        self.filas_pasos(id)
            .await
            .iter()
            .filter_map(|f| {
                let blob = f.get::<Option<Vec<u8>>, _>("valores_anteriores_cifrados")?;
                let orden: i64 = f.get("orden");
                let abierto = self
                    .state
                    .history_key
                    .open(&blob)
                    .and_then(|plano| serde_json::from_slice(&plano).ok());
                if abierto.is_none() {
                    tracing::warn!(
                        cambio = id,
                        orden,
                        "no se pueden descifrar los valores anteriores (¿otra clave?)"
                    );
                }
                Some((orden, f.get("funcion"), abierto?))
            })
            .collect()
    }

    /// Los `limite` cambios más recientes, del último al primero.
    pub async fn historial(&self, limite: u32) -> Vec<Cambio> {
        match sqlx::query("SELECT * FROM changes ORDER BY id DESC LIMIT ?")
            .bind(limite)
            .fetch_all(&self.state.db)
            .await
        {
            Ok(filas) => {
                let mut out = Vec::with_capacity(filas.len());
                for f in &filas {
                    let pasos = self.filas_pasos(f.get("id")).await;
                    out.extend(a_cambio(f, &pasos));
                }
                out
            }
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
        if CAMBIAN_IP_LAN.contains(&f.name.as_str()) {
            return Err(CambioError::Bloqueada(BLOQUEO_IP_LAN.into()));
        }
        let basica = f.origen == catalogo::Origen::Basica || !f.name.starts_with("cb");
        if basica
            && f.parametros
                .iter()
                .any(|p| p.tipo == catalogo::TipoParam::Secret)
        {
            return Err(CambioError::Bloqueada(BLOQUEO_SECRETOS_BASICA.into()));
        }
        Ok(f)
    }

    /// `explicito`: valores exactos (reversión/deshacer; ver [`componer`]).
    async fn planificar(&self, req: &ChangeRequest, explicito: bool) -> Result<Plan, CambioError> {
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
            secretos_tapados,
        } = componer(f, &self.state.mascara, op, &actual, &valores, explicito);
        if !secretos_tapados.is_empty() {
            tracing::warn!(
                funcion = %f.name,
                parametros = ?secretos_tapados,
                "el router da vacíos o tapados unos secretos que la escritura reenviaría: \
                 no se escribe"
            );
            return Err(CambioError::Bloqueada(BLOQUEO_SECRETO_TAPADO.into()));
        }
        if op == Operacion::Editar
            && let Some(c) = campos.iter().find(|c| !c.leido)
        {
            return Err(CambioError::Invalida(format!(
                "no se puede leer el valor actual de {}; sin valor anterior no se podría deshacer",
                c.nombre
            )));
        }
        // Lo que viene del router (reversión/deshacer) no se valida: puede
        // estar fuera de las opciones que conoce el catálogo.
        if op != Operacion::Borrar && !explicito {
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
            ids_antes: None,
        })
    }

    /// Ids de las instancias de la lista de `plan` (para saber cuál se crea).
    async fn ids_de_la_lista(&self, plan: &Plan) -> Result<BTreeSet<String>, hgu::Error> {
        let Some(id_param) = plan.funcion.lista.as_ref().map(|l| l.id_param.as_str()) else {
            return Ok(BTreeSet::new());
        };
        Ok(self
            .listar(&plan.lectura)
            .await?
            .iter()
            .filter_map(|e| buscar(e, id_param).map(str::to_owned))
            .collect())
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
        let c = self.comprobar_en_el_router(plan, lado).await;
        // El router puede aplicar una escritura un poco después de
        // responder: lo leído para mostrar hasta ahora quizá sea de antes.
        self.state.lecturas.invalidar();
        c
    }

    async fn comprobar_en_el_router(&self, plan: &Plan, lado: Lado) -> Comprobacion {
        let f = &plan.funcion;
        let no_leido =
            |e: hgu::Error| Comprobacion::SinLectura(format!("no se puede releer el estado: {e}"));
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
                let id_param = f.lista.as_ref().map(|l| l.id_param.as_str());
                let id_de = |e: &BTreeMap<String, String>| {
                    id_param.and_then(|p| buscar(e, p)).map(str::to_owned)
                };
                let con_valores = |e: &BTreeMap<String, String>| {
                    comparar(f, &plan.campos, e, Lado::Nuevo) == Comprobacion::Coincide(None)
                };
                // La instancia del cambio: la de id conocido; si no, la que
                // no estaba antes; sin ids de antes, por valores.
                let candidatas: Vec<&BTreeMap<String, String>> =
                    match (&plan.instancia, &plan.ids_antes, id_param) {
                        (Some(id), _, Some(_)) => lista
                            .iter()
                            .filter(|e| id_de(e).as_ref() == Some(id))
                            .collect(),
                        (None, Some(antes), Some(_)) => lista
                            .iter()
                            .filter(|e| id_de(e).is_some_and(|i| !antes.contains(&i)))
                            .collect(),
                        _ => lista.iter().filter(|e| con_valores(e)).collect(),
                    };
                match (lado, candidatas.as_slice()) {
                    (Lado::Nuevo, []) => {
                        Comprobacion::Difiere(vec!["no aparece la instancia nueva".into()])
                    }
                    (Lado::Nuevo, [e]) if con_valores(e) => {
                        Comprobacion::Coincide(plan.instancia.is_none().then(|| id_de(e)).flatten())
                    }
                    (Lado::Nuevo, [_]) => Comprobacion::Difiere(vec![
                        "la instancia nueva no tiene los valores escritos".into(),
                    ]),
                    (Lado::Nuevo, varias) => {
                        let ids: Vec<String> = varias.iter().filter_map(|e| id_de(e)).collect();
                        Comprobacion::Difiere(vec![format!(
                            "aparecen {} instancias nuevas ({}): no se sabe cuál es la de este \
                             cambio; no se borra nada",
                            varias.len(),
                            ids.join(", ")
                        )])
                    }
                    (Lado::Anterior, []) => Comprobacion::Coincide(None),
                    (Lado::Anterior, _) => {
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

    /// El router deja iniciar sesión y (si `wan`) la conexión a internet
    /// está arriba.
    async fn salud(&self, wan: bool) -> Result<(), String> {
        let c = &self.state.hgu;
        let no_responde = |e: hgu::Error| format!("el router no responde: {e}");
        read::summary(c).await.map_err(no_responde)?;
        if !wan {
            return Ok(());
        }
        let (vigilada, arriba) = self.state.wan_vigilada().await.map_err(no_responde)?;
        if !arriba {
            return Err(match vigilada {
                Some(n) => format!("la WAN ({n}) está caída"),
                None => "la WAN está caída (ninguna conexión de datos arriba)".to_owned(),
            });
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

    /// Un intento de reversión de `original` con la petición inversa `req`.
    ///
    /// Idempotente: antes de escribir comprueba si un intento anterior ya
    /// surtió efecto (`enviado`: lo que se escribió; p. ej. la instancia
    /// recreada ya existe) o si el router ya muestra lo anterior (p. ej. la
    /// instancia creada ya no existe). En ese caso termina sin escribir.
    async fn revertir_una_vez(
        &self,
        req: &ChangeRequest,
        original: &Plan,
        enviado: &mut Option<Plan>,
    ) -> Result<(), FalloReversion> {
        use FalloReversion::{Definitivo, Reintentar};
        let no_verificada =
            |v: Vec<String>| Definitivo(format!("reversión no verificada: {}", v.join("; ")));
        // El sondeo se pausa si se puede; si no, se revierte igualmente (las
        // peticiones al router van serializadas de todos modos).
        let _pausa = tokio::time::timeout(self.tiempos.pausa_max, self.state.poller.pause())
            .await
            .ok();
        if let Some(previo) = enviado.as_ref() {
            match self.comprobar(previo, Lado::Nuevo).await {
                Comprobacion::Coincide(_) => return Ok(()),
                Comprobacion::SinLectura(e) => return Err(Reintentar(e)),
                Comprobacion::Limitada | Comprobacion::Difiere(_) => {}
            }
        }
        match self.comprobar(original, Lado::Anterior).await {
            Comprobacion::Coincide(_) => return Ok(()),
            Comprobacion::SinLectura(e) => return Err(Reintentar(e)),
            Comprobacion::Limitada | Comprobacion::Difiere(_) => {}
        }
        let plan = match self.planificar(req, true).await {
            Ok(p) => p,
            // Nada que escribir pero el router no muestra lo anterior.
            Err(CambioError::SinCambios) => {
                return match self.comprobar(original, Lado::Anterior).await {
                    Comprobacion::Coincide(_) | Comprobacion::Limitada => Ok(()),
                    Comprobacion::SinLectura(e) => Err(Reintentar(e)),
                    Comprobacion::Difiere(v) => Err(no_verificada(v)),
                };
            }
            Err(CambioError::Router(e)) => return Err(Reintentar(e)),
            Err(e) => return Err(Definitivo(e.to_string())),
        };
        let mut plan = plan;
        if plan.operacion == Operacion::Crear && matches!(plan.lectura, Lectura::Xml { .. }) {
            match self.ids_de_la_lista(&plan).await {
                Ok(ids) => plan.ids_antes = Some(ids),
                Err(e) => return Err(Reintentar(e.to_string())),
            }
        }
        // Desde aquí la escritura puede haber llegado aunque falle la respuesta.
        *enviado = Some(plan.clone());
        match self.enviar(&plan).await {
            Ok(()) => {}
            // El router rechazó los valores: reintentar no cambiaría nada.
            Err(e @ hgu::Error::CallFailed { .. }) => {
                return Err(Definitivo(format!("el router rechazó la reversión: {e}")));
            }
            Err(e) => return Err(Reintentar(e.to_string())),
        }
        match self.comprobar(&plan, Lado::Nuevo).await {
            Comprobacion::Coincide(_) | Comprobacion::Limitada => Ok(()),
            Comprobacion::SinLectura(e) => Err(Reintentar(e)),
            Comprobacion::Difiere(v) => Err(no_verificada(v)),
        }
    }

    // ---- base de datos

    /// Lo que se guarda de un plan (secretos cifrados aparte).
    fn datos_plan(&self, plan: &Plan) -> DatosPlan {
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
        DatosPlan {
            operacion: plan.operacion.as_str(),
            solicitud: json(&solicitud),
            diff: json(&visible(&plan.campos).campos),
            anteriores,
            anteriores_cifrados,
            nuevos,
            nuevos_cifrados,
            ids_antes: plan.ids_antes.as_ref().map(json),
        }
    }

    /// Guarda un grupo nuevo (`aplicando`) con todos sus pasos: el principal
    /// con su plan (`aplicando`), y los demás `omitido` o `pendiente` (sin
    /// valores: se guardan al llegar a ellos).
    async fn insertar(&self, g: &NuevoGrupo<'_>) -> Result<i64, sqlx::Error> {
        let (i0, plan) = g.principal;
        let d = self.datos_plan(plan);
        let wan_antes = match g.base {
            Base::Wan(up) => Some(up),
            Base::Uptime(_) => None,
        };
        let now = series::now();
        let mut tx = self.state.db.begin().await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO changes (ts, updated, user_id, usuario, funcion, instancia, riesgo, \
             estado, operacion, diff, anteriores, anteriores_cifrados, solicitud, nuevos, \
             nuevos_cifrados, wan_antes, ids_antes, titulo) VALUES (?, ?, \
             (SELECT id FROM users WHERE username = ?), \
             ?, ?, ?, ?, 'aplicando', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(now)
        .bind(now)
        .bind(g.usuario)
        .bind(g.usuario)
        .bind(&plan.funcion.name)
        .bind(&plan.instancia)
        .bind(riesgo_str(g.riesgo))
        .bind(d.operacion)
        .bind(&d.diff)
        .bind(&d.anteriores)
        .bind(&d.anteriores_cifrados)
        .bind(&d.solicitud)
        .bind(&d.nuevos)
        .bind(&d.nuevos_cifrados)
        .bind(wan_antes)
        .bind(&d.ids_antes)
        .bind(g.titulo)
        .fetch_one(&mut *tx)
        .await?;
        for (i, paso) in g.pasos.iter().enumerate() {
            let (instancia, datos, estado) = if i == i0 {
                (&plan.instancia, &d, EstadoPaso::Aplicando)
            } else {
                (&paso.req.instancia, &DatosPlan::vacio(), g.estados[i])
            };
            sqlx::query(
                "INSERT INTO cambio_pasos (cambio_id, orden, funcion, instancia, etiqueta, \
                 operacion, solicitud, diff, valores_nuevos, valores_nuevos_cifrados, \
                 valores_anteriores, valores_anteriores_cifrados, ids_antes, estado) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(i as i64)
            .bind(&paso.req.funcion)
            .bind(instancia)
            .bind(&paso.etiqueta)
            .bind(datos.operacion)
            .bind(&datos.solicitud)
            .bind(&datos.diff)
            .bind(datos.nuevos.as_bytes())
            .bind(&datos.nuevos_cifrados)
            .bind(datos.anteriores.as_bytes())
            .bind(&datos.anteriores_cifrados)
            .bind(&datos.ids_antes)
            .bind(estado.as_str())
            .execute(&mut *tx)
            .await?;
        }
        if let Some(o) = g.deshace {
            sqlx::query("UPDATE changes SET deshecho_por = ? WHERE id = ?")
                .bind(id)
                .bind(o)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(id)
    }

    /// Guarda el plan de un paso que se va a escribir (`aplicando`).
    async fn guardar_paso(&self, id: i64, orden: usize, plan: &Plan) -> Result<(), sqlx::Error> {
        let d = self.datos_plan(plan);
        sqlx::query(
            "UPDATE cambio_pasos SET instancia = ?, operacion = ?, solicitud = ?, diff = ?, \
             valores_nuevos = ?, valores_nuevos_cifrados = ?, valores_anteriores = ?, \
             valores_anteriores_cifrados = ?, ids_antes = ?, estado = 'aplicando' \
             WHERE cambio_id = ? AND orden = ?",
        )
        .bind(&plan.instancia)
        .bind(d.operacion)
        .bind(&d.solicitud)
        .bind(&d.diff)
        .bind(d.nuevos.as_bytes())
        .bind(&d.nuevos_cifrados)
        .bind(d.anteriores.as_bytes())
        .bind(&d.anteriores_cifrados)
        .bind(&d.ids_antes)
        .bind(id)
        .bind(orden as i64)
        .execute(&self.state.db)
        .await
        .map(drop)
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

    /// Cambia el estado de un paso; `error` solo se escribe si es `Some`.
    async fn marcar_paso(&self, id: i64, orden: usize, estado: EstadoPaso, error: Option<&str>) {
        let r = sqlx::query(
            "UPDATE cambio_pasos SET estado = ?, error = COALESCE(?, error) \
             WHERE cambio_id = ? AND orden = ?",
        )
        .bind(estado.as_str())
        .bind(error)
        .bind(id)
        .bind(orden as i64)
        .execute(&self.state.db)
        .await;
        if let Err(e) = r {
            tracing::error!(
                cambio = id,
                paso = orden + 1,
                "no se puede guardar el estado {estado:?}: {e}"
            );
        }
    }

    /// La instancia creada por el paso `orden` (también la del cambio si es
    /// su primer paso escrito).
    async fn fijar_instancia(&self, id: i64, orden: usize, instancia: &str) {
        let r = async {
            sqlx::query("UPDATE cambio_pasos SET instancia = ? WHERE cambio_id = ? AND orden = ?")
                .bind(instancia)
                .bind(id)
                .bind(orden as i64)
                .execute(&self.state.db)
                .await?;
            sqlx::query(
                "UPDATE changes SET instancia = ?1 WHERE id = ?2 AND ?3 = (SELECT MIN(orden) \
                 FROM cambio_pasos WHERE cambio_id = ?2 AND estado NOT IN ('omitido', \
                 'pendiente'))",
            )
            .bind(instancia)
            .bind(id)
            .bind(orden as i64)
            .execute(&self.state.db)
            .await
        }
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

    /// Pasos de un cambio, en orden.
    async fn filas_pasos(&self, id: i64) -> Vec<sqlx::sqlite::SqliteRow> {
        sqlx::query("SELECT * FROM cambio_pasos WHERE cambio_id = ? ORDER BY orden")
            .bind(id)
            .fetch_all(&self.state.db)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(cambio = id, "no se pueden leer los pasos: {e}");
                Vec::new()
            })
    }

    /// Plan de un paso guardado (sin carga: solo para comprobar, revertir o
    /// deshacer). Los secretos se descifran con la clave del historial.
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
            serde_json::from_slice(&fila.get::<Vec<u8>, _>(col))
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
        let mut anteriores = mapa("valores_anteriores")?;
        abrir("valores_anteriores_cifrados", &mut anteriores)?;
        let mut nuevos = mapa("valores_nuevos")?;
        abrir("valores_nuevos_cifrados", &mut nuevos)?;
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
        let mut valores: BTreeMap<String, String> =
            serde_json::from_str(&fila.get::<String, _>("solicitud"))
                .map_err(|e| format!("solicitud no válida: {e}"))?;
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
            ids_antes: fila
                .get::<Option<String>, _>("ids_antes")
                .and_then(|j| serde_json::from_str(&j).ok()),
        })
    }
}

/// Lo que se guarda de un plan.
struct DatosPlan {
    operacion: &'static str,
    /// JSON de los valores enviados no secretos.
    solicitud: String,
    /// JSON del diff (secretos enmascarados).
    diff: String,
    /// JSON de los valores no secretos y, aparte, los secretos cifrados.
    anteriores: String,
    anteriores_cifrados: Option<Vec<u8>>,
    nuevos: String,
    nuevos_cifrados: Option<Vec<u8>>,
    ids_antes: Option<String>,
}

impl DatosPlan {
    /// Un paso al que aún no se ha llegado (o que se omite).
    fn vacio() -> DatosPlan {
        DatosPlan {
            operacion: Operacion::Editar.as_str(),
            solicitud: "{}".into(),
            diff: "[]".into(),
            anteriores: "{}".into(),
            anteriores_cifrados: None,
            nuevos: "{}".into(),
            nuevos_cifrados: None,
            ids_antes: None,
        }
    }
}

/// Un grupo que empieza a aplicarse.
struct NuevoGrupo<'a> {
    /// El primer paso que cambia algo (posición y plan).
    principal: (usize, &'a Plan),
    pasos: &'a [Paso],
    /// Estado inicial de cada paso (el principal se guarda `aplicando`).
    estados: &'a [EstadoPaso],
    titulo: Option<&'a str>,
    riesgo: Riesgo,
    usuario: &'a str,
    base: Base,
    deshace: Option<i64>,
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

fn a_paso(f: &sqlx::sqlite::SqliteRow) -> Option<PasoCambio> {
    Some(PasoCambio {
        orden: f.get("orden"),
        funcion: f.get("funcion"),
        instancia: f.get("instancia"),
        etiqueta: f.get("etiqueta"),
        estado: EstadoPaso::parse(&f.get::<String, _>("estado"))?,
        diff: Diff {
            campos: serde_json::from_str(&f.get::<String, _>("diff")).unwrap_or_default(),
        },
        error: f.get("error"),
    })
}

/// Un cambio con sus pasos. Función e instancia: las de su primer paso
/// escrito; diff: los de todos los pasos juntos.
fn a_cambio(f: &sqlx::sqlite::SqliteRow, pasos: &[sqlx::sqlite::SqliteRow]) -> Option<Cambio> {
    let riesgo = match f.get::<String, _>("riesgo").as_str() {
        "bajo" => Riesgo::Bajo,
        "medio" => Riesgo::Medio,
        _ => Riesgo::Alto,
    };
    let pasos: Vec<PasoCambio> = pasos.iter().filter_map(a_paso).collect();
    let principal = pasos
        .iter()
        .find(|p| !matches!(p.estado, EstadoPaso::Omitido | EstadoPaso::Pendiente));
    let diff = if pasos.is_empty() {
        Diff {
            campos: serde_json::from_str(&f.get::<String, _>("diff")).unwrap_or_default(),
        }
    } else {
        Diff {
            campos: pasos
                .iter()
                .flat_map(|p| p.diff.campos.iter().cloned())
                .collect(),
        }
    };
    Some(Cambio {
        id: f.get("id"),
        ts: f.get("ts"),
        updated: f.get("updated"),
        funcion: principal.map_or_else(|| f.get("funcion"), |p| p.funcion.clone()),
        instancia: principal.map_or_else(|| f.get("instancia"), |p| p.instancia.clone()),
        riesgo,
        estado: Estado::parse(&f.get::<String, _>("estado"))?,
        diff,
        error: f.get("error"),
        nota: f.get("nota"),
        usuario: f.get("usuario"),
        titulo: f.get("titulo"),
        pasos,
    })
}
