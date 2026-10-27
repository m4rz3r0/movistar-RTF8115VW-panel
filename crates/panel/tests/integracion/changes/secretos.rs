//! Secretos en el gestor: la máscara no se escribe y lo nuevo se guarda cifrado.

use super::*;
use crate::comun::ConfirmarLigado;

#[tokio::test]
async fn secreto_enmascarado_no_se_escribe() {
    let (p, m) = preparado().await;
    for intocado in [MASCARA, ""] {
        let ssid = format!("RED-nueva{}", intocado.len());
        let r = req(
            "cbSetWifiAccPt",
            None,
            &[
                ("dpm_cfg_wifi_acc_pt_id", "1"),
                ("ssid", &ssid),
                ("key_phrase", intocado),
                ("wep_key1_data", MASCARA),
            ],
        );
        let prep = m.preparar(r).await.unwrap();
        let nombres: Vec<_> = prep.diff.campos.iter().map(|c| c.nombre.as_str()).collect();
        assert_eq!(nombres, ["ssid"], "el secreto sin tocar no es un cambio");
        let id = m
            .confirmar_ligado(&prep.token, None, "admin")
            .await
            .unwrap();
        esperar_estado(&m, id, Estado::Aplicado).await;
        esperar_libre_gestor(&m).await;
    }
    let w = p.sim.control.writes();
    assert_eq!(w.len(), 2);
    for (_, params) in &w {
        assert!(
            params.iter().all(|(_, v)| v != MASCARA),
            "MASCARA enviada: {params:?}"
        );
        let params: BTreeMap<_, _> = params.iter().cloned().collect();
        // Se reenvía el valor actual leído del router.
        assert_eq!(params["key_phrase"], "X-REDACTED-X");
        assert_eq!(params["wep_key1_data"], "X-REDACTED-X");
    }
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "1")])
        .await
        .unwrap();
    let ap = n.get("dpm_cfg_wifi_acc_pt").unwrap();
    assert_eq!(ap.text_of("key_phrase"), Some("X-REDACTED-X"));
    assert_eq!(ap.text_of("ssid"), Some("RED-nueva0"));
}

#[tokio::test]
async fn secreto_cambiado_se_enmascara_y_se_guarda_cifrado() {
    let (p, m) = preparado().await;
    let r = req(
        "cbSetWifiAccPt",
        None,
        &[
            ("dpm_cfg_wifi_acc_pt_id", "1"),
            ("key_phrase", "clave-nueva-123"),
        ],
    );
    let prep = m.preparar(r).await.unwrap();
    assert_eq!(prep.diff.campos.len(), 1);
    let c = &prep.diff.campos[0];
    assert!(c.secreto);
    assert_eq!((c.antes.as_str(), c.despues.as_str()), (MASCARA, MASCARA));
    let id = m
        .confirmar_ligado(&prep.token, None, "admin")
        .await
        .unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    // Ni el valor nuevo ni el anterior aparecen en claro en la base de datos.
    let fila: (String, String, String, String, Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT diff, anteriores, solicitud, nuevos, anteriores_cifrados, nuevos_cifrados \
         FROM changes WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&p.state.db)
    .await
    .unwrap();
    let texto = format!("{fila:?}");
    assert!(!texto.contains("clave-nueva-123"), "{texto}");
    assert!(!texto.contains("X-REDACTED-X"), "{texto}");
    let cifrado = [fila.4, fila.5].concat();
    assert!(!cifrado.windows(15).any(|w| w == b"clave-nueva-123"));
    // El anterior se recupera para deshacer.
    let prep = m.deshacer(id).await.unwrap();
    let id2 = m
        .confirmar_ligado(&prep.token, None, "admin")
        .await
        .unwrap();
    esperar_estado(&m, id2, Estado::Aplicado).await;
    let w = p.sim.control.writes();
    let ultimo: BTreeMap<_, _> = w.last().unwrap().1.iter().cloned().collect();
    assert_eq!(ultimo["key_phrase"], "X-REDACTED-X");
}
