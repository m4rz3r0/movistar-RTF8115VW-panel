//! Contraseña del router aplicada justo al agotarse el plazo del panel.

use crate::comun::{PASS, Panel, PanelPrueba, USER, llamar, reautenticar, registro_global};
use hgu::{Client, ClientConfig};
use panel::instancia;
use serde_json::json;
use std::time::Duration;

const NUEVA: &str = "clave-nueva-de-prueba";

/// Inicios de sesión con el router desde el cambio hasta que se agota el
/// plazo (de 300 ms, un solo reintento): nueva, anterior y la nueva otra
/// vez. El siguiente, la comprobación final con la anterior, ya la ve
/// aplicada: como un router que la aplica justo después del plazo.
const INICIOS_HASTA_EL_PLAZO: u32 = 3;

/// Panel con el gestor, el plazo corto y el router que aplica la nueva al
/// acabar el plazo.
async fn preparado() -> (Panel, String) {
    registro_global();
    let (mut p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    reautenticar(&p, &cookie).await;
    p.state.plazo_rechazada = Duration::from_millis(300);
    p.app = panel::build_router(p.state.clone());
    p.sim
        .control
        .modify_pwd_tras_inicios(INICIOS_HASTA_EL_PLAZO);
    (p, cookie)
}

async fn cambiar(p: &Panel, cookie: &str) -> (u16, String) {
    llamar(
        p,
        "cambiar_contrasena_router",
        json!({ "nueva": NUEVA }),
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

async fn estado_en_el_historial(p: &Panel) -> String {
    sqlx::query_scalar(
        "SELECT estado FROM changes WHERE funcion = 'cbModifyPwd' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&p.state.db)
    .await
    .unwrap()
}

/// Al agotar el plazo la anterior ya no entra y la nueva sí: se guarda la
/// nueva, no se da por rechazada.
#[tokio::test]
async fn contrasena_router_aplicada_al_acabar_el_plazo_se_guarda() {
    let (p, cookie) = preparado().await;
    let (status, body) = cambiar(&p, &cookie).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(contrasena_guardada(&p).await, NUEVA);
    assert!(entra(&p, NUEVA).await);
    assert!(!entra(&p, PASS).await);
    assert_eq!(estado_en_el_historial(&p).await, "aplicado");
    // El panel entra con la nueva.
    p.sim.control.invalidate_sessions();
    hgu::read::summary(&p.state.hgu).await.unwrap();
}

/// Al agotar el plazo no entra ni la anterior ni la nueva: se vuelve a poner
/// la anterior, como cuando no entra ninguna desde el principio.
#[tokio::test]
async fn contrasena_router_sin_ninguna_al_acabar_el_plazo_restaura() {
    let (p, cookie) = preparado().await;
    p.sim.control.login_falla_con(NUEVA);
    let (status, body) = cambiar(&p, &cookie).await;
    assert_eq!(status, 502, "{body}");
    assert!(body.contains("ha vuelto a poner la anterior"), "{body}");
    assert!(entra(&p, PASS).await);
    assert_eq!(contrasena_guardada(&p).await, PASS);
    assert_eq!(estado_en_el_historial(&p).await, "fallido");
    p.sim.control.invalidate_sessions();
    hgu::read::summary(&p.state.hgu).await.unwrap();
}
