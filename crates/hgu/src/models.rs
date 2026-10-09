//! Modelos tipados del estado del router, construidos a partir de [`XmlNode`].
//!
//! Los analizadores no hacen `panic` con valores ausentes o extraños: los
//! números ilegibles pasan a 0 (o `None` donde el modelo lo permite) y
//! `X-REDACTED-X` es una cadena más.

use crate::xml::XmlNode;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::str::FromStr;

/// Número del texto, o el valor por defecto si no se puede leer.
fn num<T: FromStr + Default>(s: &str) -> T {
    s.trim().parse().unwrap_or_default()
}

fn text(n: &XmlNode, tag: &str) -> String {
    n.text_of(tag).unwrap_or_default().to_owned()
}

fn int<T: FromStr + Default>(n: &XmlNode, tag: &str) -> T {
    num(n.text_of(tag).unwrap_or_default())
}

fn flag(n: &XmlNode, tag: &str) -> bool {
    n.text_of(tag) == Some("1")
}

/// Resumen del sistema. Correspondencia con `cbGetSummary`: `ModelName`,
/// `SWVersion` (o `PrivateVersion` si falta), `HWVersion`, `SysUptime`,
/// `AvgLoad1/5/15`, `TotalRam`/`FreeRam` (kB) y `SysTime`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub model: String,
    pub firmware: String,
    pub hw: String,
    pub uptime_s: u64,
    pub load1: f32,
    pub load5: f32,
    pub load15: f32,
    pub ram_total_kb: u64,
    pub ram_free_kb: u64,
    pub sys_time: i64,
}

impl Summary {
    /// Interpreta el nodo `cbGetSummary`.
    pub fn from_node(n: &XmlNode) -> Self {
        let firmware = match n.text_of("SWVersion") {
            Some(v) if !v.is_empty() => v.to_owned(),
            _ => text(n, "PrivateVersion"),
        };
        Summary {
            model: text(n, "ModelName"),
            firmware,
            hw: text(n, "HWVersion"),
            uptime_s: int(n, "SysUptime"),
            load1: int(n, "AvgLoad1"),
            load5: int(n, "AvgLoad5"),
            load15: int(n, "AvgLoad15"),
            ram_total_kb: int(n, "TotalRam"),
            ram_free_kb: int(n, "FreeRam"),
            sys_time: int(n, "SysTime"),
        }
    }
}

/// Tipo de interfaz de `cbGetStatistic`, según la etiqueta del nodo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IfaceKind {
    /// Puerto físico de la LAN (`dpm_cfg_eth`: `eth0.1`…`eth0.4`).
    LanPort,
    /// Punto de acceso WiFi (`dpm_wifi_ap`: `ra*`, `qtn*`).
    WifiAp,
    /// Interfaz IP del router (`rtm_cfg_ip_intf`: `ip2` es la WAN). El
    /// tráfico reenviado por hardware no pasa por sus contadores.
    IpIntf,
    /// Cualquier otra etiqueta.
    #[default]
    Other,
}

impl IfaceKind {
    fn from_tag(tag: &str) -> Self {
        match tag {
            "dpm_cfg_eth" => IfaceKind::LanPort,
            "dpm_wifi_ap" => IfaceKind::WifiAp,
            "rtm_cfg_ip_intf" => IfaceKind::IpIntf,
            _ => IfaceKind::Other,
        }
    }

    /// Puertos de la LAN y puntos de acceso WiFi: lo que entra y sale de los
    /// dispositivos de la red (incluido el reenviado por hardware).
    pub fn is_lan_side(self) -> bool {
        matches!(self, IfaceKind::LanPort | IfaceKind::WifiAp)
    }
}

/// Contadores de una interfaz. Correspondencia con cada elemento de
/// `cbGetStatistic` que tenga `name`: `rx_byte`, `tx_byte`, `rx_pkt_err`,
/// `tx_pkt_err`, `rx_pkt_discard`, `tx_pkt_discard`; `kind` sale de la
/// etiqueta del elemento.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IfaceCounters {
    pub name: String,
    #[serde(default)]
    pub kind: IfaceKind,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_err: u64,
    pub tx_err: u64,
    pub rx_drop: u64,
    pub tx_drop: u64,
}

impl IfaceCounters {
    /// Todas las interfaces del nodo `cbGetStatistic` (`dpm_cfg_eth`, ...).
    pub fn parse_all(n: &XmlNode) -> Vec<Self> {
        n.children
            .iter()
            .filter(|c| c.get("name").is_some() && c.get("rx_byte").is_some())
            .map(|c| IfaceCounters {
                name: text(c, "name"),
                kind: IfaceKind::from_tag(&c.tag),
                rx_bytes: int(c, "rx_byte"),
                tx_bytes: int(c, "tx_byte"),
                rx_err: int(c, "rx_pkt_err"),
                tx_err: int(c, "tx_pkt_err"),
                rx_drop: int(c, "rx_pkt_discard"),
                tx_drop: int(c, "tx_pkt_discard"),
            })
            .collect()
    }
}

/// Conexión WAN. Correspondencia con `mll_wanconn_data` de `cbGetWanIPIntf`:
/// `name`, `opr_status`, `adm_state`, `wan_type`, `dis_ipv4_addr` (`None` si
/// vale `0.0.0.0` o no es una IPv4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WanConn {
    pub name: String,
    pub up: bool,
    pub enabled: bool,
    pub wan_type: String,
    pub ipv4: Option<Ipv4Addr>,
}

impl WanConn {
    /// Todas las conexiones de `cbGetWanIPIntf`.
    pub fn parse_all(n: &XmlNode) -> Vec<Self> {
        n.all("mll_wanconn_data")
            .into_iter()
            .map(|c| WanConn {
                name: text(c, "name"),
                up: flag(c, "opr_status"),
                enabled: flag(c, "adm_state"),
                wan_type: text(c, "wan_type"),
                ipv4: c
                    .text_of("dis_ipv4_addr")
                    .and_then(|s| s.parse::<Ipv4Addr>().ok())
                    .filter(|ip| !ip.is_unspecified()),
            })
            .collect()
    }
}

/// Equipo de la LAN. Correspondencia con `rtm_cfg_host` de `cbGetLanHostList`:
/// `ipv4_addr`, `mac_addr`, `hostname`, `is_active`; `iface` es
/// `intf_type:intf_id` (el router no da un nombre legible).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LanHost {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub iface: String,
    pub active: bool,
}

impl LanHost {
    /// Todos los equipos de `cbGetLanHostList`.
    pub fn parse_all(n: &XmlNode) -> Vec<Self> {
        n.all("rtm_cfg_host")
            .into_iter()
            .map(|c| LanHost {
                ip: text(c, "ipv4_addr"),
                mac: text(c, "mac_addr"),
                hostname: text(c, "hostname"),
                iface: format!("{}:{}", text(c, "intf_type"), text(c, "intf_id")),
                active: flag(c, "is_active"),
            })
            .collect()
    }
}

/// Concesión DHCP. Correspondencia con `rtm_dhcpv4_svr_pool_clt` de
/// `cbDhcpV4SvrPoolGetClt`: `mac_addr`, y de su `rtm_dhcpv4_svr_pool_clt_addr`
/// `ipv4_addr` y `lease_remaining` (`d,HH:MM:SS`). Este cmd no da el nombre del
/// equipo: `hostname` queda vacío si no aparece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DhcpLease {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub expires_s: Option<u64>,
}

/// Segundos de un tiempo `d,HH:MM:SS`.
fn parse_lease_time(s: &str) -> Option<u64> {
    let (d, hms) = s.trim().split_once(',')?;
    let mut it = hms.split(':');
    let (h, m, sec) = (it.next()?, it.next()?, it.next()?);
    if it.next().is_some() {
        return None;
    }
    let p = |x: &str| x.trim().parse::<u64>().ok();
    Some(p(d)? * 86400 + p(h)? * 3600 + p(m)? * 60 + p(sec)?)
}

impl DhcpLease {
    /// Todas las concesiones de `cbDhcpV4SvrPoolGetClt`.
    pub fn parse_all(n: &XmlNode) -> Vec<Self> {
        n.all("rtm_dhcpv4_svr_pool_clt")
            .into_iter()
            .map(|c| {
                let addr = c.get("rtm_dhcpv4_svr_pool_clt_addr");
                DhcpLease {
                    ip: addr.map(|a| text(a, "ipv4_addr")).unwrap_or_default(),
                    mac: text(c, "mac_addr"),
                    hostname: text(c, "hostname"),
                    expires_s: addr
                        .and_then(|a| a.text_of("lease_remaining"))
                        .and_then(parse_lease_time),
                }
            })
            .collect()
    }
}

/// Reserva DHCP fija. Correspondencia con `rtm_dhcpv4_svr_pool_static` de
/// `cbDhcpV4SvrPoolGetStatic`: `rtm_dhcpv4_svr_pool_static_id`, `mac_addr`,
/// `ipv4_addr`, `adm_state`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticLease {
    pub id: String,
    pub mac: String,
    pub ip: String,
    pub enabled: bool,
}

impl StaticLease {
    /// Todas las reservas de `cbDhcpV4SvrPoolGetStatic`.
    pub fn parse_all(n: &XmlNode) -> Vec<Self> {
        n.all("rtm_dhcpv4_svr_pool_static")
            .into_iter()
            .map(|c| StaticLease {
                id: text(c, "rtm_dhcpv4_svr_pool_static_id"),
                mac: text(c, "mac_addr"),
                ip: text(c, "ipv4_addr"),
                enabled: flag(c, "adm_state"),
            })
            .collect()
    }
}

/// Radio WiFi. Correspondencia con `dpm_cfg_wifi` de `cbGetWifi`:
/// `dpm_cfg_wifi_id`, `radio_adm_state`, `opr_channel` (`auto` si vale 0). La
/// banda no viene en la respuesta: se deduce del canal (>= 36 es 5 GHz) o,
/// con canal automático, de `standard` (25 es 5 GHz).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WifiRadio {
    pub id: String,
    pub band: String,
    pub enabled: bool,
    pub channel: String,
}

impl WifiRadio {
    /// Interpreta el nodo `cbGetWifi`; `None` si no trae `dpm_cfg_wifi`.
    pub fn from_node(n: &XmlNode) -> Option<Self> {
        let r = n.get("dpm_cfg_wifi")?;
        let ch: u32 = int(r, "opr_channel");
        let five = ch >= 36 || (ch == 0 && r.text_of("standard") == Some("25"));
        Some(WifiRadio {
            id: text(r, "dpm_cfg_wifi_id"),
            band: if five { "5GHz" } else { "2.4GHz" }.to_owned(),
            enabled: flag(r, "radio_adm_state"),
            channel: if ch == 0 {
                "auto".to_owned()
            } else {
                ch.to_string()
            },
        })
    }
}

/// Punto de acceso (SSID). Correspondencia con `dpm_cfg_wifi_acc_pt` de
/// `cbListWifiAccPt`: `dpm_cfg_wifi_acc_pt_id`, `ssid`, `adm_state`,
/// `authentication`/`encryption` (`security` = `auth/enc`, códigos del
/// router) y `wps_adm_state`. `radio_id` es el radio consultado.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WifiAp {
    pub id: String,
    pub radio_id: String,
    pub ssid: String,
    pub enabled: bool,
    pub security: String,
    pub wps: bool,
}

impl WifiAp {
    /// Todos los SSID de `cbListWifiAccPt` del radio `radio_id`.
    pub fn parse_all(n: &XmlNode, radio_id: &str) -> Vec<Self> {
        n.all("dpm_cfg_wifi_acc_pt")
            .into_iter()
            .map(|c| WifiAp {
                id: text(c, "dpm_cfg_wifi_acc_pt_id"),
                radio_id: radio_id.to_owned(),
                ssid: text(c, "ssid"),
                enabled: flag(c, "adm_state"),
                security: format!("{}/{}", text(c, "authentication"), text(c, "encryption")),
                wps: flag(c, "wps_adm_state"),
            })
            .collect()
    }
}

/// Estación asociada. Correspondencia con `wifi_association` dentro de cada
/// `wifi_association_group` de `cbListWifiAccPtAssociation`: `mac_addr` y
/// `rssi`. Las capturas reales solo traen los grupos, sin estaciones; los
/// nombres de los campos de estación salen de la fixture sintética `wifi_id=9`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WifiAssoc {
    pub ap_id: String,
    pub mac: String,
    pub rssi: Option<i32>,
}

impl WifiAssoc {
    /// Todas las estaciones; vacío si solo hay grupos.
    pub fn parse_all(n: &XmlNode) -> Vec<Self> {
        n.all("wifi_association_group")
            .into_iter()
            .flat_map(|g| {
                let ap_id = text(g, "dpm_cfg_wifi_acc_pt_id");
                g.all("wifi_association")
                    .into_iter()
                    .map(move |s| WifiAssoc {
                        ap_id: ap_id.clone(),
                        mac: text(s, "mac_addr"),
                        rssi: s.text_of("rssi").and_then(|v| v.trim().parse().ok()),
                    })
            })
            .collect()
    }
}

/// Estado WiFi completo.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WifiStatus {
    pub radios: Vec<WifiRadio>,
    pub aps: Vec<WifiAp>,
    pub stations: Vec<WifiAssoc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::parse_response;

    fn fixture(name: &str, file: &str) -> XmlNode {
        let path = format!("{}/fixtures/{file}.xml", env!("CARGO_MANIFEST_DIR"));
        let raw = std::fs::read_to_string(path).unwrap();
        parse_response(name, &raw).unwrap()
    }

    #[test]
    fn summary_from_fixture() {
        let s = Summary::from_node(&fixture("cbGetSummary", "cbGetSummary"));
        assert_eq!(s.model, "RTF8115VW");
        assert_eq!(s.firmware, "ES_g14.17_RTF_TEF001_V9.17_V034");
        assert_eq!(s.hw, "REV5");
        assert_eq!(s.uptime_s, 26263);
        assert!(s.load1 > 0.0 && s.load5 > 0.0 && s.load15 > 0.0);
        assert_eq!((s.ram_total_kb, s.ram_free_kb), (255412, 68708));
        assert_eq!(s.sys_time, 1791407932);
    }

    #[test]
    fn summary_tolerates_empty_node() {
        let s = Summary::from_node(&XmlNode {
            tag: "cbGetSummary".into(),
            text: String::new(),
            children: vec![],
        });
        assert_eq!(s.uptime_s, 0);
        assert_eq!(s.model, "");
    }

    #[test]
    fn counters_has_ip2() {
        let c = IfaceCounters::parse_all(&fixture("cbGetStatistic", "cbGetStatistic"));
        assert!(c.len() >= 4);
        let ip2 = c.iter().find(|c| c.name == "ip2").expect("ip2");
        assert!(ip2.rx_bytes > 0 || ip2.tx_bytes > 0);
        let eth = c.iter().find(|c| c.name == "eth0.1").unwrap();
        assert_eq!((eth.rx_bytes, eth.tx_bytes), (30960130983, 22812082599));
        assert_eq!(eth.rx_drop, 303);
    }

    #[test]
    fn counters_know_their_kind() {
        let c = IfaceCounters::parse_all(&fixture("cbGetStatistic", "cbGetStatistic"));
        let kind = |name: &str| c.iter().find(|c| c.name == name).unwrap().kind;
        assert_eq!(kind("eth0.4"), IfaceKind::LanPort);
        assert_eq!(kind("ra0"), IfaceKind::WifiAp);
        assert_eq!(kind("qtn3"), IfaceKind::WifiAp);
        assert_eq!(kind("ip2"), IfaceKind::IpIntf);
        let lan: Vec<&str> = c
            .iter()
            .filter(|c| c.kind.is_lan_side())
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(lan.len(), 12, "{lan:?}");
        assert!(!lan.contains(&"ip2"));
        let otro = IfaceCounters::parse_all(&XmlNode {
            tag: "cbGetStatistic".into(),
            text: String::new(),
            children: vec![XmlNode {
                tag: "dpm_cfg_gpon".into(),
                text: String::new(),
                children: ["name", "rx_byte"]
                    .iter()
                    .map(|t| XmlNode {
                        tag: (*t).into(),
                        text: "1".into(),
                        children: vec![],
                    })
                    .collect(),
            }],
        });
        assert_eq!(otro[0].kind, IfaceKind::Other);
        assert!(!otro[0].kind.is_lan_side());
    }

    #[test]
    fn wan_has_ip2_up() {
        let w = WanConn::parse_all(&fixture("cbGetWanIPIntf", "cbGetWanIPIntf"));
        let ip2 = w.iter().find(|w| w.name == "ip2").expect("ip2");
        assert!(ip2.up && ip2.enabled);
        assert_eq!(ip2.ipv4, Some("203.0.113.6".parse().unwrap()));
        assert_eq!(ip2.wan_type, "0");
    }

    #[test]
    fn lan_hosts_from_fixture() {
        let h = LanHost::parse_all(&fixture("cbGetLanHostList", "cbGetLanHostList"));
        assert!(h.len() >= 2);
        assert_eq!(h[0].mac, "02:00:00:00:00:01");
        assert_eq!(h[0].ip, "192.168.1.60");
        assert!(h[0].active);
        assert!(!h[1].active);
        assert_eq!(h[1].hostname, "host-1");
    }

    #[test]
    fn dhcp_leases_from_fixture() {
        let name = "cbDhcpV4SvrPoolGetClt__rtm_cfg_dhcpv4_svr_pool_id=0_all=ALL";
        let l = DhcpLease::parse_all(&fixture("cbDhcpV4SvrPoolGetClt", name));
        assert_eq!(l.len(), 8);
        assert_eq!(l[0].ip, "192.168.1.60");
        assert_eq!(l[0].expires_s, Some(28 * 60 + 15));
    }

    #[test]
    fn lease_time_formats() {
        assert_eq!(parse_lease_time("1,02:03:04"), Some(86400 + 7384));
        assert_eq!(parse_lease_time("basura"), None);
        assert_eq!(parse_lease_time(""), None);
    }

    #[test]
    fn static_leases_from_fixture() {
        let name = "cbDhcpV4SvrPoolGetStatic__rtm_cfg_dhcpv4_svr_pool_id=0_all=ALL";
        let l = StaticLease::parse_all(&fixture("cbDhcpV4SvrPoolGetStatic", name));
        assert!(!l.is_empty());
        assert_eq!(l[0].id, "0");
        assert_eq!(l[0].mac, "02:00:00:00:00:04");
        assert_eq!(l[0].ip, "192.168.1.68");
        assert!(l[0].enabled);
    }

    #[test]
    fn wifi_radios_from_fixtures() {
        let r0 =
            WifiRadio::from_node(&fixture("cbGetWifi", "cbGetWifi__dpm_cfg_wifi_0i=0")).unwrap();
        let r1 =
            WifiRadio::from_node(&fixture("cbGetWifi", "cbGetWifi__dpm_cfg_wifi_0i=1")).unwrap();
        assert_eq!((r0.id.as_str(), r0.band.as_str()), ("0", "2.4GHz"));
        assert_eq!((r1.id.as_str(), r1.band.as_str()), ("1", "5GHz"));
        assert!(r0.enabled);
        assert_eq!(r0.channel, "auto");
        assert_eq!(r1.channel, "100");
    }

    #[test]
    fn wifi_aps_from_fixtures() {
        let n = fixture("cbListWifiAccPt", "cbListWifiAccPt__dpm_cfg_wifi_id=0");
        let aps = WifiAp::parse_all(&n, "0");
        assert_eq!(aps.len(), 4);
        assert_eq!(aps[0].ssid, "RED-2");
        assert_eq!(aps[0].radio_id, "0");
        assert!(!aps[0].enabled);
        assert!(aps[0].wps);
        assert_eq!(aps[0].security, "3/4");
    }

    #[test]
    fn assoc_group_only_has_no_stations() {
        let n = fixture(
            "cbListWifiAccPtAssociation",
            "cbListWifiAccPtAssociation__dpm_cfg_wifi_id=0",
        );
        assert!(WifiAssoc::parse_all(&n).is_empty());
    }

    #[test]
    fn assoc_from_synthetic_fixture() {
        let n = fixture(
            "cbListWifiAccPtAssociation",
            "cbListWifiAccPtAssociation__dpm_cfg_wifi_id=9",
        );
        let a = WifiAssoc::parse_all(&n);
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].ap_id, "0");
        assert_eq!(a[0].mac, "02:00:00:00:01:01");
        assert_eq!(a[0].rssi, Some(-52));
        assert_eq!(a[1].rssi, Some(-71));
    }

    #[test]
    fn odd_values_never_panic() {
        assert_eq!(num::<u64>("X-REDACTED-X"), 0);
        assert_eq!(num::<i64>(""), 0);
        assert_eq!(num::<f32>("7,05"), 0.0);
    }
}
