//! Lecturas de estado del router: llaman al `Client` y devuelven modelos.

use crate::models::{
    DhcpLease, IfaceCounters, LanHost, StaticLease, Summary, WanConn, WifiAp, WifiAssoc, WifiRadio,
    WifiStatus,
};
use crate::{Client, Error};

/// Radios WiFi del router (2,4 y 5 GHz).
const RADIOS: [&str; 2] = ["0", "1"];
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

/// Radios, SSID y estaciones de ambos radios.
pub async fn wifi(c: &Client) -> Result<WifiStatus, Error> {
    let mut st = WifiStatus::default();
    for id in RADIOS {
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
        let w = wifi(&c).await.unwrap();
        assert_eq!(w.radios.len(), 2);
        assert!(w.aps.len() >= 8);
        assert!(w.stations.is_empty());
        assert_eq!(wifi_stations(&c, "9").await.unwrap().len(), 2);
    }
}
