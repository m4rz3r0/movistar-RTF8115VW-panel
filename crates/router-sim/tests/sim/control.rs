//! Controles del simulador: WAN, capas, contadores y fallos provocados.

use super::*;

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
