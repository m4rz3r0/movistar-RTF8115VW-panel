//! Reservas DHCP: alta, lectura por id y peticiones incompletas.

use super::*;

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
async fn reserva_leida_por_id_conserva_el_0i_menos_uno_de_las_fixtures() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.write(
        "dhcp.asp",
        "cbDhcpV4SvrPoolSetStatic",
        &add_static("02:00:00:00:aa:07", "192.0.2.7"),
    )
    .await
    .unwrap();
    let por_id = |id: &'static str| {
        [
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("rtm_dhcpv4_svr_pool_static_id", id),
        ]
    };
    for id in ["0", "2"] {
        let n = c
            .call("cbDhcpV4SvrPoolGetStatic", &por_id(id))
            .await
            .unwrap();
        assert_eq!(
            campo_de(
                &n,
                "rtm_dhcpv4_svr_pool_static",
                "rtm_dhcpv4_svr_pool_static_0i"
            )
            .as_deref(),
            Some("-1"),
            "lectura de una reserva (id {id}): como el router real"
        );
    }
    let todas = c
        .call("cbDhcpV4SvrPoolGetStatic", LIST_STATIC)
        .await
        .unwrap();
    let indices: Vec<&str> = todas
        .all("rtm_dhcpv4_svr_pool_static")
        .into_iter()
        .map(|e| e.text_of("rtm_dhcpv4_svr_pool_static_0i").unwrap())
        .collect();
    assert_eq!(indices, ["0", "1", "2"], "la lista conserva los índices");
}

#[tokio::test]
async fn reservas_dhcp_incompletas_o_de_otro_pool_se_rechazan() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let escribir = |p: Vec<(&'static str, &'static str)>| {
        let c = c.clone();
        async move { c.write("dhcp.asp", "cbDhcpV4SvrPoolSetStatic", &p).await }
    };
    let antes = leases(
        &c.call("cbDhcpV4SvrPoolGetStatic", LIST_STATIC)
            .await
            .unwrap(),
    );
    let casos: Vec<(&str, Vec<(&'static str, &'static str)>)> = vec![
        (
            "pool desconocido",
            vec![
                ("rtm_cfg_dhcpv4_svr_pool_id", "1"),
                ("rtm_dhcpv4_svr_pool_static_id", "-1"),
                ("mac_addr", "02:00:00:00:aa:08"),
                ("ipv4_addr", "192.0.2.8"),
            ],
        ),
        (
            "falta el id",
            vec![
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("mac_addr", "02:00:00:00:aa:08"),
                ("ipv4_addr", "192.0.2.8"),
            ],
        ),
        (
            "falta mac_addr",
            vec![
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "-1"),
                ("ipv4_addr", "192.0.2.8"),
            ],
        ),
        (
            "falta ipv4_addr",
            vec![
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "-1"),
                ("mac_addr", "02:00:00:00:aa:08"),
            ],
        ),
        (
            "no existe",
            vec![
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "99"),
                ("ipv4_addr", "192.0.2.8"),
            ],
        ),
    ];
    for (esperado, p) in casos {
        match escribir(p).await.unwrap_err() {
            Error::CallFailed { error, .. } => assert_eq!(error, esperado),
            e => panic!("{esperado}: {e:?}"),
        }
    }
    let despues = leases(
        &c.call("cbDhcpV4SvrPoolGetStatic", LIST_STATIC)
            .await
            .unwrap(),
    );
    assert_eq!(despues, antes, "nada de eso cambia el estado");
    // La creación válida sin adm_state lo deja activado, como antes.
    c.write(
        "dhcp.asp",
        "cbDhcpV4SvrPoolSetStatic",
        &[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("rtm_dhcpv4_svr_pool_static_id", "-1"),
            ("mac_addr", "02:00:00:00:aa:09"),
            ("ipv4_addr", "192.0.2.9"),
        ],
    )
    .await
    .unwrap();
    let n = c
        .call("cbDhcpV4SvrPoolGetStatic", LIST_STATIC)
        .await
        .unwrap();
    let nueva = n.all("rtm_dhcpv4_svr_pool_static").pop().unwrap();
    assert_eq!(nueva.text_of("adm_state"), Some("1"));
}
