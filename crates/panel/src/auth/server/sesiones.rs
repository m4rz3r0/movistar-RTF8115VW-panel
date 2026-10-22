//! Sesiones en SQLite y su cookie.

use super::*;

/// Id de sesión nuevo: 32 bytes del generador del sistema en base64url.
pub(super) fn new_session_id() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("generador aleatorio del sistema");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Clave con la que se guarda la sesión: SHA-256 del id. La base de datos
/// nunca contiene el id que viaja en la cookie.
pub(super) fn session_key(id: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(id.as_bytes()))
}

pub(super) fn session_cookie(id: &str) -> String {
    format!(
        "{SESSION_COOKIE}={id}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={SESSION_MAX_SECS}"
    )
}

pub(super) fn expired_cookie() -> String {
    format!("{SESSION_COOKIE}=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0")
}

/// Id de sesión de la cabecera `Cookie`, si la hay.
pub(super) fn cookie_session_id(headers: &HeaderMap) -> Option<&str> {
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
pub(super) async fn purge_expired(db: &SqlitePool) -> Result<(), sqlx::Error> {
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

pub(super) async fn create_session(db: &SqlitePool, user_id: i64) -> Result<String, sqlx::Error> {
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

/// Fila de una sesión con su usuario: id, nombre, modo de visualización,
/// tema, creada y vista por última vez.
type FilaSesion = (i64, String, Option<String>, Option<String>, i64, i64);

/// Usuario de la sesión `id` si sigue vigente. Una sesión caducada se
/// borra; una vigente renueva `last_seen` (como mucho una vez por minuto).
pub(super) async fn session_user(db: &SqlitePool, id: &str) -> Result<Option<User>, sqlx::Error> {
    let key = session_key(id);
    let row: Option<FilaSesion> = sqlx::query_as(
        "SELECT u.id, u.username, u.modo_vista, u.tema, s.created, s.last_seen
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.id = ?",
    )
    .bind(&key)
    .fetch_optional(db)
    .await?;
    let Some((user_id, username, modo_vista, tema, created, last_seen)) = row else {
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
        tema: tema
            .as_deref()
            .and_then(TemaUi::desde_codigo)
            .unwrap_or_default(),
    }))
}

pub(super) async fn user_from_headers(
    db: &SqlitePool,
    headers: &HeaderMap,
) -> Result<Option<User>, sqlx::Error> {
    match cookie_session_id(headers) {
        Some(id) => session_user(db, id).await,
        None => Ok(None),
    }
}
