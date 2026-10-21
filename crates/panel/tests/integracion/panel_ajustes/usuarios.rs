//! Usuarios del panel y sus contraseñas.

use super::*;

async fn usuarios(p: &Panel, cookie: &str) -> Vec<String> {
    let v = ok(p, "leer_panel", json!({}), cookie).await;
    serde_json::from_value(v["usuarios"].clone()).unwrap()
}

#[tokio::test]
async fn no_se_borra_el_ultimo_usuario() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let (status, _) = llamar(
        &p,
        "borrar_usuario",
        json!({ "usuario": PANEL_USER }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 401, "sin reautenticar");
    reautenticar(&p, &cookie).await;
    // Ni el último ni uno mismo.
    let (status, body) = llamar(
        &p,
        "borrar_usuario",
        json!({ "usuario": PANEL_USER }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    ok(
        &p,
        "crear_usuario",
        json!({ "usuario": "otro", "contrasena": "otra contraseña larga" }),
        &cookie,
    )
    .await;
    assert_eq!(usuarios(&p, &cookie).await, [PANEL_USER, "otro"]);
    let (status, body) = llamar(
        &p,
        "borrar_usuario",
        json!({ "usuario": PANEL_USER }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "uno mismo: {body}");
    let (status, _) = llamar(
        &p,
        "crear_usuario",
        json!({ "usuario": "otro", "contrasena": "otra contraseña larga" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 409, "ya existe");
    let (status, _) = llamar(
        &p,
        "crear_usuario",
        json!({ "usuario": "corta", "contrasena": "corta" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400);
    let (status, _) = llamar(
        &p,
        "borrar_usuario",
        json!({ "usuario": "nadie" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 404);

    // El otro usuario, con sesión, se borra y su sesión deja de valer.
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body("otro", "otra contraseña larga"),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 200);
    let cookie_otro = set_cookie(&resp).split(';').next().unwrap().to_owned();
    ok(&p, "borrar_usuario", json!({ "usuario": "otro" }), &cookie).await;
    assert_eq!(usuarios(&p, &cookie).await, [PANEL_USER]);
    let (status, _) = llamar(&p, "leer_panel", json!({}), Some(&cookie_otro)).await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn cambiar_contrasena_propia_mantiene_la_sesion() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let nueva = json!({ "usuario": PANEL_USER, "contrasena": "contraseña nueva y larga" });
    let (status, _) = llamar(&p, "cambiar_contrasena", nueva.clone(), Some(&cookie)).await;
    assert_eq!(status, 401);
    reautenticar(&p, &cookie).await;
    let mut req = post_json("/api/cambiar_contrasena", &nueva.to_string(), "127.0.0.1");
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 200);
    // Las sesiones viejas se cierran; la respuesta trae una nueva.
    let nueva_cookie = set_cookie(&resp).split(';').next().unwrap().to_owned();
    assert_ne!(nueva_cookie, cookie);
    let (status, _) = llamar(&p, "leer_panel", json!({}), Some(&cookie)).await;
    assert_eq!(status, 401);
    ok(&p, "leer_panel", json!({}), &nueva_cookie).await;
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body(PANEL_USER, "contraseña nueva y larga"),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 200, "entra con la nueva");
}

#[tokio::test]
async fn leer_panel_no_devuelve_contrasena() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let (status, _) = llamar(&p, "leer_panel", json!({}), None).await;
    assert_eq!(status, 401);
    let cookie = sesion(&p).await;
    let (status, body) = llamar(&p, "leer_panel", json!({}), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert!(!body.contains(PASS), "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["router"]["url"], p.sim.base_url);
    assert_eq!(v["router"]["usuario"], USER);
    assert_eq!(v["usuarios"], json!([PANEL_USER]));
    assert_eq!(v["solo_lectura"], false);
    assert_eq!(
        v.get("dns_preferidos"),
        None,
        "lo usarán las recomendaciones"
    );
    // Sin zona guardada, la peninsular.
    assert_eq!(v["zona_horaria"], "Europe/Madrid");
    // La página tampoco la lleva.
    let resp = get_con_cookie(&p.app, "/panel", &cookie).await;
    assert_eq!(resp.status(), 200);
    let html = body_text(resp).await;
    assert!(!html.contains(PASS));
    assert!(html.contains("Ajustes del panel"), "{html}");
}

/// Falla algún login del usuario del panel desde otra IP: el freno por
/// usuario queda activo.
async fn frenar_usuario(p: &Panel) {
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body(PANEL_USER, "no es la buena"),
            "192.168.1.60",
        ),
    )
    .await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn cambiar_contrasena_propia_con_usuario_frenado() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    reautenticar(&p, &cookie).await;
    frenar_usuario(&p).await;
    let nueva = json!({ "usuario": PANEL_USER, "contrasena": "contraseña nueva y larga" });
    let mut req = post_json("/api/cambiar_contrasena", &nueva.to_string(), "127.0.0.1");
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 200);
    let nueva_cookie = set_cookie(&resp).split(';').next().unwrap().to_owned();
    ok(&p, "leer_panel", json!({}), &nueva_cookie).await;
}

#[tokio::test]
async fn contrasena_demasiado_larga() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    reautenticar(&p, &cookie).await;
    let larga = "x".repeat(1025);
    let (status, body) = llamar(
        &p,
        "crear_usuario",
        json!({ "usuario": "otro", "contrasena": larga }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert_eq!(usuarios(&p, &cookie).await, [PANEL_USER]);
    let (status, body) = llamar(
        &p,
        "cambiar_contrasena",
        json!({ "usuario": PANEL_USER, "contrasena": larga }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    // Sigue entrando con la de antes, y con 1024 bytes justos se puede.
    ok(&p, "leer_panel", json!({}), &cookie).await;
    ok(
        &p,
        "crear_usuario",
        json!({ "usuario": "otro", "contrasena": "x".repeat(1024) }),
        &cookie,
    )
    .await;
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body("otro", &"x".repeat(1024)),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 200);
}
