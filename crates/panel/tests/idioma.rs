//! Idioma de la interfaz: cookie `rp_lang`, idioma por defecto de la
//! instancia, `Accept-Language` y español.

mod common;

use axum::{body::Body, http::Request};
use common::{body_text, ok, panel, post_form, send, sesion};
use serde_json::json;

/// HTML de `/login` con las cabeceras dadas.
async fn login_con(p: &common::Panel, cabeceras: &[(&str, &str)]) -> String {
    common::con_usuario(p).await;
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
    assert!(body.contains("Wrong username or password"), "{body}");
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

#[tokio::test]
async fn login_con_selector_de_idioma() {
    let p = panel().await;
    let html = login_con(&p, &[("accept-language", "en")]).await;
    assert!(html.contains("idioma-selector"), "{html}");
    // Cada idioma con su nombre y su `lang`.
    assert!(
        html.contains(r#"lang="es""#) && html.contains("Español"),
        "{html}"
    );
}

/// `<html lang>` de `/login` sin cookie ni cabeceras, más con
/// `Accept-Language: es` y con la cookie `rp_lang=es`.
async fn idiomas_de_login(p: &common::Panel) -> [bool; 3] {
    let en = |html: String| html.contains(r#"<html lang="en""#);
    [
        en(login_con(p, &[]).await),
        en(login_con(p, &[("accept-language", "es-ES,es;q=0.9")]).await),
        en(login_con(p, &[("cookie", "rp_lang=es"), ("accept-language", "en")]).await),
    ]
}

#[tokio::test]
async fn idioma_por_defecto_de_la_instancia() {
    let mut p = panel().await;
    let cookie = sesion(&p).await;
    // Sin fijar: manda `Accept-Language` (y sin nada, español).
    assert_eq!(idiomas_de_login(&p).await, [false, false, false]);
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "idioma": { "Fijo": "En" } } }),
        &cookie,
    )
    .await;
    // Fijado en inglés: va antes que `Accept-Language`; la cookie, antes aún.
    assert_eq!(idiomas_de_login(&p).await, [true, true, false]);

    // Tras reiniciar el panel (otro estado sobre la misma base de datos),
    // sigue igual.
    let dir = p.state.cfg.db_path.parent().unwrap().to_path_buf();
    let state = common::estado(common::config_en(&dir, "127.0.0.1".parse().unwrap()), &dir).await;
    p.app = panel::build_router(state.clone());
    p.state = state;
    assert_eq!(idiomas_de_login(&p).await, [true, true, false]);
}

/// Sin idioma por defecto fijado, «Ajustes del panel» no dice «Español»
/// (la página sigue al navegador): dice «El del navegador». Y se puede
/// volver a eso después de fijar uno.
#[tokio::test]
async fn idioma_por_defecto_sin_fijar_es_el_del_navegador() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["idioma"], "Navegador", "{v}");

    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "idioma": { "Fijo": "En" } } }),
        &cookie,
    )
    .await;
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["idioma"], json!({ "Fijo": "En" }), "{v}");
    assert_eq!(idiomas_de_login(&p).await, [true, true, false]);

    // De vuelta al del navegador: `Accept-Language` vuelve a mandar.
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "idioma": "Navegador" } }),
        &cookie,
    )
    .await;
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["idioma"], "Navegador", "{v}");
    assert_eq!(idiomas_de_login(&p).await, [false, false, false]);
    assert_eq!(p.state.idioma_por_defecto(), None);
}
