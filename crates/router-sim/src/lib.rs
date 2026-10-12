//! Router Askey RTF8115VW simulado.
//!
//! Sirve las fixtures saneadas con el mismo protocolo que el router real
//! (login básico y avanzado, cookies, `sessionKey` en las páginas `.asp`) y
//! refleja en memoria cualquier escritura avanzada en su lectura pareada del
//! catálogo (`catalogo`), con listas (crear, editar y borrar) y `cbModifyPwd`.

pub mod sanitize;

use axum::{
    Router,
    body::Bytes,
    extract::{Path, RawQuery, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use catalogo::{Catalogo, Lista};
use hgu::{fmt::mess_userpass, xml::XmlNode};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    net::SocketAddr,
    path::{Path as FsPath, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// Cuerpo que devuelve el router cuando la sesión no es válida.
const LOGGED_OUT: &str = "<script>top.location.href='te_acceso_router.asp';</script>";
const XML_HEADER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\r\n";
/// Puerto LAN del equipo de `counters_bump`.
const LAN_PORT: &str = "eth0.4";

/// Configuración del simulador.
#[derive(Clone, Debug)]
pub struct SimConfig {
    /// Usuario aceptado en el login.
    pub user: String,
    /// Contraseña aceptada en el login.
    pub password: String,
    /// Directorio con las fixtures `<name>[__<params>].xml`.
    pub fixtures_dir: PathBuf,
    /// Directorio con fixtures que sustituyen a las de `fixtures_dir` (una
    /// variante de línea, p. ej. `fixtures-variantes/dhcp-sin-voip`).
    pub capa: Option<PathBuf>,
    /// Dirección de escucha; `None` = `127.0.0.1` en un puerto libre.
    pub listen: Option<SocketAddr>,
}

impl SimConfig {
    /// Configuración que escucha en `127.0.0.1` en un puerto libre.
    pub fn new(user: &str, password: &str, fixtures_dir: impl Into<PathBuf>) -> Self {
        Self {
            user: user.to_owned(),
            password: password.to_owned(),
            fixtures_dir: fixtures_dir.into(),
            capa: None,
            listen: None,
        }
    }

    /// La misma configuración con una capa de fixtures: cada fixture se busca
    /// primero en `dir` y, si no está, en `fixtures_dir`.
    #[must_use]
    pub fn con_capa(mut self, dir: impl Into<PathBuf>) -> SimConfig {
        self.capa = Some(dir.into());
        self
    }
}

/// Simulador en marcha.
pub struct SimHandle {
    /// URL base, p. ej. `http://127.0.0.1:8081`.
    pub base_url: String,
    /// Control del estado del simulador.
    pub control: SimControl,
}

/// Punto de entrada del simulador.
pub struct Sim;

impl Sim {
    /// Carga las fixtures y arranca el servidor en segundo plano.
    ///
    /// # Panics
    /// Si las fixtures no se pueden leer o no se puede abrir el puerto.
    pub async fn start(cfg: SimConfig) -> SimHandle {
        let cargar = |dir: &FsPath| {
            load_fixtures(dir).unwrap_or_else(|e| panic!("fixtures en {}: {e}", dir.display()))
        };
        let mut fixtures = cargar(&cfg.fixtures_dir);
        if let Some(capa) = &cfg.capa {
            fixtures.extend(cargar(capa));
        }
        let wan = wan_de_las_fixtures(&fixtures);
        let inner = Arc::new(Inner {
            user: cfg.user,
            password: Mutex::new(cfg.password),
            fixtures,
            catalogo: Catalogo::load().expect("el catálogo de funciones carga"),
            wan,
            unresponsive: AtomicBool::new(false),
            state: Mutex::new(SimState {
                seed: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.subsec_nanos()),
                wan_up: true,
                ..SimState::default()
            }),
        });
        let addr = cfg
            .listen
            .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 0)));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .unwrap_or_else(|e| panic!("no se puede escuchar en {addr}: {e}"));
        let local = listener.local_addr().expect("dirección local");
        let app = Router::new()
            .route("/", get(root))
            .route("/cgi-bin/te_acceso_router.cgi", post(basic_login))
            .route("/cgi-bin/te_logout.cgi", get(logout))
            .route("/cgi-bin/{file}", post(cgi).get(basic_cgi))
            .route("/{page}", get(page))
            .layer(middleware::from_fn_with_state(
                inner.clone(),
                hang_if_unresponsive,
            ))
            .layer(middleware::from_fn(post_needs_content_length))
            .with_state(inner.clone());
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("router-sim: {e}");
            }
        });
        SimHandle {
            base_url: format!("http://{local}"),
            control: SimControl { inner },
        }
    }
}

/// Control del simulador desde las pruebas. Barato de clonar.
#[derive(Clone)]
pub struct SimControl {
    inner: Arc<Inner>,
}

impl SimControl {
    /// Si `true`, el router deja de responder (las peticiones quedan colgadas).
    pub fn set_unresponsive(&self, on: bool) {
        self.inner.unresponsive.store(on, Ordering::SeqCst);
    }

    /// Caduca todas las sesiones (básicas y avanzadas).
    pub fn invalidate_sessions(&self) {
        self.inner.state().invalidate_sessions();
    }

    /// Cambia el `opr_status` en `cbGetWanIPIntf` de la conexión a internet
    /// de las fixtures (la de la ruta por defecto; `ip2` en las de serie): 1
    /// arriba, 0 abajo.
    pub fn set_wan_up(&self, up: bool) {
        self.inner.state().wan_up = up;
    }

    /// La próxima escritura responde `Result=1` con este `ErrorString`.
    ///
    /// Una llamada con `validation_only=1` (la validación previa que hace la
    /// página WiFi) también lo consume, como cuando el router rechaza la
    /// validación.
    pub fn fail_next_write(&self, err: &str) {
        self.inner.state().fail_next = Some(err.to_owned());
    }

    /// La próxima escritura real responde OK pero no se aplica.
    ///
    /// Las llamadas con `validation_only=1` no lo consumen: nunca aplican
    /// nada, así que el efecto recae en la escritura real que las sigue.
    pub fn ignore_next_write(&self) {
        self.inner.state().ignore_next = true;
    }

    /// Si `on`, la lectura `name` (p. ej. `cbGetTr69Config`) responde
    /// `Result=1` hasta que se desactive.
    pub fn fail_reads(&self, name: &str, on: bool) {
        let mut st = self.inner.state();
        if on {
            st.failing_reads.insert(name.to_owned());
        } else {
            st.failing_reads.remove(name);
        }
    }

    /// Tras la próxima escritura real (no de solo validación), la lectura
    /// `name` empieza a fallar como con [`SimControl::fail_reads`] hasta que
    /// se desactive con `fail_reads(name, false)`.
    pub fn fail_reads_after_next_write(&self, name: &str) {
        self.inner.state().fail_reads_after_write = Some(name.to_owned());
    }

    /// La próxima escritura real no recibe respuesta (la petición se queda
    /// colgada; el cliente agota su tiempo de espera). Con `aplicar`, el
    /// router la aplica antes (p. ej. reinicia un servicio y no contesta);
    /// sin él, no aplica nada (se cortó antes de llegar). Las peticiones
    /// siguientes se responden con normalidad.
    pub fn no_answer_next_write(&self, aplicar: bool) {
        let mut st = self.inner.state();
        st.no_answer_next = true;
        if !aplicar {
            st.ignore_next = true;
        }
    }

    /// La próxima reserva DHCP nueva (`cbDhcpV4SvrPoolSetStatic` con id -1)
    /// se crea dos veces (dos ids nuevos con los mismos valores).
    pub fn create_twice_next_write(&self) {
        self.inner.state().create_twice = true;
    }

    /// Tras responder a la próxima escritura real, el router deja de
    /// responder (como [`SimControl::set_unresponsive`]) hasta que se llame a
    /// `set_unresponsive(false)`.
    pub fn hang_after_next_write(&self) {
        self.inner.state().hang_after_write = HangAfterWrite::Armed;
    }

    /// La escritura real (no de solo validación) número `n` contando desde
    /// ahora (1: la próxima) responde `Result=1` y no se aplica. Se puede
    /// armar varias veces para que fallen varias.
    pub fn fail_write_n(&self, n: usize) {
        let mut st = self.inner.state();
        let objetivo = st.real_writes + n;
        st.fail_write_at.insert(objetivo);
    }

    /// Como [`SimControl::hang_after_next_write`], pero con la escritura real
    /// número `n` contando desde ahora (1: la próxima).
    pub fn hang_after_write_n(&self, n: usize) {
        let mut st = self.inner.state();
        st.hang_at_write = Some(st.real_writes + n);
    }

    /// Tráfico de internet de un equipo en el puerto LAN 4: `rx` bytes de
    /// bajada y `tx` de subida. En `cbGetStatistic` suben la conexión a
    /// internet (rx, tx; `ip2` en las fixtures de serie) y, del lado LAN,
    /// `eth0.4` al revés (lo que baja sale por el puerto: tx). El router real
    /// apenas los cuenta en la WAN (reenvío por hardware); el panel mide en
    /// los puertos LAN.
    pub fn counters_bump(&self, rx: u64, tx: u64) {
        if let Some(wan) = &self.inner.wan {
            self.counters_bump_iface(wan, rx, tx);
        }
        self.counters_bump_iface(LAN_PORT, tx, rx);
    }

    /// Suma bytes a `rx_byte`/`tx_byte` de la interfaz `name` de
    /// `cbGetStatistic`.
    pub fn counters_bump_iface(&self, name: &str, rx: u64, tx: u64) {
        let mut st = self.inner.state();
        let e = st.counters_extra.entry(name.to_owned()).or_default();
        e.0 = e.0.saturating_add(rx);
        e.1 = e.1.saturating_add(tx);
    }

    /// Cambia la contraseña que acepta el login (las sesiones abiertas
    /// siguen valiendo).
    pub fn set_password(&self, password: &str) {
        *self
            .inner
            .password
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = password.to_owned();
    }

    /// El próximo `cbModifyPwd` responde OK sin cambiar nada (el router lo
    /// acepta pero la contraseña sigue siendo la anterior).
    pub fn modify_pwd_sin_efecto(&self) {
        self.inner.state().modify_pwd_sin_efecto = true;
    }

    /// El inicio de sesión (básico y avanzado) con `password` falla aunque
    /// sea la vigente: simula un formato que el router no acepta al entrar.
    pub fn login_falla_con(&self, password: &str) {
        self.inner.state().login_falla_con = Some(password.to_owned());
    }

    /// Valor actual de `campo` en la lectura `lectura` (sin parámetros), con
    /// lo escrito hasta ahora: el primer nodo que lo tiene. Para las pruebas
    /// que buscan secretos en el HTML.
    pub fn fixture_valor(&self, lectura: &str, campo: &str) -> Option<String> {
        let st = self.inner.state();
        let nodo = read_node(&self.inner, &st, lectura, &[]).ok()?;
        valor_de(&nodo, campo)
    }

    /// Cambia la versión de firmware (`SWVersion`) que da `cbGetSummary`.
    pub fn set_firmware(&self, firmware: &str) {
        self.inner.state().firmware = Some(firmware.to_owned());
    }

    /// Escrituras recibidas con `sessionKey` válido (sin él), en orden. No
    /// incluye las llamadas de solo validación (`validation_only=1`). Las de
    /// la UI básica (`GET /cgi-bin/<x>.cmd|.cgi|.wl?…`) aparecen con el
    /// nombre del fichero y los parámetros de la URL (incluido `action`).
    pub fn writes(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.inner.state().writes.clone()
    }
}

type Params = Vec<(String, String)>;
type FixtureKey = (String, Params);

struct Inner {
    user: String,
    /// Contraseña aceptada (cambiable con [`SimControl::set_password`]).
    password: Mutex<String>,
    fixtures: HashMap<FixtureKey, String>,
    /// Catálogo de funciones (lecturas pareadas y listas).
    catalogo: Catalogo,
    /// Conexión a internet de las fixtures (ver [`wan_de_las_fixtures`]).
    wan: Option<String>,
    unresponsive: AtomicBool,
    state: Mutex<SimState>,
}

impl Inner {
    fn password(&self) -> String {
        self.password
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn state(&self) -> MutexGuard<'_, SimState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Lo escrito en una instancia de una lectura que no es una lista: campos
/// que sustituyen a los de la respuesta de la fixture.
struct Edicion {
    /// Función de lectura pareada.
    lectura: String,
    /// `(hijo, valor)` que identifican la instancia (vacío: la única).
    ident: Params,
    campos: BTreeMap<String, String>,
}

/// Lista materializada de una lectura (la primera escritura la crea a partir
/// de las fixtures).
struct ListaViva {
    /// Primera instancia de las fixtures: molde de las nuevas.
    plantilla: XmlNode,
    nodos: Vec<XmlNode>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum HangAfterWrite {
    #[default]
    Off,
    Armed,
    /// La escritura llegó: colgarse en cuanto se responda.
    Now,
}

#[derive(Default)]
struct SimState {
    seed: u32,
    counter: u32,
    /// `_httpdSessionId_` con login básico correcto.
    basic: HashSet<String>,
    /// `SessionID` del login avanzado.
    advanced: HashSet<String>,
    session_key: Option<String>,
    wan_up: bool,
    fail_next: Option<String>,
    ignore_next: bool,
    /// Lecturas que fallan (`fail_reads`).
    failing_reads: HashSet<String>,
    /// Lectura que empieza a fallar tras la próxima escritura.
    fail_reads_after_write: Option<String>,
    /// `hang_after_next_write`.
    hang_after_write: HangAfterWrite,
    /// `no_answer_next_write`.
    no_answer_next: bool,
    /// `create_twice_next_write`.
    create_twice: bool,
    /// Escrituras reales recibidas (para `fail_write_n`/`hang_after_write_n`).
    real_writes: usize,
    /// Números de escritura real que fallan (`fail_write_n`).
    fail_write_at: HashSet<usize>,
    /// Número de escritura real tras la que se cuelga (`hang_after_write_n`).
    hang_at_write: Option<usize>,
    /// Bytes sumados a cada interfaz de `cbGetStatistic`: nombre → (rx, tx).
    counters_extra: HashMap<String, (u64, u64)>,
    writes: Vec<(String, Params)>,
    /// Escrituras por instancia de lecturas que no son listas.
    ediciones: Vec<Edicion>,
    /// Listas materializadas por función de lectura.
    listas: HashMap<String, ListaViva>,
    /// `modify_pwd_sin_efecto`.
    modify_pwd_sin_efecto: bool,
    /// `login_falla_con`.
    login_falla_con: Option<String>,
    /// Último `cbReboot`: `SysUptime` cuenta desde aquí.
    rebooted: Option<Instant>,
    /// `SWVersion` de `cbGetSummary` (`set_firmware`).
    firmware: Option<String>,
}

impl SimState {
    /// Identificador numérico nuevo (cookies, `SessionID`, `sessionKey`).
    fn next_token(&mut self) -> String {
        self.counter += 1;
        let n = u64::from(self.seed) * 1000 + u64::from(self.counter);
        format!("{}", n % 1_000_000_000_000)
    }

    fn invalidate_sessions(&mut self) {
        self.basic.clear();
        self.advanced.clear();
        self.session_key = None;
    }
}

// ---------------------------------------------------------------- fixtures

/// Clave de búsqueda: nombre y parámetros ordenados, sin `sessionKey`.
fn fixture_key(name: &str, params: &[(String, String)]) -> FixtureKey {
    let mut p: Params = params
        .iter()
        .filter(|(k, _)| k != "sessionKey")
        .cloned()
        .collect();
    p.sort();
    (name.to_owned(), p)
}

fn load_fixtures(dir: &FsPath) -> std::io::Result<HashMap<FixtureKey, String>> {
    let mut map = HashMap::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(stem) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".xml"))
        else {
            continue;
        };
        let (name, params) = sanitize::split_fixture_name(stem);
        map.insert(fixture_key(name, &params), std::fs::read_to_string(&path)?);
    }
    Ok(map)
}

/// Conexión a internet de las fixtures: la de la ruta por defecto
/// (`cbGetRoutingV4Rule` con `all=ALL` y `desc=Default`) o, sin ella, la
/// única de datos con IPv4 de internet (la misma regla que el panel).
fn wan_de_las_fixtures(fixtures: &HashMap<FixtureKey, String>) -> Option<String> {
    let leer = |name: &str, params: &[(&str, &str)]| {
        let params: Params = params
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        let raw = fixtures.get(&fixture_key(name, &params))?;
        hgu::xml::parse_response(name, raw).ok()
    };
    let conns = hgu::models::WanConn::parse_all(&leer("cbGetWanIPIntf", &[])?);
    let ruta = leer("cbGetRoutingV4Rule", &[("all", "ALL"), ("desc", "Default")])
        .and_then(|n| hgu::read::regla_por_defecto(&n));
    hgu::deteccion::elegir_wan(&conns, ruta.as_deref()).map(|w| w.name.clone())
}

// ---------------------------------------------------------------- XML

fn response(node: &XmlNode) -> String {
    format!("{XML_HEADER}<Response>{}</Response>", node.to_xml())
}

fn leaf(tag: &str, text: &str) -> XmlNode {
    XmlNode {
        tag: tag.to_owned(),
        text: text.to_owned(),
        children: Vec::new(),
    }
}

fn result_node(name: &str, mut children: Vec<XmlNode>, result: &str, error: &str) -> XmlNode {
    children.push(leaf("Result", result));
    if !error.is_empty() {
        children.push(leaf("ErrorString", error));
    }
    XmlNode {
        tag: name.to_owned(),
        text: String::new(),
        children,
    }
}

fn failure(name: &str, error: &str) -> String {
    response(&result_node(name, Vec::new(), "1", error))
}

fn ok(name: &str) -> String {
    response(&result_node(name, Vec::new(), "0", ""))
}

/// Aplica `f` a `n` y a todos sus descendientes.
fn for_each_node(n: &mut XmlNode, f: &mut impl FnMut(&mut XmlNode)) {
    f(n);
    for c in &mut n.children {
        for_each_node(c, f);
    }
}

/// Cambia el texto del hijo `tag`, si existe.
fn set_child(n: &mut XmlNode, tag: &str, text: &str) {
    if let Some(c) = n.children.iter_mut().find(|c| c.tag == tag) {
        c.text = text.to_owned();
    }
}

// ---------------------------------------------------------------- HTTP

fn cookies(headers: &HeaderMap) -> HashMap<String, String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect()
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = b
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (b[i], hex) {
            (b'%', Some(v)) => {
                out.push(v);
                i += 3;
            }
            (b'+', _) => {
                out.push(b' ');
                i += 1;
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn form(body: &[u8]) -> Params {
    String::from_utf8_lossy(body)
        .split('&')
        .filter(|kv| !kv.is_empty())
        .map(|kv| {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            (pct_decode(k), pct_decode(v))
        })
        .collect()
}

fn param<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn xml(body: String) -> Response {
    ([(header::CONTENT_TYPE, "text/xml")], body).into_response()
}

fn has_basic(st: &SimState, c: &HashMap<String, String>) -> bool {
    c.get("_httpdSessionId_")
        .is_some_and(|id| st.basic.contains(id))
}

async fn hang_if_unresponsive(State(s): State<Arc<Inner>>, req: Request, next: Next) -> Response {
    while s.unresponsive.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    next.run(req).await
}

/// Como el micro_httpd del router real: un POST sin `Content-Length` (lo que
/// hace hyper con un cuerpo vacío) recibe 400 Bad Request.
async fn post_needs_content_length(req: Request, next: Next) -> Response {
    if req.method() == axum::http::Method::POST
        && !req.headers().contains_key(header::CONTENT_LENGTH)
    {
        return (
            StatusCode::BAD_REQUEST,
            "<html><body>400 Bad Request</body></html>",
        )
            .into_response();
    }
    next.run(req).await
}

async fn root(State(s): State<Arc<Inner>>) -> Response {
    let id = s.state().next_token();
    (
        [(header::SET_COOKIE, format!("_httpdSessionId_={id}; path=/"))],
        "<html><body>router-sim</body></html>",
    )
        .into_response()
}

async fn basic_login(State(s): State<Arc<Inner>>, headers: HeaderMap, body: Bytes) -> Response {
    let f = form(&body);
    let rechazada = s.state().login_falla_con.clone();
    let good = param(&f, "loginUsername") == Some(mess_userpass(&s.user).as_str())
        && param(&f, "loginPassword") == Some(mess_userpass(&s.password()).as_str())
        && rechazada.is_none_or(|r| param(&f, "loginPassword") != Some(mess_userpass(&r).as_str()));
    if !good {
        return "<script>location.href = \"/te_acceso_router.asp\"</script>".into_response();
    }
    let mut st = s.state();
    let (id, new) = match cookies(&headers).remove("_httpdSessionId_") {
        Some(id) => (id, false),
        None => (st.next_token(), true),
    };
    st.basic.insert(id.clone());
    let body = "<script>location.href = \"/te_wifi.asp\"</script>";
    if new {
        (
            [(header::SET_COOKIE, format!("_httpdSessionId_={id}; path=/"))],
            body,
        )
            .into_response()
    } else {
        body.into_response()
    }
}

async fn logout(State(s): State<Arc<Inner>>, headers: HeaderMap) -> Response {
    let c = cookies(&headers);
    let mut st = s.state();
    if let Some(id) = c.get("_httpdSessionId_") {
        st.basic.remove(id);
    }
    if let Some(sid) = c.get("SessionID") {
        st.advanced.remove(sid);
    }
    LOGGED_OUT.into_response()
}

async fn page(
    State(s): State<Arc<Inner>>,
    Path(page): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !page.ends_with(".asp") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut st = s.state();
    if !has_basic(&st, &cookies(&headers)) {
        return LOGGED_OUT.into_response();
    }
    let key = st.next_token();
    st.session_key = Some(key.clone());
    let vars: String = PAGE_VARS
        .iter()
        .filter(|(p, _)| *p == page)
        .flat_map(|(_, vars)| vars.iter())
        .map(|(k, v)| format!("var {k}='{v}';\r\n"))
        .collect();
    format!(
        "<html><head><script>\r\nvar sessionKey='{key}';\r\n{vars}</script></head><body>{page}</body></html>"
    )
    .into_response()
}

/// Variables `var x='…';` sintéticas de algunas páginas básicas, con nombres
/// de parámetros del catálogo (incluidos secretos inventados para probar el
/// enmascarado).
pub const PAGE_VARS: &[(&str, &[(&str, &str)])] = &[(
    "te_wifi.asp",
    &[
        ("wlSsid", "RED-SIM"),
        ("wlHide", "0"),
        ("wlChannel", "6"),
        ("unitNum", "0"),
        ("wlMACaddr", "02:00:00:00:00:0d"),
        ("wlKey1", "wep-sim-secreta"),
        ("wlWpaPsk", "psk-sim-secreta"),
    ],
)];

/// Escritura de la UI básica: `GET /cgi-bin/<x>.cmd|.cgi|.wl?…&sessionKey=…`.
/// Se registra en `writes` y responde OK; no cambia el estado simulado.
async fn basic_cgi(
    State(s): State<Arc<Inner>>,
    Path(file): Path<String>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> Response {
    if ![".cmd", ".cgi", ".wl"].iter().any(|e| file.ends_with(e)) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut st = s.state();
    if !has_basic(&st, &cookies(&headers)) {
        return LOGGED_OUT.into_response();
    }
    let params = form(query.unwrap_or_default().as_bytes());
    let key_ok = st
        .session_key
        .as_deref()
        .is_some_and(|k| param(&params, "sessionKey") == Some(k));
    if !key_ok {
        return "<html><body>sessionKey no válido</body></html>".into_response();
    }
    let clean: Params = params
        .into_iter()
        .filter(|(k, _)| k != "sessionKey")
        .collect();
    st.writes.push((file, clean));
    "<html><body>ok</body></html>".into_response()
}

async fn cgi(
    State(s): State<Arc<Inner>>,
    Path(file): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(name) = file.strip_suffix(".xml") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let (respuesta, sin_respuesta) = cgi_sync(&s, name, &headers, &body);
    if sin_respuesta {
        std::future::pending::<()>().await;
    }
    respuesta
}

/// Atiende `/cgi-bin/<name>.xml`; el segundo valor dice si hay que dejar la
/// petición sin respuesta (`no_answer_next_write`).
fn cgi_sync(s: &Inner, name: &str, headers: &HeaderMap, body: &Bytes) -> (Response, bool) {
    let c = cookies(headers);
    let params = form(body);
    let mut st = s.state();
    if name == "cbCheckLogin" {
        return (xml(advanced_login(s, &mut st, &c, &params)), false);
    }
    let valid = c.get("SessionStatus").is_some_and(|v| v == "Valid")
        && c.get("SessionID")
            .is_some_and(|sid| st.advanced.contains(sid));
    if !valid {
        return (LOGGED_OUT.into_response(), false);
    }
    if is_read(name) && st.failing_reads.contains(name) {
        (xml(failure(name, "lectura fallida (simulado)")), false)
    } else if is_read(name) {
        (xml(read(s, &st, name, &params)), false)
    } else {
        let real = param(&params, "validation_only") != Some("1");
        let body = write(s, &mut st, name, &params);
        if st.hang_after_write == HangAfterWrite::Now {
            st.hang_after_write = HangAfterWrite::Off;
            s.unresponsive.store(true, Ordering::SeqCst);
        }
        let sin_respuesta = real && std::mem::take(&mut st.no_answer_next);
        (xml(body), sin_respuesta)
    }
}

fn advanced_login(
    s: &Inner,
    st: &mut SimState,
    c: &HashMap<String, String>,
    params: &[(String, String)],
) -> String {
    if !has_basic(st, c) {
        return LOGGED_OUT.into();
    }
    let good = param(params, "rqUsername") == Some(mess_userpass(&s.user).as_str())
        && param(params, "rqPasswd") == Some(mess_userpass(&s.password()).as_str())
        && st
            .login_falla_con
            .as_deref()
            .is_none_or(|r| param(params, "rqPasswd") != Some(mess_userpass(r).as_str()));
    if !good {
        return failure("cbCheckLogin", "login");
    }
    let sid = st.next_token();
    st.advanced.insert(sid.clone());
    response(&result_node(
        "cbCheckLogin",
        vec![leaf("SessionID", &sid)],
        "0",
        "",
    ))
}

fn is_read(name: &str) -> bool {
    name.contains("Get") || name.starts_with("cbList") || name.starts_with("cbCheck")
}

// ---------------------------------------------------------------- lecturas

fn read(s: &Inner, st: &SimState, name: &str, params: &[(String, String)]) -> String {
    match read_node(s, st, name, params) {
        Ok(node) => response(&node),
        Err(e) => failure(name, &e),
    }
}

/// La respuesta de la lectura `name` como nodo, con lo escrito aplicado.
fn read_node(
    s: &Inner,
    st: &SimState,
    name: &str,
    params: &[(String, String)],
) -> Result<XmlNode, String> {
    if let Some(viva) = st.listas.get(name)
        && let Some(r) = read_lista(viva, name, params)
    {
        return r;
    }
    let raw = s
        .fixtures
        .get(&fixture_key(name, params))
        .or_else(|| s.fixtures.get(&(name.to_owned(), Vec::new())));
    // Una lista que se puede escribir y todavía no se ha tocado atiende
    // también las lecturas de una instancia que solo existe en `all=ALL`.
    if raw.is_none()
        && es_lectura_de_lista(s, name)
        && let Some(r) = construir_lista(s, name)
            .ok()
            .and_then(|viva| read_lista(&viva, name, params))
    {
        return r;
    }
    let mut node = match raw {
        Some(raw) => {
            hgu::xml::parse_response(name, raw).map_err(|e| format!("fixture no válida: {e}"))?
        }
        None if params.is_empty() => {
            all_instances(s, name).ok_or_else(|| "no fixture".to_owned())?
        }
        None => return Err("no fixture".to_owned()),
    };
    aplicar_ediciones(st, name, &mut node);
    patch(s, st, name, &mut node);
    Ok(node)
}

/// Valor de `campo` en el primer nodo (en preorden) que lo tiene.
fn valor_de(n: &XmlNode, campo: &str) -> Option<String> {
    n.text_of(campo)
        .map(str::to_owned)
        .or_else(|| n.children.iter().find_map(|c| valor_de(c, campo)))
}

/// Lecturas que comparten instancias con la lectura pareada de una
/// escritura: la lista por radio de puntos de acceso WiFi muestra los mismos
/// puntos que `cbGetWifiAccPt`.
fn lecturas_hermanas(name: &str) -> &'static [&'static str] {
    match name {
        "cbListWifiAccPt" => &["cbGetWifiAccPt"],
        _ => &[],
    }
}

/// Aplica sobre la respuesta lo escrito en sus instancias: solo cambia hijos
/// que ya existen.
fn aplicar_ediciones(st: &SimState, name: &str, node: &mut XmlNode) {
    let hermanas = lecturas_hermanas(name);
    for e in &st.ediciones {
        if e.lectura == name || hermanas.contains(&e.lectura.as_str()) {
            for_each_node(node, &mut |n| aplicar_campos(n, &e.ident, &e.campos));
        }
    }
}

/// Escribe `campos` en `n` si es la instancia que `ident` identifica.
fn aplicar_campos(n: &mut XmlNode, ident: &[(String, String)], campos: &BTreeMap<String, String>) {
    if ident.iter().all(|(k, v)| n.text_of(k) == Some(v.as_str())) {
        for (k, v) in campos {
            set_child(n, k, v);
        }
    }
}

/// `true` si alguna escritura del catálogo con lista tiene `name` por lectura
/// pareada.
fn es_lectura_de_lista(s: &Inner, name: &str) -> bool {
    s.catalogo.todas().iter().any(|f| {
        f.lista.is_some()
            && f.lectura_pareada
                .as_ref()
                .is_some_and(|l| l.funcion == name)
    })
}

/// Lectura servida desde la lista materializada de `name`, si la hay y la
/// lectura pide todas (`all=ALL`, o sin parámetros) o filtra por un campo de
/// la instancia (id, índice…); `None` si no aplica.
fn read_lista(
    lista: &ListaViva,
    name: &str,
    params: &[(String, String)],
) -> Option<Result<XmlNode, String>> {
    let filtros: Vec<&(String, String)> = params
        .iter()
        .filter(|(k, _)| k != "all" && k != "sessionKey")
        .filter(|(k, _)| lista.plantilla.get(k).is_some())
        .collect();
    let todas = param(params, "all") == Some("ALL");
    let sin_parametros = params.iter().all(|(k, _)| k == "sessionKey");
    if !(todas || sin_parametros || !filtros.is_empty()) {
        return None;
    }
    let nodos: Vec<XmlNode> = lista
        .nodos
        .iter()
        .filter(|n| {
            filtros
                .iter()
                .all(|(k, v)| n.text_of(k) == Some(v.as_str()))
        })
        .cloned()
        .collect();
    if !filtros.is_empty() && nodos.is_empty() {
        return Some(Err("no existe".to_owned()));
    }
    Some(Ok(result_node(name, nodos, "0", "")))
}

/// Lista leída sin elegir instancia y sin `<name>.xml`: el router devuelve
/// todas las instancias (así responden las listas de las que hay fixture sin
/// parámetros, como `cbGetWanIPIntf`). Junta las de las fixtures de `name`
/// que eligen la instancia con el mismo parámetro (el que más fixtures
/// tenga; a igualdad, el primero por nombre), en orden de valor.
fn all_instances(s: &Inner, name: &str) -> Option<XmlNode> {
    let mut groups: BTreeMap<Vec<&str>, Vec<(&Params, &String)>> = BTreeMap::new();
    for ((n, p), raw) in &s.fixtures {
        if n == name && !p.is_empty() {
            let names = p.iter().map(|(k, _)| k.as_str()).collect();
            groups.entry(names).or_default().push((p, raw));
        }
    }
    let (_, mut group) = groups.into_iter().rev().max_by_key(|(_, g)| g.len())?;
    group.sort();
    let mut children = Vec::new();
    for (_, raw) in group {
        let n = hgu::xml::parse_response(name, raw).ok()?;
        children.extend(
            n.children
                .into_iter()
                .filter(|c| c.tag != "Result" && c.tag != "ErrorString"),
        );
    }
    Some(result_node(name, children, "0", ""))
}

/// Refleja el estado en memoria sobre la respuesta de la fixture.
fn patch(s: &Inner, st: &SimState, name: &str, node: &mut XmlNode) {
    match name {
        "cbGetSummary" => {
            if let Some(t) = st.rebooted {
                set_child(node, "SysUptime", &t.elapsed().as_secs().to_string());
            }
            if let Some(fw) = &st.firmware {
                set_child(node, "SWVersion", fw);
            }
        }
        "cbGetWanIPIntf" if !st.wan_up => for_each_node(node, &mut |n| {
            if s.wan
                .as_deref()
                .is_some_and(|w| n.text_of("name") == Some(w))
            {
                set_child(n, "opr_status", "0");
            }
        }),
        "cbGetStatistic" => for_each_node(node, &mut |n| {
            if let Some(&(rx, tx)) = n.text_of("name").and_then(|m| st.counters_extra.get(m)) {
                for (tag, extra) in [("rx_byte", rx), ("tx_byte", tx)] {
                    if let Some(v) = n.text_of(tag).and_then(|v| v.parse::<u64>().ok()) {
                        set_child(n, tag, &v.saturating_add(extra).to_string());
                    }
                }
            }
        }),
        _ => {}
    }
}

// ---------------------------------------------------------------- escrituras

fn write(s: &Inner, st: &mut SimState, name: &str, params: &[(String, String)]) -> String {
    let key_ok = st
        .session_key
        .as_deref()
        .is_some_and(|k| param(params, "sessionKey") == Some(k));
    if !key_ok {
        return failure(name, "sessionKey no válido");
    }
    let clean: Params = params
        .iter()
        .filter(|(k, _)| k != "sessionKey")
        .cloned()
        .collect();
    // Validación previa (página WiFi): nunca aplica nada ni se registra;
    // puede consumir `fail_next`, pero no `ignore_next`.
    let validation_only = param(&clean, "validation_only") == Some("1");
    let mut fail_n = false;
    if !validation_only {
        st.real_writes += 1;
        let n = st.real_writes;
        fail_n = st.fail_write_at.remove(&n);
        if st.hang_at_write == Some(n) {
            st.hang_at_write = None;
            st.hang_after_write = HangAfterWrite::Armed;
        }
        st.writes.push((name.to_owned(), clean.clone()));
        if let Some(r) = st.fail_reads_after_write.take() {
            st.failing_reads.insert(r);
        }
        if st.hang_after_write == HangAfterWrite::Armed {
            st.hang_after_write = HangAfterWrite::Now;
        }
    }
    if let Some(err) = st.fail_next.take() {
        return failure(name, &err);
    }
    if fail_n {
        return failure(name, "escritura rechazada (simulado)");
    }
    if validation_only {
        return ok(name);
    }
    if std::mem::take(&mut st.ignore_next) {
        return ok(name);
    }
    let result = match name {
        "cbModifyPwd" => modify_pwd(s, st, &clean),
        "cbReboot" => {
            st.invalidate_sessions();
            st.rebooted = Some(Instant::now());
            return response(&result_node(
                name,
                vec![leaf("Msg", "Rebooting...")],
                "0",
                "",
            ));
        }
        _ => escritura_generica(s, st, name, &clean),
    };
    match result {
        Ok(()) => ok(name),
        Err(e) => failure(name, &e),
    }
}

/// Cambia la contraseña del usuario del sim (`rqUser`, `rqOPwd`, `rqNPwd`).
/// Las sesiones abiertas siguen valiendo, como en el router.
fn modify_pwd(s: &Inner, st: &mut SimState, p: &[(String, String)]) -> Result<(), String> {
    let nueva = param(p, "rqNPwd").ok_or("falta rqNPwd")?;
    let actual = s.password();
    if param(p, "rqUser") != Some(s.user.as_str()) || param(p, "rqOPwd") != Some(actual.as_str()) {
        return Err("usuario o contraseña actual incorrectos".to_owned());
    }
    if !std::mem::take(&mut st.modify_pwd_sin_efecto) {
        *s.password.lock().unwrap_or_else(|e| e.into_inner()) = nueva.to_owned();
    }
    Ok(())
}

/// Refleja una escritura avanzada en su lectura pareada del catálogo. Una
/// función sin lectura pareada no cambia nada.
fn escritura_generica(
    s: &Inner,
    st: &mut SimState,
    name: &str,
    p: &[(String, String)],
) -> Result<(), String> {
    let Some(f) = s.catalogo.funcion(name) else {
        return Ok(());
    };
    let Some(pareada) = &f.lectura_pareada else {
        return Ok(());
    };
    let lectura = pareada.funcion.as_str();
    if let Some(lista) = &f.lista {
        return escribir_lista(s, st, lectura, lista, p);
    }
    // La instancia: los `{x}` de la lectura pareada, con el valor escrito.
    let mut ident = Params::new();
    for (_, v) in &pareada.params {
        if let Some(x) = v.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
            let valor = param(p, x).ok_or_else(|| format!("falta {x}"))?;
            ident.push((x.to_owned(), valor.to_owned()));
        }
    }
    let campos = campos_escritos(p, &[]);
    match st
        .ediciones
        .iter_mut()
        .find(|e| e.lectura == lectura && e.ident == ident)
    {
        Some(e) => e.campos.extend(campos),
        None => st.ediciones.push(Edicion {
            lectura: lectura.to_owned(),
            ident,
            campos,
        }),
    }
    Ok(())
}

/// Parámetros que llevan valor a la instancia (todos menos los de control y
/// los que ella excluye).
fn campos_escritos(p: &[(String, String)], excluir: &[&str]) -> BTreeMap<String, String> {
    p.iter()
        .filter(|(k, _)| k != "validation_only" && !excluir.contains(&k.as_str()))
        .cloned()
        .collect()
}

/// Crea, edita o borra una instancia de la lista de la lectura `lectura`.
fn escribir_lista(
    s: &Inner,
    st: &mut SimState,
    lectura: &str,
    lista: &Lista,
    p: &[(String, String)],
) -> Result<(), String> {
    materializar_lista(s, st, lectura)?;
    let create_twice = st.create_twice;
    let viva = st.listas.get_mut(lectura).ok_or("lista sin materializar")?;
    let id_param = lista.id_param.as_str();
    let pos = param(p, id_param).and_then(|id| {
        viva.nodos
            .iter()
            .position(|n| n.text_of(id_param) == Some(id))
    });
    if param(p, "del") == Some("DEL") {
        viva.nodos.remove(pos.ok_or("no existe")?);
    } else {
        let campos = campos_escritos(p, &[id_param, "del"]);
        match pos {
            Some(i) => aplicar_campos(&mut viva.nodos[i], &[], &campos),
            None => {
                let id = viva
                    .nodos
                    .iter()
                    .filter_map(|n| n.text_of(id_param)?.parse::<u64>().ok())
                    .max()
                    .map_or(0, |m| m + 1);
                let mut nuevo = viva.plantilla.clone();
                aplicar_campos(&mut nuevo, &[], &campos);
                set_child(&mut nuevo, id_param, &id.to_string());
                if create_twice {
                    let mut otro = nuevo.clone();
                    set_child(&mut otro, id_param, &(id + 1).to_string());
                    viva.nodos.push(nuevo);
                    viva.nodos.push(otro);
                } else {
                    viva.nodos.push(nuevo);
                }
            }
        }
    }
    // El índice de cada instancia (`*_0i`) es su posición.
    if let Some(indice) = id_param.strip_suffix("_id").map(|b| format!("{b}_0i")) {
        for (i, n) in viva.nodos.iter_mut().enumerate() {
            set_child(n, &indice, &i.to_string());
        }
    }
    if pos.is_none() && param(p, "del") != Some("DEL") {
        st.create_twice = false;
    }
    Ok(())
}

/// Crea la lista viva de `lectura` (si no existe) a partir de la fixture `all=ALL` (o, sin
/// ella, la de sin parámetros o la suma de las fixtures por instancia).
fn materializar_lista(s: &Inner, st: &mut SimState, lectura: &str) -> Result<(), String> {
    if !st.listas.contains_key(lectura) {
        let viva = construir_lista(s, lectura)?;
        st.listas.insert(lectura.to_owned(), viva);
    }
    Ok(())
}

/// La lista de `lectura` tal como la dan las fixtures.
fn construir_lista(s: &Inner, lectura: &str) -> Result<ListaViva, String> {
    let todas = s
        .fixtures
        .iter()
        .filter(|((n, p), _)| n == lectura && param(p, "all") == Some("ALL"))
        .min_by_key(|((_, p), _)| p.clone())
        .map(|(_, raw)| raw)
        .or_else(|| s.fixtures.get(&(lectura.to_owned(), Vec::new())));
    let node = match todas {
        Some(raw) => hgu::xml::parse_response(lectura, raw).map_err(|e| e.to_string())?,
        None => all_instances(s, lectura).ok_or("la lista no tiene fixture")?,
    };
    let nodos: Vec<XmlNode> = node
        .children
        .into_iter()
        .filter(|c| c.tag != "Result" && c.tag != "ErrorString")
        .collect();
    let plantilla = nodos
        .first()
        .cloned()
        .ok_or("la lista no tiene ninguna instancia de molde")?;
    Ok(ListaViva { plantilla, nodos })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_decoding() {
        assert_eq!(
            form(b"a=b%20c&mac=02%3A00&e=&x+y=1&p=%zz"),
            vec![
                ("a".to_owned(), "b c".to_owned()),
                ("mac".to_owned(), "02:00".to_owned()),
                ("e".to_owned(), String::new()),
                ("x y".to_owned(), "1".to_owned()),
                ("p".to_owned(), "%zz".to_owned())
            ]
        );
    }
}
