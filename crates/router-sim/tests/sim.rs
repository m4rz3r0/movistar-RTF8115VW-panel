//! Pruebas del router simulado y de las fixtures saneadas.

use hgu::{Client, ClientConfig, Error, xml::XmlNode};
use router_sim::{Sim, SimConfig, SimHandle, sanitize};
use std::{collections::HashSet, net::Ipv4Addr, path::PathBuf, sync::LazyLock, time::Duration};

const USER: &str = "prueba";
const PASS: &str = "clave-de-prueba";

fn fixtures_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../hgu/fixtures"))
}

fn variantes_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../hgu/fixtures-variantes"
    ))
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

static IPV4: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b\d{1,3}(?:\.\d{1,3}){3}\b").unwrap());

/// Valores reales que no deben aparecer, de `$RTF_SECRETS_FILE` (uno por línea).
///
/// Lo genera `tools/sanear-fixtures.sh` con los valores retirados de las
/// capturas, y se le pueden añadir a mano valores conocidos por otra vía que no
/// salen en las capturas (contraseña del router, claves WiFi…): la herramienta
/// conserva las líneas existentes al regenerarlo.
fn real_secrets() -> HashSet<String> {
    match std::env::var_os("RTF_SECRETS_FILE") {
        Some(path) => std::fs::read_to_string(path)
            .expect("RTF_SECRETS_FILE legible")
            .lines()
            .filter(|l| l.len() >= 4)
            .map(str::to_owned)
            .collect(),
        None => HashSet::new(),
    }
}

/// Comprueba un valor de la fixture `file` (de un elemento o de un parámetro
/// del nombre del fichero).
fn check_value(file: &str, tag: &str, value: &str, secrets: &HashSet<String>) {
    use sanitize::TagClass;
    let class = sanitize::tag_class(tag)
        .unwrap_or_else(|| panic!("{file}: <{tag}> no está en fixtures-tags.txt"));
    assert!(
        !secrets.contains(value),
        "{file}: <{tag}> contiene un secreto real"
    );
    assert!(
        !secrets
            .iter()
            .any(|s| s.len() >= 8 && value.contains(s.as_str())),
        "{file}: <{tag}> contiene un secreto real dentro del valor"
    );
    if value.is_empty() {
        return;
    }
    let pseudo = |prefix: &str| {
        value
            .strip_prefix(prefix)
            .is_some_and(|n| n.parse::<u32>().is_ok())
    };
    match class {
        TagClass::Redact => assert_eq!(value, sanitize::REDACTED, "{file}: <{tag}> sin sanear"),
        TagClass::Ssid => assert!(pseudo("RED-"), "{file}: <{tag}> sin sanear"),
        TagClass::Host => assert!(pseudo("host-"), "{file}: <{tag}> sin sanear"),
        TagClass::Desc => assert!(
            pseudo("desc-") || ["LAN", "Default"].contains(&value) || value.parse::<u64>().is_ok(),
            "{file}: <{tag}> sin sanear"
        ),
        TagClass::Keep => assert!(
            value == sanitize::REDACTED || !sanitize::is_sensitive(tag, value),
            "{file}: <{tag}> sin sanear"
        ),
    }
    for mac in sanitize::macs_in(tag, value) {
        assert!(
            sanitize::is_fake_mac(&mac),
            "{file}: <{tag}> contiene una MAC real"
        );
    }
    assert!(
        sanitize::global_ipv6_in(value).is_empty(),
        "{file}: <{tag}> contiene una IPv6 global"
    );
    for m in IPV4.find_iter(value) {
        if let Ok(ip) = m.as_str().parse::<Ipv4Addr>() {
            assert!(!sanitize::is_public_ipv4(ip), "{file}: <{tag}> IP pública");
        }
    }
}

#[test]
fn fixtures_have_no_secrets() {
    let secrets = real_secrets();
    // Las de serie y las de cada variante (`fixtures-variantes/*/`).
    let mut dirs = vec![fixtures_dir()];
    let variantes = variantes_dir();
    for entry in std::fs::read_dir(&variantes).expect("directorio de variantes") {
        let path = entry.unwrap().path();
        assert!(
            path.is_dir(),
            "{}: se esperaba un directorio",
            path.display()
        );
        dirs.push(path);
    }
    assert!(dirs.len() > 1, "faltan las variantes");
    let mut files = 0;
    for dir in dirs {
        for entry in std::fs::read_dir(&dir).expect("directorio de fixtures") {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let Some(stem) = name.strip_suffix(".xml") else {
                continue;
            };
            files += 1;
            for (k, v) in sanitize::split_fixture_name(stem).1 {
                check_value(&name, &k, &v, &secrets);
            }
            let raw = std::fs::read_to_string(&path).unwrap();
            for (tag, value) in leaves(&raw) {
                check_value(&name, &tag, &value, &secrets);
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

    let params = add_static("02:00:00:00:aa:01", "192.168.1.88");
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
            "192.168.1.88".to_owned()
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
async fn capa_sustituye_fixtures_y_mueve_su_wan() {
    let sim = Sim::start(
        SimConfig::new(USER, PASS, fixtures_dir()).con_capa(variantes_dir().join("dhcp-sin-voip")),
    )
    .await;
    let c = client(&sim, Duration::from_secs(5));
    let estado = |n: XmlNode| -> Vec<(String, String)> {
        n.all("mll_wanconn_data")
            .iter()
            .map(|w| {
                (
                    w.text_of("name").unwrap().to_owned(),
                    w.text_of("opr_status").unwrap().to_owned(),
                )
            })
            .collect()
    };
    let wan = c.call("cbGetWanIPIntf", &[]).await.unwrap();
    assert_eq!(estado(wan), [("ip5".to_owned(), "1".to_owned())]);
    // Lo que no está en la capa sale de las fixtures de serie.
    assert!(c.call("cbGetSummary", &[]).await.is_ok());
    sim.control.set_wan_up(false);
    let wan = c.call("cbGetWanIPIntf", &[]).await.unwrap();
    assert_eq!(estado(wan), [("ip5".to_owned(), "0".to_owned())]);
}

#[tokio::test]
async fn fail_reads_hasta_desactivar() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    sim.control.fail_reads("cbGetTr69Config", true);
    for _ in 0..2 {
        assert!(matches!(
            c.call("cbGetTr69Config", &[]).await,
            Err(Error::CallFailed { .. })
        ));
    }
    assert!(c.call("cbGetSummary", &[]).await.is_ok());
    sim.control.fail_reads("cbGetTr69Config", false);
    assert!(c.call("cbGetTr69Config", &[]).await.is_ok());
}

#[tokio::test]
async fn counters_bump_moves_ip2_and_lan_port() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let bytes = |n: &XmlNode, entry: &str, name: &str| {
        let e = n
            .all(entry)
            .into_iter()
            .find(|e| e.text_of("name") == Some(name))
            .expect("entrada");
        let v = |t| e.text_of(t).unwrap().parse::<u64>().unwrap();
        (v("rx_byte"), v("tx_byte"))
    };
    let before = c.call("cbGetStatistic", &[]).await.unwrap();
    sim.control.counters_bump(1000, 20);
    sim.control.counters_bump(1, 2);
    sim.control.counters_bump_iface("ra0", 5, 7);
    let after = c.call("cbGetStatistic", &[]).await.unwrap();
    let (rx, tx) = bytes(&before, "rtm_cfg_ip_intf", "ip2");
    assert_eq!(
        bytes(&after, "rtm_cfg_ip_intf", "ip2"),
        (rx + 1001, tx + 22)
    );
    // Lado LAN: lo que baja de internet sale (tx) por el puerto del equipo.
    let (rx, tx) = bytes(&before, "dpm_cfg_eth", "eth0.4");
    assert_eq!(bytes(&after, "dpm_cfg_eth", "eth0.4"), (rx + 22, tx + 1001));
    let (rx, tx) = bytes(&before, "dpm_wifi_ap", "ra0");
    assert_eq!(bytes(&after, "dpm_wifi_ap", "ra0"), (rx + 5, tx + 7));
    let (rx, tx) = bytes(&before, "dpm_cfg_eth", "eth0.1");
    assert_eq!(bytes(&after, "dpm_cfg_eth", "eth0.1"), (rx, tx));
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
    let params = add_static("02:00:00:00:aa:02", "192.168.1.89");
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
async fn validation_call_does_not_consume_ignore_next_write() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let get = [("dpm_cfg_wifi_acc_pt_id", "1")];
    let ssid = |n: XmlNode| {
        n.get("dpm_cfg_wifi_acc_pt")
            .unwrap()
            .text_of("ssid")
            .unwrap()
            .to_owned()
    };
    let before = ssid(c.call("cbGetWifiAccPt", &get).await.unwrap());
    sim.control.ignore_next_write();
    let mut params = vec![
        ("dpm_cfg_wifi_acc_pt_id", "1"),
        ("ssid", "RED-nueva"),
        ("validation_only", "1"),
    ];
    c.write("wifi.asp", "cbSetWifiAccPt", &params)
        .await
        .unwrap();
    params.pop();
    c.write("wifi.asp", "cbSetWifiAccPt", &params)
        .await
        .unwrap();
    assert_eq!(ssid(c.call("cbGetWifiAccPt", &get).await.unwrap()), before);
    let writes = sim.control.writes();
    assert_eq!(writes.len(), 1, "la validación no se registra");
    assert!(writes[0].1.iter().all(|(k, _)| k != "validation_only"));
    c.write("wifi.asp", "cbSetWifiAccPt", &params)
        .await
        .unwrap();
    assert_eq!(
        ssid(c.call("cbGetWifiAccPt", &get).await.unwrap()),
        "RED-nueva"
    );
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

#[tokio::test]
async fn reboot_resets_uptime() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let uptime = |n: &XmlNode| n.text_of("SysUptime").unwrap().parse::<u64>().unwrap();
    let before = uptime(&c.call("cbGetSummary", &[]).await.unwrap());
    assert!(before > 1000);
    c.write("reboot.asp", "cbReboot", &[]).await.unwrap();
    let after = uptime(&c.call("cbGetSummary", &[]).await.unwrap());
    assert!(after < 5, "{after}");
}

#[tokio::test]
async fn basic_write_por_get_con_session_key() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.basic_write(
        "te_wifi.asp",
        "te_wifi.wl",
        &[("wlSsid", "Casa nueva"), ("wlSsidIdx", "0")],
    )
    .await
    .unwrap();
    c.basic_write(
        "te_wifi.asp",
        "te_wifi.cmd?action=save",
        &[("wlMACaddr", "02:00:00:00:00:01")],
    )
    .await
    .unwrap();
    let w = sim.control.writes();
    assert_eq!(w.len(), 2);
    assert_eq!(w[0].0, "te_wifi.wl");
    assert_eq!(
        w[0].1,
        vec![
            ("wlSsid".to_owned(), "Casa nueva".to_owned()),
            ("wlSsidIdx".to_owned(), "0".to_owned())
        ]
    );
    assert_eq!(w[1].0, "te_wifi.cmd");
    assert_eq!(
        w[1].1,
        vec![
            ("action".to_owned(), "save".to_owned()),
            ("wlMACaddr".to_owned(), "02:00:00:00:00:01".to_owned())
        ]
    );
    // Sin sesión básica válida, el router redirige al login: el cliente
    // vuelve a entrar y repite con un sessionKey nuevo.
    sim.control.invalidate_sessions();
    c.basic_write("te_wifi.asp", "te_wifi.wl", &[("wlEnbl", "1")])
        .await
        .unwrap();
    assert_eq!(sim.control.writes().len(), 3);
}

#[tokio::test]
async fn reads_fail_after_next_write() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    sim.control.fail_reads_after_next_write("cbGetUpnpCfg");
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
    c.write("upnp.asp", "cbSetUpnpCfg", &[("adm_state", "1")])
        .await
        .unwrap();
    assert!(c.call("cbGetUpnpCfg", &[]).await.is_err());
    sim.control.fail_reads("cbGetUpnpCfg", false);
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
}

#[tokio::test]
async fn hangs_after_next_write() {
    let sim = start().await;
    let c = client(&sim, Duration::from_millis(300));
    sim.control.hang_after_next_write();
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
    c.write("upnp.asp", "cbSetUpnpCfg", &[("adm_state", "1")])
        .await
        .unwrap();
    let r = c.call("cbGetUpnpCfg", &[]).await;
    assert!(matches!(r, Err(Error::Network(_))), "{r:?}");
    sim.control.set_unresponsive(false);
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
}

#[tokio::test]
async fn lista_sin_selector_junta_sus_instancias() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    // Sin fixture propia sin parámetros: devuelve todas las instancias que
    // hay en sus fixtures, como hace el router con las listas que sí tienen
    // (`cbGetWanIPIntf`, `cbGetDnsRelayServer`).
    let radios = c.call("cbGetWifi", &[]).await.unwrap();
    assert_eq!(radios.all("dpm_cfg_wifi").len(), 2, "{radios:?}");
    let aps = c.call("cbListWifiAccPt", &[]).await.unwrap();
    assert_eq!(aps.all("dpm_cfg_wifi_acc_pt").len(), 8, "{aps:?}");
    // Una misma instancia en dos fixtures sale una vez.
    let ap = c.call("cbGetWifiAccPt", &[]).await.unwrap();
    assert_eq!(ap.all("dpm_cfg_wifi_acc_pt").len(), 1, "{ap:?}");
    // Con <name>.xml manda esa fixture.
    let wan = c.call("cbGetWanIPIntf", &[]).await.unwrap();
    assert_eq!(wan.all("mll_wanconn_data").len(), 3, "{wan:?}");
}

#[tokio::test]
async fn firmware_del_resumen_cambiable() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let fw = |n: &XmlNode| n.text_of("SWVersion").unwrap().to_owned();
    assert_eq!(
        fw(&c.call("cbGetSummary", &[]).await.unwrap()),
        "ES_g14.17_RTF_TEF001_V9.17_V034"
    );
    sim.control.set_firmware("ES_g14.18_RTF_TEF001_V9.18_V001");
    assert_eq!(
        fw(&c.call("cbGetSummary", &[]).await.unwrap()),
        "ES_g14.18_RTF_TEF001_V9.18_V001"
    );
}

#[tokio::test]
async fn contrasena_cambiable() {
    let sim = start().await;
    sim.control.set_password("otra-clave-del-router");
    let r = client(&sim, Duration::from_secs(2))
        .call("cbGetSummary", &[])
        .await;
    assert!(matches!(r, Err(Error::Auth(_))), "{r:?}");
    let mut cfg = ClientConfig::new(&sim.base_url, USER, "otra-clave-del-router");
    cfg.min_gap = Duration::from_millis(1);
    Client::new(cfg).call("cbGetSummary", &[]).await.unwrap();
}

// ------------------------------------------------ escritura genérica

/// Valor del primer nodo con hijo `campo` en `n` (o en sus descendientes).
fn valor(n: &XmlNode, campo: &str) -> Option<String> {
    n.text_of(campo)
        .map(str::to_owned)
        .or_else(|| n.children.iter().find_map(|c| valor(c, campo)))
}

#[tokio::test]
async fn escritura_generica_se_lee() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let antes = c.call("cbGetTimeNtp", &[]).await.unwrap();
    assert_eq!(valor(&antes, "ntp_svr_2").as_deref(), Some(""));
    c.write(
        "ntp.asp",
        "cbSetTimeNtp",
        &[("ntp_svr_2", "ntp.example"), ("campo_inexistente", "x")],
    )
    .await
    .unwrap();
    let despues = c.call("cbGetTimeNtp", &[]).await.unwrap();
    assert_eq!(valor(&despues, "ntp_svr_2").as_deref(), Some("ntp.example"));
    assert_eq!(valor(&despues, "ntp_svr_1"), valor(&antes, "ntp_svr_1"));
    assert_eq!(
        valor(&despues, "campo_inexistente"),
        None,
        "no se inventan hijos"
    );

    assert_eq!(
        valor(&c.call("cbGetTr69Config", &[]).await.unwrap(), "enableCWMP").as_deref(),
        Some("1")
    );
    c.write("cwmp.asp", "cbSetTr69Config", &[("enableCWMP", "0")])
        .await
        .unwrap();
    assert_eq!(
        valor(&c.call("cbGetTr69Config", &[]).await.unwrap(), "enableCWMP").as_deref(),
        Some("0")
    );
    assert_eq!(
        sim.control.fixture_valor("cbGetTr69Config", "enableCWMP"),
        Some("0".into())
    );
    assert_eq!(
        sim.control.fixture_valor("cbGetTr69Config", "no_existe"),
        None
    );
}

#[tokio::test]
async fn escritura_por_instancia_no_toca_otra() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let ssid = |n: &XmlNode| valor(n, "ssid").unwrap();
    let uno = [("dpm_cfg_wifi_acc_pt_id", "1")];
    let otro = c
        .call("cbListWifiAccPt", &[("dpm_cfg_wifi_id", "0")])
        .await
        .unwrap();
    let id_otro = valor(&otro, "dpm_cfg_wifi_acc_pt_id").unwrap();
    assert_ne!(id_otro, "1", "la fixture tiene un segundo punto de acceso");
    let ssid_otro = ssid(&otro);
    c.write(
        "wifi.asp",
        "cbSetWifiAccPt",
        &[("dpm_cfg_wifi_acc_pt_id", "1"), ("ssid", "RED-solo-uno")],
    )
    .await
    .unwrap();
    assert_eq!(
        ssid(&c.call("cbGetWifiAccPt", &uno).await.unwrap()),
        "RED-solo-uno"
    );
    let lista = c
        .call("cbListWifiAccPt", &[("dpm_cfg_wifi_id", "1")])
        .await
        .unwrap();
    assert_eq!(
        ssid(&lista),
        "RED-solo-uno",
        "la lista por radio también lo refleja"
    );
    let otro = c
        .call("cbListWifiAccPt", &[("dpm_cfg_wifi_id", "0")])
        .await
        .unwrap();
    assert_eq!(ssid(&otro), ssid_otro, "la otra instancia no cambia");
}

fn ids_nat(n: &XmlNode) -> Vec<(String, String)> {
    n.all("rtm_cfg_nat_port_mapping")
        .into_iter()
        .map(|e| {
            (
                e.text_of("nat_port_mapping_id").unwrap().to_owned(),
                e.text_of("desc").unwrap().to_owned(),
            )
        })
        .collect()
}

#[tokio::test]
async fn lista_crear_editar_borrar() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let todas = [("all", "ALL")];
    let inicial = ids_nat(&c.call("cbGetNatPortMapping", &todas).await.unwrap());
    assert_eq!(inicial.len(), 1);
    let id0 = inicial[0].0.clone();

    // Crear: sin id.
    c.write(
        "natportmap.asp",
        "cbSetNatPortMapping",
        &[("desc", "nueva"), ("int_ip_addr", "192.0.2.10")],
    )
    .await
    .unwrap();
    let tras = c.call("cbGetNatPortMapping", &todas).await.unwrap();
    let ids = ids_nat(&tras);
    assert_eq!(ids.len(), 2);
    let nuevo = ids.iter().find(|(_, d)| d == "nueva").unwrap().0.clone();
    assert_eq!(
        nuevo,
        (id0.parse::<u32>().unwrap() + 1).to_string(),
        "max + 1"
    );
    let por_indice = c
        .call("cbGetNatPortMapping", &[("nat_port_mapping_0i", "1")])
        .await
        .unwrap();
    assert_eq!(
        valor(&por_indice, "int_ip_addr").as_deref(),
        Some("192.0.2.10")
    );

    // Editar la existente.
    c.write(
        "natportmap.asp",
        "cbSetNatPortMapping",
        &[("nat_port_mapping_id", &id0), ("desc", "editada")],
    )
    .await
    .unwrap();
    let ids = ids_nat(&c.call("cbGetNatPortMapping", &todas).await.unwrap());
    assert_eq!(ids.len(), 2, "editar no crea");
    assert!(ids.contains(&(id0.clone(), "editada".into())));

    // Borrar la primera.
    c.write(
        "natportmap.asp",
        "cbSetNatPortMapping",
        &[("nat_port_mapping_id", &id0), ("del", "DEL")],
    )
    .await
    .unwrap();
    let ids = ids_nat(&c.call("cbGetNatPortMapping", &todas).await.unwrap());
    assert_eq!(ids, vec![(nuevo.clone(), "nueva".into())]);
    let r = c
        .call("cbGetNatPortMapping", &[("nat_port_mapping_id", &id0)])
        .await;
    assert!(r.is_err(), "una instancia borrada ya no se lee");
}

// ------------------------------------------------ cbModifyPwd y login

fn cliente_con(sim: &SimHandle, password: &str) -> Client {
    Client::new(ClientConfig {
        base_url: sim.base_url.clone(),
        user: USER.into(),
        password: password.into(),
        timeout: Duration::from_secs(5),
        min_gap: Duration::from_millis(1),
    })
}

fn cambio_pwd<'a>(actual: &'a str, nueva: &'a str) -> [(&'a str, &'a str); 3] {
    [("rqUser", USER), ("rqOPwd", actual), ("rqNPwd", nueva)]
}

#[tokio::test]
async fn modify_pwd_cambia_y_el_login_nuevo_entra() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.call("cbGetSummary", &[]).await.unwrap();
    c.write(
        "user_profile.asp",
        "cbModifyPwd",
        &cambio_pwd(PASS, "clave-nueva"),
    )
    .await
    .unwrap();
    // La sesión abierta sigue valiendo.
    c.call("cbGetSummary", &[]).await.unwrap();
    // La nueva entra; la vieja ya no.
    cliente_con(&sim, "clave-nueva")
        .call("cbGetSummary", &[])
        .await
        .unwrap();
    assert!(
        cliente_con(&sim, PASS)
            .call("cbGetSummary", &[])
            .await
            .is_err()
    );
}

#[tokio::test]
async fn modify_pwd_con_actual_mala_falla() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let e = c
        .write(
            "user_profile.asp",
            "cbModifyPwd",
            &cambio_pwd("otra", "clave-nueva"),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(&e, Error::CallFailed { result, .. } if result != "0"),
        "{e:?}"
    );
    let e = c
        .write(
            "user_profile.asp",
            "cbModifyPwd",
            &[("rqUser", "otro"), ("rqOPwd", PASS), ("rqNPwd", "x")],
        )
        .await
        .unwrap_err();
    assert!(matches!(e, Error::CallFailed { .. }));
    cliente_con(&sim, PASS)
        .call("cbGetSummary", &[])
        .await
        .unwrap();
}

#[tokio::test]
async fn modify_pwd_sin_efecto_responde_ok_y_no_cambia() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    sim.control.modify_pwd_sin_efecto();
    c.write(
        "user_profile.asp",
        "cbModifyPwd",
        &cambio_pwd(PASS, "clave-nueva"),
    )
    .await
    .unwrap();
    cliente_con(&sim, PASS)
        .call("cbGetSummary", &[])
        .await
        .unwrap();
    // Solo la próxima: la siguiente sí cambia.
    c.write(
        "user_profile.asp",
        "cbModifyPwd",
        &cambio_pwd(PASS, "clave-nueva"),
    )
    .await
    .unwrap();
    cliente_con(&sim, "clave-nueva")
        .call("cbGetSummary", &[])
        .await
        .unwrap();
}

#[tokio::test]
async fn login_falla_con_aunque_sea_la_vigente() {
    let sim = start().await;
    sim.control.login_falla_con(PASS);
    assert!(
        client(&sim, Duration::from_secs(5))
            .call("cbGetSummary", &[])
            .await
            .is_err()
    );
    sim.control.set_password("otra-clave");
    cliente_con(&sim, "otra-clave")
        .call("cbGetSummary", &[])
        .await
        .unwrap();
}
