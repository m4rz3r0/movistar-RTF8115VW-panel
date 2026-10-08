//! Pruebas del router simulado y de las fixtures saneadas.

use hgu::{Client, ClientConfig, Error, xml::XmlNode};
use router_sim::{Sim, SimConfig, SimHandle, sanitize};
use std::{collections::HashSet, net::Ipv4Addr, path::PathBuf, time::Duration};

const USER: &str = "prueba";
const PASS: &str = "clave-de-prueba";

fn fixtures_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../hgu/fixtures"))
}

async fn start() -> SimHandle {
    Sim::start(SimConfig::new(USER, PASS, fixtures_dir())).await
}

fn client(sim: &SimHandle, timeout: Duration) -> Client {
    Client::new(ClientConfig {
        base_url: sim.base_url.clone(),
        user: USER.into(),
        password: PASS.into(),
        timeout,
        min_gap: Duration::from_millis(1),
    })
}

/// Recorre las hojas `(etiqueta, texto)` de un documento.
fn leaves(raw: &str) -> Vec<(String, String)> {
    let doc = roxmltree::Document::parse(raw).expect("fixture XML válida");
    doc.descendants()
        .filter(|n| n.is_element() && !n.children().any(|c| c.is_element()))
        .map(|n| {
            (
                n.tag_name().name().to_owned(),
                n.text().unwrap_or("").trim().to_owned(),
            )
        })
        .collect()
}

fn mac_ok(v: &str) -> bool {
    let re = regex::Regex::new(r"(?i)\b[0-9a-f]{2}(?::[0-9a-f]{2}){5}\b").unwrap();
    re.find_iter(v).all(|m| {
        let m = m.as_str().to_ascii_lowercase();
        m.starts_with("02:00:00:00:") || m == "00:00:00:00:00:00" || m == "ff:ff:ff:ff:ff:ff"
    })
}

#[test]
fn fixtures_have_no_secrets() {
    let secrets: HashSet<String> = match std::env::var_os("RTF_SECRETS_FILE") {
        Some(path) => std::fs::read_to_string(path)
            .expect("RTF_SECRETS_FILE legible")
            .lines()
            .filter(|l| l.len() >= 4)
            .map(str::to_owned)
            .collect(),
        None => HashSet::new(),
    };
    let ipv4 = regex::Regex::new(r"\b\d{1,3}(?:\.\d{1,3}){3}\b").unwrap();
    let mut files = 0;
    for entry in std::fs::read_dir(fixtures_dir()).expect("directorio de fixtures") {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "xml") {
            continue;
        }
        files += 1;
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        let raw = std::fs::read_to_string(&path).unwrap();
        for (tag, value) in leaves(&raw) {
            assert!(
                !secrets.contains(&value),
                "{file}: <{tag}> contiene un secreto real"
            );
            assert!(
                !secrets
                    .iter()
                    .any(|s| s.len() >= 8 && value.contains(s.as_str())),
                "{file}: <{tag}> contiene un secreto real dentro del valor"
            );
            if value.is_empty() {
                continue;
            }
            if tag.eq_ignore_ascii_case("ssid") {
                assert!(value.starts_with("RED-"), "{file}: <{tag}> sin sanear");
            } else if sanitize::is_sensitive(&tag) {
                assert_eq!(value, sanitize::REDACTED, "{file}: <{tag}> sin sanear");
            } else if tag == "hostname" {
                assert!(value.starts_with("host-"), "{file}: <{tag}> sin sanear");
            }
            assert!(mac_ok(&value), "{file}: <{tag}> contiene una MAC real");
            for m in ipv4.find_iter(&value) {
                if let Ok(ip) = m.as_str().parse::<Ipv4Addr>() {
                    assert!(!sanitize::is_public_ipv4(ip), "{file}: <{tag}> IP pública");
                }
            }
        }
    }
    assert!(files > 50, "faltan fixtures ({files})");
}

fn leases(n: &XmlNode) -> Vec<(String, String, String)> {
    n.all("rtm_dhcpv4_svr_pool_static")
        .into_iter()
        .map(|e| {
            (
                e.text_of("rtm_dhcpv4_svr_pool_static_id")
                    .unwrap()
                    .to_owned(),
                e.text_of("mac_addr").unwrap().to_owned(),
                e.text_of("ipv4_addr").unwrap().to_owned(),
            )
        })
        .collect()
}

const LIST_STATIC: &[(&str, &str)] = &[("rtm_cfg_dhcpv4_svr_pool_id", "0"), ("all", "ALL")];

fn add_static(mac: &'static str, ip: &'static str) -> [(&'static str, &'static str); 5] {
    [
        ("adm_state", "1"),
        ("mac_addr", mac),
        ("ipv4_addr", ip),
        ("rtm_dhcpv4_svr_pool_static_id", "-1"),
        ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
    ]
}

#[tokio::test]
async fn static_lease_add_then_list() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let before = leases(
        &c.call("cbDhcpV4SvrPoolGetStatic", LIST_STATIC)
            .await
            .unwrap(),
    );
    assert!(!before.is_empty(), "la fixture trae reservas");
    let next_id = before
        .iter()
        .map(|(id, _, _)| id.parse::<u32>().unwrap())
        .max()
        .unwrap()
        + 1;

    let params = add_static("02:00:00:00:aa:01", "192.168.1.201");
    c.write("dhcp.asp", "cbDhcpV4SvrPoolSetStatic", &params)
        .await
        .unwrap();

    let after = leases(
        &c.call("cbDhcpV4SvrPoolGetStatic", LIST_STATIC)
            .await
            .unwrap(),
    );
    assert_eq!(after.len(), before.len() + 1);
    assert_eq!(
        after.last().unwrap(),
        &(
            next_id.to_string(),
            "02:00:00:00:aa:01".to_owned(),
            "192.168.1.201".to_owned()
        )
    );
    let one = c
        .call(
            "cbDhcpV4SvrPoolGetStatic",
            &[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", &next_id.to_string()),
            ],
        )
        .await
        .unwrap();
    assert_eq!(leases(&one), vec![after.last().unwrap().clone()]);

    let writes = sim.control.writes();
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].0, "cbDhcpV4SvrPoolSetStatic");
    assert!(writes[0].1.iter().all(|(k, _)| k != "sessionKey"));
    assert!(
        writes[0]
            .1
            .contains(&("mac_addr".into(), "02:00:00:00:aa:01".into()))
    );

    let id = next_id.to_string();
    c.write(
        "dhcp.asp",
        "cbDhcpV4SvrPoolSetStatic",
        &[
            ("rtm_dhcpv4_svr_pool_static_id", &id),
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("del", "DEL"),
        ],
    )
    .await
    .unwrap();
    let last = leases(
        &c.call("cbDhcpV4SvrPoolGetStatic", LIST_STATIC)
            .await
            .unwrap(),
    );
    assert_eq!(last, before);
}

#[tokio::test]
async fn bad_session_key_rejected() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let stale = c.session_key("upnp.asp").await.unwrap();
    let fresh = c.session_key("upnp.asp").await.unwrap();
    assert_ne!(stale, fresh, "el sessionKey rota en cada carga");
    for key in ["0", stale.as_str()] {
        let r = c
            .call("cbSetUpnpCfg", &[("adm_state", "0"), ("sessionKey", key)])
            .await;
        assert!(
            matches!(&r, Err(Error::CallFailed { result, .. }) if result == "1"),
            "{r:?}"
        );
    }
    assert!(sim.control.writes().is_empty());
    let n = c.call("cbGetUpnpCfg", &[]).await.unwrap();
    let before = n
        .get("upnp_cfg")
        .unwrap()
        .text_of("adm_state")
        .unwrap()
        .to_owned();
    let flipped = if before == "1" { "0" } else { "1" };
    c.write("upnp.asp", "cbSetUpnpCfg", &[("adm_state", flipped)])
        .await
        .unwrap();
    let n = c.call("cbGetUpnpCfg", &[]).await.unwrap();
    assert_eq!(
        n.get("upnp_cfg").unwrap().text_of("adm_state"),
        Some(flipped)
    );
}

fn ip2<'a>(n: &'a XmlNode, entry: &str) -> &'a XmlNode {
    n.all(entry)
        .into_iter()
        .find(|e| e.text_of("name") == Some("ip2"))
        .expect("entrada ip2")
}

#[tokio::test]
async fn wan_down_reflected() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let wan = |n: XmlNode| {
        ip2(&n, "mll_wanconn_data")
            .text_of("opr_status")
            .unwrap()
            .to_owned()
    };
    assert_eq!(wan(c.call("cbGetWanIPIntf", &[]).await.unwrap()), "1");
    sim.control.set_wan_up(false);
    assert_eq!(wan(c.call("cbGetWanIPIntf", &[]).await.unwrap()), "0");
    sim.control.set_wan_up(true);
    assert_eq!(wan(c.call("cbGetWanIPIntf", &[]).await.unwrap()), "1");
}

#[tokio::test]
async fn counters_bump_adds_to_ip2() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let bytes = |n: XmlNode| {
        let e = ip2(&n, "rtm_cfg_ip_intf");
        let v = |t| e.text_of(t).unwrap().parse::<u64>().unwrap();
        (v("rx_byte"), v("tx_byte"))
    };
    let (rx, tx) = bytes(c.call("cbGetStatistic", &[]).await.unwrap());
    sim.control.counters_bump(1000, 20);
    sim.control.counters_bump(1, 2);
    let after = bytes(c.call("cbGetStatistic", &[]).await.unwrap());
    assert_eq!(after, (rx + 1001, tx + 22));
}

#[tokio::test]
async fn ignore_next_write_keeps_state() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let before = leases(
        &c.call("cbDhcpV4SvrPoolGetStatic", LIST_STATIC)
            .await
            .unwrap(),
    );
    sim.control.ignore_next_write();
    let params = add_static("02:00:00:00:aa:02", "192.168.1.202");
    c.write("dhcp.asp", "cbDhcpV4SvrPoolSetStatic", &params)
        .await
        .unwrap();
    let after = leases(
        &c.call("cbDhcpV4SvrPoolGetStatic", LIST_STATIC)
            .await
            .unwrap(),
    );
    assert_eq!(after, before);
    assert_eq!(sim.control.writes().len(), 1, "la escritura se registra");
    c.write("dhcp.asp", "cbDhcpV4SvrPoolSetStatic", &params)
        .await
        .unwrap();
    let again = leases(
        &c.call("cbDhcpV4SvrPoolGetStatic", LIST_STATIC)
            .await
            .unwrap(),
    );
    assert_eq!(again.len(), before.len() + 1, "solo se ignora una");
}

#[tokio::test]
async fn fail_next_write_returns_error_once() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    sim.control.fail_next_write("fallo simulado");
    let r = c
        .write("upnp.asp", "cbSetUpnpCfg", &[("adm_state", "1")])
        .await;
    assert_eq!(
        r.unwrap_err(),
        Error::CallFailed {
            name: "cbSetUpnpCfg".into(),
            result: "1".into(),
            error: "fallo simulado".into()
        }
    );
    c.write("upnp.asp", "cbSetUpnpCfg", &[("adm_state", "1")])
        .await
        .unwrap();
}

#[tokio::test]
async fn wifi_write_reflected_in_getter() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let get = [("dpm_cfg_wifi_acc_pt_id", "1")];
    let n = c.call("cbGetWifiAccPt", &get).await.unwrap();
    let ap = n.get("dpm_cfg_wifi_acc_pt").unwrap();
    assert_eq!(ap.text_of("dpm_cfg_wifi_acc_pt_id"), Some("1"));
    let params = [
        ("dpm_cfg_wifi_acc_pt_id", "1"),
        ("ssid", "RED-nueva"),
        ("validation_only", "1"),
    ];
    c.write("wifi.asp", "cbSetWifiAccPt", &params)
        .await
        .unwrap();
    let n = c.call("cbGetWifiAccPt", &get).await.unwrap();
    assert_eq!(
        n.get("dpm_cfg_wifi_acc_pt").unwrap().text_of("ssid"),
        ap.text_of("ssid"),
        "validation_only no aplica"
    );
    let params = [("dpm_cfg_wifi_acc_pt_id", "1"), ("ssid", "RED-nueva")];
    c.write("wifi.asp", "cbSetWifiAccPt", &params)
        .await
        .unwrap();
    let n = c.call("cbGetWifiAccPt", &get).await.unwrap();
    assert_eq!(
        n.get("dpm_cfg_wifi_acc_pt").unwrap().text_of("ssid"),
        Some("RED-nueva")
    );
}

#[tokio::test]
async fn reboot_and_invalidate_sessions_force_relogin() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.write("reboot.asp", "cbReboot", &[]).await.unwrap();
    assert_eq!(sim.control.writes()[0].0, "cbReboot");
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
    sim.control.invalidate_sessions();
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
}

#[tokio::test]
async fn unknown_call_has_no_fixture() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let r = c.call("cbGetNoExiste", &[]).await;
    assert!(
        matches!(&r, Err(Error::CallFailed { error, .. }) if error == "no fixture"),
        "{r:?}"
    );
}

#[tokio::test]
async fn params_select_fixture_in_any_order() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let a = c
        .call(
            "cbGetLanIPv4Addr",
            &[("ip_intf_id", "1"), ("ipv4_addr_id", "2")],
        )
        .await
        .unwrap();
    let b = c
        .call(
            "cbGetLanIPv4Addr",
            &[("ipv4_addr_id", "2"), ("ip_intf_id", "1")],
        )
        .await
        .unwrap();
    assert_eq!(a, b);
    let plain = c.call("cbGetLanIPv4Addr", &[]).await.unwrap();
    let other = c
        .call("cbGetLanIPv4Addr", &[("desconocido", "1")])
        .await
        .unwrap();
    assert_eq!(plain, other, "sin coincidencia exacta se usa <name>.xml");
}

#[tokio::test]
async fn unresponsive_times_out() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(1));
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
    sim.control.set_unresponsive(true);
    let r = c.call("cbGetUpnpCfg", &[]).await;
    assert!(matches!(r, Err(Error::Network(_))), "{r:?}");
    sim.control.set_unresponsive(false);
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
}
