//! Cambiar la contraseña del router (Tarea 12): el flujo propio con prueba
//! de acceso y restauración, contra el simulador. Y la tarjeta de las
//! credenciales, que llega con la contraseña guardada (censurada con el ojo)
//! y no la envía si no se toca.

mod common;

use common::{
    PANEL_PASS, PASS, Panel, USER, aplicar, body_text, esperar_final, esperar_libre,
    get_con_cookie, llamar, ok, panel_con_gestor, post_json, send, sesion,
};
use hgu::{Client, ClientConfig};
use panel::instancia;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, Instant};

const NUEVA: &str = "clave-nueva-de-prueba";

/// Destino del registro en memoria (como en `ajustes_secretos.rs`).
#[derive(Clone, Default)]
struct Registro(Arc<std::sync::Mutex<Vec<u8>>>);

impl Registro {
    fn texto(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

impl std::io::Write for Registro {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Registro {
    type Writer = Registro;

    fn make_writer(&'a self) -> Registro {
        self.clone()
    }
}

/// El registro de todo este binario de pruebas (todas las pruebas, en todos
/// los hilos): que ninguna anote una contraseña es más estricto.
fn registro_global() -> Registro {
    static REGISTRO: std::sync::OnceLock<Registro> = std::sync::OnceLock::new();
    REGISTRO
        .get_or_init(|| {
            let registro = Registro::default();
            tracing::subscriber::set_global_default(
                tracing_subscriber::fmt()
                    .with_writer(registro.clone())
                    .with_max_level(tracing::Level::DEBUG)
                    .with_ansi(false)
                    .finish(),
            )
            .expect("el único registro global de estas pruebas");
            registro
        })
        .clone()
}

async fn reautenticar(p: &Panel, cookie: &str) {
    ok(
        p,
        "reautenticar",
        json!({ "contrasena": PANEL_PASS }),
        cookie,
    )
    .await;
}

/// Panel con el gestor, la ventana de reautenticación abierta y la cookie.
async fn preparado() -> (Panel, String) {
    registro_global();
    let (p, cookie) = panel_con_gestor().await;
    reautenticar(&p, &cookie).await;
    (p, cookie)
}

async fn cambiar(p: &Panel, cookie: &str, nueva: &str) -> (u16, String) {
    llamar(
        p,
        "cambiar_contrasena_router",
        json!({ "nueva": nueva }),
        Some(cookie),
    )
    .await
}

async fn contrasena_guardada(p: &Panel) -> String {
    instancia::leer_credenciales(&p.state.db, &p.state.history_key)
        .await
        .unwrap()
        .unwrap()
        .contrasena
}

/// ¿Entra en el simulador con `contrasena`? (Un cliente aparte.)
async fn entra(p: &Panel, contrasena: &str) -> bool {
    let mut cfg = ClientConfig::new(&p.sim.base_url, USER, contrasena);
    cfg.timeout = Duration::from_secs(2);
    cfg.min_gap = Duration::from_millis(1);
    let c = Client::new(cfg);
    let r = hgu::read::summary(&c).await.is_ok();
    c.logout().await;
    r
}

/// Los cambios de contraseña recibidos por el simulador, con sus
/// parámetros decodificados con la función compartida.
fn cambios_de_contrasena(p: &Panel) -> Vec<Vec<(String, String)>> {
    p.sim
        .control
        .writes()
        .into_iter()
        .filter(|(f, _)| f == hgu::fmt::CAMBIO_CONTRASENA)
        .map(|(_, ps)| {
            ps.into_iter()
                .map(|(k, v)| (k, hgu::fmt::codificar_cambio_contrasena(&v)))
                .collect()
        })
        .collect()
}

fn valor<'a>(ps: &'a [(String, String)], k: &str) -> &'a str {
    ps.iter()
        .find(|(n, _)| n == k)
        .map(|(_, v)| v.as_str())
        .unwrap_or_else(|| panic!("falta {k}"))
}

/// La fila del Historial del último intento: (estado, error, nota, título).
async fn ultima_fila(p: &Panel) -> (i64, String, Option<String>, Option<String>, Option<String>) {
    sqlx::query_as(
        "SELECT id, estado, error, nota, titulo FROM changes WHERE funcion = 'cbModifyPwd' \
         ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&p.state.db)
    .await
    .unwrap()
}

/// Todo lo guardado en el Historial, como texto.
async fn historial_en_texto(p: &Panel) -> String {
    let filas: Vec<String> = sqlx::query_scalar(
        "SELECT COALESCE(diff, '') || COALESCE(anteriores, '') || COALESCE(error, '') || \
         COALESCE(nota, '') || COALESCE(solicitud, '') || COALESCE(nuevos, '') FROM changes",
    )
    .fetch_all(&p.state.db)
    .await
    .unwrap();
    filas.join("\n")
}

#[tokio::test]
async fn contrasena_router_cambia_y_se_guarda() {
    registro_global();
    let (p, cookie) = panel_con_gestor().await;
    // Un sondeo rápido sobre el mismo cliente del router que el gestor.
    panel::poller::Poller::spawn_with(
        p.state.clone(),
        panel::poller::PollerConfig {
            fast: Duration::from_millis(50),
            slow: Duration::from_millis(200),
            backoff_max: Duration::from_millis(200),
            ..panel::poller::PollerConfig::default()
        },
    );
    reautenticar(&p, &cookie).await;

    let (status, body) = cambiar(&p, &cookie, NUEVA).await;
    assert_eq!(status, 200, "{body}");
    // El router tiene la nueva y el panel la ha guardado.
    assert!(entra(&p, NUEVA).await);
    assert!(!entra(&p, PASS).await);
    assert_eq!(contrasena_guardada(&p).await, NUEVA);
    let cambios = cambios_de_contrasena(&p);
    assert_eq!(cambios.len(), 1, "{cambios:?}");
    assert_eq!(valor(&cambios[0], "rqUser"), USER);
    assert_eq!(valor(&cambios[0], "rqOPwd"), PASS);
    assert_eq!(valor(&cambios[0], "rqNPwd"), NUEVA);

    // El sondeo sigue leyendo, ya con la nueva (sin sesiones abiertas).
    p.sim.control.invalidate_sessions();
    let antes = p.state.poller.latest().map_or(0, |s| s.at);
    let inicio = Instant::now();
    loop {
        let s = p.state.poller.latest();
        if s.as_ref()
            .is_some_and(|s| s.at > antes && s.error.is_none())
        {
            break;
        }
        assert!(
            inicio.elapsed() < Duration::from_secs(10),
            "el sondeo no lee: {:?}",
            p.state.poller.error()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Un cambio posterior se aplica y se vigila con la nueva.
    let id = aplicar(
        &p,
        &cookie,
        json!({ "funcion": "cbSetUpnpCfg", "instancia": null,
            "valores": [["adm_state", "1"]], "borrar": false }),
    )
    .await;
    let c = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
    esperar_libre(&p).await;

    // En el Historial: aplicado, sin valores y sin «Deshacer».
    let (id, estado, error, nota, titulo) = ultima_fila(&p).await;
    assert_eq!(estado, "aplicado");
    assert_eq!(error, None);
    assert!(nota.is_some_and(|n| n.contains("no se puede deshacer")));
    assert_eq!(
        titulo.as_deref(),
        Some("ajuste-seguridad-contrasena_router-titulo")
    );
    let g = p.state.cambios().unwrap();
    assert!(!g.se_puede_deshacer(id).await);
    let (status, _) = llamar(&p, "deshacer", json!({ "change_id": id }), Some(&cookie)).await;
    assert_ne!(status, 200, "no se deshace");
    let hist = historial_en_texto(&p).await;
    assert!(!hist.contains(NUEVA) && !hist.contains(PASS), "{hist}");
    // La página del Historial lo enseña con su título, sin la API a la vista
    // ni las contraseñas.
    let html = body_text(get_con_cookie(&p.app, "/historial", &cookie).await).await;
    assert!(html.contains("Contraseña del router"), "{html}");
    assert!(!html.contains(NUEVA) && !html.contains(PASS));
}

#[tokio::test]
async fn contrasena_router_rechazada_conserva_la_anterior() {
    let (p, cookie) = preparado().await;
    p.sim.control.modify_pwd_sin_efecto();
    let (status, body) = cambiar(&p, &cookie, NUEVA).await;
    assert_eq!(status, 502, "{body}");
    assert!(body.contains("no aceptó la contraseña nueva"), "{body}");
    assert!(entra(&p, PASS).await);
    assert!(!entra(&p, NUEVA).await);
    assert_eq!(contrasena_guardada(&p).await, PASS);
    // Sin restaurar: solo el cambio.
    assert_eq!(cambios_de_contrasena(&p).len(), 1);
    // El panel sigue entrando.
    p.sim.control.invalidate_sessions();
    hgu::read::summary(&p.state.hgu).await.unwrap();
    let (_, estado, error, _, _) = ultima_fila(&p).await;
    assert_eq!(estado, "fallido");
    assert!(error.is_some_and(|e| e.contains("no aceptó")));
}

#[tokio::test]
async fn contrasena_router_sin_entrar_restaura() {
    let (p, cookie) = preparado().await;
    // El router acepta el cambio, pero luego no deja entrar con la nueva.
    p.sim.control.login_falla_con(NUEVA);
    let (status, body) = cambiar(&p, &cookie, NUEVA).await;
    assert_eq!(status, 502, "{body}");
    assert!(body.contains("ha vuelto a poner la anterior"), "{body}");
    // El sim vuelve a la actual y las credenciales no cambian.
    assert!(entra(&p, PASS).await);
    assert_eq!(contrasena_guardada(&p).await, PASS);
    // Ida y vuelta, simétricas y con la misma codificación.
    let cambios = cambios_de_contrasena(&p);
    assert_eq!(cambios.len(), 2, "{cambios:?}");
    assert_eq!(valor(&cambios[0], "rqOPwd"), PASS);
    assert_eq!(valor(&cambios[0], "rqNPwd"), NUEVA);
    assert_eq!(valor(&cambios[1], "rqUser"), USER);
    assert_eq!(valor(&cambios[1], "rqOPwd"), NUEVA);
    assert_eq!(valor(&cambios[1], "rqNPwd"), PASS);
    p.sim.control.invalidate_sessions();
    hgu::read::summary(&p.state.hgu).await.unwrap();
}

#[tokio::test]
async fn contrasena_router_grave_explica_como_recuperar() {
    let (p, cookie) = preparado().await;
    // No entra con la nueva y la vuelta atrás tampoco se aplica.
    p.sim.control.login_falla_con(NUEVA);
    p.sim.control.fail_write_n(2);
    let (status, body) = cambiar(&p, &cookie, NUEVA).await;
    assert_eq!(status, 502, "{body}");
    assert!(
        body.contains("ni con la contraseña nueva ni con la anterior"),
        "{body}"
    );
    assert!(
        body.contains("pon a mano la contraseña que el panel tiene guardada"),
        "{body}"
    );
    assert!(!body.contains(NUEVA) && !body.contains(PASS), "{body}");
    // El panel no cambia sus credenciales.
    assert_eq!(contrasena_guardada(&p).await, PASS);
    let (_, estado, _, _, _) = ultima_fila(&p).await;
    assert_eq!(estado, "fallido");
}

#[tokio::test]
async fn contrasena_router_con_cambio_activo() {
    let (p, cookie) = preparado().await;
    // Un cambio que se queda a medias: el router deja de responder tras la
    // escritura.
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetUpnpCfg", "instancia": null,
            "valores": [["adm_state", "1"]], "borrar": false } }),
        &cookie,
    )
    .await;
    p.sim.control.hang_after_next_write();
    let cambio = {
        let app = p.app.clone();
        let mut req = post_json(
            "/api/confirmar_cambio",
            &json!({ "token": prep["token"], "palabra": null }).to_string(),
            "127.0.0.1",
        );
        req.headers_mut().insert("cookie", cookie.parse().unwrap());
        tokio::spawn(async move { send(&app, req).await.status() })
    };
    let gestor = p.state.cambios().unwrap();
    let inicio = Instant::now();
    while gestor.activo().is_none() || p.sim.control.writes().is_empty() {
        assert!(
            inicio.elapsed() < Duration::from_secs(5),
            "sin cambio activo"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let (status, body) = cambiar(&p, &cookie, NUEVA).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("otro cambio en curso"), "{body}");
    assert!(cambios_de_contrasena(&p).is_empty());
    assert_eq!(contrasena_guardada(&p).await, PASS);

    p.sim.control.set_unresponsive(false);
    let _ = cambio.await;
    esperar_libre(&p).await;
}

#[tokio::test]
async fn contrasena_router_exige_reautenticacion() {
    registro_global();
    let (p, cookie) = panel_con_gestor().await;
    for f in ["comprobar_contrasena_router", "cambiar_contrasena_router"] {
        let (status, body) = llamar(&p, f, json!({ "nueva": NUEVA }), Some(&cookie)).await;
        assert_eq!(status, 401, "{f}: {body}");
        assert!(body.contains(panel::auth::ERROR_REAUTENTICACION), "{body}");
        // Sin sesión, tampoco.
        let (status, _) = llamar(&p, f, json!({ "nueva": NUEVA }), None).await;
        assert_eq!(status, 401, "{f}");
    }
    assert!(cambios_de_contrasena(&p).is_empty());
    assert_eq!(contrasena_guardada(&p).await, PASS);
    // Con ella, la comprobación pasa y no escribe nada.
    reautenticar(&p, &cookie).await;
    ok(
        &p,
        "comprobar_contrasena_router",
        json!({ "nueva": NUEVA }),
        &cookie,
    )
    .await;
    assert!(cambios_de_contrasena(&p).is_empty());
}

#[tokio::test]
async fn contrasena_router_valida_la_nueva() {
    let (p, cookie) = preparado().await;
    for (nueva, texto) in [
        ("corta", "demasiado corta"),
        (&*"x".repeat(65), "demasiado larga"),
        ("clave\nde-prueba", "saltos de línea"),
        (PASS, "la misma que ya tiene"),
    ] {
        for f in ["comprobar_contrasena_router", "cambiar_contrasena_router"] {
            let (status, body) = llamar(&p, f, json!({ "nueva": nueva }), Some(&cookie)).await;
            assert_eq!(status, 400, "{f} {nueva:?}: {body}");
            assert!(body.contains(texto), "{f} {nueva:?}: {body}");
        }
    }
    assert!(cambios_de_contrasena(&p).is_empty());
    // En solo lectura (firmware sin probar), no.
    instancia::escribir(&p.state.db, instancia::claves::SOLO_LECTURA, &json!(true))
        .await
        .unwrap();
    let (status, body) = cambiar(&p, &cookie, NUEVA).await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("solo lectura"), "{body}");
    assert!(cambios_de_contrasena(&p).is_empty());
}

/// Ninguna contraseña en el registro, en las respuestas ni en el Historial,
/// que sí recogen lo que se ha hecho.
#[tokio::test]
async fn contrasena_router_no_aparece_en_registros() {
    let registro = registro_global();
    let (p, cookie) = preparado().await;
    // Uno que se restaura y otro que sale bien.
    p.sim.control.login_falla_con(NUEVA);
    let (status, fallido) = cambiar(&p, &cookie, NUEVA).await;
    assert_eq!(status, 502);
    let otra = "otra-clave-nueva-de-prueba";
    let mut req = post_json(
        "/api/cambiar_contrasena_router",
        &json!({ "nueva": otra }).to_string(),
        "127.0.0.1",
    );
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    let resp = send(&p.app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()["cache-control"], "no-store");
    let bien = body_text(resp).await;
    for texto in [&fallido, &bien, &historial_en_texto(&p).await] {
        for clave in [PASS, NUEVA, otra] {
            assert!(!texto.contains(clave), "{clave} en {texto}");
        }
    }
    let texto = registro.texto();
    assert!(
        texto.contains("contraseña del router cambiada"),
        "el registro no recoge nada: {texto}"
    );
    assert!(
        texto.contains("se ha vuelto a poner la anterior"),
        "{texto}"
    );
    for clave in [PASS, NUEVA, otra] {
        assert!(!texto.contains(clave), "{clave} en el registro");
    }
}

// El nombre del brief, con el de la función del router.
#[allow(non_snake_case)]
#[tokio::test]
async fn biblioteca_sigue_bloqueando_cbModifyPwd() {
    let (p, cookie) = preparado().await;
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbModifyPwd", "instancia": null,
            "valores": [["rqUser", USER], ["rqOPwd", PASS], ["rqNPwd", NUEVA]],
            "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert!((400..500).contains(&status), "{status}: {body}");
    assert!(body.contains("bloqueada"), "{body}");
    assert!(p.sim.control.writes().is_empty());
    // Y el gestor la rechaza también sin pasar por la Biblioteca.
    let g = p.state.cambios().unwrap();
    assert!(g.escribible("cbModifyPwd").is_err());
    assert!(entra(&p, PASS).await);
}

// ---- Tarjeta de las credenciales: la contraseña guardada, con el ojo

#[tokio::test]
async fn credenciales_llegan_con_la_contrasena_guardada() {
    let (p, cookie) = panel_con_gestor().await;
    let v: Value = ok(&p, "leer_conexion_router", json!({}), &cookie).await;
    assert_eq!(v["contrasena"], PASS);
    // «Ajustes del panel» no la lleva.
    let (_, panel) = llamar(&p, "leer_panel", json!({}), Some(&cookie)).await;
    assert!(!panel.contains(PASS), "{panel}");
    let resp = get_con_cookie(&p.app, "/panel", &cookie).await;
    assert!(!body_text(resp).await.contains(PASS));
    // La página la lleva solo en el campo, sin caché.
    let resp = get_con_cookie(&p.app, "/router/conexion", &cookie).await;
    assert_eq!(resp.headers()["cache-control"], "no-store");
    let html = body_text(resp).await;
    assert!(html.contains(&format!("value=\"{PASS}\"")), "{html}");
    for marca in [
        "data-1p-ignore",
        "data-lpignore=\"true\"",
        "data-bwignore",
        "data-form-type=\"other\"",
        "autocomplete=\"new-password\"",
    ] {
        assert!(html.contains(marca), "{marca}");
    }
    // Con su nombre, no con el prefijo de Leptos para componentes.
    assert!(!html.contains("attr:data-"), "{html}");
    assert!(html.contains("Cambiar la contraseña del router"));
    assert!(html.contains("Contraseña con la que el panel entra en el router"));
}

#[tokio::test]
async fn credenciales_sin_tocar_la_contrasena_se_queda_la_guardada() {
    let (p, cookie) = preparado().await;
    let sin = json!({ "url": p.sim.base_url, "usuario": USER, "contrasena": null });
    let r = ok(&p, "panel_probar_router", sin.clone(), &cookie).await;
    assert_eq!(r["modelo"], "RTF8115VW", "{r}");
    ok(&p, "panel_guardar_router", sin, &cookie).await;
    assert_eq!(contrasena_guardada(&p).await, PASS);
    // Sin el campo, igual.
    ok(
        &p,
        "panel_guardar_router",
        json!({ "url": p.sim.base_url, "usuario": USER }),
        &cookie,
    )
    .await;
    assert_eq!(contrasena_guardada(&p).await, PASS);
    // Con otra, la prueba (y no entra).
    let (status, _) = llamar(
        &p,
        "panel_guardar_router",
        json!({ "url": p.sim.base_url, "usuario": USER, "contrasena": NUEVA }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 502);
    assert_eq!(contrasena_guardada(&p).await, PASS);
}

/// Sin gestor de cambios instalado (como el panel recién instalado antes de
/// arrancarlo), el flujo funciona igual.
#[tokio::test]
async fn contrasena_router_sin_gestor() {
    registro_global();
    let p = common::panel().await;
    let cookie = sesion(&p).await;
    reautenticar(&p, &cookie).await;
    let (status, body) = cambiar(&p, &cookie, NUEVA).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(contrasena_guardada(&p).await, NUEVA);
}
