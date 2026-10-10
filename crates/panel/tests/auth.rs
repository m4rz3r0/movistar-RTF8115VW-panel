//! Pruebas de autenticación: login, sesiones, freno de fuerza bruta, CSRF y
//! órdenes `panel user …`.

mod common;

use axum::{body::Body, http::Request};
use common::{
    PANEL_PASS, PANEL_USER, Panel, body_text, get, get_con_cookie, login_body, panel,
    panel_con_proxy, post_form, send, sesion, set_cookie,
};
use std::{
    io::Write,
    process::{Command, Stdio},
};

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

#[tokio::test]
async fn login_ok_crea_cookie_con_atributos() {
    let p = panel().await;
    crear_usuario(&p).await;
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
    let cookie = set_cookie(&resp);
    let mut parts = cookie.split("; ");
    let (name, id) = parts.next().unwrap().split_once('=').unwrap();
    assert_eq!(name, COOKIE_NAME);
    assert_eq!(id.len(), 43, "32 bytes en base64url sin relleno");
    assert!(
        id.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "{id}"
    );
    let attrs: Vec<&str> = parts.collect();
    for attr in [
        "HttpOnly",
        "Secure",
        "SameSite=Strict",
        "Path=/",
        "Max-Age=43200",
    ] {
        assert!(attrs.contains(&attr), "falta {attr} en {cookie}");
    }
    assert!(!attrs.iter().any(|a| a.starts_with("Domain")), "{cookie}");

    // La base de datos guarda solo el hash del id.
    let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM sessions")
        .fetch_all(&p.state.db)
        .await
        .unwrap();
    assert_eq!(ids.len(), 1);
    assert_ne!(ids[0], id);

    // Con la cookie se entra en la portada.
    let resp = get_con_cookie(&p.app, "/", &format!("{COOKIE_NAME}={id}")).await;
    assert_eq!(resp.status(), 200);
    assert!(body_text(resp).await.contains("<h1>Estado</h1>"));
}

#[tokio::test]
async fn login_mal_incrementa_retardo() {
    let p = panel().await;
    crear_usuario(&p).await;

    let (status, body) = login_desde(&p, PANEL_USER, "contraseña equivocada", "127.0.0.1").await;
    assert_eq!(status, 401, "{body}");
    assert!(body.contains("Usuario o contraseña incorrectos"), "{body}");
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, Some(1));
    assert_eq!(fallos(&p, "user", PANEL_USER).await, Some(1));

    // Durante la espera se rechaza incluso la contraseña buena, sin contar
    // un fallo más.
    let (status, body) = login_desde(&p, PANEL_USER, PANEL_PASS, "127.0.0.1").await;
    assert_eq!(status, 429, "{body}");
    assert!(body.contains("Demasiados intentos, espera 1 s"), "{body}");
    assert_eq!(fallos(&p, "user", PANEL_USER).await, Some(1));

    // El freno por usuario se aplica también desde otra IP.
    let (status, _) = login_desde(&p, PANEL_USER, PANEL_PASS, "10.0.0.7").await;
    assert_eq!(status, 429);

    retrasar_fallos(&p, 1).await;
    let (status, _) = login_desde(&p, PANEL_USER, "otra mala", "127.0.0.1").await;
    assert_eq!(status, 401);
    assert_eq!(fallos(&p, "user", PANEL_USER).await, Some(2));
    let (status, body) = login_desde(&p, PANEL_USER, PANEL_PASS, "127.0.0.1").await;
    assert_eq!(status, 429);
    assert!(body.contains("espera 2 s"), "{body}");

    // Tras esperar, el login correcto entra y limpia los contadores.
    retrasar_fallos(&p, 2).await;
    let (status, body) = login_desde(&p, PANEL_USER, PANEL_PASS, "127.0.0.1").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, None);
    assert_eq!(fallos(&p, "user", PANEL_USER).await, None);
}

#[tokio::test]
async fn retardo_maximo_60_s() {
    let p = panel().await;
    sqlx::query(
        "INSERT INTO login_attempts VALUES ('user', 'x', 40, strftime('%s', 'now') * 1000)",
    )
    .execute(&p.state.db)
    .await
    .unwrap();
    let (status, body) = login_desde(&p, "x", "lo que sea", "127.0.0.1").await;
    assert_eq!(status, 429);
    assert!(body.contains("espera 60 s"), "{body}");
}

#[tokio::test]
async fn usuario_inexistente_mismo_error() {
    let p = panel().await;
    crear_usuario(&p).await;
    let (status, body) = login_desde(&p, "nadie", "contraseña cualquiera", "127.0.0.1").await;
    assert_eq!(status, 401);
    assert!(body.contains("Usuario o contraseña incorrectos"), "{body}");
    assert_eq!(fallos(&p, "user", "nadie").await, Some(1));
}

#[tokio::test]
async fn xff_ignorado_si_no_viene_del_proxy() {
    let p = panel_con_proxy("192.0.2.11".parse().unwrap()).await;
    let mut req = post_form("/api/login", &login_body("x", "mala"), "127.0.0.1");
    req.headers_mut()
        .insert("x-forwarded-for", "1.2.3.4".parse().unwrap());
    assert_eq!(send(&p.app, req).await.status(), 401);
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, Some(1));
    assert_eq!(fallos(&p, "ip", "1.2.3.4").await, None);
}

#[tokio::test]
async fn xff_usado_si_viene_del_proxy() {
    let p = panel().await; // trusted_proxy = 127.0.0.1
    let mut req = post_form("/api/login", &login_body("x", "mala"), "127.0.0.1");
    req.headers_mut()
        .insert("x-forwarded-for", "9.9.9.9, 1.2.3.4".parse().unwrap());
    assert_eq!(send(&p.app, req).await.status(), 401);
    assert_eq!(fallos(&p, "ip", "1.2.3.4").await, Some(1));
    assert_eq!(fallos(&p, "ip", "9.9.9.9").await, None);
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, None);
}

#[tokio::test]
async fn sesion_caduca_por_inactividad() {
    let p = panel().await;
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
    let p = panel().await;
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
    let p = panel().await;
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
    let p = panel().await;
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
    let p = panel().await;
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
    let p = panel().await;
    let resp = send(&p.app, post_form("/api/quien_soy", "", "127.0.0.1")).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn quien_soy_con_sesion() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let mut req = post_form("/api/quien_soy", "", "127.0.0.1");
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(body_text(resp).await, format!("\"{PANEL_USER}\""));
}

#[tokio::test]
async fn logout_sin_sesion_caduca_la_cookie() {
    let p = panel().await;
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
async fn login_con_cookie_previa_la_sustituye() {
    let p = panel().await;
    let vieja = sesion(&p).await;
    let mut req = post_form(
        "/api/login",
        &login_body(PANEL_USER, PANEL_PASS),
        "127.0.0.1",
    );
    req.headers_mut().insert("cookie", vieja.parse().unwrap());
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 200);
    let sesiones: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert_eq!(sesiones, 1, "la sesión anterior se borra");
    assert_eq!(get_con_cookie(&p.app, "/", &vieja).await.status(), 303);
}

#[tokio::test]
async fn contrasena_enorme_es_un_fallo_normal() {
    let p = panel().await;
    crear_usuario(&p).await;
    let enorme = "x".repeat(1025);
    let (status, body) = login_desde(&p, PANEL_USER, &enorme, "127.0.0.1").await;
    assert_eq!(status, 401, "{body}");
    assert!(body.contains("Usuario o contraseña incorrectos"), "{body}");
    assert_eq!(fallos(&p, "user", PANEL_USER).await, Some(1));
}

#[tokio::test]
async fn logout_borra_la_sesion() {
    let p = panel().await;
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

#[tokio::test]
async fn post_con_origin_ajeno_403() {
    let p = panel().await;
    crear_usuario(&p).await;
    let ajeno = |h: &'static str, v: &'static str| {
        let mut req = post_form(
            "/api/login",
            &login_body(PANEL_USER, PANEL_PASS),
            "127.0.0.1",
        );
        req.headers_mut().insert(h, v.parse().unwrap());
        req
    };
    // Origin ajeno sin Sec-Fetch-Site (cliente sin Fetch Metadata).
    let mut req = ajeno("origin", "https://evil.example");
    req.headers_mut().remove("sec-fetch-site");
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 403);
    assert!(resp.headers().get("set-cookie").is_none());
    // Lo que manda un navegador desde otro sitio.
    let mut req = ajeno("origin", "https://evil.example");
    req.headers_mut()
        .insert("sec-fetch-site", "cross-site".parse().unwrap());
    assert_eq!(send(&p.app, req).await.status(), 403);
    let resp = send(&p.app, ajeno("sec-fetch-site", "cross-site")).await;
    assert_eq!(resp.status(), 403, "cross-site aunque Origin coincida");

    // `Origin: null` sin Sec-Fetch-Site se rechaza.
    let mut req = ajeno("origin", "null");
    req.headers_mut().remove("sec-fetch-site");
    assert_eq!(send(&p.app, req).await.status(), 403);

    // `Origin: null` con `Sec-Fetch-Site: same-origin` (navegador real) pasa.
    let resp = send(&p.app, ajeno("origin", "null")).await;
    assert_eq!(resp.status(), 200);

    // Sin ninguna de las dos cabeceras (clientes sin Fetch Metadata) se acepta.
    let mut req = Request::post("/api/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(login_body(PANEL_USER, PANEL_PASS)))
        .unwrap();
    req.extensions_mut().insert(axum::extract::ConnectInfo(
        "127.0.0.1:50000".parse::<std::net::SocketAddr>().unwrap(),
    ));
    assert_eq!(send(&p.app, req).await.status(), 200);
}

/// Ejecuta el binario `panel` con `args`, `RP_DB=db` y `stdin` como entrada.
fn panel_cli(db: &std::path::Path, args: &[&str], stdin: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_panel"))
        .args(args)
        .env("RP_DB", db)
        .env("RP_ROUTER_SECRET", "/no/existe")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[tokio::test]
async fn cli_alta_y_cambio_de_contrasena() {
    let p = panel().await;
    let db = p.state.cfg.db_path.clone();

    let out = panel_cli(&db, &["user", "add", "admin"], "corta\n");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("12"));

    let out = panel_cli(&db, &["user", "add", "admin"], "primera clave larga\n");
    assert!(out.status.success(), "{out:?}");
    let (status, body) = login_desde(&p, "admin", "primera clave larga", "127.0.0.1").await;
    assert_eq!(status, 200, "{body}");

    let out = panel_cli(&db, &["user", "add", "admin"], "primera clave larga\n");
    assert!(!out.status.success(), "el usuario ya existe");

    let out = panel_cli(&db, &["user", "passwd", "admin"], "segunda clave larga\n");
    assert!(out.status.success(), "{out:?}");
    let sesiones: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert_eq!(sesiones, 0, "cambiar la contraseña cierra las sesiones");
    let (status, body) = login_desde(&p, "admin", "segunda clave larga", "127.0.0.2").await;
    assert_eq!(status, 200, "{body}");

    let out = panel_cli(&db, &["user", "passwd", "nadie"], "segunda clave larga\n");
    assert!(!out.status.success());

    let out = panel_cli(&db, &["user"], "");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("uso:"));
}
