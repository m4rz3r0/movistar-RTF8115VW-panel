//! Pruebas de los datos de la instancia: credenciales del router cifradas en
//! la base de datos, su importación desde el entorno y el arranque sin
//! router configurado.

mod common;

use common::{Panel, body_text, get, get_con_cookie, sesion};
use panel::{
    config::Config,
    crypto::HistoryKey,
    db,
    instancia::{self, CredencialesRouter, claves},
    poller::{Poller, PollerConfig},
};
use serde_json::{Value, json};
use std::{
    path::Path,
    time::{Duration, Instant},
};

const CLAVE_ROUTER: &str = "contraseña-del-router-1";

fn credenciales() -> CredencialesRouter {
    CredencialesRouter {
        url: "http://192.0.2.1".into(),
        usuario: "admin".into(),
        contrasena: CLAVE_ROUTER.into(),
    }
}

/// Escribe el fichero de la contraseña del router (600).
fn escribir_secreto(path: &Path, texto: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, format!("{texto}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}

/// Configuración con las tres variables del router apuntando a `url`.
fn config_con_router(dir: &Path, url: &str) -> Config {
    let secreto = dir.join("router.secret");
    Config {
        router_url: Some(url.into()),
        router_user: Some(common::USER.into()),
        router_secret_file: Some(secreto),
        ..common::config_en(dir, "127.0.0.1".parse().unwrap())
    }
}

#[tokio::test]
async fn credenciales_cifradas_en_reposo() {
    let dir = tempfile::tempdir().unwrap();
    let pool = db::open(&dir.path().join("panel.db")).await.unwrap();
    let clave = HistoryKey::load_or_create(&dir.path().join("history.key")).unwrap();
    assert_eq!(
        instancia::leer_credenciales(&pool, &clave).await.unwrap(),
        None
    );

    instancia::guardar_credenciales(&pool, &clave, &credenciales())
        .await
        .unwrap();
    let (url, usuario, blob): (String, String, Vec<u8>) =
        sqlx::query_as("SELECT url, usuario, contrasena FROM router_credenciales")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        (url.as_str(), usuario.as_str()),
        ("http://192.0.2.1", "admin")
    );
    assert!(
        !blob
            .windows(CLAVE_ROUTER.len())
            .any(|w| w == CLAVE_ROUTER.as_bytes()),
        "la contraseña está en claro en la base de datos"
    );
    assert_eq!(
        instancia::leer_credenciales(&pool, &clave).await.unwrap(),
        Some(credenciales())
    );

    // Guardar de nuevo sustituye la única fila.
    let otras = CredencialesRouter {
        contrasena: "otra".into(),
        ..credenciales()
    };
    instancia::guardar_credenciales(&pool, &clave, &otras)
        .await
        .unwrap();
    assert_eq!(
        instancia::leer_credenciales(&pool, &clave).await.unwrap(),
        Some(otras)
    );

    // Con otra clave maestra no se descifra, y el error no lleva el secreto.
    let ajena = HistoryKey::load_or_create(&dir.path().join("otra.key")).unwrap();
    let e = instancia::leer_credenciales(&pool, &ajena)
        .await
        .unwrap_err();
    assert!(!e.contains("otra"), "{e}");
}

#[tokio::test]
async fn ajustes_de_la_instancia() {
    let dir = tempfile::tempdir().unwrap();
    let pool = db::open(&dir.path().join("panel.db")).await.unwrap();
    assert_eq!(instancia::leer(&pool, claves::IDIOMA).await, None);
    instancia::escribir(&pool, claves::IDIOMA, &json!("es"))
        .await
        .unwrap();
    instancia::escribir(&pool, claves::DNS_PREFERIDOS, &json!(["9.9.9.9"]))
        .await
        .unwrap();
    instancia::escribir(&pool, claves::IDIOMA, &json!("en"))
        .await
        .unwrap();
    assert_eq!(
        instancia::leer(&pool, claves::IDIOMA).await,
        Some(json!("en"))
    );
    assert_eq!(
        instancia::leer(&pool, claves::DNS_PREFERIDOS).await,
        Some(json!(["9.9.9.9"]))
    );
}

#[tokio::test]
async fn importa_credenciales_del_entorno_una_vez() {
    let dir = tempfile::tempdir().unwrap();
    escribir_secreto(&dir.path().join("router.secret"), "primera");
    let state = common::estado(
        config_con_router(dir.path(), "http://192.0.2.1"),
        dir.path(),
    )
    .await;
    assert!(!state.hgu.configurado());
    instancia::arrancar_router(&state).await.unwrap();
    assert!(state.hgu.configurado());
    let guardadas = instancia::leer_credenciales(&state.db, &state.history_key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        guardadas,
        CredencialesRouter {
            url: "http://192.0.2.1".into(),
            usuario: common::USER.into(),
            contrasena: "primera".into(),
        }
    );
    state.db.close().await;
    drop(state);

    // Segundo arranque con otro contenido en el fichero: manda la base de
    // datos.
    escribir_secreto(&dir.path().join("router.secret"), "segunda");
    let state = common::estado(
        config_con_router(dir.path(), "http://192.0.2.9"),
        dir.path(),
    )
    .await;
    instancia::arrancar_router(&state).await.unwrap();
    assert!(state.hgu.configurado());
    let guardadas = instancia::leer_credenciales(&state.db, &state.history_key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(guardadas.contrasena, "primera");
    assert_eq!(guardadas.url, "http://192.0.2.1");
}

#[tokio::test]
async fn secreto_legible_por_otros_impide_importar() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let secreto = dir.path().join("router.secret");
    escribir_secreto(&secreto, "no-debe-salir");
    std::fs::set_permissions(&secreto, std::fs::Permissions::from_mode(0o644)).unwrap();
    let state = common::estado(
        config_con_router(dir.path(), "http://192.0.2.1"),
        dir.path(),
    )
    .await;
    let e = instancia::arrancar_router(&state).await.unwrap_err();
    assert!(e.contains("permisos"), "{e}");
    assert!(!e.contains("no-debe-salir"), "{e}");
    assert!(!state.hgu.configurado());
    assert_eq!(
        instancia::leer_credenciales(&state.db, &state.history_key)
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn variables_router_parciales_fallan() {
    let dir = tempfile::tempdir().unwrap();
    escribir_secreto(&dir.path().join("router.secret"), "no-debe-salir");
    let completa = config_con_router(dir.path(), "http://192.0.2.1");
    let parciales = [
        Config {
            router_user: None,
            router_secret_file: None,
            ..completa.clone()
        },
        Config {
            router_url: None,
            ..completa.clone()
        },
        Config {
            router_secret_file: None,
            ..completa.clone()
        },
    ];
    for cfg in parciales {
        let state = common::estado(cfg, dir.path()).await;
        let e = instancia::arrancar_router(&state).await.unwrap_err();
        for var in ["RP_ROUTER_URL", "RP_ROUTER_USER", "RP_ROUTER_SECRET"] {
            assert!(e.contains(var), "{e}");
        }
        assert!(!e.contains("no-debe-salir"), "{e}");
        assert!(!state.hgu.configurado());
        state.db.close().await;
    }
}

async fn sin_router() -> Panel {
    common::panel_sin_router("127.0.0.1".parse().unwrap()).await
}

#[tokio::test]
async fn sin_credenciales_queda_sin_configurar() {
    let p = sin_router().await;
    assert!(!p.state.hgu.configurado());
    let body: Value = serde_json::from_str(&body_text(get(&p.app, "/salud").await).await).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["router"], "sin_configurar");
}

#[tokio::test]
async fn sondeo_sin_configurar_no_cuenta_como_fallo() {
    let p = sin_router().await;
    let h = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(20),
            ..PollerConfig::default()
        },
    );
    let start = Instant::now();
    while !h.sin_configurar() {
        assert!(start.elapsed() < Duration::from_secs(5), "no llega");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    // Varios ciclos después sigue igual, sin fallos ni error.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(h.sin_configurar());
    assert_eq!(h.failures(), 0);
    assert_eq!(h.error(), None);
    assert!(h.latest().is_none());
}

#[tokio::test]
async fn estado_muestra_router_sin_configurar() {
    let p = sin_router().await;
    let cookie = sesion(&p).await;
    let (status, body) = common::llamar(&p, "estado_actual", json!({}), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["sin_configurar"], true);
    assert_eq!(v["error"], Value::Null);
    assert_eq!(v["avisos"], json!([]));

    let html = body_text(get_con_cookie(&p.app, "/", &cookie).await).await;
    assert!(html.contains("Router sin configurar"), "{html}");
    assert!(
        html.contains("Configúralo en Router → Conexión con el router"),
        "{html}"
    );
}
