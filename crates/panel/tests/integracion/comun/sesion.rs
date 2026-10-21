//! Usuario del panel, inicio de sesión y reautenticación.

use super::{PANEL_PASS, PANEL_USER, Panel, ok, post_form, send, set_cookie};
use serde_json::json;

/// Cuerpo del formulario de login.
pub fn login_body(usuario: &str, contrasena: &str) -> String {
    let enc = |s: &str| form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>();
    format!("usuario={}&contrasena={}", enc(usuario), enc(contrasena))
}

/// Crea el usuario del panel, inicia sesión desde 127.0.0.1 y devuelve la
/// cookie lista para la cabecera `Cookie`.
pub async fn sesion(p: &Panel) -> String {
    panel::auth::create_user(&p.state.db, PANEL_USER, PANEL_PASS)
        .await
        .unwrap();
    sesion_de(p).await
}

/// Panel ya instalado: con el usuario del panel (si no estaba). Sin
/// usuarios, toda página redirige al asistente de instalación.
pub async fn con_usuario(p: &Panel) {
    match panel::auth::create_user(&p.state.db, PANEL_USER, PANEL_PASS).await {
        Ok(()) | Err(panel::auth::UserError::Exists) => {}
        Err(e) => panic!("{e}"),
    }
}

/// Inicia sesión desde 127.0.0.1 con el usuario del panel (ya creado) y
/// devuelve la cookie lista para la cabecera `Cookie`.
pub async fn sesion_de(p: &Panel) -> String {
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body(PANEL_USER, PANEL_PASS),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 200);
    set_cookie(&resp)
        .split(';')
        .next()
        .unwrap()
        .trim()
        .to_owned()
}

/// Reautentica la sesión `cookie` con la contraseña del panel.
pub async fn reautenticar(p: &Panel, cookie: &str) {
    ok(
        p,
        "reautenticar",
        json!({ "contrasena": PANEL_PASS }),
        cookie,
    )
    .await;
}
