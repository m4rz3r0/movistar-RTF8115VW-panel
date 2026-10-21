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
    AuthUser, LoginError, MAX_PASSWORD_BYTES, MIN_PASSWORD_CHARS, REAUTH_SECS, SESSION_COOKIE,
    User, UserError, abrir_ventana_de_reautenticacion, attempt_login, change_own_password,
    client_ip, create_user, delay_secs, delete_user, guardar_modo_vista, hash_password,
    poner_cookie_de_sesion, reautenticar_sesion, reauthenticate, require_page_session,
    require_reauth, require_user, set_password, verify, warm_up,
};

/// Mensaje de error de las server functions sin sesión vigente (con estado
/// 401). Es un código fijo, no un texto: la interfaz lo reconoce y lo
/// traduce en el idioma de la página (ver `ui::texto_error`).
pub const ERROR_SESION: &str = "sesion-caducada";

/// Mensaje de error (con estado 401) de las server functions que exigen
/// haber vuelto a escribir la contraseña del panel hace poco (ver
/// `reautenticar_sesion`). Código fijo: la interfaz pide la contraseña y repite.
pub const ERROR_REAUTENTICACION: &str = "reautenticacion-necesaria";

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
///
/// Sin JavaScript (el formulario enviado antes de hidratar o con el wasm
/// roto, que piden `text/html`), redirige: si entra, a `next` (solo rutas
/// locales, [`safe_next`]); si no, de vuelta a la página del formulario
/// (con su `?next=`) y el error en la dirección, que la página muestra.
#[server(endpoint = "login")]
pub async fn login(
    usuario: String,
    contrasena: String,
    #[server(default)] next: Option<String>,
) -> Result<(), ServerFnError> {
    server::login_endpoint(usuario, contrasena, next).await
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
    use crate::{redes::RedIp, state::AppState, vista::ModoVista};
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
    /// Longitud máxima de la contraseña, en bytes: no se admite al darla de
    /// alta o cambiarla, y en el login más larga es un fallo normal, sin
    /// gastar Argon2.
    pub const MAX_PASSWORD_BYTES: usize = 1024;
    /// Longitud máxima del nombre de usuario, en caracteres.
    const MAX_USERNAME_CHARS: usize = 64;
    /// Caducidad por inactividad.
    const SESSION_IDLE_SECS: i64 = 30 * 60;
    /// Caducidad absoluta.
    const SESSION_MAX_SECS: i64 = 12 * 60 * 60;
    /// Ventana de reautenticación: tras volver a escribir la contraseña, las
    /// operaciones delicadas no la piden de nuevo durante este tiempo.
    pub const REAUTH_SECS: i64 = 5 * 60;
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
        /// Su modo de visualización (sin elegir, el de por defecto).
        pub modo_vista: ModoVista,
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
        /// Más de [`MAX_PASSWORD_BYTES`] bytes: el login no la aceptaría.
        PasswordTooLong,
        Exists,
        NotFound,
        /// No se borra el último usuario.
        LastUser,
        /// Nadie se borra a sí mismo.
        OwnUser,
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
                UserError::PasswordTooLong => write!(
                    f,
                    "la contraseña puede tener como mucho {MAX_PASSWORD_BYTES} bytes"
                ),
                UserError::Exists => write!(f, "el usuario ya existe"),
                UserError::NotFound => write!(f, "el usuario no existe"),
                UserError::LastUser => write!(f, "no se puede borrar el último usuario"),
                UserError::OwnUser => write!(f, "no se puede borrar el usuario propio"),
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
        if password.len() > MAX_PASSWORD_BYTES {
            return Err(UserError::PasswordTooLong);
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
        change_password(db, username, password, false)
            .await
            .map(drop)
    }

    /// Cambia la contraseña del propio usuario `user` (ya autenticado), cierra
    /// todas sus sesiones y abre otra nueva, todo en una transacción y sin
    /// pasar por el freno del login. Devuelve el id de la sesión nueva (para
    /// [`poner_cookie_de_sesion`]).
    pub async fn change_own_password(
        db: &SqlitePool,
        user: &User,
        password: &str,
    ) -> Result<String, UserError> {
        change_password(db, &user.username, password, true)
            .await
            .map(|id| id.expect("se pidió una sesión nueva"))
    }

    async fn change_password(
        db: &SqlitePool,
        username: &str,
        password: &str,
        new_session: bool,
    ) -> Result<Option<String>, UserError> {
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
        let session = if new_session {
            let sid = new_session_id();
            let now = now_secs();
            sqlx::query(
                "INSERT INTO sessions (id, user_id, created, last_seen) VALUES (?, ?, ?, ?)",
            )
            .bind(session_key(&sid))
            .bind(id)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;
            Some(sid)
        } else {
            None
        };
        tx.commit().await?;
        Ok(session)
    }

    /// Borra `username` (y con él sus sesiones), salvo que sea `actual` (el
    /// usuario que lo pide) o el último que queda. Una sola sentencia: dos
    /// borrados a la vez no dejan el panel sin usuarios.
    pub async fn delete_user(
        db: &SqlitePool,
        username: &str,
        actual: i64,
    ) -> Result<(), UserError> {
        let borrado: Option<i64> = sqlx::query_scalar(
            "DELETE FROM users
             WHERE username = ? AND id != ? AND (SELECT COUNT(*) FROM users) > 1
             RETURNING id",
        )
        .bind(username)
        .bind(actual)
        .fetch_optional(db)
        .await?;
        if borrado.is_some() {
            return Ok(());
        }
        let id: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(db)
            .await?;
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(db)
            .await?;
        Err(match id {
            None => UserError::NotFound,
            Some(_) if total <= 1 => UserError::LastUser,
            Some(_) => UserError::OwnUser,
        })
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
        let row: Option<(i64, String, Option<String>, i64, i64)> = sqlx::query_as(
            "SELECT u.id, u.username, u.modo_vista, s.created, s.last_seen
             FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.id = ?",
        )
        .bind(&key)
        .fetch_optional(db)
        .await?;
        let Some((user_id, username, modo_vista, created, last_seen)) = row else {
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
            modo_vista: modo_vista
                .as_deref()
                .and_then(ModoVista::desde_codigo)
                .unwrap_or_default(),
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

    /// IP del cliente: la del socket, salvo que el socket sea uno de los
    /// proxies de confianza (`proxies`). Entonces se recorre
    /// `X-Forwarded-For` de derecha a izquierda (del proxy más cercano hacia
    /// el cliente) saltando las direcciones de confianza: la primera que no
    /// lo es, es el cliente. Lo que queda a su izquierda lo escribió el
    /// cliente y no cuenta. Si se llega a un valor que no es una IP, o todos
    /// son de confianza, se usa la última dirección de confianza vista (la
    /// más a la izquierda; el propio socket si la cabecera falta).
    pub fn client_ip(peer: IpAddr, headers: &HeaderMap, proxies: &[RedIp]) -> IpAddr {
        let de_confianza = |ip: IpAddr| proxies.iter().any(|red| red.contiene(ip));
        let mut ultima = peer.to_canonical();
        if !de_confianza(ultima) {
            return ultima;
        }
        let valores: Vec<&str> = headers
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(','))
            .collect();
        for valor in valores.into_iter().rev() {
            let Ok(ip) = valor.trim().parse::<IpAddr>() else {
                return ultima;
            };
            let ip = ip.to_canonical();
            if !de_confianza(ip) {
                return ip;
            }
            ultima = ip;
        }
        ultima
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
    /// cambiar la contraseña del router o exportar). Mismo freno que el
    /// login: los fallos cuentan para la IP y para el usuario, y durante la
    /// espera se rechaza sin comprobar.
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

        if password.len() > MAX_PASSWORD_BYTES {
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
        internal_con(StatusCode::INTERNAL_SERVER_ERROR, e)
    }

    /// Como [`internal`], con otro estado (el login sin JavaScript redirige
    /// también sus errores).
    fn internal_con<T>(status: StatusCode, e: impl fmt::Display) -> Result<T, ServerFnError> {
        eprintln!("panel: error interno en la autenticación: {e}");
        fail(status, &crate::t!("comun-error-interno"))
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

    /// ¿Lo envió un formulario HTML sin JavaScript? El navegador pide
    /// `text/html`; el cliente de las server functions, no (el mismo
    /// criterio que `server_fn` para redirigir).
    fn sin_javascript(headers: &HeaderMap) -> bool {
        headers
            .get(header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("text/html"))
    }

    pub(super) async fn login_endpoint(
        usuario: String,
        contrasena: String,
        next: Option<String>,
    ) -> Result<(), ServerFnError> {
        let state = app_state()?;
        let ConnectInfo(peer) = leptos_axum::extract::<ConnectInfo<SocketAddr>>().await?;
        let headers: HeaderMap = leptos_axum::extract().await?;
        let ip = client_ip(peer.ip(), &headers, &state.cfg.trusted_proxies);
        let sin_js = sin_javascript(&headers);
        // Sin JavaScript, un error vuelve a la página del formulario con una
        // redirección (`server_fn` pone la dirección: la de `Referer`, con
        // el error); un 4xx dejaría al navegador en una página en blanco.
        let estado = |s: StatusCode| if sin_js { StatusCode::SEE_OTHER } else { s };
        match attempt_login(&state, ip, &usuario, &contrasena).await {
            Ok(id) => {
                if let Err(e) = delete_cookie_session(&state.db, &headers).await {
                    return internal_con(estado(StatusCode::INTERNAL_SERVER_ERROR), e);
                }
                set_cookie(&session_cookie(&id));
                if sin_js {
                    leptos_axum::redirect(super::safe_next(next.as_deref()));
                }
                Ok(())
            }
            Err(LoginError::Throttled(secs)) => fail(
                estado(StatusCode::TOO_MANY_REQUESTS),
                &crate::t!("login-error-intentos", s = secs),
            ),
            Err(LoginError::BadCredentials) => fail(
                estado(StatusCode::UNAUTHORIZED),
                &crate::t!("login-error-credenciales"),
            ),
            Err(LoginError::Internal(e)) => {
                internal_con(estado(StatusCode::INTERNAL_SERVER_ERROR), e)
            }
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

    /// Guarda el modo de visualización del usuario de la sesión (sin sesión,
    /// 401). El de por defecto se guarda como `NULL`.
    pub async fn guardar_modo_vista(modo: ModoVista) -> Result<(), ServerFnError> {
        let state = app_state()?;
        let user = require_user().await?;
        let valor = (modo != ModoVista::default()).then_some(modo.codigo());
        let res = sqlx::query("UPDATE users SET modo_vista = ? WHERE id = ?")
            .bind(valor)
            .bind(user.id)
            .execute(&state.db)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => internal(e),
        }
    }

    /// Vuelve a pedir la contraseña al usuario de la sesión (con el freno
    /// del login, por la IP del cliente) y, si es buena, abre la ventana de
    /// reautenticación de esta sesión durante [`REAUTH_SECS`].
    pub async fn reautenticar_sesion(contrasena: &str) -> Result<(), ServerFnError> {
        let state = app_state()?;
        let user = require_user().await?;
        let ConnectInfo(peer) = leptos_axum::extract::<ConnectInfo<SocketAddr>>().await?;
        let headers: HeaderMap = leptos_axum::extract().await?;
        let ip = client_ip(peer.ip(), &headers, &state.cfg.trusted_proxies);
        match reauthenticate(&state, ip, &user, contrasena).await {
            Ok(()) => {}
            Err(LoginError::Throttled(secs)) => {
                return fail(
                    StatusCode::TOO_MANY_REQUESTS,
                    &crate::t!("login-error-intentos", s = secs),
                );
            }
            Err(LoginError::BadCredentials) => {
                return fail(StatusCode::FORBIDDEN, &crate::t!("panel-error-contrasena"));
            }
            Err(LoginError::Internal(e)) => return internal(e),
        }
        abrir_ventana_de_reautenticacion().await?;
        tracing::info!(usuario = %user.username, "reautenticación en los ajustes");
        Ok(())
    }

    /// Abre la ventana de reautenticación de la sesión de la petición durante
    /// [`REAUTH_SECS`]. Solo tras comprobar la contraseña
    /// ([`reauthenticate`]).
    pub async fn abrir_ventana_de_reautenticacion() -> Result<(), ServerFnError> {
        let state = app_state()?;
        let headers: HeaderMap = leptos_axum::extract().await?;
        let Some(id) = cookie_session_id(&headers) else {
            return fail(StatusCode::UNAUTHORIZED, super::ERROR_SESION);
        };
        if let Err(e) = sqlx::query("UPDATE sessions SET reautenticado_hasta = ? WHERE id = ?")
            .bind(now_secs() + REAUTH_SECS)
            .bind(session_key(id))
            .execute(&state.db)
            .await
        {
            return internal(e);
        }
        Ok(())
    }

    /// Como [`require_user`], y además la sesión tiene abierta la ventana de
    /// reautenticación ([`reautenticar_sesion`]); si no, 401 con
    /// [`super::ERROR_REAUTENTICACION`].
    pub async fn require_reauth() -> Result<User, ServerFnError> {
        let state = app_state()?;
        let user = require_user().await?;
        let headers: HeaderMap = leptos_axum::extract().await?;
        let hasta: Option<i64> = match cookie_session_id(&headers) {
            Some(id) => {
                match sqlx::query_scalar::<_, Option<i64>>(
                    "SELECT reautenticado_hasta FROM sessions WHERE id = ?",
                )
                .bind(session_key(id))
                .fetch_optional(&state.db)
                .await
                {
                    Ok(h) => h.flatten(),
                    Err(e) => return internal(e),
                }
            }
            None => None,
        };
        if hasta.is_some_and(|h| h > now_secs()) {
            Ok(user)
        } else {
            fail(StatusCode::UNAUTHORIZED, super::ERROR_REAUTENTICACION)
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
            // Con una sesión vigente, el formulario no hace falta: a `next`
            // (como al entrar).
            return match user_from_headers(&state.db, req.headers()).await {
                Ok(Some(_)) => {
                    let destino = req.uri().query().and_then(|q| {
                        form_urlencoded::parse(q.as_bytes())
                            .find(|(k, _)| k == "next")
                            .map(|(_, v)| v.into_owned())
                    });
                    Redirect::to(super::safe_next(destino.as_deref())).into_response()
                }
                Ok(None) => next.run(req).await,
                Err(e) => {
                    eprintln!("panel: error interno en la autenticación: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            };
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
            let proxy: IpAddr = "192.0.2.100".parse().unwrap();
            let proxies = ["192.0.2.100".parse::<RedIp>().unwrap()];
            let proxies = &proxies[..];
            let otro: IpAddr = "192.0.2.101".parse().unwrap();
            let ip = |s: &str| s.parse::<IpAddr>().unwrap();
            // Fuera del proxy, X-Forwarded-For no cuenta.
            assert_eq!(client_ip(otro, &xff(&["198.51.100.4"]), proxies), otro);
            // Desde el proxy, el último valor.
            assert_eq!(
                client_ip(proxy, &xff(&["9.9.9.9, 198.51.100.4"]), proxies),
                ip("198.51.100.4")
            );
            assert_eq!(
                client_ip(proxy, &xff(&["9.9.9.9", "198.51.100.8"]), proxies),
                ip("198.51.100.8")
            );
            // Sin cabecera o con basura, la del proxy.
            assert_eq!(client_ip(proxy, &xff(&[]), proxies), proxy);
            assert_eq!(
                client_ip(proxy, &xff(&["198.51.100.4, nada"]), proxies),
                proxy
            );
            // IPv4 mapeada en IPv6 se compara como IPv4.
            assert_eq!(
                client_ip(ip("::ffff:192.0.2.100"), &xff(&["198.51.100.4"]), proxies),
                ip("198.51.100.4")
            );
        }

        #[test]
        fn ip_del_cliente_tras_una_cadena_de_proxies() {
            let ip = |s: &str| s.parse::<IpAddr>().unwrap();
            // A (192.0.2.10) y B (192.0.2.20), de confianza: cliente → A → B.
            let proxies = [
                "192.0.2.10".parse::<RedIp>().unwrap(),
                "192.0.2.20".parse::<RedIp>().unwrap(),
            ];
            let proxies = &proxies[..];
            let b = ip("192.0.2.20");
            assert_eq!(
                client_ip(b, &xff(&["198.51.100.4, 192.0.2.10"]), proxies),
                ip("198.51.100.4")
            );
            assert_eq!(
                client_ip(b, &xff(&["198.51.100.4", "192.0.2.10"]), proxies),
                ip("198.51.100.4")
            );
            // Lo que el cliente pone a la izquierda no cuenta.
            assert_eq!(
                client_ip(
                    b,
                    &xff(&["203.0.113.66, 198.51.100.4, 192.0.2.10"]),
                    proxies
                ),
                ip("198.51.100.4")
            );
            // Un valor de confianza falsificado a la izquierda, tampoco.
            assert_eq!(
                client_ip(b, &xff(&["192.0.2.10, 198.51.100.4, 192.0.2.10"]), proxies),
                ip("198.51.100.4")
            );
            // Basura en medio: la última dirección de confianza vista.
            assert_eq!(
                client_ip(b, &xff(&["198.51.100.4, nada, 192.0.2.10"]), proxies),
                ip("192.0.2.10")
            );
            assert_eq!(client_ip(b, &xff(&["198.51.100.4, nada"]), proxies), b);
            // Todos de confianza: el de más a la izquierda.
            assert_eq!(
                client_ip(b, &xff(&["192.0.2.20, 192.0.2.10"]), proxies),
                ip("192.0.2.20")
            );
            // IPv4 mapeada, tanto en el socket como en la cabecera.
            assert_eq!(
                client_ip(
                    ip("::ffff:192.0.2.20"),
                    &xff(&["::ffff:198.51.100.4, ::ffff:192.0.2.10"]),
                    proxies
                ),
                ip("198.51.100.4")
            );
            // Un socket que no es de confianza manda, aunque traiga la cadena.
            assert_eq!(
                client_ip(
                    ip("203.0.113.9"),
                    &xff(&["198.51.100.4, 192.0.2.10"]),
                    proxies
                ),
                ip("203.0.113.9")
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
