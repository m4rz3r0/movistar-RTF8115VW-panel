//! Autenticación del panel: usuarios con contraseña Argon2id, sesiones en
//! SQLite con cookie `__Host-rp_session`, freno de fuerza bruta por IP y por
//! usuario, y las server functions `login`, `logout` y `quien_soy`.
//!
//! Toda server function salvo `login` y `logout` empieza con [`require_user`]; las
//! páginas SSR quedan protegidas por [`require_page_session`] (extractor
//! [`AuthUser`]), que redirige a `/login?next=<ruta>`.

use leptos::prelude::*;

#[cfg(feature = "ssr")]
pub use server::{
    AuthUser, LoginError, MIN_PASSWORD_CHARS, SESSION_COOKIE, User, UserError, attempt_login,
    client_ip, create_user, delay_secs, hash_password, poner_cookie_de_sesion, reauthenticate,
    require_page_session, require_user, set_password, verify, warm_up,
};

/// Mensaje de error de las server functions sin sesión vigente (con estado
/// 401). Es un código fijo, no un texto: la interfaz lo reconoce y lo
/// traduce en el idioma de la página (ver `ui::texto_error`).
pub const ERROR_SESION: &str = "sesion-caducada";

/// Destino tras el login: `next` solo si es una ruta local (empieza por una
/// sola `/`); si no, la portada. Evita redirecciones abiertas a otro sitio
/// (`//otro.example`, `/\otro.example`, `https://…`).
pub fn safe_next(next: Option<&str>) -> &str {
    match next {
        Some(n)
            if n.starts_with('/')
                && !n.starts_with("//")
                && !n.starts_with("/\\")
                && !n.chars().any(char::is_control) =>
        {
            n
        }
        _ => "/",
    }
}

/// Inicia sesión. Devuelve la cookie de sesión en `Set-Cookie`; los errores
/// son "usuario o contraseña incorrectos" (401) o "demasiados intentos" (429),
/// sin distinguir si el usuario existe.
#[server(endpoint = "login")]
pub async fn login(usuario: String, contrasena: String) -> Result<(), ServerFnError> {
    server::login_endpoint(usuario, contrasena).await
}

/// Cierra la sesión: borra la fila de la cookie (si la hay) y siempre
/// devuelve la cookie caducada. No exige sesión vigente.
#[server(endpoint = "logout")]
pub async fn logout() -> Result<(), ServerFnError> {
    server::logout_endpoint().await
}

/// Nombre del usuario de la sesión.
#[server(endpoint = "quien_soy")]
pub async fn quien_soy() -> Result<String, ServerFnError> {
    Ok(require_user().await?.username)
}

#[cfg(feature = "ssr")]
mod server {
    use crate::state::AppState;
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
    };
    use axum::{
        extract::{ConnectInfo, FromRequestParts, Request, State},
        http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
        middleware::Next,
        response::{IntoResponse, Redirect, Response},
    };
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use leptos::prelude::*;
    use leptos_axum::ResponseOptions;
    use sha2::{Digest, Sha256};
    use sqlx::SqlitePool;
    use std::{
        fmt,
        net::{IpAddr, SocketAddr},
        sync::LazyLock,
        time::{SystemTime, UNIX_EPOCH},
    };

    /// Nombre de la cookie de sesión.
    pub const SESSION_COOKIE: &str = "__Host-rp_session";
    /// Longitud mínima de la contraseña del panel, en caracteres.
    pub const MIN_PASSWORD_CHARS: usize = 12;
    /// Longitud máxima de la contraseña en el login, en bytes: más larga es un
    /// fallo normal, sin gastar Argon2.
    const MAX_LOGIN_PASSWORD_BYTES: usize = 1024;
    /// Longitud máxima del nombre de usuario, en caracteres.
    const MAX_USERNAME_CHARS: usize = 64;
    /// Caducidad por inactividad.
    const SESSION_IDLE_SECS: i64 = 30 * 60;
    /// Caducidad absoluta.
    const SESSION_MAX_SECS: i64 = 12 * 60 * 60;
    /// `last_seen` se actualiza como mucho una vez por minuto.
    const TOUCH_EVERY_SECS: i64 = 60;
    /// Retardo máximo del freno de fuerza bruta.
    const MAX_DELAY_SECS: i64 = 60;
    /// Los contadores sin fallos nuevos durante un día se olvidan.
    const ATTEMPTS_FORGET_MS: i64 = 24 * 60 * 60 * 1000;

    /// Usuario autenticado.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct User {
        pub id: i64,
        pub username: String,
    }

    fn now_ms() -> i64 {
        let d = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("reloj posterior a 1970");
        i64::try_from(d.as_millis()).expect("milisegundos en i64")
    }

    fn now_secs() -> i64 {
        now_ms() / 1000
    }

    // ---- Contraseñas ----

    /// Hash Argon2id (parámetros por defecto del crate `argon2`) en formato
    /// PHC, con sal aleatoria del sistema.
    pub fn hash_password(password: &str) -> String {
        let hash: PasswordHash = Argon2::default()
            .hash_password(password.as_bytes())
            .expect("Argon2 con parámetros por defecto y sal del sistema");
        hash.to_string()
    }

    /// Comprueba `password` contra el hash PHC `hash`. Un hash mal formado
    /// cuenta como no válido.
    pub fn verify(password: &str, hash: &str) -> bool {
        PasswordHash::new(hash).is_ok_and(|h| {
            Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok()
        })
    }

    /// Hash con el que se verifica cuando el usuario no existe, para que el
    /// coste (y el tiempo) sea el mismo que con un usuario real.
    static DUMMY_HASH: LazyLock<String> =
        LazyLock::new(|| hash_password("contraseña ficticia para igualar tiempos"));

    /// Calcula ya el hash ficticio, para que el primer login con un usuario
    /// inexistente no tarde más que los demás. Se llama al arrancar.
    pub fn warm_up() {
        LazyLock::force(&DUMMY_HASH);
    }

    async fn hash_blocking(password: &str) -> String {
        let password = password.to_owned();
        tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .expect("la tarea de hash no entra en pánico")
    }

    // ---- Usuarios ----

    /// Error al dar de alta un usuario o cambiar su contraseña.
    #[derive(Debug)]
    pub enum UserError {
        InvalidUsername,
        PasswordTooShort,
        Exists,
        NotFound,
        Db(sqlx::Error),
    }

    impl fmt::Display for UserError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                UserError::InvalidUsername => write!(
                    f,
                    "nombre de usuario no válido: de 1 a {MAX_USERNAME_CHARS} caracteres, sin espacios ni caracteres de control"
                ),
                UserError::PasswordTooShort => write!(
                    f,
                    "la contraseña debe tener al menos {MIN_PASSWORD_CHARS} caracteres"
                ),
                UserError::Exists => write!(f, "el usuario ya existe"),
                UserError::NotFound => write!(f, "el usuario no existe"),
                UserError::Db(e) => write!(f, "error de la base de datos: {e}"),
            }
        }
    }

    impl std::error::Error for UserError {}

    impl From<sqlx::Error> for UserError {
        fn from(e: sqlx::Error) -> Self {
            UserError::Db(e)
        }
    }

    fn check_username(username: &str) -> Result<(), UserError> {
        let len = username.chars().count();
        if len == 0
            || len > MAX_USERNAME_CHARS
            || username
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(UserError::InvalidUsername);
        }
        Ok(())
    }

    fn check_password(password: &str) -> Result<(), UserError> {
        if password.chars().count() < MIN_PASSWORD_CHARS {
            return Err(UserError::PasswordTooShort);
        }
        Ok(())
    }

    /// Da de alta `username` con la contraseña `password`.
    pub async fn create_user(
        db: &SqlitePool,
        username: &str,
        password: &str,
    ) -> Result<(), UserError> {
        check_username(username)?;
        check_password(password)?;
        let hash = hash_blocking(password).await;
        let res =
            sqlx::query("INSERT INTO users (username, password_hash, created) VALUES (?, ?, ?)")
                .bind(username)
                .bind(hash)
                .bind(now_secs())
                .execute(db)
                .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(UserError::Exists),
            Err(e) => Err(e.into()),
        }
    }

    /// Cambia la contraseña de `username` y cierra todas sus sesiones.
    pub async fn set_password(
        db: &SqlitePool,
        username: &str,
        password: &str,
    ) -> Result<(), UserError> {
        check_password(password)?;
        let hash = hash_blocking(password).await;
        let mut tx = db.begin().await?;
        let id: Option<i64> = sqlx::query_scalar(
            "UPDATE users SET password_hash = ? WHERE username = ? RETURNING id",
        )
        .bind(hash)
        .bind(username)
        .fetch_optional(&mut *tx)
        .await?;
        let id = id.ok_or(UserError::NotFound)?;
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- Sesiones ----

    /// Id de sesión nuevo: 32 bytes del generador del sistema en base64url.
    fn new_session_id() -> String {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("generador aleatorio del sistema");
        URL_SAFE_NO_PAD.encode(bytes)
    }

    /// Clave con la que se guarda la sesión: SHA-256 del id. La base de datos
    /// nunca contiene el id que viaja en la cookie.
    fn session_key(id: &str) -> String {
        URL_SAFE_NO_PAD.encode(Sha256::digest(id.as_bytes()))
    }

    fn session_cookie(id: &str) -> String {
        format!(
            "{SESSION_COOKIE}={id}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={SESSION_MAX_SECS}"
        )
    }

    fn expired_cookie() -> String {
        format!("{SESSION_COOKIE}=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0")
    }

    /// Id de sesión de la cabecera `Cookie`, si la hay.
    fn cookie_session_id(headers: &HeaderMap) -> Option<&str> {
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(name, _)| *name == SESSION_COOKIE)
            .map(|(_, id)| id)
            .filter(|id| !id.is_empty())
    }

    /// Borra las sesiones caducadas y los contadores de fallos olvidados.
    async fn purge_expired(db: &SqlitePool) -> Result<(), sqlx::Error> {
        let now = now_secs();
        sqlx::query("DELETE FROM sessions WHERE last_seen <= ? OR created <= ?")
            .bind(now - SESSION_IDLE_SECS)
            .bind(now - SESSION_MAX_SECS)
            .execute(db)
            .await?;
        sqlx::query("DELETE FROM login_attempts WHERE last_failure_ms <= ?")
            .bind(now_ms() - ATTEMPTS_FORGET_MS)
            .execute(db)
            .await?;
        Ok(())
    }

    async fn create_session(db: &SqlitePool, user_id: i64) -> Result<String, sqlx::Error> {
        purge_expired(db).await?;
        let id = new_session_id();
        let now = now_secs();
        sqlx::query("INSERT INTO sessions (id, user_id, created, last_seen) VALUES (?, ?, ?, ?)")
            .bind(session_key(&id))
            .bind(user_id)
            .bind(now)
            .bind(now)
            .execute(db)
            .await?;
        Ok(id)
    }

    /// Usuario de la sesión `id` si sigue vigente. Una sesión caducada se
    /// borra; una vigente renueva `last_seen` (como mucho una vez por minuto).
    async fn session_user(db: &SqlitePool, id: &str) -> Result<Option<User>, sqlx::Error> {
        let key = session_key(id);
        let row: Option<(i64, String, i64, i64)> = sqlx::query_as(
            "SELECT u.id, u.username, s.created, s.last_seen
             FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.id = ?",
        )
        .bind(&key)
        .fetch_optional(db)
        .await?;
        let Some((user_id, username, created, last_seen)) = row else {
            return Ok(None);
        };
        let now = now_secs();
        if now - last_seen >= SESSION_IDLE_SECS || now - created >= SESSION_MAX_SECS {
            sqlx::query("DELETE FROM sessions WHERE id = ?")
                .bind(&key)
                .execute(db)
                .await?;
            return Ok(None);
        }
        if now - last_seen >= TOUCH_EVERY_SECS {
            sqlx::query("UPDATE sessions SET last_seen = ? WHERE id = ?")
                .bind(now)
                .bind(&key)
                .execute(db)
                .await?;
        }
        Ok(Some(User {
            id: user_id,
            username,
        }))
    }

    async fn user_from_headers(
        db: &SqlitePool,
        headers: &HeaderMap,
    ) -> Result<Option<User>, sqlx::Error> {
        match cookie_session_id(headers) {
            Some(id) => session_user(db, id).await,
            None => Ok(None),
        }
    }

    // ---- Freno de fuerza bruta ----

    /// IP del cliente: la del socket, salvo que el socket sea el proxy de
    /// confianza; entonces, el último valor de `X-Forwarded-For` (el que añadió
    /// el proxy). Si el proxy no lo manda o no es una IP, se usa la del proxy.
    pub fn client_ip(peer: IpAddr, headers: &HeaderMap, trusted_proxy: IpAddr) -> IpAddr {
        let peer = peer.to_canonical();
        if peer != trusted_proxy.to_canonical() {
            return peer;
        }
        headers
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(','))
            .last()
            .and_then(|ip| ip.trim().parse::<IpAddr>().ok())
            .map_or(peer, |ip| ip.to_canonical())
    }

    /// Espera tras `failures` fallos seguidos: `min(2^(fallos-1), 60)` s.
    pub fn delay_secs(failures: i64) -> i64 {
        match failures {
            ..=0 => 0,
            1..=6 => (1 << (failures - 1)).min(MAX_DELAY_SECS),
            _ => MAX_DELAY_SECS,
        }
    }

    /// Milisegundos que faltan para poder intentar de nuevo (0 si ya se puede).
    async fn wait_ms(db: &SqlitePool, ip: &str, user: &str) -> Result<i64, sqlx::Error> {
        let rows: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT failures, last_failure_ms FROM login_attempts
             WHERE (kind = 'ip' AND key = ?) OR (kind = 'user' AND key = ?)",
        )
        .bind(ip)
        .bind(user)
        .fetch_all(db)
        .await?;
        let now = now_ms();
        Ok(rows
            .into_iter()
            .map(|(failures, last)| last + delay_secs(failures) * 1000 - now)
            .max()
            .unwrap_or(0)
            .max(0))
    }

    async fn record_failure(db: &SqlitePool, ip: &str, user: &str) -> Result<(), sqlx::Error> {
        let now = now_ms();
        for (kind, key) in [("ip", ip), ("user", user)] {
            sqlx::query(
                "INSERT INTO login_attempts (kind, key, failures, last_failure_ms)
                 VALUES (?, ?, 1, ?)
                 ON CONFLICT (kind, key) DO UPDATE
                 SET failures = failures + 1, last_failure_ms = excluded.last_failure_ms",
            )
            .bind(kind)
            .bind(key)
            .bind(now)
            .execute(db)
            .await?;
        }
        Ok(())
    }

    async fn clear_failures(db: &SqlitePool, ip: &str, user: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM login_attempts
             WHERE (kind = 'ip' AND key = ?) OR (kind = 'user' AND key = ?)",
        )
        .bind(ip)
        .bind(user)
        .execute(db)
        .await?;
        Ok(())
    }

    // ---- Login ----

    /// Motivo por el que no se inicia sesión.
    #[derive(Debug)]
    pub enum LoginError {
        /// El freno está activo: faltan estos segundos.
        Throttled(i64),
        /// Usuario inexistente o contraseña errónea (no se distingue).
        BadCredentials,
        Internal(String),
    }

    impl From<sqlx::Error> for LoginError {
        fn from(e: sqlx::Error) -> Self {
            LoginError::Internal(e.to_string())
        }
    }

    /// Intenta iniciar sesión desde `ip`; devuelve el id de la sesión nueva.
    ///
    /// Los intentos se serializan con `state.login_lock`, así que el freno no
    /// se puede esquivar con peticiones simultáneas. Durante la espera se
    /// rechaza sin comprobar la contraseña ni contar otro fallo.
    pub async fn attempt_login(
        state: &AppState,
        ip: IpAddr,
        username: &str,
        password: &str,
    ) -> Result<String, LoginError> {
        let _turn = state.login_lock.lock().await;
        let id = check_throttled(state, ip, Account::Name(username), password).await?;
        Ok(create_session(&state.db, id).await?)
    }

    /// Vuelve a pedir la contraseña del usuario de la sesión (p. ej. antes de
    /// revelar un secreto). Mismo freno que el login: los fallos cuentan para
    /// la IP y para el usuario, y durante la espera se rechaza sin comprobar.
    pub async fn reauthenticate(
        state: &AppState,
        ip: IpAddr,
        user: &User,
        password: &str,
    ) -> Result<(), LoginError> {
        let _turn = state.login_lock.lock().await;
        check_throttled(state, ip, Account::Id(user.id, &user.username), password)
            .await
            .map(drop)
    }

    /// Cuenta cuya contraseña se comprueba.
    enum Account<'a> {
        /// Por nombre (login): puede no existir.
        Name(&'a str),
        /// Usuario de la sesión: id y nombre.
        Id(i64, &'a str),
    }

    /// Comprueba la contraseña con el freno de fuerza bruta; devuelve el id
    /// del usuario. Hay que llamarla con `state.login_lock` tomado.
    async fn check_throttled(
        state: &AppState,
        ip: IpAddr,
        account: Account<'_>,
        password: &str,
    ) -> Result<i64, LoginError> {
        let db = &state.db;
        let ip_key = ip.to_string();
        let username = match account {
            Account::Name(n) | Account::Id(_, n) => n,
        };
        let user_key: String = username.chars().take(MAX_USERNAME_CHARS).collect();

        let wait = wait_ms(db, &ip_key, &user_key).await?;
        if wait > 0 {
            return Err(LoginError::Throttled((wait + 999) / 1000));
        }

        if password.len() > MAX_LOGIN_PASSWORD_BYTES {
            record_failure(db, &ip_key, &user_key).await?;
            return Err(LoginError::BadCredentials);
        }

        let row: Option<(i64, String)> = match account {
            Account::Name(n) => {
                sqlx::query_as("SELECT id, password_hash FROM users WHERE username = ?")
                    .bind(n)
                    .fetch_optional(db)
                    .await?
            }
            Account::Id(id, _) => {
                sqlx::query_as("SELECT id, password_hash FROM users WHERE id = ?")
                    .bind(id)
                    .fetch_optional(db)
                    .await?
            }
        };
        let (user_id, hash) = match row {
            Some((id, hash)) => (Some(id), hash),
            None => (None, DUMMY_HASH.clone()),
        };
        let password = password.to_owned();
        let ok = tokio::task::spawn_blocking(move || verify(&password, &hash))
            .await
            .map_err(|e| LoginError::Internal(e.to_string()))?;

        match user_id {
            Some(id) if ok => {
                clear_failures(db, &ip_key, &user_key).await?;
                Ok(id)
            }
            _ => {
                record_failure(db, &ip_key, &user_key).await?;
                Err(LoginError::BadCredentials)
            }
        }
    }

    fn app_state() -> Result<AppState, ServerFnError> {
        use_context::<AppState>().ok_or_else(|| ServerFnError::new("falta el estado del panel"))
    }

    /// Fija el estado HTTP de la respuesta de la server function y devuelve el
    /// error con `msg`.
    fn fail<T>(status: StatusCode, msg: &str) -> Result<T, ServerFnError> {
        if let Some(resp) = use_context::<ResponseOptions>() {
            resp.set_status(status);
        }
        Err(ServerFnError::new(msg))
    }

    fn internal<T>(e: impl fmt::Display) -> Result<T, ServerFnError> {
        eprintln!("panel: error interno en la autenticación: {e}");
        fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            &crate::t!("comun-error-interno"),
        )
    }

    /// Pone en la respuesta la cookie de la sesión `id` (de
    /// [`attempt_login`]).
    pub fn poner_cookie_de_sesion(id: &str) {
        set_cookie(&session_cookie(id));
    }

    fn set_cookie(value: &str) {
        if let (Some(resp), Ok(value)) = (
            use_context::<ResponseOptions>(),
            HeaderValue::from_str(value),
        ) {
            resp.insert_header(header::SET_COOKIE, value);
        }
    }

    pub(super) async fn login_endpoint(
        usuario: String,
        contrasena: String,
    ) -> Result<(), ServerFnError> {
        let state = app_state()?;
        let ConnectInfo(peer) = leptos_axum::extract::<ConnectInfo<SocketAddr>>().await?;
        let headers: HeaderMap = leptos_axum::extract().await?;
        let ip = client_ip(peer.ip(), &headers, state.cfg.trusted_proxy);
        match attempt_login(&state, ip, &usuario, &contrasena).await {
            Ok(id) => {
                if let Err(e) = delete_cookie_session(&state.db, &headers).await {
                    return internal(e);
                }
                set_cookie(&session_cookie(&id));
                Ok(())
            }
            Err(LoginError::Throttled(secs)) => fail(
                StatusCode::TOO_MANY_REQUESTS,
                &crate::t!("login-error-intentos", s = secs),
            ),
            Err(LoginError::BadCredentials) => fail(
                StatusCode::UNAUTHORIZED,
                &crate::t!("login-error-credenciales"),
            ),
            Err(LoginError::Internal(e)) => internal(e),
        }
    }

    /// Borra la sesión de la cookie que trae la petición, si la hay.
    async fn delete_cookie_session(
        db: &SqlitePool,
        headers: &HeaderMap,
    ) -> Result<(), sqlx::Error> {
        if let Some(id) = cookie_session_id(headers) {
            sqlx::query("DELETE FROM sessions WHERE id = ?")
                .bind(session_key(id))
                .execute(db)
                .await?;
        }
        Ok(())
    }

    pub(super) async fn logout_endpoint() -> Result<(), ServerFnError> {
        let state = app_state()?;
        let headers: HeaderMap = leptos_axum::extract().await?;
        set_cookie(&expired_cookie());
        if let Err(e) = delete_cookie_session(&state.db, &headers).await {
            return internal(e);
        }
        Ok(())
    }

    /// Usuario de la sesión de la petición en curso. Para usar al principio de
    /// toda server function salvo `login`: sin sesión vigente responde 401.
    pub async fn require_user() -> Result<User, ServerFnError> {
        let state = app_state()?;
        let headers: HeaderMap = leptos_axum::extract().await?;
        match user_from_headers(&state.db, &headers).await {
            Ok(Some(user)) => Ok(user),
            Ok(None) => fail(StatusCode::UNAUTHORIZED, super::ERROR_SESION),
            Err(e) => internal(e),
        }
    }

    // ---- Páginas SSR ----

    /// Extractor para rutas SSR: el usuario de la sesión, o una redirección a
    /// `/login?next=<ruta pedida>` si no hay sesión vigente.
    #[derive(Clone, Debug)]
    pub struct AuthUser(pub User);

    impl FromRequestParts<AppState> for AuthUser {
        type Rejection = Response;

        async fn from_request_parts(
            parts: &mut Parts,
            state: &AppState,
        ) -> Result<Self, Self::Rejection> {
            match user_from_headers(&state.db, &parts.headers).await {
                Ok(Some(user)) => Ok(AuthUser(user)),
                Ok(None) => {
                    let next = parts.uri.path_and_query().map_or("/", |pq| pq.as_str());
                    let next: String = form_urlencoded::byte_serialize(next.as_bytes()).collect();
                    Err(Redirect::to(&format!("/login?next={next}")).into_response())
                }
                Err(e) => {
                    eprintln!("panel: error interno en la autenticación: {e}");
                    Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
                }
            }
        }
    }

    /// Middleware de las rutas de páginas: exige sesión salvo en `/login` y en
    /// las server functions (`/api/*`), que se protegen con [`require_user`].
    ///
    /// Sin usuarios (panel recién instalado), toda página salvo `/instalar`
    /// redirige a `/instalar`. Con usuarios, `/instalar` solo existe mientras
    /// la instalación sigue en curso (y con sesión); si no, 404.
    pub async fn require_page_session(
        State(state): State<AppState>,
        req: Request,
        next: Next,
    ) -> Response {
        let path = req.uri().path();
        if path.starts_with("/api/") {
            return next.run(req).await;
        }
        let instalar = path == "/instalar";
        match crate::instalar::hay_usuarios(&state).await {
            Ok(false) if instalar => return next.run(req).await,
            Ok(false) => return Redirect::to("/instalar").into_response(),
            Ok(true) if instalar && !crate::instalar::en_curso(&state).await => {
                return StatusCode::NOT_FOUND.into_response();
            }
            Ok(true) => {}
            Err(e) => {
                eprintln!("panel: error interno en la autenticación: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
        if path == "/login" {
            return next.run(req).await;
        }
        let (mut parts, body) = req.into_parts();
        match AuthUser::from_request_parts(&mut parts, &state).await {
            Ok(user) => {
                parts.extensions.insert(user);
                next.run(Request::from_parts(parts, body)).await
            }
            Err(redirect) => redirect,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn retardo_exponencial_con_tope() {
            let esperados = [(0, 0), (1, 1), (2, 2), (3, 4), (4, 8), (5, 16), (6, 32)];
            for (fallos, secs) in esperados {
                assert_eq!(delay_secs(fallos), secs, "{fallos}");
            }
            for fallos in [7, 8, 100, i64::MAX] {
                assert_eq!(delay_secs(fallos), 60, "{fallos}");
            }
        }

        fn xff(values: &[&'static str]) -> HeaderMap {
            let mut h = HeaderMap::new();
            for v in values {
                h.append("x-forwarded-for", v.parse().unwrap());
            }
            h
        }

        #[test]
        fn ip_del_cliente() {
            let proxy: IpAddr = "192.0.2.11".parse().unwrap();
            let otro: IpAddr = "192.168.1.69".parse().unwrap();
            let ip = |s: &str| s.parse::<IpAddr>().unwrap();
            // Fuera del proxy, X-Forwarded-For no cuenta.
            assert_eq!(client_ip(otro, &xff(&["1.2.3.4"]), proxy), otro);
            // Desde el proxy, el último valor.
            assert_eq!(
                client_ip(proxy, &xff(&["9.9.9.9, 1.2.3.4"]), proxy),
                ip("1.2.3.4")
            );
            assert_eq!(
                client_ip(proxy, &xff(&["9.9.9.9", "5.6.7.8"]), proxy),
                ip("5.6.7.8")
            );
            // Sin cabecera o con basura, la del proxy.
            assert_eq!(client_ip(proxy, &xff(&[]), proxy), proxy);
            assert_eq!(client_ip(proxy, &xff(&["1.2.3.4, nada"]), proxy), proxy);
            // IPv4 mapeada en IPv6 se compara como IPv4.
            assert_eq!(
                client_ip(ip("::ffff:192.0.2.11"), &xff(&["1.2.3.4"]), proxy),
                ip("1.2.3.4")
            );
        }

        #[test]
        fn hash_y_verificacion() {
            let h = hash_password("una contraseña larga");
            assert!(h.starts_with("$argon2id$"), "{h}");
            assert!(verify("una contraseña larga", &h));
            assert!(!verify("otra contraseña larga", &h));
            assert!(!verify("una contraseña larga", "no es un hash"));
            assert_ne!(h, hash_password("una contraseña larga"), "sal aleatoria");
        }

        #[test]
        fn cookie_de_sesion() {
            let mut h = HeaderMap::new();
            h.append(
                "cookie",
                "a=1; __Host-rp_session=abc ; b=2".parse().unwrap(),
            );
            assert_eq!(cookie_session_id(&h), Some("abc"));
            let mut h = HeaderMap::new();
            h.append("cookie", "x=1".parse().unwrap());
            h.append("cookie", "__Host-rp_session=def".parse().unwrap());
            assert_eq!(cookie_session_id(&h), Some("def"));
            let mut h = HeaderMap::new();
            h.append("cookie", "__Host-rp_session=".parse().unwrap());
            assert_eq!(cookie_session_id(&h), None);
        }

        #[test]
        fn ids_de_sesion_distintos_y_hash_distinto_del_id() {
            let a = new_session_id();
            assert_eq!(a.len(), 43);
            assert_ne!(a, new_session_id());
            assert_ne!(session_key(&a), a);
            assert_eq!(session_key(&a), session_key(&a));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::safe_next;

    #[test]
    fn next_solo_rutas_locales() {
        assert_eq!(safe_next(Some("/")), "/");
        assert_eq!(safe_next(Some("/wifi?x=1")), "/wifi?x=1");
        for malo in [
            "",
            "wifi",
            "//evil.example",
            "/\\evil.example",
            "https://evil.example/",
            "/a\nb",
        ] {
            assert_eq!(safe_next(Some(malo)), "/", "{malo:?}");
        }
        assert_eq!(safe_next(None), "/");
    }
}
