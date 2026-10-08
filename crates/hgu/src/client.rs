//! Cliente HTTP del router: sesiones básica y avanzada, reintento y serialización.

use crate::error::Error;
use crate::fmt::{mess_userpass, page_vars};
use crate::xml::{XmlError, XmlNode, parse_response};
use reqwest::Method;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::Instant};

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
    /// Pausa mínima entre peticiones consecutivas (por defecto 150 ms).
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
            min_gap: Duration::from_millis(150),
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
    cookies: BTreeMap<String, String>,
    basic_ok: bool,
    adv_ok: bool,
    last_request: Option<Instant>,
}

struct Shared {
    cfg: ClientConfig,
    http: reqwest::Client,
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

impl Client {
    /// Crea el cliente; no hace ninguna petición hasta la primera llamada.
    pub fn new(cfg: ClientConfig) -> Client {
        let http = reqwest::Client::builder()
            .timeout(cfg.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("configuración de reqwest válida");
        Client {
            shared: Arc::new(Shared {
                cfg,
                http,
                state: Mutex::new(State::default()),
            }),
        }
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

    /// Escritura: obtiene el `sessionKey` de `page` y llama a `name` con él.
    pub async fn write(
        &self,
        page: &str,
        name: &str,
        params: &[(&str, &str)],
    ) -> Result<XmlNode, Error> {
        let mut st = self.shared.state.lock().await;
        let key = self.session_key_locked(&mut st, page).await?;
        let mut all: Vec<(&str, &str)> = params.to_vec();
        all.push(("sessionKey", &key));
        self.call_locked(&mut st, name, &all).await
    }

    /// Devuelve el HTML de una página básica (reinicia sesión una vez si hace falta).
    pub async fn basic_page(&self, path: &str) -> Result<String, Error> {
        let mut st = self.shared.state.lock().await;
        self.page_locked(&mut st, path).await
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
        let path = format!("/cgi-bin/{name}.xml");
        let body = form_body(params);
        for attempt in 0..2 {
            if !self.ensure_advanced(st).await? {
                tracing::debug!("{name}: sesión básica caducada (intento {attempt})");
                continue;
            }
            let raw = self
                .send(st, Method::POST, &path, Some(body.clone()))
                .await?;
            if looks_logged_out(&raw) {
                tracing::debug!("{name}: sesión caducada (intento {attempt})");
                Self::invalidate(st);
                continue;
            }
            match parse_response(name, &raw) {
                Ok(node) if node.result_ok() => return Ok(node),
                Ok(node) => {
                    return Err(Error::CallFailed {
                        name: name.to_owned(),
                        result: node.text_of("Result").unwrap_or("").to_owned(),
                        error: node.text_of("ErrorString").unwrap_or("").to_owned(),
                    });
                }
                Err(XmlError::NotResponse) => {
                    tracing::debug!("{name}: sesión caducada (intento {attempt})");
                    Self::invalidate(st);
                }
                Err(e) => return Err(Error::Unexpected(format!("{name}: {e}"))),
            }
        }
        Err(Error::SessionExpired(format!(
            "{name}: el router sigue pidiendo iniciar sesión"
        )))
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

    async fn session_key_locked(&self, st: &mut State, page: &str) -> Result<String, Error> {
        let html = self.page_locked(st, page).await?;
        page_vars(&html)
            .remove("sessionKey")
            .ok_or_else(|| Error::Unexpected(format!("{page}: no contiene sessionKey")))
    }

    async fn ensure_basic(&self, st: &mut State) -> Result<(), Error> {
        if st.basic_ok {
            return Ok(());
        }
        tracing::debug!("login básico");
        self.send(st, Method::GET, "/", None).await?;
        let body = form_body(&[
            ("curWebPage", "/te_wifi.asp"),
            ("loginUsername", &mess_userpass(&self.shared.cfg.user)),
            ("loginPassword", &mess_userpass(&self.shared.cfg.password)),
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
        let user = mess_userpass(&self.shared.cfg.user);
        let body = form_body(&[
            ("rqUsername", &user),
            ("rqPasswd", &mess_userpass(&self.shared.cfg.password)),
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
        if let Some(last) = st.last_request {
            let wait = self.shared.cfg.min_gap.saturating_sub(last.elapsed());
            if !wait.is_zero() {
                tokio::time::sleep(wait).await;
            }
        }
        let url = format!("{}{}", self.shared.cfg.base_url.trim_end_matches('/'), path);
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
            req = req
                .header("Content-Type", "application/x-www-form-urlencoded")
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
        if status.is_server_error() {
            return Err(Error::Unexpected(format!("{path}: HTTP {status}")));
        }
        Ok(text)
    }
}
