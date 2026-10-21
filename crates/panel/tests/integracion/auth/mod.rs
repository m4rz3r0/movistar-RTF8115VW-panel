//! Pruebas de autenticación: login, sesiones, freno de fuerza bruta, CSRF y
//! órdenes `panel user …`.

use crate::comun::{
    PANEL_PASS, PANEL_USER, Panel, PanelPrueba, body_text, get, get_con_cookie, login_body,
    post_form, send, sesion, set_cookie,
};
use axum::{body::Body, http::Request};
use std::{
    io::Write,
    process::{Command, Stdio},
};

mod freno;
mod login;
mod sesiones;
mod sin_js;

const COOKIE_NAME: &str = "__Host-rp_session";

async fn crear_usuario(p: &Panel) {
    panel::auth::create_user(&p.state.db, PANEL_USER, PANEL_PASS)
        .await
        .unwrap();
}

/// Fallos registrados para `(kind, key)` en `login_attempts`.
async fn fallos(p: &Panel, kind: &str, key: &str) -> Option<i64> {
    sqlx::query_scalar("SELECT failures FROM login_attempts WHERE kind = ? AND key = ?")
        .bind(kind)
        .bind(key)
        .fetch_optional(&p.state.db)
        .await
        .unwrap()
}

/// Adelanta el reloj del freno: los fallos pasan a ser de hace `secs` s más.
async fn retrasar_fallos(p: &Panel, secs: i64) {
    sqlx::query("UPDATE login_attempts SET last_failure_ms = last_failure_ms - ?")
        .bind(secs * 1000)
        .execute(&p.state.db)
        .await
        .unwrap();
}

async fn login_desde(p: &Panel, usuario: &str, contrasena: &str, peer: &str) -> (u16, String) {
    let resp = send(
        &p.app,
        post_form("/api/login", &login_body(usuario, contrasena), peer),
    )
    .await;
    let status = resp.status().as_u16();
    (status, body_text(resp).await)
}
