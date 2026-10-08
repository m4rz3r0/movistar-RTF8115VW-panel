//! Router Askey RTF8115VW simulado.
//!
//! Sirve las fixtures saneadas con el mismo protocolo que el router real
//! (login básico y avanzado, cookies, `sessionKey` en las páginas `.asp`) y
//! mantiene un estado mínimo en memoria para las escrituras más habituales.

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
/// Interfaz WAN principal (Internet).
const WAN: &str = "ip2";

/// Configuración del simulador.
#[derive(Clone, Debug)]
pub struct SimConfig {
    /// Usuario aceptado en el login.
    pub user: String,
    /// Contraseña aceptada en el login.
    pub password: String,
    /// Directorio con las fixtures `<name>[__<params>].xml`.
    pub fixtures_dir: PathBuf,
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
            listen: None,
        }
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
        let fixtures = load_fixtures(&cfg.fixtures_dir)
            .unwrap_or_else(|e| panic!("fixtures en {}: {e}", cfg.fixtures_dir.display()));
        let leases = initial_leases(&fixtures);
        let inner = Arc::new(Inner {
            user: cfg.user,
            password: cfg.password,
            fixtures,
            unresponsive: AtomicBool::new(false),
            state: Mutex::new(SimState {
                seed: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.subsec_nanos()),
                wan_up: true,
                leases,
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

    /// Cambia el `opr_status` de `ip2` en `cbGetWanIPIntf` (1 arriba, 0 abajo).
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

    /// Suma bytes a `rx_byte`/`tx_byte` de `ip2` en `cbGetStatistic`.
    pub fn counters_bump(&self, rx: u64, tx: u64) {
        let mut st = self.inner.state();
        st.rx_extra = st.rx_extra.saturating_add(rx);
        st.tx_extra = st.tx_extra.saturating_add(tx);
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
    password: String,
    fixtures: HashMap<FixtureKey, String>,
    unresponsive: AtomicBool,
    state: Mutex<SimState>,
}

impl Inner {
    fn state(&self) -> MutexGuard<'_, SimState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Lease {
    id: u32,
    adm_state: String,
    mac_addr: String,
    ipv4_addr: String,
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
    rx_extra: u64,
    tx_extra: u64,
    writes: Vec<(String, Params)>,
    leases: Vec<Lease>,
    upnp_adm_state: Option<String>,
    /// Campos escritos por punto de acceso (`dpm_cfg_wifi_acc_pt_id`).
    wifi: BTreeMap<String, BTreeMap<String, String>>,
    /// Último `cbReboot`: `SysUptime` cuenta desde aquí.
    rebooted: Option<Instant>,
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

fn initial_leases(fixtures: &HashMap<FixtureKey, String>) -> Vec<Lease> {
    let key = fixture_key(
        "cbDhcpV4SvrPoolGetStatic",
        &[
            ("rtm_cfg_dhcpv4_svr_pool_id".into(), "0".into()),
            ("all".into(), "ALL".into()),
        ],
    );
    let Some(node) = fixtures
        .get(&key)
        .and_then(|raw| hgu::xml::parse_response("cbDhcpV4SvrPoolGetStatic", raw).ok())
    else {
        return Vec::new();
    };
    node.all("rtm_dhcpv4_svr_pool_static")
        .into_iter()
        .filter_map(|e| {
            Some(Lease {
                id: e.text_of("rtm_dhcpv4_svr_pool_static_id")?.parse().ok()?,
                adm_state: e.text_of("adm_state")?.to_owned(),
                mac_addr: e.text_of("mac_addr")?.to_owned(),
                ipv4_addr: e.text_of("ipv4_addr")?.to_owned(),
            })
        })
        .collect()
}

// ---------------------------------------------------------------- XML

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn write_node(n: &XmlNode, out: &mut String) {
    out.push_str(&format!("<{}>", n.tag));
    if n.children.is_empty() {
        out.push_str(&escape(&n.text));
    }
    for c in &n.children {
        write_node(c, out);
    }
    out.push_str(&format!("</{}>", n.tag));
}

fn response(node: &XmlNode) -> String {
    let mut out = format!("{XML_HEADER}<Response>");
    write_node(node, &mut out);
    out.push_str("</Response>");
    out
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
    let good = param(&f, "loginUsername") == Some(mess_userpass(&s.user).as_str())
        && param(&f, "loginPassword") == Some(mess_userpass(&s.password).as_str());
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
    format!(
        "<html><head><script>\r\nvar sessionKey='{key}';\r\n</script></head><body>{page}</body></html>"
    )
    .into_response()
}

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
    let c = cookies(&headers);
    let params = form(&body);
    let mut st = s.state();
    if name == "cbCheckLogin" {
        return xml(advanced_login(&s, &mut st, &c, &params));
    }
    let valid = c.get("SessionStatus").is_some_and(|v| v == "Valid")
        && c.get("SessionID")
            .is_some_and(|sid| st.advanced.contains(sid));
    if !valid {
        return LOGGED_OUT.into_response();
    }
    if is_read(name) && st.failing_reads.contains(name) {
        xml(failure(name, "lectura fallida (simulado)"))
    } else if is_read(name) {
        xml(read(&s, &st, name, &params))
    } else {
        xml(write(&mut st, name, &params))
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
        && param(params, "rqPasswd") == Some(mess_userpass(&s.password).as_str());
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
    if name == "cbDhcpV4SvrPoolGetStatic"
        && param(params, "rtm_cfg_dhcpv4_svr_pool_id") == Some("0")
    {
        return read_leases(st, params);
    }
    let raw = s
        .fixtures
        .get(&fixture_key(name, params))
        .or_else(|| s.fixtures.get(&(name.to_owned(), Vec::new())));
    let Some(raw) = raw else {
        return failure(name, "no fixture");
    };
    let mut node = match hgu::xml::parse_response(name, raw) {
        Ok(n) => n,
        Err(e) => return failure(name, &format!("fixture no válida: {e}")),
    };
    patch(st, name, &mut node);
    response(&node)
}

fn read_leases(st: &SimState, params: &[(String, String)]) -> String {
    const NAME: &str = "cbDhcpV4SvrPoolGetStatic";
    let wanted = param(params, "rtm_dhcpv4_svr_pool_static_id");
    if param(params, "all") != Some("ALL") && wanted.is_none() {
        return failure(NAME, "faltan parámetros");
    }
    let entries: Vec<XmlNode> = st
        .leases
        .iter()
        .enumerate()
        .filter(|(_, l)| wanted.is_none_or(|w| w == l.id.to_string()))
        .map(|(i, l)| XmlNode {
            tag: "rtm_dhcpv4_svr_pool_static".into(),
            text: String::new(),
            children: vec![
                leaf("rtm_dhcpv4_svr_pool_static_0i", &i.to_string()),
                leaf("rtm_dhcpv4_svr_pool_static_id", &l.id.to_string()),
                leaf("adm_state", &l.adm_state),
                leaf("mac_addr", &l.mac_addr),
                leaf("ipv4_addr", &l.ipv4_addr),
            ],
        })
        .collect();
    if wanted.is_some() && entries.is_empty() {
        return failure(NAME, "no existe");
    }
    response(&result_node(NAME, entries, "0", ""))
}

/// Refleja el estado en memoria sobre la respuesta de la fixture.
fn patch(st: &SimState, name: &str, node: &mut XmlNode) {
    match name {
        "cbGetUpnpCfg" => {
            if let Some(v) = &st.upnp_adm_state {
                for_each_node(node, &mut |n| {
                    if n.tag == "upnp_cfg" {
                        set_child(n, "adm_state", v);
                    }
                });
            }
        }
        "cbGetWifiAccPt" | "cbListWifiAccPt" => for_each_node(node, &mut |n| {
            let fields = n
                .text_of("dpm_cfg_wifi_acc_pt_id")
                .and_then(|id| st.wifi.get(id))
                .cloned();
            for (k, v) in fields.iter().flatten() {
                set_child(n, k, v);
            }
        }),
        "cbGetSummary" => {
            if let Some(t) = st.rebooted {
                set_child(node, "SysUptime", &t.elapsed().as_secs().to_string());
            }
        }
        "cbGetWanIPIntf" if !st.wan_up => for_each_node(node, &mut |n| {
            if n.text_of("name") == Some(WAN) {
                set_child(n, "opr_status", "0");
            }
        }),
        "cbGetStatistic" => for_each_node(node, &mut |n| {
            if n.text_of("name") == Some(WAN) {
                for (tag, extra) in [("rx_byte", st.rx_extra), ("tx_byte", st.tx_extra)] {
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

fn write(st: &mut SimState, name: &str, params: &[(String, String)]) -> String {
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
    if !validation_only {
        st.writes.push((name.to_owned(), clean.clone()));
    }
    if let Some(err) = st.fail_next.take() {
        return failure(name, &err);
    }
    if validation_only {
        return ok(name);
    }
    if std::mem::take(&mut st.ignore_next) {
        return ok(name);
    }
    let result = match name {
        "cbDhcpV4SvrPoolSetStatic" => set_static_lease(st, &clean),
        "cbSetUpnpCfg" => set_upnp(st, &clean),
        "cbSetWifiAccPt" => set_wifi_acc_pt(st, &clean),
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
        _ => Ok(()),
    };
    match result {
        Ok(()) => ok(name),
        Err(e) => failure(name, e),
    }
}

fn set_static_lease(st: &mut SimState, p: &[(String, String)]) -> Result<(), &'static str> {
    if param(p, "rtm_cfg_dhcpv4_svr_pool_id") != Some("0") {
        return Err("pool desconocido");
    }
    let id = param(p, "rtm_dhcpv4_svr_pool_static_id").ok_or("falta el id")?;
    if param(p, "del") == Some("DEL") {
        let before = st.leases.len();
        st.leases.retain(|l| l.id.to_string() != id);
        return if st.leases.len() < before {
            Ok(())
        } else {
            Err("no existe")
        };
    }
    if id == "-1" {
        let lease = Lease {
            id: st.leases.iter().map(|l| l.id + 1).max().unwrap_or(0),
            adm_state: param(p, "adm_state").unwrap_or("1").to_owned(),
            mac_addr: param(p, "mac_addr").ok_or("falta mac_addr")?.to_owned(),
            ipv4_addr: param(p, "ipv4_addr").ok_or("falta ipv4_addr")?.to_owned(),
        };
        st.leases.push(lease);
        return Ok(());
    }
    let lease = st
        .leases
        .iter_mut()
        .find(|l| l.id.to_string() == id)
        .ok_or("no existe")?;
    if let Some(v) = param(p, "adm_state") {
        lease.adm_state = v.to_owned();
    }
    if let Some(v) = param(p, "mac_addr") {
        lease.mac_addr = v.to_owned();
    }
    if let Some(v) = param(p, "ipv4_addr") {
        lease.ipv4_addr = v.to_owned();
    }
    Ok(())
}

fn set_upnp(st: &mut SimState, p: &[(String, String)]) -> Result<(), &'static str> {
    let v = param(p, "adm_state").ok_or("falta adm_state")?;
    st.upnp_adm_state = Some(v.to_owned());
    Ok(())
}

fn set_wifi_acc_pt(st: &mut SimState, p: &[(String, String)]) -> Result<(), &'static str> {
    let id = param(p, "dpm_cfg_wifi_acc_pt_id").ok_or("falta dpm_cfg_wifi_acc_pt_id")?;
    let fields = st.wifi.entry(id.to_owned()).or_default();
    for (k, v) in p {
        if k != "dpm_cfg_wifi_acc_pt_id" && k != "validation_only" {
            fields.insert(k.clone(), v.clone());
        }
    }
    Ok(())
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
