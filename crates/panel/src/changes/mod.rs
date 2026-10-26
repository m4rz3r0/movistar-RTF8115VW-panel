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
//! Un único cambio activo a la vez: otro `confirmar` mientras tanto espera
//! su turno en la cola (`cola.rs`; como mucho [`MAX_COLA`], después
//! [`CambioError::ColaLlena`]). Al arrancar, [`ChangeManager::reanudar_pendientes`]
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

mod acceso;
mod cola;
mod consultas;
mod especiales;
mod grupo;
mod interruptores;
mod motivos;
mod nota;
mod persistencia;
pub mod plan;
mod preparar;
mod reanudar;
mod router;
mod terminar;
mod tipos;
mod vigilancia;

pub use acceso::Enlace;
pub use cola::{ErrorQuitar, MAX_COLA};
/// Motivo (clave i18n) de un 409 cuando la regla «sin ping» del
/// cortafuegos ya está en el router.
pub(crate) use especiales::SIN_PING_YA_PUESTA;
/// La lista de usuarios del router, como la lee el gestor (para el cambio
/// de otro usuario, que no pasa por él).
pub(crate) use especiales::{LECTURA_USUARIOS, es_de_usuarios, usuarios_de};
use grupo::{Ejecucion, PasoGuardado};
pub use grupo::{EstadoPaso, GrupoRequest, Paso, PasoCambio, PasoPreparado, PreparadoGrupo};
pub use motivos::*;
pub use nota::Nota;
use plan::*;
pub use plan::{PALABRA_ACCION, PALABRA_REINICIO};
/// Reglas del gestor que comparte el explorador (qué identifica una
/// instancia y cómo se busca un campo en una lectura): una sola copia.
pub(crate) use plan::{buscar, identidad};
pub use terminar::ErrorTerminar;
pub use tipos::*;

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

/// Mientras viva, el gestor está ocupado con un cambio (o lo tiene apartado
/// con [`ChangeManager::excluir_cambios`]): no empieza ninguno.
pub struct Reserva {
    m: Arc<ChangeManager>,
    /// El hueco ya se vació al acabar la cola (`cola.rs`): al soltarla no
    /// se toca.
    suelta: bool,
}

impl Reserva {
    fn fijar(&self, id: i64) {
        lock(&self.m.hueco).activo = Some(Some(id));
    }
}

impl Drop for Reserva {
    fn drop(&mut self) {
        if !self.suelta {
            let cola = {
                let mut h = lock(&self.m.hueco);
                h.activo = None;
                h.con_cola = false;
                std::mem::take(&mut h.cola)
            };
            // La tarea se cortó (`detener`) con cambios esperando: no se
            // aplican.
            self.m.cancelar_al_cortarse(cola);
        }
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
    /// El cambio activo y la cola detrás de él.
    hueco: Mutex<cola::Hueco>,
    /// Tarea del cambio en curso (aplicación y vigilancia).
    tarea: Mutex<Option<AbortHandle>>,
    /// `true` cuando [`ChangeManager::reanudar_pendientes`] ha terminado.
    listo: watch::Sender<bool>,
    /// `reanudar_pendientes` ya empezó (solo se hace una vez).
    reanudando: std::sync::atomic::AtomicBool,
    /// Interruptores de la validación real de este gestor
    /// ([`ChangeManager::desactivadas`]; ver `interruptores.rs`).
    interruptores: interruptores::Interruptores,
    /// Por dónde llega el panel al router, y cuándo se averiguó
    /// ([`ChangeManager::enlace`]).
    enlace: Mutex<Option<(Instant, Enlace)>>,
    /// Vigilar todos los cambios ([`ChangeManager::vigilar_siempre`]).
    siempre: std::sync::atomic::AtomicBool,
    /// La vigilancia en curso, para terminarla a mano
    /// ([`ChangeManager::terminar`]).
    vigilancia: Mutex<Option<terminar::EnVigilancia>>,
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
            hueco: Mutex::new(cola::Hueco::default()),
            tarea: Mutex::new(None),
            listo: watch::Sender::new(false),
            reanudando: std::sync::atomic::AtomicBool::new(false),
            interruptores: interruptores::Interruptores::new(state.cfg.todo_habilitado()),
            enlace: Mutex::new(None),
            siempre: std::sync::atomic::AtomicBool::new(false),
            vigilancia: Mutex::new(None),
        })
    }

    /// Vigila todos los cambios, también los que no pueden cortar el acceso
    /// del panel. Solo para las pruebas de la reversión automática: el
    /// servidor no lo usa nunca.
    #[doc(hidden)]
    pub fn vigilar_siempre(&self) {
        self.siempre
            .store(true, std::sync::atomic::Ordering::SeqCst);
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
        lock(&self.hueco).activo.flatten()
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
        self.reservar(false)
    }

    /// Espera a que el hueco quede libre y lo reserva (al retomar: lo que
    /// lo ocupe, p. ej. un cambio de credenciales, termina pronto).
    /// `id`: el cambio que se va a retomar (para el registro).
    async fn esperar_hueco(self: &Arc<Self>, id: i64) -> Reserva {
        let mut avisado = false;
        loop {
            if let Ok(r) = self.reservar(false) {
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

    /// Tiempos con los que vigila este gestor.
    pub fn tiempos(&self) -> &Tiempos {
        &self.tiempos
    }

    /// Reserva el hueco libre. `con_cola`: lo tiene la tarea de un cambio,
    /// que sigue con la cola al terminar (se puede encolar detrás).
    fn reservar(self: &Arc<Self>, con_cola: bool) -> Result<Reserva, CambioError> {
        let mut h = lock(&self.hueco);
        if h.activo.is_some() {
            return Err(CambioError::EnCurso);
        }
        h.activo = Some(None);
        h.con_cola = con_cola;
        Ok(Reserva {
            m: self.clone(),
            suelta: false,
        })
    }
}
