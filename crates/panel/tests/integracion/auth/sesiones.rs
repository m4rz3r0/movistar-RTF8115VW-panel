//! Sesiones: caducidad, renovación, páginas protegidas y cierre.

use super::*;

#[tokio::test]
async fn sesion_caduca_por_inactividad() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    sqlx::query("UPDATE sessions SET last_seen = last_seen - 30 * 60")
        .execute(&p.state.db)
        .await
        .unwrap();
    let resp = get_con_cookie(&p.app, "/", &cookie).await;
    assert_eq!(resp.status(), 303);
    assert_eq!(resp.headers()["location"], "/login?next=%2F");
    let quedan: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert_eq!(quedan, 0, "la sesión caducada se borra");
}

#[tokio::test]
async fn sesion_caduca_a_las_12_horas() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    sqlx::query("UPDATE sessions SET created = created - 12 * 3600")
        .execute(&p.state.db)
        .await
        .unwrap();
    let resp = get_con_cookie(&p.app, "/", &cookie).await;
    assert_eq!(resp.status(), 303);
}

#[tokio::test]
async fn actividad_renueva_la_sesion_como_mucho_cada_minuto() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let last_seen = || async {
        sqlx::query_scalar::<_, i64>("SELECT last_seen FROM sessions")
            .fetch_one(&p.state.db)
            .await
            .unwrap()
    };
    let antes = last_seen().await;
    assert_eq!(get_con_cookie(&p.app, "/", &cookie).await.status(), 200);
    assert_eq!(last_seen().await, antes, "sin escritura antes de un minuto");

    sqlx::query("UPDATE sessions SET last_seen = last_seen - 20 * 60")
        .execute(&p.state.db)
        .await
        .unwrap();
    assert_eq!(get_con_cookie(&p.app, "/", &cookie).await.status(), 200);
    assert!(last_seen().await >= antes);
}

#[tokio::test]
async fn pagina_sin_sesion_redirige_a_login() {
    let p = PanelPrueba::nuevo().arrancar().await;
    crear_usuario(&p).await;
    let resp = get(&p.app, "/?a=1&b=2").await;
    assert_eq!(resp.status(), 303);
    assert_eq!(
        resp.headers()["location"],
        "/login?next=%2F%3Fa%3D1%26b%3D2"
    );
    let resp = get_con_cookie(&p.app, "/", &format!("{COOKIE_NAME}=inventada")).await;
    assert_eq!(resp.status(), 303);
}

#[tokio::test]
async fn pagina_de_login_es_publica() {
    let p = PanelPrueba::nuevo().arrancar().await;
    crear_usuario(&p).await;
    let resp = get(&p.app, "/login?next=%2F").await;
    assert_eq!(resp.status(), 200);
    let html = body_text(resp).await;
    for texto in [
        "Usuario",
        "Contraseña",
        "Entrar",
        "name=\"usuario\"",
        "name=\"contrasena\"",
    ] {
        assert!(html.contains(texto), "falta {texto}: {html}");
    }
}

#[tokio::test]
async fn server_fn_sin_sesion_401() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let resp = send(&p.app, post_form("/api/quien_soy", "", "127.0.0.1")).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn quien_soy_con_sesion() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let mut req = post_form("/api/quien_soy", "", "127.0.0.1");
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(body_text(resp).await, format!("\"{PANEL_USER}\""));
}

#[tokio::test]
async fn logout_sin_sesion_caduca_la_cookie() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let mut req = post_form("/api/logout", "", "127.0.0.1");
    req.headers_mut().insert(
        "cookie",
        format!("{COOKIE_NAME}=caducada-o-inventada")
            .parse()
            .unwrap(),
    );
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 200);
    assert!(set_cookie(&resp).contains("Max-Age=0"));
}

#[tokio::test]
async fn logout_borra_la_sesion() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let mut req = post_form("/api/logout", "", "127.0.0.1");
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 200);
    let borrada = set_cookie(&resp);
    assert!(
        borrada.starts_with(&format!("{COOKIE_NAME}=;")),
        "{borrada}"
    );
    assert!(borrada.contains("Max-Age=0"), "{borrada}");
    assert_eq!(get_con_cookie(&p.app, "/", &cookie).await.status(), 303);
}
