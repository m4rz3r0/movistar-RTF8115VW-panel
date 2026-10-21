//! Pruebas del gestor de cambios por grupos de pasos contra `router-sim`,
//! con tiempos de vigilancia cortos.

use crate::comun::{
    Panel, Registro, esperar_estado, esperar_hasta, esperar_hasta_con, esperar_libre_gestor,
    gestor, get, panel_rapido, preparado, req, tiempos_rapidos,
};
use catalogo::Riesgo;
use hgu::read;
use panel::{
    changes::{
        Cambio, CambioError, ChangeManager, ChangeRequest, Estado, EstadoPaso, GrupoRequest, Paso,
        Tiempos,
    },
    poller::{Poller, PollerConfig},
};
use std::{sync::Arc, time::Duration};

mod aplicar;
mod reanudar;
mod revertir;

const LEASES: &str = "cbDhcpV4SvrPoolSetStatic";

const POOL: &str = "rtm_cfg_dhcpv4_svr_pool_id";

const UPNP: &str = "cbSetUpnpCfg";

const WIFI: &str = "cbSetWifiAccPt";

fn paso(r: ChangeRequest, etiqueta: &str) -> Paso {
    Paso {
        req: r,
        etiqueta: Some(etiqueta.into()),
    }
}

/// UPnP (`adm_state` 0 en las fixtures).
fn upnp(v: &str) -> Paso {
    paso(req(UPNP, None, &[("adm_state", v)]), "ajuste-upnp")
}

/// IP de la reserva 0 (`192.168.1.68` en las fixtures).
fn lease0(ip: &str) -> Paso {
    paso(
        req(LEASES, Some("0"), &[(POOL, "0"), ("ipv4_addr", ip)]),
        "ajuste-reserva",
    )
}

/// SSID del punto de acceso 1 (`RED-1` en las fixtures). Riesgo medio.
fn ssid(v: &str) -> Paso {
    paso(
        req(WIFI, None, &[("dpm_cfg_wifi_acc_pt_id", "1"), ("ssid", v)]),
        "ajuste-ssid",
    )
}

fn grupo(pasos: Vec<Paso>) -> GrupoRequest {
    GrupoRequest {
        pasos,
        titulo: Some("ajuste-prueba".into()),
    }
}

/// Espera a que el cambio llegue a un estado final.
async fn esperar_final(m: &ChangeManager, id: i64) -> Cambio {
    esperar_hasta_con(Duration::from_secs(10), async || {
        let c = m.cambio(id).await.expect("el cambio existe");
        if !matches!(c.estado, Estado::Aplicando | Estado::Vigilando) {
            return Ok(c);
        }
        Err(format!("el cambio {id} sigue en {:?}", c.estado))
    })
    .await
}

async fn ip_de_lease0(p: &Panel) -> String {
    read::static_leases(&p.state.hgu)
        .await
        .unwrap()
        .into_iter()
        .find(|l| l.id == "0")
        .map(|l| l.ip)
        .unwrap()
}

async fn upnp_actual(p: &Panel) -> String {
    let n = p.state.hgu.call("cbGetUpnpCfg", &[]).await.unwrap();
    n.get("upnp_cfg")
        .and_then(|u| u.text_of("adm_state"))
        .unwrap()
        .to_owned()
}

async fn ssid_actual(p: &Panel) -> String {
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "1")])
        .await
        .unwrap();
    n.get("dpm_cfg_wifi_acc_pt")
        .and_then(|a| a.text_of("ssid"))
        .unwrap()
        .to_owned()
}

/// Funciones de las escrituras recibidas, en orden.
fn escritas(p: &Panel) -> Vec<String> {
    p.sim.control.writes().into_iter().map(|(f, _)| f).collect()
}

fn estados_pasos(c: &Cambio) -> Vec<EstadoPaso> {
    c.pasos.iter().map(|p| p.estado).collect()
}

/// Alta de una reserva nueva (`192.168.1.99`).
fn nueva_reserva() -> Paso {
    paso(
        req(
            LEASES,
            None,
            &[
                (POOL, "0"),
                ("adm_state", "1"),
                ("mac_addr", "02:00:00:00:00:99"),
                ("ipv4_addr", "192.168.1.99"),
            ],
        ),
        "ajuste-reserva-nueva",
    )
}

/// Arranca `confirmar_grupo` en otra tarea (para cortar el panel a medias).
fn confirmar_en_segundo_plano(
    m: &Arc<ChangeManager>,
    token: &str,
) -> tokio::task::JoinHandle<Result<i64, CambioError>> {
    let (m, token) = (m.clone(), token.to_owned());
    tokio::spawn(async move { m.confirmar_grupo(&token, &[], "admin").await })
}

async fn esperar_escrituras(p: &Panel, n: usize) {
    esperar_hasta(
        Duration::from_secs(5),
        &format!("sin escritura {n}"),
        async || p.sim.control.writes().len() >= n,
    )
    .await;
}
