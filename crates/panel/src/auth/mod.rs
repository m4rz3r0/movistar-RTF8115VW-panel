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
    client_ip, create_user, delay_secs, delete_user, guardar_modo_vista, guardar_tema,
    hash_password, poner_cookie_de_sesion, reautenticar_sesion, reauthenticate,
    require_page_session, require_reauth, require_user, set_password, verify, warm_up,
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
mod server;

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
