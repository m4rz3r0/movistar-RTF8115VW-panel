//! Longitudes de los textos y secretos, como la web oficial y el estándar
//! WiFi: el gestor rechaza lo que el router no admitiría.

use super::*;

/// Prepara un cambio de la WiFi 2,4 GHz (`cbSetWifiAccPt`) con `campos`.
async fn preparar_wifi(
    m: &ChangeManager,
    campos: &[(&str, &str)],
) -> Result<panel::changes::Preparado, CambioError> {
    let mut v = vec![("dpm_cfg_wifi_acc_pt_id", "1")];
    v.extend_from_slice(campos);
    m.preparar(req("cbSetWifiAccPt", None, &v)).await
}

#[tokio::test]
async fn ssid_de_mas_de_32_bytes_se_rechaza() {
    let (p, m) = preparado().await;
    // 33 letras, y 17 «ñ» (17 caracteres pero 34 bytes): el límite es de bytes.
    for ssid in ["a".repeat(33), "ñ".repeat(17)] {
        let r = preparar_wifi(&m, &[("ssid", &ssid)]).await;
        assert!(
            matches!(&r, Err(CambioError::Invalida(e)) if e.contains("ssid") && e.contains("32")),
            "{ssid}: {r:?}"
        );
    }
    let r = preparar_wifi(&m, &[("ssid", "")]).await;
    assert!(matches!(&r, Err(CambioError::Invalida(_))), "vacío: {r:?}");
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn clave_wpa_fuera_de_8_a_63_se_rechaza_sin_enseñarla() {
    let (p, m) = preparado().await;
    let hex_malo = "g".repeat(64);
    for clave in [
        "clave-7",
        &"x".repeat(64),
        &hex_malo,
        "clave\u{e9}-no-ascii",
    ] {
        let r = preparar_wifi(&m, &[("key_phrase", clave)]).await;
        assert!(
            matches!(&r, Err(CambioError::Invalida(e)) if e.contains("key_phrase") && e.contains("8") && !e.contains(clave)),
            "{r:?}"
        );
    }
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn ssid_y_clave_validos_pasan() {
    let (_p, m) = preparado().await;
    let hex = "0123456789abcdef".repeat(4);
    for (ssid, clave) in [
        ("a".repeat(32), "clave-08".to_owned()),
        ("ñ".repeat(16), "x".repeat(63)),
        ("Casa".to_owned(), hex),
    ] {
        let r = preparar_wifi(&m, &[("ssid", &ssid), ("key_phrase", &clave)]).await;
        let prep = r.unwrap_or_else(|e| panic!("{ssid}: {e:?}"));
        assert_eq!(prep.diff.campos.len(), 2);
    }
}

#[tokio::test]
async fn clave_wep_solo_de_5_o_13_caracteres_o_10_o_26_cifras_hex() {
    let (_p, m) = preparado().await;
    for clave in ["clav", "clave6", &"a".repeat(14), &"g".repeat(10)] {
        let r = preparar_wifi(&m, &[("wep_key1_data", clave)]).await;
        assert!(
            matches!(&r, Err(CambioError::Invalida(e)) if e.contains("wep_key1_data") && !e.contains(clave)),
            "{r:?}"
        );
    }
    for clave in ["clav5", &"c".repeat(13), "0123456789", &"ab".repeat(13)] {
        preparar_wifi(&m, &[("wep_key1_data", clave)])
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
    }
}
