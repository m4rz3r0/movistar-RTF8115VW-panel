//! Servidor de la autenticación (feature `ssr`): constantes, usuario y reloj; el
//! resto, por partes en los submódulos.

use crate::{redes::RedIp, state::AppState, tema::TemaUi, vista::ModoVista};
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
    /// Su tema de la interfaz (sin elegir, automático).
    pub tema: TemaUi,
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

mod contrasenas;
pub use contrasenas::*;
mod usuarios;
pub use usuarios::*;
mod sesiones;
use sesiones::*;
mod freno;
pub use freno::*;
mod login;
pub use login::*;
mod paginas;
pub use paginas::*;

#[cfg(test)]
mod tests;
