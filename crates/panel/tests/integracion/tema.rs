//! Tema de la interfaz (automático, oscuro, claro): preferencia por usuario
//! y `<html data-tema>` pintado por el servidor, sin esperar a JavaScript.

use crate::comun::{PanelPrueba, body_text, get_con_cookie, llamar, ok, sesion};
use serde_json::json;

#[tokio::test]
async fn el_tema_llega_al_html() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    // Sin elegir: automático, y el navegador decide con su preferencia.
    let html = body_text(get_con_cookie(&p.app, "/", &cookie).await).await;
    assert!(html.contains(r#"data-tema="automatico""#), "{html}");
    assert!(
        html.contains(r#"<meta name="color-scheme" content="dark light""#),
        "{html}"
    );
    ok(&p, "guardar_tema_ui", json!({ "tema": "claro" }), &cookie).await;
    for ruta in ["/", "/panel", "/ajustes/wifi"] {
        let html = body_text(get_con_cookie(&p.app, ruta, &cookie).await).await;
        assert!(html.contains(r#"data-tema="claro""#), "{ruta}: {html}");
    }
    // La pantalla de entrada, sin sesión: automático.
    let html = body_text(get_con_cookie(&p.app, "/login", "").await).await;
    assert!(html.contains(r#"data-tema="automatico""#), "{html}");
}

#[tokio::test]
async fn preferencia_de_tema_se_guarda() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    assert_eq!(
        ok(&p, "leer_tema_ui", json!({}), &cookie).await,
        json!("automatico")
    );
    ok(&p, "guardar_tema_ui", json!({ "tema": "oscuro" }), &cookie).await;
    assert_eq!(
        ok(&p, "leer_tema_ui", json!({}), &cookie).await,
        json!("oscuro")
    );
    // Se conserva en una sesión nueva del mismo usuario.
    let nueva = crate::comun::sesion_de(&p).await;
    assert_eq!(
        ok(&p, "leer_tema_ui", json!({}), &nueva).await,
        json!("oscuro")
    );
    // Un tema que no existe no se guarda.
    let (status, body) = llamar(
        &p,
        "guardar_tema_ui",
        json!({ "tema": "rosa" }),
        Some(&cookie),
    )
    .await;
    assert_ne!(status, 200, "{body}");
    assert_eq!(
        ok(&p, "leer_tema_ui", json!({}), &cookie).await,
        json!("oscuro")
    );
    // Volver a automático lo deja como al principio.
    ok(
        &p,
        "guardar_tema_ui",
        json!({ "tema": "automatico" }),
        &cookie,
    )
    .await;
    let html = body_text(get_con_cookie(&p.app, "/", &cookie).await).await;
    assert!(html.contains(r#"data-tema="automatico""#), "{html}");
}

#[tokio::test]
async fn guardar_tema_exige_sesion() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let (status, body) = llamar(&p, "guardar_tema_ui", json!({ "tema": "claro" }), None).await;
    assert_eq!(status, 401, "{body}");
    let (status, body) = llamar(&p, "leer_tema_ui", json!({}), None).await;
    assert_eq!(status, 401, "{body}");
}
