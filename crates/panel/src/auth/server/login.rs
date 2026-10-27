//! Login, logout, reautenticación y las comprobaciones de las server functions.

use super::*;

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
pub(super) enum Account<'a> {
    /// Por nombre (login): puede no existir.
    Name(&'a str),
    /// Usuario de la sesión: id y nombre.
    Id(i64, &'a str),
}

/// Comprueba la contraseña con el freno de fuerza bruta; devuelve el id
/// del usuario. Hay que llamarla con `state.login_lock` tomado.
pub(super) async fn check_throttled(
    state: &AppState,
    ip: IpAddr,
    account: Account<'_>,
    password: &str,
) -> Result<i64, LoginError> {
    let db = &state.db;
    let ip_key = clave_ip(ip);

    let wait = wait_ms(db, &ip_key).await?;
    if wait > 0 {
        return Err(LoginError::Throttled((wait + 999) / 1000));
    }

    if password.len() > MAX_PASSWORD_BYTES {
        record_failure(db, &ip_key).await?;
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
            clear_failures(db, &ip_key).await?;
            Ok(id)
        }
        _ => {
            record_failure(db, &ip_key).await?;
            Err(LoginError::BadCredentials)
        }
    }
}

pub(super) fn app_state() -> Result<AppState, ServerFnError> {
    use_context::<AppState>().ok_or_else(|| ServerFnError::new("falta el estado del panel"))
}

/// Fija el estado HTTP de la respuesta de la server function y devuelve el
/// error con `msg`.
pub(super) fn fail<T>(status: StatusCode, msg: &str) -> Result<T, ServerFnError> {
    if let Some(resp) = use_context::<ResponseOptions>() {
        resp.set_status(status);
    }
    Err(ServerFnError::new(msg))
}

pub(super) fn internal<T>(e: impl fmt::Display) -> Result<T, ServerFnError> {
    internal_con(StatusCode::INTERNAL_SERVER_ERROR, e)
}

/// Como [`internal`], con otro estado (el login sin JavaScript redirige
/// también sus errores).
fn internal_con<T>(status: StatusCode, e: impl fmt::Display) -> Result<T, ServerFnError> {
    tracing::error!("error interno en la autenticación: {e}");
    fail(status, &crate::t!("comun-error-interno"))
}

/// Pone en la respuesta la cookie de la sesión `id` (de
/// [`attempt_login`]).
pub fn poner_cookie_de_sesion(id: &str) {
    set_cookie(&session_cookie(id));
}

pub(super) fn set_cookie(value: &str) {
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
pub(super) fn sin_javascript(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("text/html"))
}

pub(in crate::auth) async fn login_endpoint(
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
                leptos_axum::redirect(super::super::safe_next(next.as_deref()));
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
        Err(LoginError::Internal(e)) => internal_con(estado(StatusCode::INTERNAL_SERVER_ERROR), e),
    }
}

/// Borra la sesión de la cookie que trae la petición, si la hay.
pub(super) async fn delete_cookie_session(
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

pub(in crate::auth) async fn logout_endpoint() -> Result<(), ServerFnError> {
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
        Ok(None) => fail(StatusCode::UNAUTHORIZED, super::super::ERROR_SESION),
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

/// Guarda el tema de la interfaz del usuario de la sesión (sin sesión, 401).
/// El de por defecto se guarda como `NULL`.
pub async fn guardar_tema(tema: TemaUi) -> Result<(), ServerFnError> {
    let state = app_state()?;
    let user = require_user().await?;
    let valor = (tema != TemaUi::default()).then_some(tema.codigo());
    let res = sqlx::query("UPDATE users SET tema = ? WHERE id = ?")
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
        return fail(StatusCode::UNAUTHORIZED, super::super::ERROR_SESION);
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
/// [`super::super::ERROR_REAUTENTICACION`].
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
        fail(
            StatusCode::UNAUTHORIZED,
            super::super::ERROR_REAUTENTICACION,
        )
    }
}
