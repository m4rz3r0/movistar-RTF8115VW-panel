//! Pruebas de autenticación: login, sesiones, freno de fuerza bruta, CSRF y
//! órdenes `panel user …`.

mod comun;

use axum::{body::Body, http::Request};
use comun::{
    PANEL_PASS, PANEL_USER, Panel, PanelPrueba, body_text, get, get_con_cookie, login_body,
    post_form, send, sesion, set_cookie,
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
    let p = PanelPrueba::nuevo().arrancar().await;
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
    let p = PanelPrueba::nuevo().arrancar().await;
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
    let (status, _) = login_desde(&p, PANEL_USER, PANEL_PASS, "192.0.2.2").await;
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
    let p = PanelPrueba::nuevo().arrancar().await;
    // El último fallo, ahora mismo (en milisegundos: `strftime('%s')`
    // redondeaba al segundo y, con la máquina cargada, la espera que quedaba
    // ya era de 59 s).
    let ahora_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    sqlx::query("INSERT INTO login_attempts VALUES ('user', 'x', 40, ?)")
        .bind(ahora_ms)
        .execute(&p.state.db)
        .await
        .unwrap();
    let (status, body) = login_desde(&p, "x", "lo que sea", "127.0.0.1").await;
    assert_eq!(status, 429);
    // 40 fallos esperan lo máximo, 60 s, menos lo que haya tardado la
    // petición: nunca más de 60.
    let segundos: i64 = body
        .split("espera ")
        .nth(1)
        .and_then(|r| r.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{body}"));
    assert!((55..=60).contains(&segundos), "{body}");
}

#[tokio::test]
async fn usuario_inexistente_mismo_error() {
    let p = PanelPrueba::nuevo().arrancar().await;
    crear_usuario(&p).await;
    let (status, body) = login_desde(&p, "nadie", "contraseña cualquiera", "127.0.0.1").await;
    assert_eq!(status, 401);
    assert!(body.contains("Usuario o contraseña incorrectos"), "{body}");
    assert_eq!(fallos(&p, "user", "nadie").await, Some(1));
}

#[tokio::test]
async fn xff_ignorado_si_no_viene_del_proxy() {
    let p = PanelPrueba::nuevo().proxy("192.0.2.100").arrancar().await;
    let mut req = post_form("/api/login", &login_body("x", "mala"), "127.0.0.1");
    req.headers_mut()
        .insert("x-forwarded-for", "198.51.100.4".parse().unwrap());
    assert_eq!(send(&p.app, req).await.status(), 401);
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, Some(1));
    assert_eq!(fallos(&p, "ip", "198.51.100.4").await, None);
}

#[tokio::test]
async fn xff_usado_si_viene_del_proxy() {
    let p = PanelPrueba::nuevo().arrancar().await; // proxy de confianza: 127.0.0.1
    let mut req = post_form("/api/login", &login_body("x", "mala"), "127.0.0.1");
    req.headers_mut()
        .insert("x-forwarded-for", "9.9.9.9, 198.51.100.4".parse().unwrap());
    assert_eq!(send(&p.app, req).await.status(), 401);
    assert_eq!(fallos(&p, "ip", "198.51.100.4").await, Some(1));
    assert_eq!(fallos(&p, "ip", "9.9.9.9").await, None);
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, None);
}

/// Con una red de proxies (p. ej. la de los contenedores), cuenta
/// `X-Forwarded-For` si el par está en la red, y no si está fuera.
#[tokio::test]
async fn proxy_de_confianza_por_red() {
    let p = PanelPrueba::nuevo()
        .proxy("198.51.100.0/24")
        .arrancar()
        .await;
    // Un usuario distinto en cada intento, para que su freno no cuente.
    let intento = |peer: &str, cliente: &'static str| {
        let mut req = post_form("/api/login", &login_body(peer, "mala"), peer);
        req.headers_mut()
            .insert("x-forwarded-for", cliente.parse().unwrap());
        req
    };
    assert_eq!(
        send(&p.app, intento("198.51.100.5", "203.0.113.4"))
            .await
            .status(),
        401
    );
    assert_eq!(fallos(&p, "ip", "203.0.113.4").await, Some(1));
    assert_eq!(fallos(&p, "ip", "198.51.100.5").await, None);
    assert_eq!(
        send(&p.app, intento("192.0.2.5", "203.0.113.8"))
            .await
            .status(),
        401
    );
    assert_eq!(fallos(&p, "ip", "192.0.2.5").await, Some(1));
    assert_eq!(fallos(&p, "ip", "203.0.113.8").await, None);
}

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
async fn login_con_cookie_previa_la_sustituye() {
    let p = PanelPrueba::nuevo().arrancar().await;
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
    let p = PanelPrueba::nuevo().arrancar().await;
    crear_usuario(&p).await;
    let enorme = "x".repeat(1025);
    let (status, body) = login_desde(&p, PANEL_USER, &enorme, "127.0.0.1").await;
    assert_eq!(status, 401, "{body}");
    assert!(body.contains("Usuario o contraseña incorrectos"), "{body}");
    assert_eq!(fallos(&p, "user", PANEL_USER).await, Some(1));
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

#[tokio::test]
async fn post_con_origin_ajeno_403() {
    let p = PanelPrueba::nuevo().arrancar().await;
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
    let p = PanelPrueba::nuevo().arrancar().await;
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
        format!("{}/login{consulta}", comun::ORIGIN)
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
        .strip_prefix(comun::ORIGIN)
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
