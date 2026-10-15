//! Pruebas del cliente contra un router falso mínimo.

use axum::{
    Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, header},
    response::IntoResponse,
    routing::{get, post},
};
use hgu::{Client, ClientConfig, Error, fmt::mess_userpass};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

const USER: &str = "1234";
const PASS: &str = "secreto";

#[derive(Default)]
struct Fake {
    basic_logins: AtomicUsize,
    adv_logins: AtomicUsize,
    in_flight: AtomicUsize,
    overlaps: AtomicUsize,
    malformed_logout: std::sync::atomic::AtomicBool,
    basic_valid: std::sync::atomic::AtomicBool,
    always_logout: std::sync::atomic::AtomicBool,
    valid_sid: Mutex<Option<String>>,
    last_write: Mutex<Option<HashMap<String, String>>>,
    /// Con esto, cada inicio de sesión avanzado cambia el `sessionKey` de las
    /// páginas (como el router: la clave vieja deja de valer).
    rotate_key_on_login: std::sync::atomic::AtomicBool,
    key: Mutex<Option<String>>,
    cookies_seen: Mutex<Vec<String>>,
    /// Retraso extra (ms) de cada llamada `.xml`.
    delay_ms: std::sync::atomic::AtomicU64,
    /// Nombres de las llamadas `.xml` con sesión válida, en orden de llegada.
    llamadas: Mutex<Vec<String>>,
}

type S = Arc<Fake>;

fn form(body: &[u8]) -> HashMap<String, String> {
    let s = String::from_utf8_lossy(body);
    s.split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_owned(), pct_decode(v)))
        .collect()
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
            i += 3;
        } else if b[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap()
}

const LOGGED_OUT: &str = "<script>top.location.href='te_acceso_router.asp';</script>";

async fn root() -> impl IntoResponse {
    (
        [(header::SET_COOKIE, "_httpdSessionId_=abc; path=/")],
        "<html></html>",
    )
}

async fn basic_login(State(s): State<S>, body: Bytes) -> String {
    s.basic_logins.fetch_add(1, Ordering::SeqCst);
    let f = form(&body);
    assert_eq!(f["curWebPage"], "/te_wifi.asp");
    if f["loginUsername"] == mess_userpass(USER) && f["loginPassword"] == mess_userpass(PASS) {
        s.basic_valid.store(true, Ordering::SeqCst);
        "<script>location.href = \"/te_wifi.asp\"</script>".into()
    } else {
        "<script>location.href = \"/te_acceso_router.asp\"</script>".into()
    }
}

async fn adv_login(State(s): State<S>, body: Bytes) -> String {
    let n = s.adv_logins.fetch_add(1, Ordering::SeqCst) + 1;
    let f = form(&body);
    assert_eq!(f["rqTimeout"], "600");
    if !s.basic_valid.load(Ordering::SeqCst) {
        return LOGGED_OUT.into();
    }
    if f["rqUsername"] == mess_userpass(USER) && f["rqPasswd"] == mess_userpass(PASS) {
        let sid = format!("sid{n}");
        *s.valid_sid.lock().unwrap() = Some(sid.clone());
        if s.rotate_key_on_login.load(Ordering::SeqCst) {
            *s.key.lock().unwrap() = Some(format!("k{n}"));
        }
        format!(
            "<Response><cbCheckLogin><Result>0</Result><SessionID>{sid}</SessionID></cbCheckLogin></Response>"
        )
    } else {
        "<Response><cbCheckLogin><Result>1</Result></cbCheckLogin></Response>".into()
    }
}

async fn api(
    State(s): State<S>,
    Path(name): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> String {
    if s.in_flight.fetch_add(1, Ordering::SeqCst) > 0 {
        s.overlaps.fetch_add(1, Ordering::SeqCst);
    }
    tokio::time::sleep(Duration::from_millis(5 + s.delay_ms.load(Ordering::SeqCst))).await;
    s.in_flight.fetch_sub(1, Ordering::SeqCst);

    let cookie = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    s.cookies_seen.lock().unwrap().push(cookie.clone());
    assert_eq!(headers["x-requested-with"], "XMLHttpRequest");
    let sid = s.valid_sid.lock().unwrap().clone();
    let ok = !s.always_logout.load(Ordering::SeqCst)
        && sid.is_some_and(|sid| {
            cookie.contains(&format!("SessionID={sid}")) && cookie.contains("SessionStatus=Valid")
        });
    if !ok {
        if s.malformed_logout.load(Ordering::SeqCst) {
            return "<script>if(a&&b)top.location.href='te_acceso_router.asp';</script>".into();
        }
        return LOGGED_OUT.into();
    }
    let name = name.trim_end_matches(".xml");
    s.llamadas.lock().unwrap().push(name.to_owned());
    let f = form(&body);
    match name {
        "cbGet" => "<Response><cbGet><Result>0</Result><v>7</v></cbGet></Response>".into(),
        // Lecturas con nombre propio, para ver el orden de llegada.
        n if n.starts_with("cbGet") => format!("<Response><{n}><Result>0</Result></{n}></Response>"),
        "cbFail" => {
            "<Response><cbFail><Result>1</Result><ErrorString>no se pudo</ErrorString></cbFail></Response>"
                .into()
        }
        "cbSet" => {
            let key = s.key.lock().unwrap().clone().unwrap_or_else(|| "42".into());
            if f.get("sessionKey") != Some(&key) {
                return "<Response><cbSet><Result>1</Result><ErrorString>clave</ErrorString></cbSet></Response>"
                    .into();
            }
            *s.last_write.lock().unwrap() = Some(f);
            "<Response><cbSet><Result>0</Result></cbSet></Response>".into()
        }
        other => panic!("llamada inesperada {other}"),
    }
}

/// Escrituras de la UI básica: `boom.wl` falla con 500, `mala.wl` responde
/// como un sessionKey rechazado; el resto, OK.
async fn basic_cgi(Path(name): Path<String>) -> axum::response::Response {
    match name.as_str() {
        "boom.wl" => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "boom").into_response(),
        "mala.wl" => "<html><body>sessionKey no válido</body></html>".into_response(),
        _ => "<html><body>ok</body></html>".into_response(),
    }
}

async fn dhcp_page(State(s): State<S>) -> String {
    let key = s.key.lock().unwrap().clone().unwrap_or_else(|| "42".into());
    format!("<html><script>\nvar sessionKey='{key}';\n</script></html>")
}

async fn start() -> (String, S) {
    let state: S = Arc::new(Fake::default());
    let app = Router::new()
        .route("/", get(root))
        .route("/dhcp.asp", get(dhcp_page))
        .route("/cgi-bin/te_acceso_router.cgi", post(basic_login))
        .route("/cgi-bin/cbCheckLogin.xml", post(adv_login))
        .route("/cgi-bin/{name}", post(api).get(basic_cgi))
        .with_state(state.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (url, state)
}

fn client(url: &str, pass: &str) -> Client {
    Client::new(ClientConfig {
        base_url: url.to_owned(),
        user: USER.into(),
        password: pass.into(),
        timeout: Duration::from_secs(5),
        min_gap: Duration::from_millis(1),
    })
}

#[tokio::test]
async fn call_logs_in_lazily_and_returns_node() {
    let (url, s) = start().await;
    let c = client(&url, PASS);
    assert_eq!(s.basic_logins.load(Ordering::SeqCst), 0);
    let n = c.call("cbGet", &[("a", "b c")]).await.unwrap();
    assert_eq!(n.text_of("v"), Some("7"));
    assert_eq!(s.basic_logins.load(Ordering::SeqCst), 1);
    assert_eq!(s.adv_logins.load(Ordering::SeqCst), 1);
    c.call("cbGet", &[]).await.unwrap();
    assert_eq!(s.adv_logins.load(Ordering::SeqCst), 1);
    let cookie = s.cookies_seen.lock().unwrap()[0].clone();
    for part in [
        "_httpdSessionId_=abc",
        "SessionID=sid1",
        "SessionStatus=Valid",
        "TimeOut=600",
        "LoginName=",
    ] {
        assert!(cookie.contains(part), "{part} en {cookie}");
    }
}

#[tokio::test]
async fn relogin_once_after_session_invalidated() {
    let (url, s) = start().await;
    let c = client(&url, PASS);
    c.call("cbGet", &[]).await.unwrap();
    *s.valid_sid.lock().unwrap() = None;
    let n = c.call("cbGet", &[]).await.unwrap();
    assert_eq!(n.text_of("v"), Some("7"));
    assert_eq!(s.adv_logins.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn no_login_loop_when_session_keeps_failing() {
    let (url, s) = start().await;
    s.always_logout.store(true, Ordering::SeqCst);
    let c = client(&url, PASS);
    let r = c.call("cbGet", &[]).await;
    assert!(matches!(r, Err(Error::SessionExpired(_))), "{r:?}");
    assert!(s.adv_logins.load(Ordering::SeqCst) <= 2);
}

#[tokio::test]
async fn wrong_password_is_auth_error() {
    let (url, s) = start().await;
    let c = client(&url, "mala");
    let r = c.call("cbGet", &[]).await;
    assert!(matches!(r, Err(Error::Auth(_))), "{r:?}");
    assert!(!format!("{r:?}").contains("mala"));
    assert_eq!(s.adv_logins.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn result_one_is_call_failed_with_error_string() {
    let (url, _s) = start().await;
    let c = client(&url, PASS);
    let r = c.call("cbFail", &[]).await;
    assert_eq!(
        r.unwrap_err(),
        Error::CallFailed {
            name: "cbFail".into(),
            result: "1".into(),
            error: "no se pudo".into()
        }
    );
}

#[tokio::test]
async fn write_sends_session_key_from_page() {
    let (url, s) = start().await;
    let c = client(&url, PASS);
    assert_eq!(c.session_key("dhcp.asp").await.unwrap(), "42");
    c.write("dhcp.asp", "cbSet", &[("x", "1")]).await.unwrap();
    let w = s.last_write.lock().unwrap().clone().unwrap();
    assert_eq!(w["sessionKey"], "42");
    assert_eq!(w["x"], "1");
}

#[tokio::test]
async fn write_after_session_expired_uses_a_fresh_session_key() {
    let (url, s) = start().await;
    s.rotate_key_on_login.store(true, Ordering::SeqCst);
    let c = client(&url, PASS);
    c.write("dhcp.asp", "cbSet", &[("x", "1")]).await.unwrap();
    assert_eq!(
        s.last_write.lock().unwrap().as_ref().unwrap()["sessionKey"],
        "k1"
    );
    // Alguien entra en la web del router: la sesión avanzada del cliente
    // caduca y, al volver a entrar, la clave de la página es otra.
    *s.valid_sid.lock().unwrap() = None;
    c.write("dhcp.asp", "cbSet", &[("x", "2")]).await.unwrap();
    let w = s.last_write.lock().unwrap().clone().unwrap();
    assert_eq!((w["sessionKey"].as_str(), w["x"].as_str()), ("k2", "2"));
    assert_eq!(s.adv_logins.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn requests_are_serialized() {
    let (url, s) = start().await;
    let c = client(&url, PASS);
    let mut tasks = Vec::new();
    for _ in 0..20 {
        let c = c.clone();
        tasks.push(tokio::spawn(async move { c.call("cbGet", &[]).await }));
    }
    for t in tasks {
        t.await.unwrap().unwrap();
    }
    assert_eq!(s.overlaps.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn malformed_logged_out_body_triggers_relogin() {
    let (url, s) = start().await;
    let c = client(&url, PASS);
    c.call("cbGet", &[]).await.unwrap();
    *s.valid_sid.lock().unwrap() = None;
    s.malformed_logout.store(true, Ordering::SeqCst);
    let n = c.call("cbGet", &[]).await.unwrap();
    assert_eq!(n.text_of("v"), Some("7"));
    assert_eq!(s.adv_logins.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn expired_basic_session_during_advanced_login_retries() {
    let (url, s) = start().await;
    let c = client(&url, PASS);
    c.basic_page("dhcp.asp").await.unwrap();
    s.basic_valid.store(false, Ordering::SeqCst);
    let n = c.call("cbGet", &[]).await.unwrap();
    assert_eq!(n.text_of("v"), Some("7"));
    assert_eq!(s.basic_logins.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn basic_write_errors_hide_the_query() {
    let (url, _s) = start().await;
    let c = client(&url, PASS);
    assert!(
        c.basic_write("dhcp.asp", "ok.wl", &[("a", "1")])
            .await
            .unwrap()
            .contains("ok")
    );
    let e = c
        .basic_write(
            "dhcp.asp",
            "boom.wl?action=save",
            &[("wlWpaPsk", "secreta")],
        )
        .await
        .unwrap_err();
    let texto = format!("{e} {e:?}");
    assert!(matches!(e, Error::Unexpected(_)), "{e:?}");
    assert!(texto.contains("boom.wl"), "{texto}");
    for oculto in ["?", "secreta", "sessionKey", "42", "action"] {
        assert!(!texto.contains(oculto), "{oculto} en {texto}");
    }
    let e = c
        .basic_write("dhcp.asp", "mala.wl", &[("x", "1")])
        .await
        .unwrap_err();
    assert!(
        matches!(&e, Error::CallFailed { name, .. } if name == "mala.wl"),
        "{e:?}"
    );
}

fn cfg(url: &str, pass: &str) -> ClientConfig {
    ClientConfig {
        base_url: url.to_owned(),
        user: USER.into(),
        password: pass.into(),
        timeout: Duration::from_secs(5),
        min_gap: Duration::from_millis(1),
    }
}

#[tokio::test]
async fn sin_configurar_devuelve_noconfigurado() {
    let (_url, s) = start().await;
    let c = Client::sin_configurar();
    assert!(!c.configurado());
    assert_eq!(
        c.call("cbGetSummary", &[]).await.unwrap_err(),
        Error::NoConfigurado
    );
    assert_eq!(
        c.basic_page("dhcp.asp").await.unwrap_err(),
        Error::NoConfigurado
    );
    assert_eq!(Error::NoConfigurado.to_string(), "router sin configurar");
    c.logout().await;
    assert_eq!(s.basic_logins.load(Ordering::SeqCst), 0);
    assert!(s.cookies_seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn reconfigurar_cambia_credenciales_y_sesion() {
    let (url, s) = start().await;
    let c = client(&url, "mala");
    let r = c.call("cbGet", &[]).await;
    assert!(matches!(r, Err(Error::Auth(_))), "{r:?}");
    let antes = s.basic_logins.load(Ordering::SeqCst);
    c.reconfigurar(cfg(&url, PASS)).await;
    assert!(c.configurado());
    let n = c.call("cbGet", &[]).await.unwrap();
    assert_eq!(n.text_of("v"), Some("7"));
    assert_eq!(s.basic_logins.load(Ordering::SeqCst), antes + 1);
    assert_eq!(s.adv_logins.load(Ordering::SeqCst), 1);
    // Una sesión abierta se descarta al reconfigurar.
    c.reconfigurar(cfg(&url, PASS)).await;
    c.call("cbGet", &[]).await.unwrap();
    assert_eq!(s.adv_logins.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn reconfigurar_espera_peticion_en_curso() {
    let (url_a, a) = start().await;
    let (url_b, b) = start().await;
    a.delay_ms.store(300, Ordering::SeqCst);
    let c = client(&url_a, PASS);
    let lenta = {
        let c = c.clone();
        tokio::spawn(async move { c.call("cbGet", &[]).await })
    };
    // Deja que la llamada tome el cerrojo y entre en la petición lenta.
    while a.adv_logins.load(Ordering::SeqCst) == 0 {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let cambio = {
        let c = c.clone();
        let cfg_b = cfg(&url_b, PASS);
        tokio::spawn(async move { c.reconfigurar(cfg_b).await })
    };
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert!(
        !cambio.is_finished(),
        "reconfigurar no esperó a la petición"
    );
    // La petición en curso termina contra el router viejo, con sus cookies.
    lenta.await.unwrap().unwrap();
    cambio.await.unwrap();
    assert!(b.cookies_seen.lock().unwrap().is_empty());
    assert_eq!(b.basic_logins.load(Ordering::SeqCst), 0);
    // Tras el cambio, sesión nueva en el router nuevo.
    c.call("cbGet", &[]).await.unwrap();
    assert_eq!(b.basic_logins.load(Ordering::SeqCst), 1);
    assert_eq!(b.adv_logins.load(Ordering::SeqCst), 1);
    assert_eq!(a.adv_logins.load(Ordering::SeqCst), 1);
}

/// Lanza `c.call(nombre)` y espera a que esté esperando el cerrojo.
async fn llamada_en_espera(c: &Client, nombre: &'static str) -> tokio::task::JoinHandle<()> {
    let c = c.clone();
    let t = tokio::spawn(async move {
        c.call(nombre, &[]).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(30)).await;
    t
}

#[tokio::test]
async fn turno_sin_llamadas_intercaladas() {
    let (url, s) = start().await;
    s.delay_ms.store(20, Ordering::SeqCst);
    let c = client(&url, PASS);
    c.call("cbGet", &[]).await.unwrap();
    let mut turno = c.turno().await;
    let otra = llamada_en_espera(&c, "cbGetOtra").await;
    for n in ["cbGetA", "cbGetB", "cbGetC"] {
        turno.call(n, &[]).await.unwrap();
    }
    drop(turno);
    otra.await.unwrap();
    let llamadas = s.llamadas.lock().unwrap().clone();
    assert_eq!(
        llamadas[1..],
        ["cbGetA", "cbGetB", "cbGetC", "cbGetOtra"],
        "{llamadas:?}"
    );
    assert_eq!(s.overlaps.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn turno_cede_el_cerrojo_tras_el_maximo() {
    let (url, s) = start().await;
    let c = client(&url, PASS);
    c.call("cbGet", &[]).await.unwrap();
    let mut turno = c.turno().await;
    let otra = llamada_en_espera(&c, "cbGetOtra").await;
    for _ in 0..=hgu::client::LECTURAS_POR_TURNO {
        turno.call("cbGetT", &[]).await.unwrap();
    }
    drop(turno);
    otra.await.unwrap();
    let llamadas = s.llamadas.lock().unwrap().clone();
    let posicion = llamadas[1..].iter().position(|n| n == "cbGetOtra");
    assert_eq!(
        posicion,
        Some(hgu::client::LECTURAS_POR_TURNO),
        "{llamadas:?}"
    );
}

#[tokio::test]
async fn escrituras_cuenta_escrituras_y_reconfiguraciones_no_lecturas() {
    let (url, _s) = start().await;
    let c = client(&url, PASS);
    let inicio = c.escrituras();
    c.call("cbGet", &[]).await.unwrap();
    c.turno().await.call("cbGetA", &[]).await.unwrap();
    assert_eq!(c.escrituras(), inicio);
    c.write("dhcp.asp", "cbSet", &[("x", "1")]).await.unwrap();
    let tras_escribir = c.escrituras();
    assert!(tras_escribir > inicio);
    c.basic_write("dhcp.asp", "ok.wl", &[]).await.unwrap();
    let tras_basica = c.escrituras();
    assert!(tras_basica > tras_escribir);
    // También una escritura que falla: pudo llegar al router.
    let _ = c.basic_write("dhcp.asp", "boom.wl", &[]).await;
    assert!(c.escrituras() > tras_basica);
    let antes = c.escrituras();
    c.reconfigurar(ClientConfig::new(&url, USER, PASS)).await;
    assert!(c.escrituras() > antes);
}
