//! Cliente HTTP del router: sesiones básica y avanzada, reintento y serialización.

use crate::error::Error;
use crate::fmt::{mess_userpass, page_vars};
use crate::xml::{XmlError, XmlNode, parse_response};
use reqwest::Method;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{Mutex, MutexGuard},
    time::Instant,
};

mod por_id;
pub use por_id::TOPE_POSICIONES;

/// Lecturas seguidas como mucho en un [`Turno`] antes de ceder el cerrojo
/// una vez a quien espera (el sondeo, el gestor de cambios): una tanda
/// larga no los deja sin turno. Un tema de Ajustes hace hasta 5.
pub const LECTURAS_POR_TURNO: usize = 6;

/// Pausa mínima de serie entre peticiones consecutivas al router.
pub const PAUSA_MINIMA: Duration = Duration::from_millis(150);

/// Configuración del cliente.
#[derive(Clone)]
pub struct ClientConfig {
    /// URL base, p. ej. `http://192.168.1.1`.
    pub base_url: String,
    /// Usuario del router.
    pub user: String,
    /// Contraseña del router (nunca se registra).
    pub password: String,
    /// Tiempo máximo por petición (por defecto 15 s).
    pub timeout: Duration,
    /// Pausa mínima entre peticiones consecutivas (por defecto
    /// [`PAUSA_MINIMA`], 150 ms).
    pub min_gap: Duration,
}

impl ClientConfig {
    /// Configuración con los tiempos por defecto.
    pub fn new(base_url: &str, user: &str, password: &str) -> Self {
        Self {
            base_url: base_url.to_owned(),
            user: user.to_owned(),
            password: password.to_owned(),
            timeout: Duration::from_secs(15),
            min_gap: PAUSA_MINIMA,
        }
    }
}

impl std::fmt::Debug for ClientConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientConfig")
            .field("base_url", &self.base_url)
            .field("user", &self.user)
            .field("password", &"***")
            .field("timeout", &self.timeout)
            .field("min_gap", &self.min_gap)
            .finish()
    }
}

#[derive(Default)]
struct State {
    /// `None` mientras el router no está configurado.
    cfg: Option<ClientConfig>,
    cookies: BTreeMap<String, String>,
    basic_ok: bool,
    adv_ok: bool,
    last_request: Option<Instant>,
}

struct Shared {
    http: reqwest::Client,
    /// Espejo de `state.cfg.is_some()` para consultarlo sin esperar el cerrojo.
    configurado: AtomicBool,
    /// Ver [`Client::escrituras`].
    escrituras: AtomicU64,
    state: Mutex<State>,
}

/// Cliente del router. Barato de clonar; todas las peticiones se serializan.
#[derive(Clone)]
pub struct Client {
    shared: Arc<Shared>,
}

/// Codifica como `encodeURIComponent`.
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn form_body(params: &[(&str, &str)]) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn looks_logged_out(body: &str) -> bool {
    body.len() < 1024 && body.contains("te_acceso_router")
}

/// Ruta sin la consulta: los errores nunca muestran `?…` (en la UI básica
/// lleva el `sessionKey` y los valores escritos).
fn sin_consulta(path: &str) -> &str {
    path.split('?').next().unwrap_or(path)
}

/// Error conocido en la respuesta (corta) de una escritura de la UI básica.
fn basic_error(body: &str) -> Option<&'static str> {
    if body.len() >= 1024 {
        return None;
    }
    let b = body.to_lowercase();
    let clave = b.contains("sessionkey") || b.contains("session key");
    let mala = b.contains("no válid") || b.contains("invalid") || b.contains("inválid");
    (clave && mala).then_some("el router rechazó el sessionKey")
}

impl Client {
    /// Crea el cliente; no hace ninguna petición hasta la primera llamada.
    pub fn new(cfg: ClientConfig) -> Client {
        Self::con(Some(cfg))
    }

    /// Cliente sin router: toda llamada da [`Error::NoConfigurado`] y no sale
    /// ninguna petición hasta que se llame a [`Client::reconfigurar`].
    pub fn sin_configurar() -> Client {
        Self::con(None)
    }

    fn con(cfg: Option<ClientConfig>) -> Client {
        // El tiempo máximo se fija aquí y no cambia al reconfigurar.
        let timeout = cfg
            .as_ref()
            .map_or_else(|| ClientConfig::new("", "", "").timeout, |c| c.timeout);
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("configuración de reqwest válida");
        Client {
            shared: Arc::new(Shared {
                http,
                configurado: AtomicBool::new(cfg.is_some()),
                escrituras: AtomicU64::new(0),
                state: Mutex::new(State {
                    cfg,
                    ..State::default()
                }),
            }),
        }
    }

    /// Cambia URL, usuario y contraseña (y la pausa entre peticiones) y
    /// descarta la sesión y las cookies. Espera a que termine la petición en
    /// curso: toma el mismo cerrojo que todas las llamadas. El tiempo máximo
    /// por petición sigue siendo el del cliente original.
    pub async fn reconfigurar(&self, cfg: ClientConfig) {
        let mut st = self.shared.state.lock().await;
        self.contar_escritura();
        Self::invalidate(&mut st);
        st.last_request = None;
        st.cfg = Some(cfg);
        self.shared.configurado.store(true, Ordering::SeqCst);
    }

    /// Contador que cambia con cada escritura (al empezar y al terminar) y
    /// con cada [`Client::reconfigurar`]: lo leído antes de un cambio del
    /// contador puede no ser ya lo que muestra el router. Lo usa la caché de
    /// lecturas del panel.
    pub fn escrituras(&self) -> u64 {
        self.shared.escrituras.load(Ordering::SeqCst)
    }

    fn contar_escritura(&self) {
        self.shared.escrituras.fetch_add(1, Ordering::SeqCst);
    }

    /// Toma el cerrojo para una tanda de lecturas seguidas ([`Turno::call`]):
    /// nadie más (el sondeo, el gestor) llama al router entre ellas. Las
    /// peticiones siguen en serie y con `min_gap`. Tras
    /// [`LECTURAS_POR_TURNO`] lecturas, el turno cede el cerrojo una vez
    /// antes de seguir.
    pub async fn turno(&self) -> Turno<'_> {
        Turno {
            client: self,
            st: Some(self.shared.state.lock().await),
            hechas: 0,
        }
    }

    /// Toma el cerrojo del cliente sin cederlo hasta soltar lo devuelto: nadie
    /// más (el sondeo, el gestor, las páginas) llama al router por este
    /// cliente mientras tanto, así que nadie cierra ni renueva su sesión. Lo
    /// usa el cambio de la contraseña del router, que depende de la sesión
    /// abierta para volver atrás. Las llamadas por este cliente esperan; las
    /// de otro cliente (otra sesión) no.
    pub async fn en_exclusiva(&self) -> Exclusiva<'_> {
        Exclusiva {
            client: self,
            st: self.shared.state.lock().await,
        }
    }

    /// Si hay un router configurado.
    pub fn configurado(&self) -> bool {
        self.shared.configurado.load(Ordering::SeqCst)
    }

    /// Llama a `/cgi-bin/<name>.xml` y devuelve el nodo `<name>`.
    pub async fn call(&self, name: &str, params: &[(&str, &str)]) -> Result<XmlNode, Error> {
        let mut st = self.shared.state.lock().await;
        self.call_locked(&mut st, name, params).await
    }

    /// Obtiene el `sessionKey` de una página (p. ej. `dhcp.asp`).
    pub async fn session_key(&self, page: &str) -> Result<String, Error> {
        let mut st = self.shared.state.lock().await;
        self.session_key_locked(&mut st, page).await
    }

    /// Escritura: obtiene el `sessionKey` de `page` (ya con la sesión
    /// avanzada abierta) y llama a `name` con él. Si la sesión había
    /// caducado, entra de nuevo y repite una vez con un `sessionKey` nuevo:
    /// el de antes no vale en la sesión nueva.
    pub async fn write(
        &self,
        page: &str,
        name: &str,
        params: &[(&str, &str)],
    ) -> Result<XmlNode, Error> {
        let mut st = self.shared.state.lock().await;
        self.contar_escritura();
        let r = self.write_locked(&mut st, page, name, params).await;
        self.contar_escritura();
        r
    }

    async fn write_locked(
        &self,
        st: &mut State,
        page: &str,
        name: &str,
        params: &[(&str, &str)],
    ) -> Result<XmlNode, Error> {
        for attempt in 0..2 {
            if !self.ensure_advanced(st).await? {
                tracing::debug!("{name}: sesión básica caducada (intento {attempt})");
                continue;
            }
            let key = self.session_key_locked(st, page).await?;
            let mut all: Vec<(&str, &str)> = params.to_vec();
            all.push(("sessionKey", &key));
            if let Some(node) = self.call_once(st, name, &all).await? {
                return Ok(node);
            }
            tracing::debug!("{name}: sesión caducada (intento {attempt})");
        }
        Err(Error::SessionExpired(format!(
            "{name}: el router sigue pidiendo iniciar sesión"
        )))
    }

    /// Escritura de la UI básica: obtiene el `sessionKey` de `page` y hace
    /// `GET /cgi-bin/<path>` con `params` y `sessionKey` en la URL (`path`
    /// puede llevar ya su `?action=…`). Los valores van tal cual: ofuscar
    /// las claves (XOR 0x1f) es cosa de quien llama. Si el router pide
    /// iniciar sesión, entra de nuevo y repite una vez con un `sessionKey`
    /// nuevo. Una respuesta de error conocida (`sessionKey` no válido) es
    /// [`Error::CallFailed`]. Devuelve el cuerpo de la respuesta.
    pub async fn basic_write(
        &self,
        page: &str,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<String, Error> {
        let mut st = self.shared.state.lock().await;
        self.contar_escritura();
        let r = self.basic_write_locked(&mut st, page, path, params).await;
        self.contar_escritura();
        r
    }

    async fn basic_write_locked(
        &self,
        st: &mut State,
        page: &str,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<String, Error> {
        let path = path.trim_start_matches('/');
        let sep = if path.contains('?') { '&' } else { '?' };
        for _ in 0..2 {
            let key = self.session_key_locked(st, page).await?;
            let mut all: Vec<(&str, &str)> = params.to_vec();
            all.push(("sessionKey", &key));
            let url = format!("/cgi-bin/{path}{sep}{}", form_body(&all));
            let body = self.send(st, Method::GET, &url, None).await?;
            if let Some(error) = basic_error(&body) {
                return Err(Error::CallFailed {
                    name: sin_consulta(path).to_owned(),
                    result: String::new(),
                    error: error.to_owned(),
                });
            }
            if !looks_logged_out(&body) {
                return Ok(body);
            }
            Self::invalidate(st);
        }
        Err(Error::SessionExpired(format!(
            "{}: el router sigue pidiendo iniciar sesión",
            sin_consulta(path)
        )))
    }

    /// Devuelve el HTML de una página básica (reinicia sesión una vez si hace falta).
    pub async fn basic_page(&self, path: &str) -> Result<String, Error> {
        let mut st = self.shared.state.lock().await;
        self.page_locked(&mut st, path).await
    }

    /// Inicia sesión, la básica y la avanzada, sin leer nada más: comprueba
    /// unas credenciales que quizá no pueden leer otra cosa (las de otro
    /// usuario de la web del router). Error si el router las rechaza.
    pub async fn entrar(&self) -> Result<(), Error> {
        let mut st = self.shared.state.lock().await;
        for _ in 0..2 {
            if self.ensure_advanced(&mut st).await? {
                return Ok(());
            }
        }
        Err(Error::SessionExpired(
            "el router sigue pidiendo iniciar sesión".into(),
        ))
    }

    /// Cierra la sesión en el router (best effort) y olvida las cookies.
    pub async fn logout(&self) {
        let mut st = self.shared.state.lock().await;
        if (st.basic_ok || st.adv_ok)
            && let Err(e) = self
                .send(&mut st, Method::GET, "/cgi-bin/te_logout.cgi", None)
                .await
        {
            tracing::debug!("logout: {e}");
        }
        Self::invalidate(&mut st);
    }

    fn invalidate(st: &mut State) {
        st.cookies.clear();
        st.basic_ok = false;
        st.adv_ok = false;
    }

    async fn call_locked(
        &self,
        st: &mut State,
        name: &str,
        params: &[(&str, &str)],
    ) -> Result<XmlNode, Error> {
        for attempt in 0..2 {
            if !self.ensure_advanced(st).await? {
                tracing::debug!("{name}: sesión básica caducada (intento {attempt})");
                continue;
            }
            if let Some(node) = self.call_once(st, name, params).await? {
                return Ok(node);
            }
            tracing::debug!("{name}: sesión caducada (intento {attempt})");
        }
        Err(Error::SessionExpired(format!(
            "{name}: el router sigue pidiendo iniciar sesión"
        )))
    }

    /// Una llamada con la sesión avanzada ya abierta. `Ok(None)` si el router
    /// pide iniciar sesión otra vez (la sesión queda invalidada).
    async fn call_once(
        &self,
        st: &mut State,
        name: &str,
        params: &[(&str, &str)],
    ) -> Result<Option<XmlNode>, Error> {
        let path = format!("/cgi-bin/{name}.xml");
        let raw = self
            .send(st, Method::POST, &path, Some(form_body(params)))
            .await?;
        if looks_logged_out(&raw) {
            Self::invalidate(st);
            return Ok(None);
        }
        match parse_response(name, &raw) {
            Ok(node) if node.result_ok() => Ok(Some(node)),
            // El texto del error: `ErrorString` o, si no viene, `Msg` (como
            // responde el router a un alta de usuario que no acepta).
            Ok(node) => Err(Error::CallFailed {
                name: name.to_owned(),
                result: node.text_of("Result").unwrap_or("").to_owned(),
                error: node
                    .text_of("ErrorString")
                    .filter(|e| !e.is_empty())
                    .or_else(|| node.text_of("Msg"))
                    .unwrap_or("")
                    .to_owned(),
            }),
            Err(XmlError::NotResponse) => {
                Self::invalidate(st);
                Ok(None)
            }
            Err(e) => Err(Error::Unexpected(format!("{name}: {e}"))),
        }
    }

    async fn page_locked(&self, st: &mut State, path: &str) -> Result<String, Error> {
        let path = if path.starts_with('/') {
            path.to_owned()
        } else {
            format!("/{path}")
        };
        for _ in 0..2 {
            self.ensure_basic(st).await?;
            let body = self.send(st, Method::GET, &path, None).await?;
            if !looks_logged_out(&body) {
                return Ok(body);
            }
            Self::invalidate(st);
        }
        Err(Error::SessionExpired(format!(
            "{path}: el router sigue pidiendo iniciar sesión"
        )))
    }

    /// El `sessionKey` de `page`: su `var sessionKey` o, si no lo trae, su
    /// `var sessKey` (las páginas del cortafuegos solo traen este).
    async fn session_key_locked(&self, st: &mut State, page: &str) -> Result<String, Error> {
        let html = self.page_locked(st, page).await?;
        let mut vars = page_vars(&html);
        vars.remove("sessionKey")
            .or_else(|| vars.remove("sessKey"))
            .ok_or_else(|| Error::Unexpected(format!("{page}: no contiene sessionKey")))
    }

    async fn ensure_basic(&self, st: &mut State) -> Result<(), Error> {
        if st.basic_ok {
            return Ok(());
        }
        tracing::debug!("login básico");
        let (user, password) = {
            let cfg = st.cfg.as_ref().ok_or(Error::NoConfigurado)?;
            (mess_userpass(&cfg.user), mess_userpass(&cfg.password))
        };
        self.send(st, Method::GET, "/", None).await?;
        let body = form_body(&[
            ("curWebPage", "/te_wifi.asp"),
            ("loginUsername", &user),
            ("loginPassword", &password),
        ]);
        let resp = self
            .send(
                st,
                Method::POST,
                "/cgi-bin/te_acceso_router.cgi",
                Some(body),
            )
            .await?;
        if resp.contains("location.href = \"/te_wifi.asp\"") {
            st.basic_ok = true;
            Ok(())
        } else {
            Err(Error::Auth("el router rechazó usuario o contraseña"))
        }
    }

    /// `Ok(false)`: la sesión básica había caducado (ya invalidada); hay que reintentar.
    async fn ensure_advanced(&self, st: &mut State) -> Result<bool, Error> {
        if st.adv_ok {
            return Ok(true);
        }
        self.ensure_basic(st).await?;
        tracing::debug!("login avanzado");
        let (user, password) = {
            let cfg = st.cfg.as_ref().ok_or(Error::NoConfigurado)?;
            (mess_userpass(&cfg.user), mess_userpass(&cfg.password))
        };
        let body = form_body(&[
            ("rqUsername", &user),
            ("rqPasswd", &password),
            ("rqTimeout", "600"),
        ]);
        let raw = self
            .send(st, Method::POST, "/cgi-bin/cbCheckLogin.xml", Some(body))
            .await?;
        let node = match parse_response("cbCheckLogin", &raw) {
            Ok(n) => n,
            Err(XmlError::NotResponse) => {
                Self::invalidate(st);
                return Ok(false);
            }
            Err(e) => return Err(Error::Unexpected(format!("cbCheckLogin: {e}"))),
        };
        if !node.result_ok() {
            Self::invalidate(st);
            return Err(Error::Auth("el router rechazó usuario o contraseña"));
        }
        let sid = node
            .text_of("SessionID")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Unexpected("cbCheckLogin sin SessionID".into()))?;
        st.cookies.insert("SessionID".into(), sid.to_owned());
        st.cookies.insert("SessionStatus".into(), "Valid".into());
        st.cookies.insert("LoginName".into(), urlencode(&user));
        st.cookies.insert("TimeOut".into(), "600".into());
        st.adv_ok = true;
        Ok(true)
    }

    /// Una petición HTTP: respeta `min_gap`, envía las cookies y guarda las nuevas.
    async fn send(
        &self,
        st: &mut State,
        method: Method,
        path: &str,
        body: Option<String>,
    ) -> Result<String, Error> {
        let (base_url, min_gap) = {
            let cfg = st.cfg.as_ref().ok_or(Error::NoConfigurado)?;
            (cfg.base_url.trim_end_matches('/').to_owned(), cfg.min_gap)
        };
        if let Some(last) = st.last_request {
            let wait = min_gap.saturating_sub(last.elapsed());
            if !wait.is_zero() {
                tokio::time::sleep(wait).await;
            }
        }
        let url = format!("{base_url}{path}");
        let mut req = self
            .shared
            .http
            .request(method, &url)
            .header("X-Requested-With", "XMLHttpRequest");
        if !st.cookies.is_empty() {
            let cookie = st
                .cookies
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; ");
            req = req.header("Cookie", cookie);
        }
        if let Some(b) = body {
            // Explícito: con un cuerpo vacío hyper no envía `Content-Length` y
            // el micro_httpd del router responde 400 a ese POST.
            req = req
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Content-Length", b.len())
                .body(b);
        }
        let result = async {
            let resp = req.send().await?;
            let set: Vec<String> = resp
                .headers()
                .get_all("set-cookie")
                .iter()
                .filter_map(|v| v.to_str().ok().map(str::to_owned))
                .collect();
            let status = resp.status();
            let text = resp.text().await?;
            Ok::<_, reqwest::Error>((status, set, text))
        }
        .await;
        st.last_request = Some(Instant::now());
        let (status, set, text) =
            result.map_err(|e| Error::Network(e.without_url().to_string()))?;
        for c in set {
            if let Some((k, v)) = c.split(';').next().and_then(|nv| nv.split_once('=')) {
                st.cookies.insert(k.trim().to_owned(), v.trim().to_owned());
            }
        }
        if status.is_client_error() || status.is_server_error() {
            return Err(Error::Unexpected(format!(
                "{}: HTTP {status}",
                sin_consulta(path)
            )));
        }
        Ok(text)
    }
}

/// Turno del cerrojo del cliente para una tanda de lecturas seguidas (ver
/// [`Client::turno`]). Lo suelta al soltarse.
pub struct Turno<'a> {
    client: &'a Client,
    /// Siempre `Some` fuera de [`Turno::call`].
    st: Option<MutexGuard<'a, State>>,
    /// Lecturas desde que se tomó el cerrojo por última vez.
    hechas: usize,
}

impl Turno<'_> {
    /// Como [`Client::call`], dentro del turno. Cada
    /// [`LECTURAS_POR_TURNO`] lecturas cede el cerrojo y espera a volver a
    /// tenerlo (el `Mutex` de tokio es justo: pasa antes quien esperaba).
    pub async fn call(&mut self, name: &str, params: &[(&str, &str)]) -> Result<XmlNode, Error> {
        self.ceder_si_toca().await;
        let st = self.st.as_mut().expect("el turno tiene el cerrojo");
        self.client.call_locked(st, name, params).await
    }

    /// Como [`Client::basic_page`], dentro del turno (cuenta como una
    /// lectura más de [`LECTURAS_POR_TURNO`]).
    pub async fn pagina(&mut self, path: &str) -> Result<String, Error> {
        self.ceder_si_toca().await;
        let st = self.st.as_mut().expect("el turno tiene el cerrojo");
        self.client.page_locked(st, path).await
    }

    /// Cuenta una lectura y, cada [`LECTURAS_POR_TURNO`], cede el cerrojo y
    /// espera a volver a tenerlo.
    async fn ceder_si_toca(&mut self) {
        if self.hechas >= LECTURAS_POR_TURNO {
            self.st = None;
            self.st = Some(self.client.shared.state.lock().await);
            self.hechas = 0;
        }
        self.hechas += 1;
    }
}

/// El cerrojo del cliente, sin ceder (ver [`Client::en_exclusiva`]).
pub struct Exclusiva<'a> {
    client: &'a Client,
    st: MutexGuard<'a, State>,
}

impl Exclusiva<'_> {
    /// Como [`Client::write`], con el cerrojo ya tomado.
    pub async fn write(
        &mut self,
        page: &str,
        name: &str,
        params: &[(&str, &str)],
    ) -> Result<XmlNode, Error> {
        self.client.contar_escritura();
        let r = self
            .client
            .write_locked(&mut self.st, page, name, params)
            .await;
        self.client.contar_escritura();
        r
    }
}
