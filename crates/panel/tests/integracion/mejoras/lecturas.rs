//! Mejoras con lecturas que fallan: la red de invitados, una pestaña o
//! todas. Lo que no se lee no cuenta, y nada nombra la API.

use super::{dns_del_pool, escribir, grupo, lote, mejoras, paso, pendientes, preparar};
use crate::comun::{PanelPrueba, llamar, ok};
use serde_json::{Value, json};

/// Las lecturas que hacen los lotes, por función.
const LECTURAS: [&str; 7] = [
    "cbGetTr69Config",
    "cbGetWifiAccPt",
    "cbGetUpnpCfg",
    "cbGetDiagConfig",
    "cbGetSyslog",
    "cbGetDhcpv4ServerPool",
    "cbGetTimeNtp",
];

/// Vigilancia 1: la red de invitados no se puede leer.
#[tokio::test]
async fn lote_con_invitados_sin_leer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.sim
        .control
        .fail_reads_con("cbGetWifiAccPt", "dpm_cfg_wifi_acc_pt_id", "2", true);
    let m = mejoras(&p, &cookie).await;
    for clave in ["wps@invitados", "wpa2@invitados", "aislar_invitados"] {
        let e = &paso(&m, "seguridad", clave)["estado"];
        assert!(e["SinLeer"]["motivo"].is_string(), "{clave}: {e}");
    }
    assert_eq!(paso(&m, "seguridad", "wps@red-5")["estado"], "Pendiente");
    assert_eq!(m["pendientes"], 5, "{m}");
    let g = grupo(&p, &cookie, "seguridad", json!({})).await;
    for paso in g["pasos"].as_array().unwrap() {
        assert!(
            !paso["titulo"]
                .as_str()
                .unwrap()
                .starts_with("ajuste-wifi-invitados"),
            "{paso}"
        );
    }
    assert!(g["aviso"].is_string(), "{g}");
    assert!(g["aviso"].as_str().unwrap().contains('3'), "{g}");
}

/// Vigilancia 2: falla la lectura de la gestión remota.
#[tokio::test]
async fn mejoras_con_una_lectura_fallida() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.sim.control.fail_reads("cbGetTr69Config", true);
    let m = mejoras(&p, &cookie).await;
    assert_eq!(lote(&m, "seguridad")["estado"], pendientes(3, 9), "{m}");
    assert_eq!(lote(&m, "privacidad")["estado"], pendientes(1, 3), "{m}");
    assert_eq!(m["pendientes"], 4);
    let motivo = &paso(&m, "privacidad", "gestion_remota")["estado"]["SinLeer"]["motivo"];
    assert!(motivo.as_str().unwrap().contains("Seguridad"), "{motivo}");
}

#[tokio::test]
async fn contador_sin_lecturas_no_sale() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for f in LECTURAS {
        p.sim.control.fail_reads(f, true);
    }
    p.state.lecturas.invalidar();
    assert_eq!(
        ok(&p, "contar_mejoras", json!({}), &cookie).await,
        Value::Null
    );
    let m = mejoras(&p, &cookie).await;
    assert_eq!(m["pendientes"], Value::Null);
    for id in ["seguridad", "privacidad"] {
        assert!(
            lote(&m, id)["estado"]["SinLeer"]["motivo"].is_string(),
            "{m}"
        );
    }
    // Sin nada leído, la revisión no se abre y lo dice del lote.
    let (status, body) = preparar(&p, &cookie, "seguridad", json!({})).await;
    assert_eq!(status, 502, "{body}");
    assert!(body.contains("lote"), "{body}");
    assert!(!body.contains("este ajuste"), "{body}");
}

/// Con la gestión remota sin leer y lo demás de Privacidad hecho, el lote
/// no sale «aplicado» ni la revisión dice «ya aplicado».
#[tokio::test]
async fn sin_leer_y_lo_demas_hecho_no_es_aplicado() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    dns_del_pool(&p, "9.9.9.9", "149.112.112.112").await;
    escribir(
        &p,
        "ntp.asp",
        "cbSetTimeNtp",
        &[("ntp_svr_1", "hora.roa.es")],
    )
    .await;
    p.sim.control.fail_reads("cbGetTr69Config", true);
    let m = mejoras(&p, &cookie).await;
    let motivo = lote(&m, "privacidad")["estado"]["SinLeer"]["motivo"]
        .as_str()
        .unwrap_or_else(|| panic!("{m}"))
        .to_owned();
    assert!(motivo.contains('2'), "{motivo}");
    let (status, body) = preparar(&p, &cookie, "privacidad", json!({})).await;
    assert_eq!(status, 409, "{body}");
    assert!(!body.contains("ya está aplicado"), "{body}");
    assert!(body.contains("leer"), "{body}");
}

#[tokio::test]
async fn mejoras_sin_nombres_de_la_api() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.sim.control.fail_reads("cbGetSyslog", true);
    let (status, body) = llamar(&p, "leer_mejoras", json!({}), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    let api = regex_lite_cb(&body);
    assert!(api.is_none(), "{api:?}: {body}");
}

/// Primer `cb` seguido de una mayúscula (un nombre de la API) en `texto`.
fn regex_lite_cb(texto: &str) -> Option<&str> {
    texto
        .match_indices("cb")
        .find(|(i, _)| {
            texto[i + 2..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
        })
        .map(|(i, _)| &texto[i..(i + 12).min(texto.len())])
}
