//! Usuarios del panel: alta, contraseña y baja.

use super::*;

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

pub(super) fn check_username(username: &str) -> Result<(), UserError> {
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

pub(super) fn check_password(password: &str) -> Result<(), UserError> {
    if password.chars().count() < MIN_PASSWORD_CHARS {
        return Err(UserError::PasswordTooShort);
    }
    if password.len() > MAX_PASSWORD_BYTES {
        return Err(UserError::PasswordTooLong);
    }
    Ok(())
}

/// Da de alta `username` con la contraseña `password`.
pub async fn create_user(db: &SqlitePool, username: &str, password: &str) -> Result<(), UserError> {
    check_username(username)?;
    check_password(password)?;
    let hash = hash_blocking(password).await;
    let res = sqlx::query("INSERT INTO users (username, password_hash, created) VALUES (?, ?, ?)")
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

pub(super) async fn change_password(
    db: &SqlitePool,
    username: &str,
    password: &str,
    new_session: bool,
) -> Result<Option<String>, UserError> {
    check_password(password)?;
    let hash = hash_blocking(password).await;
    let mut tx = db.begin().await?;
    let id: Option<i64> =
        sqlx::query_scalar("UPDATE users SET password_hash = ? WHERE username = ? RETURNING id")
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
        sqlx::query("INSERT INTO sessions (id, user_id, created, last_seen) VALUES (?, ?, ?, ?)")
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
pub async fn delete_user(db: &SqlitePool, username: &str, actual: i64) -> Result<(), UserError> {
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
