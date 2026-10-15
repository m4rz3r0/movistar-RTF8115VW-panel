//! Lecturas para mostrar (`leer_tema`, `leer_dispositivos`,
//! `leer_red_casa`): caché corta que se invalida al escribir y tanda sin
//! lecturas de otros intercaladas. Lo que prepara un cambio sigue leyendo el
//! router.

mod common;

use common::{Panel, esperar_final, esperar_libre, ok, panel, panel_con_gestor, sesion};
use panel::poller::{Poller, PollerConfig};
use serde_json::{Value, json};
use std::time::Duration;

async fn leer(p: &Panel, cookie: &str, tema: &str) -> Value {
    ok(p, "leer_tema", json!({ "tema": tema }), cookie).await
}

/// Lecturas que ha recibido el simulador desde `desde`.
fn lecturas_desde(p: &Panel, desde: usize) -> Vec<(String, Vec<(String, String)>)> {
    p.sim.control.reads().into_iter().skip(desde).collect()
}

/// Valor legible del campo `campo` del ajuste `ajuste` en un tema leído.
fn valor(tema: &Value, ajuste: &str, campo: &str) -> Value {
    let a = tema["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == ajuste)
        .unwrap_or_else(|| panic!("falta {ajuste}: {tema}"));
    a["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == campo)
        .unwrap_or_else(|| panic!("falta {campo}: {a}"))["valor"]
        .clone()
}

#[tokio::test]
async fn volver_a_un_tema_no_lee_el_router() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    for tema in ["wifi", "red_local", "seguridad"] {
        let antes = p.sim.control.reads().len();
        let primera = leer(&p, &cookie, tema).await;
        assert!(!lecturas_desde(&p, antes).is_empty(), "{tema}");
        let antes = p.sim.control.reads().len();
        let segunda = leer(&p, &cookie, tema).await;
        assert_eq!(lecturas_desde(&p, antes), vec![], "{tema}");
        assert_eq!(primera, segunda, "{tema}");
    }
}

/// Red local lee en su tanda también lo que piden después sus tarjetas de
/// dispositivos y reservas: la lista y la red de casa salen de la caché.
#[tokio::test]
async fn dispositivos_y_red_de_casa_tambien_de_la_cache() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    leer(&p, &cookie, "red_local").await;
    let antes = p.sim.control.reads().len();
    let dispositivos = ok(&p, "leer_dispositivos", json!({}), &cookie).await;
    let red = ok(&p, "leer_red_casa", json!({}), &cookie).await;
    assert_eq!(lecturas_desde(&p, antes), vec![]);
    assert!(
        !dispositivos.as_array().unwrap().is_empty(),
        "{dispositivos}"
    );
    assert!(red.is_array(), "{red}");
}

/// Sin pasar por Red local, `leer_dispositivos` y `leer_red_casa` también
/// guardan lo leído.
#[tokio::test]
async fn dispositivos_y_red_de_casa_se_guardan() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let primera = ok(&p, "leer_dispositivos", json!({}), &cookie).await;
    let red = ok(&p, "leer_red_casa", json!({}), &cookie).await;
    let antes = p.sim.control.reads().len();
    assert_eq!(
        ok(&p, "leer_dispositivos", json!({}), &cookie).await,
        primera
    );
    assert_eq!(ok(&p, "leer_red_casa", json!({}), &cookie).await, red);
    assert_eq!(lecturas_desde(&p, antes), vec![]);
}

/// Tras un cambio por el gestor, la siguiente lectura va al router y trae
/// el valor nuevo.
#[tokio::test]
async fn tras_escribir_vuelve_a_leer_el_valor_nuevo() {
    let (p, cookie) = panel_con_gestor().await;
    let antes = leer(&p, &cookie, "puertos").await;
    assert_eq!(
        valor(&antes, "puertos.upnp", "activo"),
        json!({ "Booleano": false })
    );
    let g = ok(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "puertos.upnp", "instancia": null, "borrar": false,
                       "valores": { "activo": "true" } } }),
        &cookie,
    )
    .await;
    let id = ok(
        &p,
        "confirmar_ajuste",
        json!({ "token": g["token"], "desmarcados": [] }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    esperar_libre(&p).await;
    let desde = p.sim.control.reads().len();
    let despues = leer(&p, &cookie, "puertos").await;
    assert!(
        lecturas_desde(&p, desde)
            .iter()
            .any(|(n, _)| n == "cbGetUpnpCfg"),
        "no volvió a leer"
    );
    assert_eq!(
        valor(&despues, "puertos.upnp", "activo"),
        json!({ "Booleano": true })
    );
}

/// Una escritura por el cliente del panel (aunque no pase por el gestor)
/// invalida lo guardado.
#[tokio::test]
async fn cualquier_escritura_del_cliente_invalida() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    leer(&p, &cookie, "puertos").await;
    p.state
        .hgu
        .write("upnp.asp", "cbSetUpnpCfg", &[("adm_state", "1")])
        .await
        .unwrap();
    let despues = leer(&p, &cookie, "puertos").await;
    assert_eq!(
        valor(&despues, "puertos.upnp", "activo"),
        json!({ "Booleano": true })
    );
}

#[tokio::test]
async fn tras_caducar_vuelve_a_leer() {
    let mut p = panel().await;
    p.state.lecturas = std::sync::Arc::new(panel::lecturas::CacheLecturas::new(
        Duration::from_millis(300),
    ));
    p.app = panel::build_router(p.state.clone());
    let cookie = sesion(&p).await;
    leer(&p, &cookie, "sistema").await;
    let antes = p.sim.control.reads().len();
    leer(&p, &cookie, "sistema").await;
    assert_eq!(lecturas_desde(&p, antes), vec![]);
    tokio::time::sleep(Duration::from_millis(400)).await;
    leer(&p, &cookie, "sistema").await;
    assert_eq!(lecturas_desde(&p, antes).len(), 2);
}

/// `preparar_ajuste` lee el router aunque la página se pintase de la caché:
/// el «antes» es lo que tiene el router ahora, cambiado fuera del panel.
#[tokio::test]
async fn preparar_ajuste_no_usa_la_cache() {
    let (p, cookie) = panel_con_gestor().await;
    let pagina = leer(&p, &cookie, "wifi").await;
    assert_ne!(
        valor(&pagina, "wifi.red_24", "nombre"),
        json!({ "Texto": "cambiado-fuera" })
    );
    // Otro cliente (como la app del operador): el panel no se entera.
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, common::USER, common::PASS);
    cfg.min_gap = Duration::from_millis(1);
    hgu::Client::new(cfg)
        .write(
            "wifi.asp",
            "cbSetWifiAccPt",
            &[("dpm_cfg_wifi_acc_pt_id", "0"), ("ssid", "cambiado-fuera")],
        )
        .await
        .unwrap();
    // Para mostrar, aún lo guardado…
    let pagina = leer(&p, &cookie, "wifi").await;
    assert_ne!(
        valor(&pagina, "wifi.red_24", "nombre"),
        json!({ "Texto": "cambiado-fuera" })
    );
    // …pero el resumen del cambio parte de lo que hay en el router.
    let g = ok(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "wifi.red_24", "instancia": null, "borrar": false,
                       "valores": { "nombre": "casa-nueva" } } }),
        &cookie,
    )
    .await;
    let antes = &g["pasos"][0]["campos"][0]["antes"];
    assert_eq!(antes, "cambiado-fuera", "{g}");
}

/// Con el sondeo en marcha y un router lento, las lecturas de un tema llegan
/// seguidas: el sondeo no se cuela entre ellas.
#[tokio::test]
async fn las_lecturas_de_un_tema_van_seguidas() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    p.sim.control.set_retardo(Duration::from_millis(20));
    // Un sondeo que siempre quiere el turno.
    let sondeo = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(10),
            slow: Duration::from_millis(10),
            ..PollerConfig::default()
        },
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    // Puertos: tres lecturas que el sondeo no hace.
    let del_tema: std::collections::BTreeSet<String> = p
        .state
        .ajustes
        .por_tema(ajustes::Tema::from_slug("puertos").unwrap())
        .map(|a| a.leer.funcion.clone())
        .collect();
    assert_eq!(del_tema.len(), 3);
    let antes = p.sim.control.reads().len();
    leer(&p, &cookie, "puertos").await;
    let leidas: Vec<String> = lecturas_desde(&p, antes)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    drop(sondeo);
    assert!(
        leidas.iter().any(|n| !del_tema.contains(n)),
        "el sondeo no leyó nada mientras: la prueba no prueba nada ({leidas:?})"
    );
    let primera = leidas
        .iter()
        .position(|n| del_tema.contains(n))
        .expect("leyó el tema");
    let seguidas = &leidas[primera..primera + del_tema.len()];
    assert!(
        seguidas.iter().all(|n| del_tema.contains(n)),
        "lecturas del sondeo entre las del tema: {leidas:?}"
    );
}
