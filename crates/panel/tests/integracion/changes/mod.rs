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

mod aplicar;
mod deshacer;
mod reanudar;
mod reintentos;
mod secretos;
mod vigilancia;

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
