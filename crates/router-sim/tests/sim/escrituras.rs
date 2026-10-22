//! Escrituras que se leen después, por la web avanzada y por la básica.

use super::*;

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

/// El pool DHCP es una lista cuya fixture sin parámetros solo trae el índice
/// (ninguna instancia): una escritura del pool 0 se ve en su lectura, sin
/// tocar el resto del pool.
#[tokio::test]
async fn escritura_del_pool_dhcp_se_lee() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let leer = || {
        c.call(
            "cbGetDhcpv4ServerPool",
            &[("rtm_cfg_dhcpv4_svr_pool_0i", "0")],
        )
    };
    let antes = leer().await.unwrap();
    c.write(
        "dhcp.asp",
        "cbSetDhcpv4ServerPool",
        &[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("ipv4_addr_min", "192.0.2.10"),
        ],
    )
    .await
    .unwrap();
    let despues = leer().await.unwrap();
    assert_eq!(
        valor(&despues, "ipv4_addr_min").as_deref(),
        Some("192.0.2.10")
    );
    assert_eq!(
        valor(&despues, "ipv4_addr_max"),
        valor(&antes, "ipv4_addr_max")
    );
    assert_eq!(
        valor(&despues, "rtm_cfg_dhcpv4_svr_pool_id").as_deref(),
        Some("0")
    );
}

/// Encender o apagar una red WiFi (`adm_state`) cambia también si emite
/// (`opr_status`), en su lectura y en la lista por radio, como en el router.
#[tokio::test]
async fn encender_una_red_wifi_la_pone_a_emitir() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let uno = [("dpm_cfg_wifi_acc_pt_id", "1")];
    let radio = [("dpm_cfg_wifi_id", "1")];
    let emite = |n: &XmlNode| valor(n, "opr_status").unwrap();
    assert_eq!(emite(&c.call("cbGetWifiAccPt", &uno).await.unwrap()), "0");
    for estado in ["1", "0"] {
        c.write(
            "wifi.asp",
            "cbSetWifiAccPt",
            &[("dpm_cfg_wifi_acc_pt_id", "1"), ("adm_state", estado)],
        )
        .await
        .unwrap();
        assert_eq!(
            emite(&c.call("cbGetWifiAccPt", &uno).await.unwrap()),
            estado
        );
        assert_eq!(
            emite(&c.call("cbListWifiAccPt", &radio).await.unwrap()),
            estado,
            "la lista por radio también lo refleja"
        );
    }
}

/// El registro del sistema: lo que se escribe se ve en su lectura.
#[tokio::test]
async fn escritura_del_registro_se_lee() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.write(
        "syslog.asp",
        "cbSetSyslog",
        &[("syslog_mode", "2"), ("remote_ip", "192.0.2.5")],
    )
    .await
    .unwrap();
    let n = c.call("cbGetSyslog", &[]).await.unwrap();
    assert_eq!(valor(&n, "syslog_mode").as_deref(), Some("2"));
    assert_eq!(valor(&n, "remote_ip").as_deref(), Some("192.0.2.5"));
}
