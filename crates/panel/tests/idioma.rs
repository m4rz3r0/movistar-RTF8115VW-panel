//! Idioma de la interfaz: cookie `rp_lang`, `Accept-Language` y español por
//! defecto.

mod common;

use axum::{body::Body, http::Request};
use common::{body_text, panel, post_form, send, sesion};

/// HTML de `/login` con las cabeceras dadas.
async fn login_con(p: &common::Panel, cabeceras: &[(&str, &str)]) -> String {
    let mut req = Request::get("/login");
    for (k, v) in cabeceras {
        req = req.header(*k, *v);
    }
    let resp = send(&p.app, req.body(Body::empty()).unwrap()).await;
    assert_eq!(resp.status(), 200);
    body_text(resp).await
}

#[tokio::test]
async fn idioma_desconocido_es_espanol() {
    let p = panel().await;
    let html = login_con(&p, &[("cookie", "rp_lang=xx"), ("accept-language", "fr")]).await;
    assert!(html.contains(r#"<html lang="es""#), "{html}");
    assert!(html.contains("Acceso al panel"), "{html}");
    // Sin nada, también español.
    let html = login_con(&p, &[]).await;
    assert!(html.contains(r#"<html lang="es""#), "{html}");
}

#[tokio::test]
async fn idioma_por_cookie_y_cabecera() {
    let p = panel().await;
    let html = login_con(&p, &[("cookie", "rp_lang=en")]).await;
    assert!(html.contains(r#"<html lang="en""#), "{html}");
    assert!(html.contains("Panel sign-in"), "{html}");
    assert!(!html.contains("Acceso al panel"), "{html}");

    let html = login_con(&p, &[("accept-language", "en-GB,en;q=0.9")]).await;
    assert!(html.contains(r#"<html lang="en""#), "{html}");
    assert!(html.contains("Panel sign-in"), "{html}");

    // La cookie manda sobre la cabecera.
    let html = login_con(
        &p,
        &[("cookie", "otra=1; rp_lang=es"), ("accept-language", "en")],
    )
    .await;
    assert!(html.contains(r#"<html lang="es""#), "{html}");
}

#[tokio::test]
async fn errores_del_servidor_en_el_idioma_de_la_peticion() {
    let p = panel().await;
    let _ = sesion(&p).await;
    let mut req = post_form("/api/login", "usuario=admin&contrasena=mala", "127.0.0.1");
    req.headers_mut()
        .insert("cookie", "rp_lang=en".parse().unwrap());
    let body = body_text(send(&p.app, req).await).await;
    assert!(body.contains("Wrong user name or password"), "{body}");
}

#[tokio::test]
async fn cambiar_idioma_fija_la_cookie() {
    let p = panel().await;
    let resp = send(
        &p.app,
        post_form("/api/cambiar_idioma", "idioma=En", "127.0.0.1"),
    )
    .await;
    assert_eq!(resp.status(), 200);
    let cookie = common::set_cookie(&resp);
    assert!(cookie.starts_with("rp_lang=en;"), "{cookie}");
    for parte in ["Path=/", "SameSite=Lax", "Secure", "Max-Age=31536000"] {
        assert!(cookie.contains(parte), "{cookie}");
    }
    // Un idioma que no existe no se acepta.
    let resp = send(
        &p.app,
        post_form("/api/cambiar_idioma", "idioma=Fr", "127.0.0.1"),
    )
    .await;
    assert_ne!(resp.status(), 200);
}
