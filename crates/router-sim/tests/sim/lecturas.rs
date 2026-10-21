//! Lecturas: fixtures por parámetros, listas, puntos de acceso y catálogo.

use super::*;

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

/// `reads()` anota cada lectura con sus parámetros, también las que fallan,
/// y no las escrituras.
#[tokio::test]
async fn lecturas_anotadas() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let antes = sim.control.reads().len();
    c.call("cbGetTimeNtp", &[]).await.unwrap();
    c.call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "0")])
        .await
        .unwrap();
    sim.control.fail_reads("cbGetUpnpCfg", true);
    assert!(c.call("cbGetUpnpCfg", &[]).await.is_err());
    c.write("ntp.asp", "cbSetTimeNtp", &[("ntp_svr_2", "ntp.example")])
        .await
        .unwrap();
    let leidas: Vec<(String, Vec<(String, String)>)> =
        sim.control.reads().into_iter().skip(antes).collect();
    let par = |k: &str, v: &str| (k.to_owned(), v.to_owned());
    assert_eq!(
        leidas,
        vec![
            ("cbGetTimeNtp".to_owned(), vec![]),
            (
                "cbGetWifiAccPt".to_owned(),
                vec![par("dpm_cfg_wifi_acc_pt_id", "0")]
            ),
            ("cbGetUpnpCfg".to_owned(), vec![]),
        ]
    );
}

/// Cualquier punto de acceso se lee por id, aunque solo esté en la lista por
/// radio (los ids 0 y 2 no tienen fixture propia), y una escritura en uno se
/// lee en ese y no en los demás.
#[tokio::test]
async fn cualquier_punto_de_acceso_se_lee_por_id() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let leer = async |id: &str| {
        c.call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", id)])
            .await
            .unwrap_or_else(|e| panic!("punto de acceso {id}: {e}"))
    };
    let mut antes = Vec::new();
    for id in ["0", "1", "2"] {
        let n = leer(id).await;
        assert_eq!(valor(&n, "dpm_cfg_wifi_acc_pt_id").as_deref(), Some(id));
        assert_eq!(
            valor(&n, "dpm_cfg_wifi_acc_pt_0i").as_deref(),
            Some("-1"),
            "leído por id, como el router"
        );
        assert!(valor(&n, "authentication").is_some(), "{id}");
        antes.push(valor(&n, "ssid").unwrap());
    }
    // Por índice también.
    let n = c
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_0i", "2")])
        .await
        .unwrap();
    assert_eq!(valor(&n, "ssid").as_deref(), Some(antes[2].as_str()));
    // Un id que no existe sigue fallando.
    assert!(
        c.call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "99")])
            .await
            .is_err()
    );
    c.write(
        "wifi.asp",
        "cbSetWifiAccPt",
        &[("dpm_cfg_wifi_acc_pt_id", "2"), ("ssid", "RED-invitados")],
    )
    .await
    .unwrap();
    assert_eq!(
        valor(&leer("2").await, "ssid").as_deref(),
        Some("RED-invitados")
    );
    assert_eq!(valor(&leer("0").await, "ssid").unwrap(), antes[0]);
    assert_eq!(valor(&leer("1").await, "ssid").unwrap(), antes[1]);
}

/// Cada ajuste del catálogo se lee en el sim con su lectura y sus
/// parámetros, y la respuesta trae el elemento de cada campo (salvo en una
/// lista vacía, como la de DNS dinámico de las fixtures).
#[tokio::test]
async fn cada_ajuste_del_catalogo_se_lee() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let catalogo = ajustes::CatalogoAjustes::load().unwrap();
    let mut fallos = Vec::new();
    for a in catalogo.todos() {
        let params: Vec<(&str, &str)> = a
            .leer
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let n = match c.call(&a.leer.funcion, &params).await {
            Ok(n) => n,
            Err(e) => {
                fallos.push(format!("{}: {e}", a.id));
                continue;
            }
        };
        let vacia = a.lista && n.children.iter().all(|h| h.tag == "Result");
        if vacia {
            continue;
        }
        for campo in &a.campos {
            let elementos =
                std::iter::once(&campo.lee).chain(campo.ademas.iter().map(|ad| &ad.lee));
            for lee in elementos {
                if valor(&n, lee).is_none() {
                    fallos.push(format!("{}.{}: falta {lee}", a.id, campo.id));
                }
            }
        }
    }
    assert!(fallos.is_empty(), "{fallos:#?}");
}
