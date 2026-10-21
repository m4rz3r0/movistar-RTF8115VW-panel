//! Pruebas del gestor de cambios contra `router-sim`, con tiempos de
//! vigilancia cortos.

use crate::comun::{
    Panel, esperar_estado, esperar_hasta, esperar_hasta_con, esperar_libre_gestor, gestor,
    panel_rapido, preparado, req, tiempos_rapidos,
};
use catalogo::{Riesgo, mask::MASCARA};
use hgu::read;
use panel::{
    changes::{CambioError, ChangeManager, ChangeRequest, Estado, Tiempos},
    poller::{Poller, PollerConfig},
};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

const LEASES: &str = "cbDhcpV4SvrPoolSetStatic";
const POOL: &str = "rtm_cfg_dhcpv4_svr_pool_id";

/// Cambia la IP de la reserva 0 (`192.168.1.68` en las fixtures).
fn lease0(ip: &str) -> ChangeRequest {
    req(LEASES, Some("0"), &[(POOL, "0"), ("ipv4_addr", ip)])
}

async fn ip_de_lease(p: &Panel, id: &str) -> Option<String> {
    read::static_leases(&p.state.hgu)
        .await
        .unwrap()
        .into_iter()
        .find(|l| l.id == id)
        .map(|l| l.ip)
}

fn nueva_reserva() -> ChangeRequest {
    req(
        LEASES,
        None,
        &[
            (POOL, "0"),
            ("adm_state", "1"),
            ("mac_addr", "02:00:00:00:00:99"),
            ("ipv4_addr", "192.168.1.99"),
        ],
    )
}

#[tokio::test]
async fn aplica_y_verifica_static_lease() {
    let (p, m) = preparado().await;
    // Con el sondeo en marcha: el gestor lo pausa para escribir.
    let h = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(20),
            slow: Duration::from_millis(100),
            ..PollerConfig::default()
        },
    );
    let prep = m.preparar(nueva_reserva()).await.unwrap();
    assert_eq!(prep.riesgo, Riesgo::Bajo);
    assert_eq!(prep.palabra, None);
    let campos: Vec<_> = prep
        .diff
        .campos
        .iter()
        .map(|c| (c.nombre.as_str(), c.antes.as_str(), c.despues.as_str()))
        .collect();
    assert!(
        campos.contains(&("ipv4_addr", "", "192.168.1.99")),
        "{campos:?}"
    );
    assert!(campos.contains(&("mac_addr", "", "02:00:00:00:00:99")));

    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    assert_eq!(m.activo(), Some(id));
    let c = esperar_estado(&m, id, Estado::Aplicado).await;
    assert_eq!(c.usuario.as_deref(), Some("admin"));
    assert_eq!(c.funcion, LEASES);
    // La instancia creada queda en el historial (para deshacer).
    assert_eq!(c.instancia.as_deref(), Some("2"));
    assert_eq!(ip_de_lease(&p, "2").await.as_deref(), Some("192.168.1.99"));
    let w = p.sim.control.writes();
    assert_eq!(w.len(), 1);
    let params: BTreeMap<_, _> = w[0].1.iter().cloned().collect();
    assert_eq!(params["rtm_dhcpv4_svr_pool_static_id"], "-1");
    assert_eq!(params["mac_addr"], "02:00:00:00:00:99");
    esperar_libre_gestor(&m).await;
    assert!(h.latest().is_some());
}

#[tokio::test]
async fn edicion_envia_la_instancia_completa() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    assert_eq!(prep.diff.campos.len(), 1);
    let c = &prep.diff.campos[0];
    assert_eq!(
        (c.nombre.as_str(), c.antes.as_str(), c.despues.as_str()),
        ("ipv4_addr", "192.168.1.68", "192.168.1.80")
    );
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    let w = p.sim.control.writes();
    let params: BTreeMap<_, _> = w[0].1.iter().cloned().collect();
    // Los campos no tocados se envían con su valor leído.
    assert_eq!(params["mac_addr"], "02:00:00:00:00:04");
    assert_eq!(params["adm_state"], "1");
    assert_eq!(params["rtm_dhcpv4_svr_pool_static_id"], "0");
    assert_eq!(params[POOL], "0");
    assert!(!params.contains_key("del"));
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
}

#[tokio::test]
async fn sin_cambios_rechazado() {
    let (p, m) = preparado().await;
    let r = m.preparar(lease0("192.168.1.68")).await;
    assert!(matches!(r, Err(CambioError::SinCambios)), "{r:?}");
    // IP/MAC se comparan sin distinguir mayúsculas.
    let r = m
        .preparar(req(
            LEASES,
            Some("0"),
            &[(POOL, "0"), ("mac_addr", "02:00:00:00:00:0A")],
        ))
        .await;
    assert!(r.is_ok(), "MAC distinta: {r:?}");
    let r = m
        .preparar(req(
            LEASES,
            Some("1"),
            &[(POOL, "0"), ("ipv4_addr", " 192.168.1.69 ")],
        ))
        .await;
    assert!(matches!(r, Err(CambioError::SinCambios)), "{r:?}");
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn bloqueada_rechazada() {
    let (p, m) = preparado().await;
    let r = m
        .preparar(req("cbRestoreConfig", None, &[("file", "x")]))
        .await;
    assert!(
        matches!(&r, Err(CambioError::Bloqueada(motivo)) if motivo.contains("configuración")),
        "{r:?}"
    );
    let r = m.preparar(req("cbGetSummary", None, &[])).await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    let r = m.preparar(req("cbNoExiste", None, &[])).await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    let r = m
        .preparar(req(LEASES, Some("0"), &[(POOL, "0"), ("inventado", "1")]))
        .await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    let r = m.preparar(lease0("192.168.1.300")).await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn verificacion_falla_si_router_ignora() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    p.sim.control.ignore_next_write();
    let r = m.confirmar(&prep.token, None, "admin").await;
    let Err(CambioError::Verificacion(fallos)) = r else {
        panic!("{r:?}");
    };
    assert!(fallos.iter().any(|f| f.contains("ipv4_addr")), "{fallos:?}");
    let ultimo = m.historial(1).await.remove(0);
    assert_eq!(ultimo.estado, Estado::Fallido);
    // Sin reversión: una sola escritura.
    assert_eq!(p.sim.control.writes().len(), 1);
    assert_eq!(m.activo(), None);
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

#[tokio::test]
async fn error_del_router_deja_fallido() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    p.sim.control.fail_next_write("no se puede");
    let r = m.confirmar(&prep.token, None, "admin").await;
    assert!(
        matches!(&r, Err(CambioError::Router(e)) if e.contains("no se puede")),
        "{r:?}"
    );
    let ultimo = m.historial(1).await.remove(0);
    assert_eq!(ultimo.estado, Estado::Fallido);
    assert!(ultimo.error.unwrap().contains("no se puede"));
    assert_eq!(m.activo(), None);
}

/// I1: la escritura llega y el router la aplica, pero no contesta (reinicia
/// un servicio, va cargado…): se relee, se ve aplicada y se vigila, con
/// deshacer disponible.
#[tokio::test]
async fn escritura_sin_respuesta_aplicada_se_vigila() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    p.sim.control.no_answer_next_write(true);
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    let c = esperar_estado(&m, id, Estado::Aplicado).await;
    let nota = c.nota.unwrap_or_default();
    assert!(nota.contains("no confirmó"), "{nota}");
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
    esperar_libre_gestor(&m).await;
    assert!(m.se_puede_deshacer(id).await);
}

/// I1: igual, pero la WAN cae durante la vigilancia: se revierte.
#[tokio::test]
async fn escritura_sin_respuesta_aplicada_se_revierte_si_cae_la_wan() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    p.sim.control.no_answer_next_write(true);
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_wan_up(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

/// I1: la escritura se corta sin aplicarse: el router sigue con lo de
/// antes ⇒ fallido («no se aplicó»), sin vigilar ni revertir.
#[tokio::test]
async fn escritura_sin_respuesta_no_aplicada_queda_fallida() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    p.sim.control.no_answer_next_write(false);
    let r = m.confirmar(&prep.token, None, "admin").await;
    assert!(
        matches!(&r, Err(CambioError::Router(e)) if e.contains("no se aplicó")),
        "{r:?}"
    );
    let ultimo = m.historial(1).await.remove(0);
    assert_eq!(ultimo.estado, Estado::Fallido);
    assert_eq!(p.sim.control.writes().len(), 1, "sin reversión");
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
    assert_eq!(m.activo(), None);
}

#[tokio::test]
async fn revierte_si_se_pierde_la_wan() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_wan_up(false);
    let c = esperar_estado(&m, id, Estado::Revertido).await;
    assert!(c.error.unwrap().contains("WAN"));
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
    let w = p.sim.control.writes();
    assert_eq!(w.len(), 2, "cambio y reversión");
    let revert: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(revert["ipv4_addr"], "192.168.1.68");
    assert_eq!(revert["rtm_dhcpv4_svr_pool_static_id"], "0");
    esperar_libre_gestor(&m).await;
}

#[tokio::test]
async fn wan_ya_caida_no_provoca_reversion() {
    let (p, m) = preparado().await;
    p.sim.control.set_wan_up(false);
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    let c = esperar_estado(&m, id, Estado::Aplicado).await;
    assert!(c.nota.unwrap().contains("WAN"));
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
}

#[tokio::test]
async fn revierte_si_el_router_deja_de_responder() {
    let p = panel_rapido(Duration::from_millis(100)).await;
    let m = gestor(&p, tiempos_rapidos()).await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_unresponsive(true);
    // Tres fallos seguidos (100 ms de espera cada uno) y, cuando el router
    // vuelve, se revierte.
    tokio::time::sleep(Duration::from_millis(600)).await;
    p.sim.control.set_unresponsive(false);
    let c = esperar_estado(&m, id, Estado::Revertido).await;
    assert!(c.error.unwrap().contains("no responde"));
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

#[tokio::test]
async fn secreto_enmascarado_no_se_escribe() {
    let (p, m) = preparado().await;
    for intocado in [MASCARA, ""] {
        let ssid = format!("RED-nueva{}", intocado.len());
        let r = req(
            "cbSetWifiAccPt",
            None,
            &[
                ("dpm_cfg_wifi_acc_pt_id", "1"),
                ("ssid", &ssid),
                ("key_phrase", intocado),
                ("wep_key1_data", MASCARA),
            ],
        );
        let prep = m.preparar(r).await.unwrap();
        let nombres: Vec<_> = prep.diff.campos.iter().map(|c| c.nombre.as_str()).collect();
        assert_eq!(nombres, ["ssid"], "el secreto sin tocar no es un cambio");
        let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
        esperar_estado(&m, id, Estado::Aplicado).await;
        esperar_libre_gestor(&m).await;
    }
    let w = p.sim.control.writes();
    assert_eq!(w.len(), 2);
    for (_, params) in &w {
        assert!(
            params.iter().all(|(_, v)| v != MASCARA),
            "MASCARA enviada: {params:?}"
        );
        let params: BTreeMap<_, _> = params.iter().cloned().collect();
        // Se reenvía el valor actual leído del router.
        assert_eq!(params["key_phrase"], "X-REDACTED-X");
        assert_eq!(params["wep_key1_data"], "X-REDACTED-X");
    }
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "1")])
        .await
        .unwrap();
    let ap = n.get("dpm_cfg_wifi_acc_pt").unwrap();
    assert_eq!(ap.text_of("key_phrase"), Some("X-REDACTED-X"));
    assert_eq!(ap.text_of("ssid"), Some("RED-nueva0"));
}

#[tokio::test]
async fn secreto_cambiado_se_enmascara_y_se_guarda_cifrado() {
    let (p, m) = preparado().await;
    let r = req(
        "cbSetWifiAccPt",
        None,
        &[
            ("dpm_cfg_wifi_acc_pt_id", "1"),
            ("key_phrase", "clave-nueva-123"),
        ],
    );
    let prep = m.preparar(r).await.unwrap();
    assert_eq!(prep.diff.campos.len(), 1);
    let c = &prep.diff.campos[0];
    assert!(c.secreto);
    assert_eq!((c.antes.as_str(), c.despues.as_str()), (MASCARA, MASCARA));
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    // Ni el valor nuevo ni el anterior aparecen en claro en la base de datos.
    let fila: (String, String, String, String, Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT diff, anteriores, solicitud, nuevos, anteriores_cifrados, nuevos_cifrados \
         FROM changes WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&p.state.db)
    .await
    .unwrap();
    let texto = format!("{fila:?}");
    assert!(!texto.contains("clave-nueva-123"), "{texto}");
    assert!(!texto.contains("X-REDACTED-X"), "{texto}");
    let cifrado = [fila.4, fila.5].concat();
    assert!(!cifrado.windows(15).any(|w| w == b"clave-nueva-123"));
    // El anterior se recupera para deshacer.
    let prep = m.deshacer(id).await.unwrap();
    let id2 = m.confirmar(&prep.token, None, "admin").await.unwrap();
    esperar_estado(&m, id2, Estado::Aplicado).await;
    let w = p.sim.control.writes();
    let ultimo: BTreeMap<_, _> = w.last().unwrap().1.iter().cloned().collect();
    assert_eq!(ultimo["key_phrase"], "X-REDACTED-X");
}

#[tokio::test]
async fn cambios_concurrentes_no_se_intercalan() {
    let (p, m) = preparado().await;
    let a = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let b = m
        .preparar(req("cbSetUpnpCfg", None, &[("adm_state", "1")]))
        .await
        .unwrap();
    let (ra, rb) = tokio::join!(
        m.confirmar(&a.token, None, "admin"),
        m.confirmar(&b.token, None, "admin")
    );
    let (ok, otro, token_otro) = match (ra, rb) {
        (Ok(id), Err(e)) => (id, e, b.token.clone()),
        (Err(e), Ok(id)) => (id, e, a.token.clone()),
        r => panic!("se esperaba uno aceptado y otro en curso: {r:?}"),
    };
    assert!(matches!(otro, CambioError::EnCurso), "{otro:?}");
    assert_eq!(p.sim.control.writes().len(), 1);
    // Mientras se vigila, sigue ocupado.
    let r = m.confirmar(&token_otro, None, "admin").await;
    assert!(matches!(r, Err(CambioError::EnCurso)), "{r:?}");
    esperar_estado(&m, ok, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    // El token rechazado por estar ocupado sigue valiendo.
    let id = m.confirmar(&token_otro, None, "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    assert_eq!(p.sim.control.writes().len(), 2);
}

#[tokio::test]
async fn token_de_un_solo_uso() {
    let (_p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    let r = m.confirmar(&prep.token, None, "admin").await;
    assert!(matches!(r, Err(CambioError::TokenInvalido)), "{r:?}");
    let r = m.confirmar("inventado", None, "admin").await;
    assert!(matches!(r, Err(CambioError::TokenInvalido)), "{r:?}");
}

#[tokio::test]
async fn pausa_que_no_llega_no_escribe() {
    let p = panel_rapido(Duration::from_secs(5)).await;
    let m = gestor(
        &p,
        Tiempos {
            pausa_max: Duration::from_millis(300),
            ..tiempos_rapidos()
        },
    )
    .await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(20),
            ..PollerConfig::default()
        },
    );
    // El ciclo del sondeo queda colgado esperando al router.
    p.sim.control.set_unresponsive(true);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let r = m.confirmar(&prep.token, None, "admin").await;
    assert!(
        matches!(&r, Err(CambioError::Router(e)) if e.contains("el sondeo no se detuvo a tiempo")),
        "{r:?}"
    );
    p.sim.control.set_unresponsive(false);
    assert!(p.sim.control.writes().is_empty());
    assert_eq!(m.activo(), None);
    assert!(m.historial(10).await.is_empty());
}

#[tokio::test]
async fn reanuda_tras_reinicio() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        ..tiempos_rapidos()
    };
    let a = gestor(&p, lento).await;
    let prep = a.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = a.confirmar(&prep.token, None, "admin").await.unwrap();
    assert_eq!(a.cambio(id).await.unwrap().estado, Estado::Vigilando);
    // El proceso muere a mitad de la vigilancia.
    a.detener();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(a.cambio(id).await.unwrap().estado, Estado::Vigilando);

    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    b.reanudar_pendientes().await;
    assert_eq!(b.activo(), Some(id));
    let c = esperar_estado(&b, id, Estado::Aplicado).await;
    assert!(c.nota.unwrap().contains("reinicio"));
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
}

#[tokio::test]
async fn reanuda_y_marca_fallido_si_no_llego_a_aplicarse() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        ..tiempos_rapidos()
    };
    let a = gestor(&p, lento).await;
    let prep = a.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = a.confirmar(&prep.token, None, "admin").await.unwrap();
    a.detener();
    // El router vuelve a tener el valor anterior (p. ej. no llegó a guardarlo).
    p.state
        .hgu
        .write(
            "dhcp.asp",
            LEASES,
            &[
                (POOL, "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("ipv4_addr", "192.168.1.68"),
            ],
        )
        .await
        .unwrap();
    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    b.reanudar_pendientes().await;
    let c = b.cambio(id).await.unwrap();
    assert_eq!(c.estado, Estado::Fallido);
    assert!(c.error.unwrap().contains("interrumpido antes de aplicar"));
    assert_eq!(b.activo(), None);
}

#[tokio::test]
async fn deshacer_restaura_valor_anterior() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    // Solo se deshace un cambio ya aplicado.
    let r = m.deshacer(id).await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;

    let deshacer = m.deshacer(id).await.unwrap();
    let c = &deshacer.diff.campos[0];
    assert_eq!(
        (c.nombre.as_str(), c.antes.as_str(), c.despues.as_str()),
        ("ipv4_addr", "192.168.1.80", "192.168.1.68")
    );
    let id2 = m.confirmar(&deshacer.token, None, "admin").await.unwrap();
    esperar_estado(&m, id2, Estado::Aplicado).await;
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

#[tokio::test]
async fn deshacer_una_reserva_nueva_la_borra() {
    let (p, m) = preparado().await;
    let prep = m.preparar(nueva_reserva()).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    let deshacer = m.deshacer(id).await.unwrap();
    assert!(
        deshacer
            .diff
            .campos
            .iter()
            .any(|c| c.nombre == "ipv4_addr" && c.antes == "192.168.1.99" && c.despues.is_empty())
    );
    let id2 = m.confirmar(&deshacer.token, None, "admin").await.unwrap();
    esperar_estado(&m, id2, Estado::Aplicado).await;
    assert_eq!(ip_de_lease(&p, "2").await, None);
    let w = p.sim.control.writes();
    let borrado: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(borrado["del"], "DEL");
    assert_eq!(borrado["rtm_dhcpv4_svr_pool_static_id"], "2");
    assert_eq!(borrado[POOL], "0");
    esperar_libre_gestor(&m).await;
    // Y deshacer el borrado la vuelve a crear.
    let otra = m.deshacer(id2).await.unwrap();
    let id3 = m.confirmar(&otra.token, None, "admin").await.unwrap();
    esperar_estado(&m, id3, Estado::Aplicado).await;
    let leases = read::static_leases(&p.state.hgu).await.unwrap();
    assert!(
        leases
            .iter()
            .any(|l| l.ip == "192.168.1.99" && l.mac == "02:00:00:00:00:99")
    );
}

#[tokio::test]
async fn reboot_exige_palabra() {
    let (p, m) = preparado().await;
    let prep = m.preparar(req("cbReboot", None, &[])).await.unwrap();
    assert_eq!(prep.riesgo, Riesgo::Alto);
    assert_eq!(prep.palabra.as_deref(), Some("REINICIAR"));
    for palabra in [None, Some("reiniciar"), Some("SI")] {
        let r = m.confirmar(&prep.token, palabra, "admin").await;
        assert!(matches!(r, Err(CambioError::PalabraIncorrecta)), "{r:?}");
    }
    assert!(p.sim.control.writes().is_empty());
    let id = m
        .confirmar(&prep.token, Some("REINICIAR"), "admin")
        .await
        .unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    let w = p.sim.control.writes();
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].0, "cbReboot");
}

#[tokio::test]
async fn reboot_sin_vuelta_es_fallido() {
    let p = panel_rapido(Duration::from_millis(100)).await;
    let m = gestor(&p, tiempos_rapidos()).await;
    let prep = m.preparar(req("cbReboot", None, &[])).await.unwrap();
    let id = m
        .confirmar(&prep.token, Some("REINICIAR"), "admin")
        .await
        .unwrap();
    // El router no vuelve a responder tras reiniciar.
    p.sim.control.set_unresponsive(true);
    let c = esperar_estado(&m, id, Estado::Fallido).await;
    p.sim.control.set_unresponsive(false);
    assert!(c.error.unwrap().contains("no volvió"));
}

#[tokio::test]
async fn bloqueos_de_la_v1() {
    let (p, m) = preparado().await;
    // Web básica con parámetros secretos (aunque no se toque el secreto).
    for f in ["te_wifi.wl", "te_wifi_5ghz.wl", "te_wifi_guest.wl"] {
        let r = m.preparar(req(f, None, &[("wlSsid", "Casa")])).await;
        assert!(
            matches!(&r, Err(CambioError::Bloqueada(e)) if e.contains("bloqueada en v1") && e.contains("web básica")),
            "{f}: {r:?}"
        );
    }
    // Cambios de la IP LAN del router.
    for f in [
        "cbSetLanIPv4Addr",
        "cbSetLanIPv4Intf",
        "te_red_local.cgi",
        "te_mapa_red_local.cgi",
    ] {
        let r = m.preparar(req(f, None, &[])).await;
        assert!(
            matches!(&r, Err(CambioError::Bloqueada(e)) if e.contains("bloqueada en v1") && e.contains("IP del router")),
            "{f}: {r:?}"
        );
    }
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn editar_un_campo_ilegible_se_rechaza() {
    let (p, m) = preparado().await;
    // `validation_only` no aparece en la lectura pareada.
    let r = m
        .preparar(req(
            "cbSetWifiAccPt",
            None,
            &[("dpm_cfg_wifi_acc_pt_id", "1"), ("validation_only", "0")],
        ))
        .await;
    assert!(
        matches!(&r, Err(CambioError::Invalida(e)) if e.contains("no se puede leer el valor actual de validation_only")),
        "{r:?}"
    );
    // Web básica sin secretos cuya lectura no muestra el campo.
    let r = m
        .preparar(req("te_upnp.cmd", None, &[("enblUpnp", "1")]))
        .await;
    assert!(matches!(&r, Err(CambioError::Invalida(_))), "{r:?}");
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn historial_en_orden() {
    let (_p, m) = preparado().await;
    let mut ids = Vec::new();
    for ip in ["192.168.1.80", "192.168.1.81"] {
        let prep = m.preparar(lease0(ip)).await.unwrap();
        let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
        esperar_estado(&m, id, Estado::Aplicado).await;
        esperar_libre_gestor(&m).await;
        ids.push(id);
    }
    let h = m.historial(10).await;
    assert_eq!(h.iter().map(|c| c.id).collect::<Vec<_>>(), [ids[1], ids[0]]);
    assert_eq!(h[0].riesgo, Riesgo::Bajo);
    assert_eq!(h[0].diff.campos[0].despues, "192.168.1.81");
    assert!(h[0].updated >= h[0].ts);
    assert_eq!(m.historial(1).await.len(), 1);
}

#[tokio::test]
async fn reanuda_con_la_wan_de_antes_del_cambio() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        ..tiempos_rapidos()
    };
    let a = gestor(&p, lento).await;
    let prep = a.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = a.confirmar(&prep.token, None, "admin").await.unwrap();
    a.detener();
    // El cambio tumbó la WAN y el panel se reinició: se vigila con la WAN de
    // antes (arriba), así que se revierte.
    p.sim.control.set_wan_up(false);
    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    b.reanudar_pendientes().await;
    let c = esperar_estado(&b, id, Estado::Revertido).await;
    assert!(c.error.unwrap().contains("WAN"));
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

#[tokio::test]
async fn revierte_una_creacion() {
    let (p, m) = preparado().await;
    let prep = m.preparar(nueva_reserva()).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_wan_up(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    assert_eq!(ip_de_lease(&p, "2").await, None);
    let w = p.sim.control.writes();
    let borrado: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(borrado["del"], "DEL");
    assert_eq!(borrado["rtm_dhcpv4_svr_pool_static_id"], "2");
}

/// D1: una reserva nueva idéntica a una que ya existía (como cuando el
/// router «añade» sobrescribiendo): la instancia del cambio es la que
/// aparece nueva, no la primera con esos valores; al revertir se borra ésa
/// y la de antes sigue.
#[tokio::test]
async fn crear_un_duplicado_revierte_solo_la_nueva() {
    let (p, m) = preparado().await;
    let antes = read::static_leases(&p.state.hgu).await.unwrap();
    let l0 = antes
        .iter()
        .find(|l| l.id == "0")
        .expect("reserva 0")
        .clone();
    let dup = req(
        LEASES,
        None,
        &[
            (POOL, "0"),
            ("adm_state", "1"),
            ("mac_addr", &l0.mac),
            ("ipv4_addr", &l0.ip),
        ],
    );
    let prep = m.preparar(dup).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    let c = esperar_estado(&m, id, Estado::Vigilando).await;
    assert_eq!(c.instancia.as_deref(), Some("2"), "la nueva, no la 0");
    p.sim.control.set_wan_up(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    let despues = read::static_leases(&p.state.hgu).await.unwrap();
    assert!(
        despues.iter().any(|l| l.id == "0"),
        "la reserva de antes sigue"
    );
    assert!(!despues.iter().any(|l| l.id == "2"));
    let w = p.sim.control.writes();
    let borrado: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(borrado["rtm_dhcpv4_svr_pool_static_id"], "2");
}

/// D1: si aparecen dos instancias nuevas no se sabe cuál es la del cambio:
/// fallido con un motivo claro y no se borra nada.
#[tokio::test]
async fn crear_con_dos_instancias_nuevas_no_borra_nada() {
    let (p, m) = preparado().await;
    let prep = m.preparar(nueva_reserva()).await.unwrap();
    p.sim.control.create_twice_next_write();
    let r = m.confirmar(&prep.token, None, "admin").await;
    let Err(CambioError::Verificacion(v)) = r else {
        panic!("{r:?}");
    };
    assert!(v.iter().any(|m| m.contains("2 instancias nuevas")), "{v:?}");
    let ultimo = m.historial(1).await.remove(0);
    assert_eq!(ultimo.estado, Estado::Fallido);
    assert_eq!(p.sim.control.writes().len(), 1, "no se borra nada");
    let leases = read::static_leases(&p.state.hgu).await.unwrap();
    assert_eq!(leases.iter().filter(|l| l.ip == "192.168.1.99").count(), 2);
}

#[tokio::test]
async fn revierte_un_secreto_que_estaba_vacio() {
    let (p, m) = preparado().await;
    let ap = [("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "")];
    p.state
        .hgu
        .write("wifi.asp", "cbSetWifiAccPt", &ap)
        .await
        .unwrap();
    let prep = m
        .preparar(req(
            "cbSetWifiAccPt",
            None,
            &[
                ("dpm_cfg_wifi_acc_pt_id", "1"),
                ("key_phrase", "clave-nueva-123"),
            ],
        ))
        .await
        .unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_wan_up(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    let w = p.sim.control.writes();
    assert_eq!(w.len(), 3, "preparación, cambio y reversión");
    let revert: BTreeMap<_, _> = w[2].1.iter().cloned().collect();
    assert_eq!(revert["key_phrase"], "");
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "1")])
        .await
        .unwrap();
    let key = n
        .get("dpm_cfg_wifi_acc_pt")
        .unwrap()
        .text_of("key_phrase")
        .map(str::to_owned);
    assert_eq!(key.as_deref(), Some(""));
}

#[tokio::test]
async fn confirmar_abandonado_termina_igual() {
    let (_p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    // Quien confirma se va (p. ej. se cierra la pestaña) a mitad de camino.
    let r = tokio::time::timeout(
        Duration::from_millis(2),
        m.confirmar(&prep.token, None, "admin"),
    )
    .await;
    assert!(r.is_err(), "debía abandonarse antes de terminar: {r:?}");
    let id = esperar_hasta_con(Duration::from_secs(10), async || {
        m.historial(1)
            .await
            .first()
            .map(|c| c.id)
            .ok_or_else(|| "sin cambio".to_owned())
    })
    .await;
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
}

#[tokio::test]
async fn relectura_fallida_tras_revertir_se_reintenta() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim
        .control
        .fail_reads_after_next_write("cbDhcpV4SvrPoolGetStatic");
    p.sim.control.set_wan_up(false);
    esperar_hasta(Duration::from_secs(5), "sin reversión", async || {
        p.sim.control.writes().len() >= 2
    })
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(m.cambio(id).await.unwrap().estado, Estado::Vigilando);
    p.sim.control.fail_reads("cbDhcpV4SvrPoolGetStatic", false);
    esperar_estado(&m, id, Estado::Revertido).await;
    assert_eq!(p.sim.control.writes().len(), 2, "no se reescribe");
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

#[tokio::test]
async fn revierte_un_valor_fuera_de_las_opciones() {
    let (p, m) = preparado().await;
    // El router tiene un valor que el catálogo no conoce.
    p.state
        .hgu
        .write(
            "dhcp.asp",
            LEASES,
            &[
                (POOL, "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("adm_state", "2"),
            ],
        )
        .await
        .unwrap();
    let prep = m
        .preparar(req(LEASES, Some("0"), &[(POOL, "0"), ("adm_state", "1")]))
        .await
        .unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_wan_up(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    let n = p
        .state
        .hgu
        .call(
            "cbDhcpV4SvrPoolGetStatic",
            &[(POOL, "0"), ("rtm_dhcpv4_svr_pool_static_id", "0")],
        )
        .await
        .unwrap();
    let adm = n
        .get("rtm_dhcpv4_svr_pool_static")
        .unwrap()
        .text_of("adm_state")
        .map(str::to_owned);
    assert_eq!(adm.as_deref(), Some("2"));
}

#[tokio::test]
async fn no_se_deshace_dos_veces() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    let u1 = m.deshacer(id).await.unwrap();
    let u2 = m.deshacer(id).await.unwrap();
    let id2 = m.confirmar(&u1.token, None, "admin").await.unwrap();
    // Con el primero en curso, ni otro deshacer ni el segundo token.
    let r = m.deshacer(id).await;
    assert!(
        matches!(&r, Err(CambioError::Invalida(e)) if e.contains("ya se ha deshecho")),
        "{r:?}"
    );
    esperar_estado(&m, id2, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    let r = m.confirmar(&u2.token, None, "admin").await;
    assert!(
        matches!(&r, Err(CambioError::Invalida(e)) if e.contains("ya se ha deshecho")),
        "{r:?}"
    );
    let r = m.deshacer(id).await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    assert_eq!(p.sim.control.writes().len(), 2);
    assert_eq!(m.historial(10).await.len(), 2);
}

#[tokio::test]
async fn confirmar_espera_a_que_se_retome_lo_pendiente() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        ..tiempos_rapidos()
    };
    let a = gestor(&p, lento).await;
    let prep = a.preparar(lease0("192.168.1.80")).await.unwrap();
    let pendiente = a.confirmar(&prep.token, None, "admin").await.unwrap();
    a.detener();

    let b = ChangeManager::new(p.state.clone(), tiempos_rapidos());
    let otro = b
        .preparar(req("cbSetUpnpCfg", None, &[("adm_state", "1")]))
        .await
        .unwrap();
    let (r, ()) = tokio::join!(b.confirmar(&otro.token, None, "admin"), async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        b.reanudar_pendientes().await;
    });
    // El cambio nuevo esperó y el retomado se quedó el hueco.
    assert!(matches!(r, Err(CambioError::EnCurso)), "{r:?}");
    assert_eq!(b.activo(), Some(pendiente));
    esperar_estado(&b, pendiente, Estado::Aplicado).await;
    esperar_libre_gestor(&b).await;
    let id = b.confirmar(&otro.token, None, "admin").await.unwrap();
    esperar_estado(&b, id, Estado::Aplicado).await;
}

#[tokio::test]
async fn instalar_deja_un_solo_gestor() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    assert!(p.state.cambios().is_none());
    let a = ChangeManager::instalar(&p.state, tiempos_rapidos());
    let b = ChangeManager::instalar(&p.state.clone(), tiempos_rapidos());
    assert!(Arc::ptr_eq(&a, &b));
    assert!(Arc::ptr_eq(&p.state.cambios().unwrap(), &a));
}

/// Tras confirmar, la lectura de reservas falla justo después de la próxima
/// escritura (la de la reversión) y se pierde la WAN; al cabo de 100 ms la
/// lectura vuelve.
async fn revertir_con_relectura_fallida(p: &Panel, m: &ChangeManager, id: i64) {
    p.sim
        .control
        .fail_reads_after_next_write("cbDhcpV4SvrPoolGetStatic");
    p.sim.control.set_wan_up(false);
    esperar_hasta(Duration::from_secs(5), "sin reversión", async || {
        p.sim.control.writes().len() >= 2
    })
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    p.sim.control.fail_reads("cbDhcpV4SvrPoolGetStatic", false);
    esperar_estado(m, id, Estado::Revertido).await;
    assert_eq!(
        p.sim.control.writes().len(),
        2,
        "una sola escritura de reversión"
    );
}

#[tokio::test]
async fn reintento_de_revertir_un_borrado_no_duplica() {
    let (p, m) = preparado().await;
    let borrar = ChangeRequest {
        borrar: true,
        ..req(LEASES, Some("1"), &[(POOL, "0")])
    };
    let prep = m.preparar(borrar).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    assert_eq!(ip_de_lease(&p, "1").await, None);
    revertir_con_relectura_fallida(&p, &m, id).await;
    let recreadas: Vec<_> = read::static_leases(&p.state.hgu)
        .await
        .unwrap()
        .into_iter()
        .filter(|l| l.ip == "192.168.1.69")
        .collect();
    assert_eq!(recreadas.len(), 1, "{recreadas:?}");
    let w = p.sim.control.writes();
    let crear: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(crear["rtm_dhcpv4_svr_pool_static_id"], "-1");
}

#[tokio::test]
async fn reintento_de_revertir_una_creacion_acepta_que_ya_no_exista() {
    // Cliente con 100 ms de espera: la relectura tras la reversión agota el
    // tiempo (no es un "no existe" del router) y hay que reintentar.
    let p = panel_rapido(Duration::from_millis(100)).await;
    let m = gestor(&p, tiempos_rapidos()).await;
    let prep = m.preparar(nueva_reserva()).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.hang_after_next_write();
    p.sim.control.set_wan_up(false);
    esperar_hasta(Duration::from_secs(5), "sin reversión", async || {
        p.sim.control.writes().len() >= 2
    })
    .await;
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(m.cambio(id).await.unwrap().estado, Estado::Vigilando);
    p.sim.control.set_unresponsive(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    assert_eq!(
        p.sim.control.writes().len(),
        2,
        "una sola escritura de reversión"
    );
    assert_eq!(ip_de_lease(&p, "2").await, None);
    let w = p.sim.control.writes();
    let borrado: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(borrado["del"], "DEL");
}

#[tokio::test]
async fn router_que_rechaza_la_reversion_falla_sin_reintentar() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.fail_next_write("valores no admitidos");
    p.sim.control.set_wan_up(false);
    let c = esperar_estado(&m, id, Estado::Fallido).await;
    let e = c.error.unwrap();
    assert!(e.contains("valores no admitidos"), "{e}");
    // Sin reintentos: el cambio y un único intento de reversión.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(p.sim.control.writes().len(), 2);
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
}
