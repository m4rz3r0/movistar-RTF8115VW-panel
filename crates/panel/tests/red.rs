//! Pruebas de la detección de la red del router (conexión a internet, LAN,
//! telefonía, televisión y radios) y de la red de seguridad WAN del gestor
//! de cambios con conexiones distintas de `ip2`.

mod comun;

use comun::{
    Panel, PanelPrueba, body_text, esperar_hasta, esperar_hasta_con, get_con_cookie, sesion,
};
use hgu::{
    deteccion::RedDetectada,
    models::{TipoConexion, WanConn},
};
use panel::{
    changes::{ChangeManager, ChangeRequest, Estado, Tiempos, plan::wan_arriba},
    instancia::{self, CredencialesRouter, claves},
    poller::{Poller, PollerConfig},
};
use router_sim::Sim;
use serde_json::json;
use std::{collections::BTreeMap, path::Path, time::Duration};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../hgu/fixtures");

/// Conexiones de la fixture de serie: `ip2` (datos, PPPoE), `ip3` (VoIP) e
/// `ip4` (IPTV), las tres arriba.
fn conexiones() -> Vec<WanConn> {
    let raw = std::fs::read_to_string(format!("{FIXTURES}/cbGetWanIPIntf.xml")).unwrap();
    WanConn::parse_all(&hgu::xml::parse_response("cbGetWanIPIntf", &raw).unwrap())
}

/// Una instancia que ya tenía credenciales (actualizada desde una versión
/// sin detección) detecta `ip2` al arrancar y lo guarda.
#[tokio::test]
async fn instancia_existente_detecta_ip2_al_arrancar() {
    let sim = Sim::start(comun::sim_config()).await;
    let dir = tempfile::tempdir().unwrap();
    let state = comun::estado(comun::config_en(dir.path()), dir.path()).await;
    instancia::guardar_credenciales(
        &state.db,
        &state.history_key,
        &CredencialesRouter {
            url: sim.base_url.clone(),
            usuario: comun::USER.into(),
            contrasena: comun::PASS.into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        instancia::leer(&state.db, claves::RED_DETECTADA).await,
        None
    );
    assert_eq!(state.wan(), None);

    instancia::arrancar_router(&state).await.unwrap();

    assert_eq!(state.wan().as_deref(), Some("ip2"));
    let red = state.red().unwrap();
    assert_eq!(red.tipo, Some(TipoConexion::Pppoe));
    // La subred exacta la comprueban las pruebas de `hgu::deteccion`.
    assert!(red.lan.as_deref().is_some_and(|l| l.ends_with("/24")));
    assert_eq!(red.router_lan, Some("192.168.1.1".parse().unwrap()));
    assert!(red.voip && red.iptv);
    assert_eq!(red.radios, ["0", "1"]);
    let guardada: RedDetectada = serde_json::from_value(
        instancia::leer(&state.db, claves::RED_DETECTADA)
            .await
            .expect("red_detectada guardada"),
    )
    .unwrap();
    assert_eq!(guardada, red);

    // Al volver a arrancar, la detección guardada está desde el principio.
    let otro = comun::estado(comun::config_en(dir.path()), dir.path()).await;
    assert_eq!(otro.wan().as_deref(), Some("ip2"));
}

/// La conexión elegida a mano manda sobre la detectada.
#[tokio::test]
async fn wan_manual_manda() {
    let sim = Sim::start(comun::sim_config()).await;
    let dir = tempfile::tempdir().unwrap();
    let db = panel::db::open(&dir.path().join("panel.db")).await.unwrap();
    instancia::escribir(&db, claves::WAN_MANUAL, &json!("ip4"))
        .await
        .unwrap();
    db.close().await;
    let state = comun::estado(comun::config_en(dir.path()), dir.path()).await;
    assert_eq!(state.wan().as_deref(), Some("ip4"));
    instancia::guardar_credenciales(
        &state.db,
        &state.history_key,
        &CredencialesRouter {
            url: sim.base_url.clone(),
            usuario: comun::USER.into(),
            contrasena: comun::PASS.into(),
        },
    )
    .await
    .unwrap();
    instancia::arrancar_router(&state).await.unwrap();
    // Se detecta (y guarda) igual, pero manda la manual.
    assert_eq!(state.red().unwrap().wan.as_deref(), Some("ip2"));
    assert_eq!(state.wan().as_deref(), Some("ip4"));

    // Sin la manual, la detectada.
    state.fijar_red(state.red(), None);
    assert_eq!(state.wan().as_deref(), Some("ip2"));
    state.fijar_red(None, Some("ip3".into()));
    assert_eq!(state.wan().as_deref(), Some("ip3"));
}

#[test]
fn sin_wan_detectada_usa_cualquier_conexion_de_datos() {
    let mut conns = conexiones();
    assert!(wan_arriba(&conns, None));
    assert!(wan_arriba(&conns, Some("ip2")));
    let ip2 = conns.iter().position(|c| c.name == "ip2").unwrap();
    conns[ip2].up = false;
    // Solo quedan arriba la de telefonía y la de televisión.
    assert!(!wan_arriba(&conns, None));
    assert!(!wan_arriba(&conns, Some("ip2")));
    // Una conexión elegida que no existe está caída.
    assert!(!wan_arriba(&conns, Some("ip9")));
    // Una de datos arriba pero sin IPv4 no cuenta.
    conns[ip2].up = true;
    conns[ip2].ipv4 = None;
    assert!(!wan_arriba(&conns, None));
}

/// Gestor con un cliente rápido sobre el panel `p` (vigilancia de 300 ms).
async fn gestor_rapido(p: &mut Panel) -> std::sync::Arc<ChangeManager> {
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, comun::USER, comun::PASS);
    cfg.timeout = Duration::from_millis(500);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    let m = ChangeManager::new(
        p.state.clone(),
        Tiempos {
            bajo: Duration::from_millis(300),
            medio: Duration::from_millis(300),
            alto: Duration::from_millis(600),
            intervalo: Duration::from_millis(30),
            fallos_para_revertir: 3,
            pausa_max: Duration::from_secs(2),
        },
    );
    m.reanudar_pendientes().await;
    m
}

/// Cambia la IP de la reserva 0 (`192.168.1.68` en las fixtures).
fn lease0(ip: &str) -> ChangeRequest {
    ChangeRequest {
        funcion: "cbDhcpV4SvrPoolSetStatic".into(),
        instancia: Some("0".into()),
        valores: [("rtm_cfg_dhcpv4_svr_pool_id", "0"), ("ipv4_addr", ip)]
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect::<BTreeMap<_, _>>(),
        borrar: false,
    }
}

/// Aplica un cambio, corta la WAN durante la vigilancia y espera la
/// reversión; devuelve el motivo.
async fn cortar_wan_y_esperar_reversion(p: &Panel, m: &std::sync::Arc<ChangeManager>) -> String {
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_wan_up(false);
    esperar_hasta_con(Duration::from_secs(10), async || {
        let c = m.cambio(id).await.unwrap();
        if c.estado == Estado::Revertido {
            return Ok(c.error.unwrap_or_default());
        }
        assert!(
            !matches!(c.estado, Estado::Aplicado | Estado::Fallido),
            "no se revirtió: {c:?}"
        );
        Err(format!("sin final: {c:?}"))
    })
    .await
}

/// Línea DHCP sin telefonía (`ip5`): cortar la WAN durante la vigilancia
/// revierte el cambio, con la conexión detectada y sin ella.
#[tokio::test]
async fn vigilancia_revierte_en_linea_dhcp() {
    let mut p = PanelPrueba::nuevo()
        .capa(&comun::variante("dhcp-sin-voip"))
        .arrancar()
        .await;
    assert_eq!(p.state.wan().as_deref(), Some("ip5"));
    assert!(!p.state.red().unwrap().voip);
    let m = gestor_rapido(&mut p).await;
    let motivo = cortar_wan_y_esperar_reversion(&p, &m).await;
    assert!(motivo.contains("WAN"), "{motivo}");

    // Sin nada detectado (la detección falló), igual.
    let mut p = PanelPrueba::nuevo()
        .capa(&comun::variante("dhcp-sin-voip"))
        .arrancar()
        .await;
    p.state.fijar_red(None, None);
    assert_eq!(p.state.wan(), None);
    let m = gestor_rapido(&mut p).await;
    let motivo = cortar_wan_y_esperar_reversion(&p, &m).await;
    assert!(motivo.contains("WAN"), "{motivo}");
}

/// Capa con la que la detección no puede decidir: sin ruta por defecto y con
/// dos conexiones de datos con IP de internet (`ip2` e `ip4`).
fn capa_ambigua(dir: &Path) {
    let wan = std::fs::read_to_string(format!("{FIXTURES}/cbGetWanIPIntf.xml"))
        .unwrap()
        .replace(
            "<opr_app_mask>4</opr_app_mask>",
            "<opr_app_mask>1</opr_app_mask>",
        );
    std::fs::write(dir.join("cbGetWanIPIntf.xml"), wan).unwrap();
    std::fs::write(
        dir.join("cbGetRoutingV4Rule__all=ALL_desc=Default.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\r\n<Response><cbGetRoutingV4Rule>\
         <Result>0</Result></cbGetRoutingV4Rule></Response>",
    )
    .unwrap();
}

#[tokio::test]
async fn estado_sin_wan_avisa() {
    let capa = tempfile::tempdir().unwrap();
    capa_ambigua(capa.path());
    let mut p = PanelPrueba::nuevo().capa(capa.path()).arrancar().await;
    let red = p.state.red().expect("detección hecha");
    assert_eq!(red.wan, None);
    let cookie = sesion(&p).await;
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, comun::USER, comun::PASS);
    cfg.timeout = Duration::from_millis(200);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    let h = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(50),
            slow: Duration::from_millis(200),
            backoff_max: Duration::from_millis(200),
            ..PollerConfig::default()
        },
    );
    esperar_hasta(Duration::from_secs(5), "primer dato", async || h.healthy()).await;
    assert_eq!(h.latest().unwrap().wan_intf, None);
    let html = body_text(get_con_cookie(&p.app, "/", &cookie).await).await;
    assert!(html.contains("Conexión a internet sin detectar"), "{html}");
}

/// Si la conexión a internet desaparece del router, el sondeo vuelve a
/// detectar sin esperar a la hora.
#[tokio::test]
async fn sondeo_redetecta_si_falta_la_wan() {
    let mut p = PanelPrueba::nuevo().arrancar().await;
    let mut red = p.state.red().unwrap();
    red.wan = Some("ip9".into());
    p.state.fijar_red(Some(red), None);
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, comun::USER, comun::PASS);
    cfg.timeout = Duration::from_millis(200);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    let h = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(50),
            slow: Duration::from_millis(200),
            backoff_max: Duration::from_millis(200),
            ..PollerConfig::default()
        },
    );
    let state = p.state.clone();
    esperar_hasta(Duration::from_secs(5), "nueva detección", async || {
        state.wan().as_deref() == Some("ip2")
    })
    .await;
    esperar_hasta(Duration::from_secs(5), "instantánea con ip2", async || {
        h.latest()
            .is_some_and(|s| s.wan_intf.as_deref() == Some("ip2"))
    })
    .await;
}

/// Con un cambio vigilándose, una detección nueva (la del arranque o la de
/// guardar las credenciales, que pueden acabar en segundo plano) no se
/// anota: la red de seguridad del cambio sigue con su conexión.
#[tokio::test]
async fn deteccion_no_se_anota_con_cambio_activo() {
    let mut p = PanelPrueba::nuevo().arrancar().await;
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, comun::USER, comun::PASS);
    cfg.timeout = Duration::from_millis(500);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    // Vigilancia larga: el cambio sigue activo mientras dura la prueba.
    let m = ChangeManager::instalar(
        &p.state,
        Tiempos {
            bajo: Duration::from_secs(60),
            medio: Duration::from_secs(60),
            alto: Duration::from_secs(60),
            intervalo: Duration::from_millis(30),
            fallos_para_revertir: 3,
            pausa_max: Duration::from_secs(2),
        },
    );
    m.reanudar_pendientes().await;
    let mut anterior = p.state.red().unwrap();
    anterior.wan = Some("ip4".into());
    p.state.fijar_red(Some(anterior.clone()), None);
    instancia::escribir(&p.state.db, claves::RED_DETECTADA, &json!(anterior))
        .await
        .unwrap();
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    esperar_hasta(Duration::from_secs(5), "sin vigilancia", async || {
        m.cambio(id).await.unwrap().estado == Estado::Vigilando
    })
    .await;
    assert!(instancia::detectar_y_guardar(&p.state).await.is_err());
    assert_eq!(p.state.wan().as_deref(), Some("ip4"), "no cambia a mitad");
    assert_eq!(p.state.red(), Some(anterior.clone()));
    assert_eq!(
        instancia::leer(&p.state.db, claves::RED_DETECTADA).await,
        Some(json!(anterior))
    );
}

/// Una conexión elegida a mano que ya no existe en el router: la red de
/// seguridad y Estado usan la detectada, y el sondeo no vuelve a detectar en
/// cada vuelta (no serviría de nada).
#[tokio::test]
async fn wan_manual_inexistente_usa_la_detectada() {
    let conns = conexiones();
    let mut p = PanelPrueba::nuevo().arrancar().await;
    let mut red = p.state.red().unwrap();
    // Una marca que una detección nueva borraría.
    red.radios = vec!["9".into()];
    red.momento = panel::series::now();
    p.state.fijar_red(Some(red.clone()), Some("ip9".into()));
    assert_eq!(p.state.wan().as_deref(), Some("ip9"));
    assert_eq!(p.state.wan_en(&conns).as_deref(), Some("ip2"));
    assert!(wan_arriba(&conns, p.state.wan_en(&conns).as_deref()));
    // Con la manual presente, manda la manual.
    p.state.fijar_red(Some(red.clone()), Some("ip4".into()));
    assert_eq!(p.state.wan_en(&conns).as_deref(), Some("ip4"));
    p.state.fijar_red(Some(red.clone()), Some("ip9".into()));

    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, comun::USER, comun::PASS);
    cfg.timeout = Duration::from_millis(200);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    let h = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(50),
            slow: Duration::from_millis(100),
            backoff_max: Duration::from_millis(200),
            ..PollerConfig::default()
        },
    );
    esperar_hasta(Duration::from_secs(5), "instantánea con ip2", async || {
        h.latest()
            .is_some_and(|s| s.wan_intf.as_deref() == Some("ip2"))
    })
    .await;
    // Varias vueltas lentas sin volver a detectar.
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(p.state.red().unwrap().radios, ["9"], "no re-detecta");
    assert_eq!(p.state.wan_manual().as_deref(), Some("ip9"));
}
