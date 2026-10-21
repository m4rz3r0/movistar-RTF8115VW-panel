//! Deshacer un cambio de ajustes y sus errores.

use super::*;

/// `deshacer_ajuste` del cambio `id`: estado y cuerpo.
async fn deshacer(p: &Panel, cookie: &str, id: i64) -> (u16, String) {
    llamar(p, "deshacer_ajuste", json!({ "cambio": id }), Some(cookie)).await
}

#[tokio::test]
async fn deshacer_un_cambio_que_no_es_de_ajustes_da_404() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    // Un cambio del explorador: sin título de ajuste.
    let id = aplicar(
        &p,
        &cookie,
        json!({ "funcion": "cbSetUpnpCfg", "instancia": null, "valores": [["adm_state", "1"]], "borrar": false }),
    )
    .await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    esperar_libre(&p).await;
    let (status, body) = deshacer(&p, &cookie, id).await;
    assert_eq!(status, 404, "{body}");
    assert!(body.contains("no se hizo desde Ajustes"), "{body}");
}

#[tokio::test]
async fn deshacer_en_vigilancia_da_400() {
    let (p, cookie) = panel_vigilando().await;
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    let id = confirmar(&p, &cookie, &g, &[]).await;
    let (status, body) = deshacer(&p, &cookie, id).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("ya no se puede deshacer"), "{body}");
}

#[tokio::test]
async fn deshacer_dos_veces_da_400() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = resumen(
        &p,
        &cookie,
        "wifi.red_24",
        json!({ "nombre": "casa-nueva" }),
    )
    .await;
    let id = confirmar(&p, &cookie, &g, &[]).await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    esperar_libre(&p).await;
    let (status, body) = deshacer(&p, &cookie, id).await;
    assert_eq!(status, 200, "{body}");
    let inverso: Value = serde_json::from_str(&body).unwrap();
    let otro = confirmar(&p, &cookie, &inverso, &[]).await;
    assert_eq!(esperar_final(&p, &cookie, otro).await["estado"], "Aplicado");
    esperar_libre(&p).await;
    let (status, body) = deshacer(&p, &cookie, id).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("ya no se puede deshacer"), "{body}");
    // El deshacer sí se puede deshacer, y sigue siendo del mismo ajuste.
    let (status, body) = deshacer(&p, &cookie, otro).await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("ajuste-wifi-red_24-titulo"), "{body}");
}

/// Espejo de puerto: encenderlo sin elegir la toma escribiría la toma 0. El
/// servidor lo rechaza (400, en el idioma de la petición, junto a la toma) y
/// lo acepta con la toma; la vista lleva la regla para la página.
#[tokio::test]
async fn espejo_sin_toma_da_400() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let tema = leer(&p, &cookie, "sistema").await;
    let espejo = campo(ajuste(&tema, "sistema.diagnostico"), "espejo");
    assert_eq!(
        espejo["requiere"],
        json!({ "campo": "puerto", "cuando": "true" }),
        "{espejo}"
    );
    let (status, body) = preparar_ajuste(
        &p,
        &cookie,
        "sistema.diagnostico",
        json!({ "espejo": "true" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("Toma que recibe la copia"), "{body}");
    assert!(body.contains("Copia encendida"), "{body}");
    let en = format!("{cookie}; rp_lang=en");
    let (status, body) =
        preparar_ajuste(&p, &en, "sistema.diagnostico", json!({ "espejo": "true" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("Socket that receives the copy"), "{body}");
    assert!(p.sim.control.writes().is_empty());
    // Apagarlo no exige toma; encenderlo con ella, sí vale.
    let (status, body) = preparar_ajuste(
        &p,
        &cookie,
        "sistema.diagnostico",
        json!({ "espejo": "true", "puerto": "puerto_2" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
}
