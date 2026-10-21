//! Inicio de sesión sin JavaScript: redirección a next o vuelta con el error.

use super::*;

// ---- Login sin JavaScript (el formulario enviado antes de hidratar, o con el
// wasm roto): el navegador manda el formulario tal cual y sigue la respuesta.

/// `POST /api/login` como un formulario HTML sin JavaScript, enviado desde
/// `/login{consulta}`.
async fn login_sin_js(p: &Panel, cuerpo: &str, consulta: &str) -> axum::response::Response {
    let mut req = post_form("/api/login", cuerpo, "127.0.0.1");
    let h = req.headers_mut();
    h.insert(
        "accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
            .parse()
            .unwrap(),
    );
    h.insert(
        "referer",
        format!("{}/login{consulta}", crate::comun::ORIGIN)
            .parse()
            .unwrap(),
    );
    send(&p.app, req).await
}

fn destino(resp: &axum::response::Response) -> String {
    assert!(
        resp.status().is_redirection(),
        "sin redirección: {}",
        resp.status()
    );
    resp.headers()["location"].to_str().unwrap().to_owned()
}

#[tokio::test]
async fn login_sin_js_bueno_lleva_a_next() {
    let p = PanelPrueba::nuevo().arrancar().await;
    crear_usuario(&p).await;
    let cuerpo = format!("{}&next=%2Fhistorial", login_body(PANEL_USER, PANEL_PASS));
    let resp = login_sin_js(&p, &cuerpo, "?next=%2Fhistorial").await;
    assert_eq!(destino(&resp), "/historial");
    let cookie = set_cookie(&resp);
    assert!(cookie.starts_with(&format!("{COOKIE_NAME}=")), "{cookie}");
    // Esa cookie es una sesión vigente.
    let sesion = cookie.split(';').next().unwrap().to_owned();
    let mut req = post_form("/api/quien_soy", "", "127.0.0.1");
    req.headers_mut().insert("cookie", sesion.parse().unwrap());
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 200);
    assert!(body_text(resp).await.contains(PANEL_USER));
}

#[tokio::test]
async fn login_sin_js_bueno_sin_next_o_ajeno_lleva_a_la_portada() {
    let p = PanelPrueba::nuevo().arrancar().await;
    crear_usuario(&p).await;
    let bueno = login_body(PANEL_USER, PANEL_PASS);
    let resp = login_sin_js(&p, &bueno, "").await;
    assert_eq!(destino(&resp), "/");
    for ajeno in [
        "%2F%2Fotro.example",
        "https%3A%2F%2Fotro.example%2F",
        "%2F%5Cotro.example",
    ] {
        let resp = login_sin_js(&p, &format!("{bueno}&next={ajeno}"), "").await;
        assert_eq!(destino(&resp), "/", "{ajeno}");
    }
}

#[tokio::test]
async fn login_sin_js_malo_vuelve_al_login_con_el_error_y_next() {
    let p = PanelPrueba::nuevo().arrancar().await;
    crear_usuario(&p).await;
    let cuerpo = format!("{}&next=%2Fhistorial", login_body(PANEL_USER, "mala"));
    let resp = login_sin_js(&p, &cuerpo, "?next=%2Fhistorial").await;
    let vuelta = destino(&resp);
    // Sin sesión nueva.
    assert!(
        resp.headers()
            .get_all("set-cookie")
            .iter()
            .all(|c| !c.to_str().unwrap().starts_with(&format!("{COOKIE_NAME}="))),
    );
    let ruta = vuelta
        .strip_prefix(crate::comun::ORIGIN)
        .unwrap_or(&vuelta)
        .to_owned();
    assert!(ruta.starts_with("/login?"), "{ruta}");
    // La página de vuelta dice qué pasó y conserva `next`.
    let resp = get(&p.app, &ruta).await;
    assert_eq!(resp.status(), 200);
    let html = body_text(resp).await;
    assert!(html.contains("Usuario o contraseña incorrectos"), "{html}");
    assert!(
        html.contains(r#"name="next" value="/historial""#),
        "sin next: {html}"
    );
}

#[tokio::test]
async fn login_con_javascript_sigue_sin_redirigir() {
    // Lo que manda el wasm (sin `Accept: text/html`): 200 y la cookie; la
    // página navega ella misma.
    let p = PanelPrueba::nuevo().arrancar().await;
    crear_usuario(&p).await;
    let cuerpo = format!("{}&next=%2Fhistorial", login_body(PANEL_USER, PANEL_PASS));
    let resp = send(&p.app, post_form("/api/login", &cuerpo, "127.0.0.1")).await;
    assert_eq!(resp.status(), 200);
    assert!(resp.headers().get("location").is_none());
    assert!(set_cookie(&resp).starts_with(&format!("{COOKIE_NAME}=")));
}

#[tokio::test]
async fn login_con_sesion_valida_lleva_a_next() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let sesion = sesion(&p).await;
    let resp = get_con_cookie(&p.app, "/login", &sesion).await;
    assert_eq!(resp.status(), 303);
    assert_eq!(resp.headers()["location"], "/");
    let resp = get_con_cookie(&p.app, "/login?next=%2Fhistorial", &sesion).await;
    assert_eq!(resp.headers()["location"], "/historial");
    let resp = get_con_cookie(&p.app, "/login?next=%2F%2Fotro.example", &sesion).await;
    assert_eq!(resp.headers()["location"], "/");
    // Sin sesión válida, el formulario.
    let resp = get_con_cookie(&p.app, "/login", &format!("{COOKIE_NAME}=inventada")).await;
    assert_eq!(resp.status(), 200);
}
