//! Lecturas de estado del router: llaman al `Client` y devuelven modelos.

use crate::models::{
    DhcpLease, IfaceCounters, LanHost, StaticLease, Summary, WanConn, WifiAp, WifiAssoc, WifiRadio,
    WifiStatus,
};
use crate::xml::XmlNode;
use crate::{Client, Error};
use std::net::Ipv4Addr;

/// Radios WiFi que se leen si no se han detectado (2,4 y 5 GHz).
pub const RADIOS_POR_DEFECTO: [&str; 2] = ["0", "1"];
const POOL: [(&str, &str); 2] = [("rtm_cfg_dhcpv4_svr_pool_id", "0"), ("all", "ALL")];

/// `cbGetSummary`.
pub async fn summary(c: &Client) -> Result<Summary, Error> {
    Ok(Summary::from_node(&c.call("cbGetSummary", &[]).await?))
}

/// `cbGetStatistic`.
pub async fn counters(c: &Client) -> Result<Vec<IfaceCounters>, Error> {
    Ok(IfaceCounters::parse_all(
        &c.call("cbGetStatistic", &[]).await?,
    ))
}

/// `cbGetWanIPIntf`.
pub async fn wan(c: &Client) -> Result<Vec<WanConn>, Error> {
    Ok(WanConn::parse_all(&c.call("cbGetWanIPIntf", &[]).await?))
}

/// Interfaz IP (`ip_intf_id`) de la ruta IPv4 por defecto
/// (`cbGetRoutingV4Rule` con `all=ALL` y `desc=Default`). Ver
/// [`regla_por_defecto`].
pub async fn ruta_por_defecto(c: &Client) -> Result<Option<String>, Error> {
    let n = c
        .call("cbGetRoutingV4Rule", &[("all", "ALL"), ("desc", "Default")])
        .await?;
    Ok(regla_por_defecto(&n))
}

/// `ip_intf_id` de la regla con destino `0.0.0.0/0` de una respuesta de
/// `cbGetRoutingV4Rule`: mejor la de descripción `Default` y, si no, la
/// primera activa. `None` si no hay ninguna.
pub fn regla_por_defecto(n: &XmlNode) -> Option<String> {
    let por_defecto: Vec<_> = n
        .all("rtm_cfg_rulev4")
        .into_iter()
        .filter(|r| r.text_of("dst_addr") == Some("0.0.0.0"))
        .filter(|r| r.text_of("dst_mask").is_none_or(|m| m == "0.0.0.0"))
        .filter(|r| {
            r.text_of("ip_intf_id")
                .is_some_and(|i| i.parse::<u32>().is_ok())
        })
        .collect();
    let regla = por_defecto
        .iter()
        .find(|r| r.text_of("desc") == Some("Default"))
        .or_else(|| {
            por_defecto
                .iter()
                .find(|r| r.text_of("opr_status") != Some("0"))
        })?;
    regla.text_of("ip_intf_id").map(str::to_owned)
}

/// Dirección y máscara IPv4 del router en la LAN (`cbGetLanIPv4Addr`); `None`
/// si no trae ninguna legible.
pub async fn lan_ipv4(c: &Client) -> Result<Option<(Ipv4Addr, Ipv4Addr)>, Error> {
    let n = c.call("cbGetLanIPv4Addr", &[]).await?;
    Ok(n.all("rtm_cfg_ipv4_addr").into_iter().find_map(|a| {
        let ip = a.text_of("ipv4_addr")?.parse::<Ipv4Addr>().ok()?;
        let mask = a.text_of("ipv4_mask")?.parse::<Ipv4Addr>().ok()?;
        (!ip.is_unspecified()).then_some((ip, mask))
    }))
}

/// `cbGetLanHostList`.
pub async fn lan_hosts(c: &Client) -> Result<Vec<LanHost>, Error> {
    Ok(LanHost::parse_all(&c.call("cbGetLanHostList", &[]).await?))
}

/// `cbDhcpV4SvrPoolGetClt` (pool 0).
pub async fn dhcp_leases(c: &Client) -> Result<Vec<DhcpLease>, Error> {
    let n = c.call("cbDhcpV4SvrPoolGetClt", &POOL).await?;
    Ok(DhcpLease::parse_all(&n))
}

/// `cbDhcpV4SvrPoolGetStatic` (pool 0).
pub async fn static_leases(c: &Client) -> Result<Vec<StaticLease>, Error> {
    let n = c.call("cbDhcpV4SvrPoolGetStatic", &POOL).await?;
    Ok(StaticLease::parse_all(&n))
}

/// Estaciones asociadas a los SSID de un radio (`cbListWifiAccPtAssociation`).
pub async fn wifi_stations(c: &Client, radio_id: &str) -> Result<Vec<WifiAssoc>, Error> {
    let n = c
        .call(
            "cbListWifiAccPtAssociation",
            &[("dpm_cfg_wifi_id", radio_id)],
        )
        .await?;
    Ok(WifiAssoc::parse_all(&n))
}

/// Radios, SSID y estaciones de los `radios` (ids de `cbGetWifi`; ver
/// [`RADIOS_POR_DEFECTO`]).
pub async fn wifi(c: &Client, radios: &[String]) -> Result<WifiStatus, Error> {
    let mut st = WifiStatus::default();
    for id in radios {
        let id = id.as_str();
        let n = c.call("cbGetWifi", &[("dpm_cfg_wifi_0i", id)]).await?;
        st.radios.extend(WifiRadio::from_node(&n));
        let n = c
            .call("cbListWifiAccPt", &[("dpm_cfg_wifi_id", id)])
            .await?;
        st.aps.extend(WifiAp::parse_all(&n, id));
        st.stations.extend(wifi_stations(c, id).await?);
    }
    Ok(st)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientConfig;
    use router_sim::{Sim, SimConfig};

    #[tokio::test]
    async fn read_all_from_sim() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
        let sim = Sim::start(SimConfig::new("u", "clave-de-prueba", dir)).await;
        let mut cfg = ClientConfig::new(&sim.base_url, "u", "clave-de-prueba");
        cfg.min_gap = std::time::Duration::from_millis(1);
        let c = Client::new(cfg);

        assert_eq!(summary(&c).await.unwrap().model, "RTF8115VW");
        assert!(counters(&c).await.unwrap().iter().any(|i| i.name == "ip2"));
        assert!(
            wan(&c)
                .await
                .unwrap()
                .iter()
                .any(|w| w.name == "ip2" && w.up)
        );
        assert!(!lan_hosts(&c).await.unwrap().is_empty());
        assert!(!dhcp_leases(&c).await.unwrap().is_empty());
        assert!(!static_leases(&c).await.unwrap().is_empty());
        let radios = RADIOS_POR_DEFECTO.map(String::from);
        let w = wifi(&c, &radios).await.unwrap();
        assert_eq!(w.radios.len(), 2);
        assert!(w.aps.len() >= 8);
        assert!(w.stations.is_empty());
        assert_eq!(wifi_stations(&c, "9").await.unwrap().len(), 2);
        assert_eq!(ruta_por_defecto(&c).await.unwrap().as_deref(), Some("2"));
        assert_eq!(
            lan_ipv4(&c).await.unwrap(),
            Some((
                Ipv4Addr::new(192, 168, 1, 1),
                Ipv4Addr::new(255, 255, 255, 0)
            ))
        );
    }
}
