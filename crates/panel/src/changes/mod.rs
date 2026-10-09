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
//! se reenvía el valor actual leído del router y no cuenta como cambio. Los
//! registros (`tracing`) y el historial solo muestran secretos enmascarados.

mod plan;

use plan::*;
pub use plan::{PALABRA_ACCION, PALABRA_REINICIO};

use crate::{crypto::HistoryKey, poller::PollerHandle, series, state::AppState};
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
    collections::{BTreeMap, HashMap},
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
const BLOQUEO_IP_LAN: &str = "bloqueada en v1: cambiar la IP del router corta la conexión del \
                              propio panel; hazlo desde la web del router";
const BLOQUEO_SECRETOS_BASICA: &str = "bloqueada en v1: falta validar con el router real el \
                                       formato de los secretos de la web básica";

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
        Arc::new(ChangeManager {
            state: Recursos {
                db: state.db,
                hgu: state.hgu,
                catalogo: state.catalogo,
                mascara: state.mascara,
                poller: state.poller,
                history_key: state.history_key,
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
        self.preparar_con(req, false, None).await
    }

    async fn preparar_con(
        &self,
        req: ChangeRequest,
        explicito: bool,
        deshace: Option<i64>,
    ) -> Result<Preparado, CambioError> {
        let plan = self.planificar(&req, explicito).await?;
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
                explicito,
                deshace,
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
    ///
    /// Desde la reserva del hueco, el trabajo corre en una tarea propia: si
    /// este futuro se abandona, el cambio sigue hasta un estado final.
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
        // Un cambio nuevo no puede adelantarse a uno pendiente de retomar.
        let mut listo = self.listo.subscribe();
        if tokio::time::timeout(self.tiempos.pausa_max, listo.wait_for(|v| *v))
            .await
            .is_err()
        {
            return Err(CambioError::Router(
                "el gestor aún está retomando cambios pendientes; inténtalo de nuevo".into(),
            ));
        }
        let reserva = self.reservar()?;
        let Some(pendiente) = lock(&self.pendientes).remove(token) else {
            return Err(CambioError::TokenInvalido);
        };
        let (tx, rx) = oneshot::channel();
        let m = self.clone();
        let usuario = usuario.to_owned();
        let h = tokio::spawn(async move {
            match m.aplicar(&pendiente, &usuario, &reserva).await {
                Ok((id, plan, base, nota)) => {
                    let _ = tx.send(Ok(id));
                    m.seguir(id, plan, base, nota).await;
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                }
            }
            drop(reserva);
        });
        *lock(&self.tarea) = Some(h.abort_handle());
        rx.await.unwrap_or_else(|_| {
            Err(CambioError::Router(
                "la aplicación del cambio se interrumpió".into(),
            ))
        })
    }

    /// Escribe y verifica (con el sondeo en pausa). Devuelve lo necesario
    /// para vigilar.
    async fn aplicar(
        &self,
        p: &Pendiente,
        usuario: &str,
        reserva: &Reserva,
    ) -> Result<(i64, Plan, Base, Option<String>), CambioError> {
        let pausa = tokio::time::timeout(self.tiempos.pausa_max, self.state.poller.pause())
            .await
            .map_err(|_| {
                CambioError::Router("el sondeo no se detuvo a tiempo; no se ha escrito nada".into())
            })?;
        if let Some(o) = p.deshace
            && let Some(e) = self.ya_deshecho(o).await
        {
            return Err(CambioError::Invalida(e));
        }
        let mut plan = self.planificar(&p.req, p.explicito).await?;
        let f = plan.funcion.clone();
        let c = &self.state.hgu;
        let no_responde =
            |e: hgu::Error| CambioError::Router(format!("el router no responde: {e}"));
        let base = if f.name == REBOOT {
            Base::Uptime(read::summary(c).await.map_err(no_responde)?.uptime_s)
        } else {
            Base::Wan(wan_arriba(&read::wan(c).await.map_err(no_responde)?))
        };
        let id = self
            .insertar(&plan, usuario, base, p.deshace)
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
                    plan.instancia = Some(n);
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
            Comprobacion::SinLectura(e) => {
                self.marcar(id, Estado::Fallido, Some(&e), None).await;
                return Err(CambioError::Verificacion(vec![e]));
            }
        }
        self.marcar(id, Estado::Vigilando, None, nota.as_deref())
            .await;
        drop(pausa);
        Ok((id, plan, base, nota))
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
        let vigente = [Estado::Aplicando, Estado::Vigilando, Estado::Aplicado]
            .iter()
            .any(|e| e.as_str() == estado);
        vigente.then(|| format!("el cambio {original} ya se ha deshecho (cambio {d})"))
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
        if let Some(e) = self.ya_deshecho(change_id).await {
            return Err(CambioError::Invalida(e));
        }
        let plan = self.reconstruir(&fila).map_err(CambioError::Invalida)?;
        let req = inversa(&plan).ok_or_else(|| {
            CambioError::Invalida(
                "este cambio no se puede deshacer (acción o valores anteriores desconocidos)"
                    .into(),
            )
        })?;
        self.preparar_con(req, true, Some(change_id)).await
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
            match self.reconstruir(&fila) {
                Ok(plan) => {
                    let aplicando = fila.get::<String, _>("estado") == Estado::Aplicando.as_str();
                    // La WAN como estaba antes del cambio (nunca la de ahora).
                    let wan_antes = fila.get::<Option<i64>, _>("wan_antes") != Some(0);
                    self.reanudar(id, plan, aplicando, wan_antes).await
                }
                Err(e) => self.marcar(id, Estado::Fallido, Some(&e), None).await,
            }
        }
    }

    /// Retoma un cambio; `aplicando`: se cortó antes de terminar de
    /// escribir y verificar.
    async fn reanudar(self: &Arc<Self>, id: i64, mut plan: Plan, aplicando: bool, wan_antes: bool) {
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
            Comprobacion::Coincide(nueva) => {
                if let Some(n) = nueva
                    && plan.instancia.is_none()
                {
                    self.fijar_instancia(id, &n).await;
                    plan.instancia = Some(n);
                }
                RETOMADA.to_owned()
            }
            Comprobacion::SinLectura(e) => {
                let e = format!("interrumpido; no se puede comprobar el estado: {e}");
                self.marcar(id, Estado::Fallido, Some(&e), None).await;
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
        let base = Base::Wan(wan_antes);
        self.marcar(id, Estado::Vigilando, None, Some(&nota)).await;
        self.vigilar_en_segundo_plano(id, plan, base, Some(nota), reserva);
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
        } = componer(f, &self.state.mascara, op, &actual, &valores, explicito);
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

    /// Escribe los valores anteriores y verifica. Los fallos transitorios
    /// (router sin respuesta, relectura fallida) se reintentan durante
    /// `alto`; los deterministas terminan en `fallido` en el acto.
    async fn revertir(&self, id: i64, plan: &Plan, motivo: &str, nota: Option<&str>) {
        let Some(req) = inversa(plan) else {
            let e = format!("{motivo}; no se puede revertir: valores anteriores desconocidos");
            self.marcar(id, Estado::Fallido, Some(&e), nota).await;
            return;
        };
        tracing::warn!(cambio = id, "{motivo}; revirtiendo");
        let fin = Instant::now() + self.tiempos.alto;
        // Plan del último intento que llegó a escribir (o pudo llegar).
        let mut enviado = None;
        let ultimo = loop {
            let error = match self.revertir_una_vez(&req, plan, &mut enviado).await {
                Ok(()) => {
                    let e = format!("{motivo}; revertido a los valores anteriores");
                    self.marcar(id, Estado::Revertido, Some(&e), nota).await;
                    return;
                }
                Err(FalloReversion::Definitivo(e)) => {
                    let e = format!("{motivo}; no se pudo revertir: {e}");
                    self.marcar(id, Estado::Fallido, Some(&e), nota).await;
                    return;
                }
                Err(FalloReversion::Reintentar(e)) => e,
            };
            tracing::warn!(cambio = id, "reversión: {error}; se reintenta");
            if Instant::now() >= fin {
                break error;
            }
            tokio::time::sleep(self.tiempos.intervalo).await;
        };
        let e = format!("{motivo}; no se pudo revertir: {ultimo}");
        self.marcar(id, Estado::Fallido, Some(&e), nota).await;
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

    async fn insertar(
        &self,
        plan: &Plan,
        usuario: &str,
        base: Base,
        deshace: Option<i64>,
    ) -> Result<i64, sqlx::Error> {
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
        let wan_antes = match base {
            Base::Wan(up) => Some(up),
            Base::Uptime(_) => None,
        };
        let now = series::now();
        let mut tx = self.state.db.begin().await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO changes (ts, updated, user_id, usuario, funcion, instancia, riesgo, \
             estado, operacion, diff, anteriores, anteriores_cifrados, solicitud, nuevos, \
             nuevos_cifrados, wan_antes) VALUES (?, ?, (SELECT id FROM users WHERE username = ?), \
             ?, ?, ?, ?, 'aplicando', ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
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
        .bind(wan_antes)
        .fetch_one(&mut *tx)
        .await?;
        if let Some(o) = deshace {
            sqlx::query("UPDATE changes SET deshecho_por = ? WHERE id = ?")
                .bind(id)
                .bind(o)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(id)
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
