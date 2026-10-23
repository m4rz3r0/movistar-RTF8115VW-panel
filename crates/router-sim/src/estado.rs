//! Estado en memoria del simulador: fixtures, sesiones, escrituras y fallos programados.

use crate::usuarios::Usuario;
use catalogo::Catalogo;
use hgu::xml::XmlNode;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64},
    },
    time::{Duration, Instant},
};

pub(crate) type Params = Vec<(String, String)>;
pub(crate) type FixtureKey = (String, Params);

pub(crate) struct Inner {
    /// Usuario del simulador: el que cambia `cbModifyPwd`.
    pub(crate) user: String,
    /// Usuarios de la web del router, con el del simulador (su contraseña
    /// se cambia con [`SimControl::set_password`]).
    pub(crate) usuarios: Mutex<Vec<Usuario>>,
    pub(crate) fixtures: HashMap<FixtureKey, String>,
    /// Catálogo de funciones (lecturas pareadas y listas).
    pub(crate) catalogo: Catalogo,
    /// Conexión a internet de las fixtures (ver [`wan_de_las_fixtures`]).
    pub(crate) wan: Option<String>,
    pub(crate) unresponsive: AtomicBool,
    /// Retardo de cada respuesta en milisegundos (`set_retardo`).
    pub(crate) retardo_ms: AtomicU64,
    pub(crate) state: Mutex<SimState>,
}

impl Inner {
    /// Contraseña del usuario del simulador (vacía si alguien lo borró).
    pub(crate) fn password(&self) -> String {
        self.usuarios()
            .iter()
            .find(|u| u.nombre == self.user)
            .map(|u| u.contrasena.clone())
            .unwrap_or_default()
    }

    /// Pone `nueva` como contraseña del usuario del simulador.
    pub(crate) fn fijar_password(&self, nueva: String) {
        if let Some(u) = self.usuarios().iter_mut().find(|u| u.nombre == self.user) {
            u.contrasena = nueva;
        }
    }

    pub(crate) fn usuarios(&self) -> MutexGuard<'_, Vec<Usuario>> {
        self.usuarios.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn state(&self) -> MutexGuard<'_, SimState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Aplica la contraseña pendiente de un `cbModifyPwd` aplazado si ya
    /// toca (`modify_pwd_con_retraso`, `modify_pwd_tras_inicios`). La llama
    /// cada inicio de sesión; `basico` dice si es el básico, el único que
    /// cuenta para `modify_pwd_tras_inicios`.
    pub(crate) fn aplicar_contrasena_pendiente(&self, st: &mut SimState, basico: bool) {
        let toca = match &mut st.pwd_pendiente {
            Some((_, Aplazada::Desde(desde))) => Instant::now() >= *desde,
            Some((_, Aplazada::TrasInicios(n))) if basico => {
                let toca = *n == 0;
                *n = n.saturating_sub(1);
                toca
            }
            _ => false,
        };
        if toca && let Some((nueva, _)) = st.pwd_pendiente.take() {
            self.fijar_password(nueva);
        }
    }
}

/// Cuándo aplica el router la contraseña de un `cbModifyPwd` aplazado.
pub(crate) enum Aplazada {
    /// En el primer inicio de sesión desde este instante.
    Desde(Instant),
    /// En el inicio de sesión básico que sigue a estos (que aún no la ven).
    TrasInicios(u32),
}

/// Lo escrito en una instancia de una lectura que no es una lista: campos
/// que sustituyen a los de la respuesta de la fixture.
pub(crate) struct Edicion {
    /// Función de lectura pareada.
    pub(crate) lectura: String,
    /// `(hijo, valor)` que identifican la instancia (vacío: la única).
    pub(crate) ident: Params,
    pub(crate) campos: BTreeMap<String, String>,
}

/// Lista materializada de una lectura (la primera escritura la crea a partir
/// de las fixtures).
pub(crate) struct ListaViva {
    /// Primera instancia de las fixtures: molde de las nuevas.
    pub(crate) plantilla: XmlNode,
    pub(crate) nodos: Vec<XmlNode>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HangAfterWrite {
    #[default]
    Off,
    Armed,
    /// La escritura llegó: colgarse en cuanto se responda.
    Now,
}

#[derive(Default)]
pub(crate) struct SimState {
    pub(crate) seed: u32,
    pub(crate) counter: u32,
    /// `_httpdSessionId_` con login básico correcto.
    pub(crate) basic: HashSet<String>,
    /// `SessionID` del login avanzado.
    pub(crate) advanced: HashSet<String>,
    pub(crate) session_key: Option<String>,
    pub(crate) wan_up: bool,
    pub(crate) fail_next: Option<String>,
    pub(crate) ignore_next: bool,
    /// Lecturas que fallan (`fail_reads`).
    pub(crate) failing_reads: HashSet<String>,
    /// Lecturas que fallan con un parámetro (`fail_reads_con`): nombre,
    /// parámetro y valor.
    pub(crate) failing_reads_con: HashSet<(String, String, String)>,
    /// Lectura que empieza a fallar tras la próxima escritura.
    pub(crate) fail_reads_after_write: Option<String>,
    /// `hang_after_next_write`.
    pub(crate) hang_after_write: HangAfterWrite,
    /// `no_answer_next_write`.
    pub(crate) no_answer_next: bool,
    /// `create_twice_next_write`.
    pub(crate) create_twice: bool,
    /// Escrituras reales recibidas (para `fail_write_n`/`hang_after_write_n`).
    pub(crate) real_writes: usize,
    /// Números de escritura real que fallan (`fail_write_n`).
    pub(crate) fail_write_at: HashSet<usize>,
    /// Número de escritura real tras la que se cuelga (`hang_after_write_n`).
    pub(crate) hang_at_write: Option<usize>,
    /// Bytes sumados a cada interfaz de `cbGetStatistic`: nombre → (rx, tx).
    pub(crate) counters_extra: HashMap<String, (u64, u64)>,
    pub(crate) writes: Vec<(String, Params)>,
    /// Lecturas avanzadas recibidas con sesión válida (`reads`).
    pub(crate) reads: Vec<(String, Params)>,
    /// Escrituras por instancia de lecturas que no son listas.
    pub(crate) ediciones: Vec<Edicion>,
    /// Listas materializadas por función de lectura.
    pub(crate) listas: HashMap<String, ListaViva>,
    /// `modify_pwd_sin_efecto`.
    pub(crate) modify_pwd_sin_efecto: bool,
    /// `modify_pwd_con_retraso`: el retraso del próximo `cbModifyPwd`.
    pub(crate) modify_pwd_retraso: Option<Duration>,
    /// `modify_pwd_tras_inicios`: los inicios de sesión del próximo
    /// `cbModifyPwd` que aún no ven la nueva.
    pub(crate) modify_pwd_tras_inicios: Option<u32>,
    /// Contraseña de un `cbModifyPwd` aplazado y cuándo vale.
    pub(crate) pwd_pendiente: Option<(String, Aplazada)>,
    /// `login_falla_con`.
    pub(crate) login_falla_con: Option<String>,
    /// Último `cbReboot`: `SysUptime` cuenta desde aquí.
    pub(crate) rebooted: Option<Instant>,
    /// `SWVersion` de `cbGetSummary` (`set_firmware`).
    pub(crate) firmware: Option<String>,
    /// Filtros del cortafuegos (ver `cortafuegos`).
    pub(crate) cortafuegos: Vec<hgu::cortafuegos::Filtro>,
    /// `cortafuegos_ilegible`: la página de la lista sale sin `var filter`.
    pub(crate) cortafuegos_ilegible: bool,
}

impl SimState {
    /// Identificador numérico nuevo (cookies, `SessionID`, `sessionKey`).
    pub(crate) fn next_token(&mut self) -> String {
        self.counter += 1;
        let n = u64::from(self.seed) * 1000 + u64::from(self.counter);
        format!("{}", n % 1_000_000_000_000)
    }

    pub(crate) fn invalidate_sessions(&mut self) {
        self.basic.clear();
        self.advanced.clear();
        self.session_key = None;
    }
}
