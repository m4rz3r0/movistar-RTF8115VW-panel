//! Guardar secretos: claves vacías, tapadas, de solo lectura y de la voz.

use super::*;

#[tokio::test]
async fn cambiar_contrasena_wifi_y_deshacer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let original = clave_del_punto(&p, "1").await;
    let (status, body) =
        preparar(&p, &cookie, "wifi.red_5", json!({ "contrasena": CLAVE_2 })).await;
    assert_eq!(status, 200, "{body}");
    assert!(
        !body.contains(CLAVE_2) && !body.contains(&original),
        "{body}"
    );
    let g: Value = serde_json::from_str(&body).unwrap();
    let campos = cambios_del_grupo(&g);
    assert_eq!(campos.len(), 1, "{g}");
    assert_eq!(campos[0]["campo"], "ajuste-wifi-red_5-contrasena");
    assert_eq!(campos[0]["secreto"], true);
    assert_eq!(campos[0]["antes"], panel::ui::explorador::MASCARA);
    assert_eq!(campos[0]["despues"], panel::ui::explorador::MASCARA);
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(clave_del_punto(&p, "1").await, CLAVE_2);

    let (status, body) = llamar(
        &p,
        "deshacer_ajuste",
        json!({ "cambio": id }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert!(
        !body.contains(CLAVE_2) && !body.contains(&original),
        "{body}"
    );
    let inverso: Value = serde_json::from_str(&body).unwrap();
    let campos = cambios_del_grupo(&inverso);
    assert_eq!(campos.len(), 1, "{inverso}");
    assert_eq!(campos[0]["antes"], panel::ui::explorador::MASCARA);
    assert_eq!(campos[0]["despues"], panel::ui::explorador::MASCARA);
    aplicar_ajuste(&p, &cookie, &inverso).await;
    assert_eq!(clave_del_punto(&p, "1").await, original);
}

/// «Vacío = sin tocar» nunca acaba en un "" enviado: una clave vacía en la
/// petición no vale.
#[tokio::test]
async fn contrasena_vacia_no_se_envia() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar(&p, &cookie, "wifi.red_5", json!({ "contrasena": "" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(p.sim.control.writes().is_empty());
}

/// Un secreto de solo lectura (el de la web básica, mientras no se valide
/// su formato) no se escribe aunque la petición lo traiga, ni con la
/// máscara; se puede «Mostrar».
#[tokio::test]
async fn campo_secreto_solo_lectura_rechaza_escritura() {
    let toml = r#"
[[ajuste]]
id = "wifi.red_basica"
tema = "wifi"
nivel = "basico"
leer = { funcion = "cbGetWifiAccPt", params = { dpm_cfg_wifi_acc_pt_id = "1" } }

[[ajuste.campo]]
id = "nombre"
escribe = { funcion = "cbSetWifiAccPt", parametro = "ssid" }
lee = "ssid"
tipo = "texto"

[[ajuste.campo]]
id = "contrasena"
escribe = { funcion = "te_wifi.wl", parametro = "wlWpaPsk" }
lee = "key_phrase"
tipo = "secreto"
solo_lectura = "ajuste-puertos-alg-motivo"
"#;
    let (p, cookie) = PanelPrueba::nuevo()
        .ajustes(toml)
        .arrancar_con_gestor()
        .await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    for v in [CLAVE_2, panel::ui::explorador::MASCARA] {
        let (status, body) =
            preparar(&p, &cookie, "wifi.red_basica", json!({ "contrasena": v })).await;
        assert_eq!(status, 400, "{v}: {body}");
        let (status, body) = preparar(
            &p,
            &cookie,
            "wifi.red_basica",
            json!({ "nombre": "otra", "contrasena": v }),
        )
        .await;
        assert_eq!(status, 400, "con otro campo, {v}: {body}");
    }
    assert!(
        p.sim
            .control
            .writes()
            .iter()
            .all(|(f, _)| f != "te_wifi.wl")
    );
    assert_eq!(
        p.sim.control.writes().len(),
        1,
        "solo la escritura de la prueba"
    );
    // Lo que la página necesita: el motivo, y el valor para el ojo.
    let tema = ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    let campo = crate::comun::ajustes_de(&tema)[0]["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "contrasena")
        .unwrap()
        .clone();
    assert_eq!(campo["solo_lectura"], "ajuste-puertos-alg-motivo");
    assert_eq!(campo["valor_secreto"], CLAVE_1);
}

/// La línea de voz con su contraseña (que el router da vacía) escrita: se
/// guarda, y va la escrita.
#[tokio::test]
async fn voz_con_la_clave_escrita_se_guarda() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    assert_eq!(de_la_voz(&p, "auth_password").await, "");
    let tema = ok(&p, "leer_tema", json!({ "tema": "telefono" }), &cookie).await;
    let campo = campo_de(&tema, "telefono.linea", "contrasena");
    assert_eq!(campo["tapado"], true, "{campo}");
    // El router no la deja leer: el campo sale vacío.
    assert_eq!(campo["valor_secreto"], Value::Null, "{campo}");
    let (status, body) = preparar(
        &p,
        &cookie,
        "telefono.linea",
        json!({ "activa": "false", "contrasena": CLAVE_3 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert!(!body.contains(CLAVE_3));
    let g: Value = serde_json::from_str(&body).unwrap();
    aplicar_ajuste(&p, &cookie, &g).await;
    let enviado = ultima_escritura(&p, "cbSetVoiceLine");
    assert!(
        enviado.contains(&("auth_password".into(), CLAVE_3.into())),
        "va la escrita"
    );
    assert_eq!(de_la_voz(&p, "adm_state").await, "0");
    // Como el router, el simulador sigue sin dejarla leer.
    assert_eq!(de_la_voz(&p, "auth_password").await, "");
}

/// Sin escribirla, no se guarda: 400 junto al campo, con el motivo, y nada
/// se escribe. Desde la Biblioteca, el gestor tampoco la reenvía vacía.
#[tokio::test]
async fn voz_con_la_clave_vacia_se_bloquea_con_el_motivo() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for valores in [
        json!({ "activa": "false" }),
        json!({ "activa": "false", "contrasena": panel::ui::explorador::MASCARA }),
    ] {
        let (status, body) = preparar(&p, &cookie, "telefono.linea", valores.clone()).await;
        assert_eq!(status, 400, "{valores}: {body}");
        assert!(body.contains("Contraseña de la línea"), "{body}");
        assert!(body.contains(PIDE_LA_CLAVE), "{body}");
    }
    let (status, body) = preparar(&p, &cookie, "telefono.linea", json!({ "contrasena": "" })).await;
    assert_eq!(status, 400, "{body}");
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetVoiceLine", "instancia": null,
                "valores": [["interwork_id", "0"], ["adm_state", "0"]], "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("clave"), "{body}");
    assert!(
        p.sim.control.writes().is_empty(),
        "{:?}",
        p.sim.control.writes()
    );
}

/// Las claves WEP no van marcadas: vacías (sin usar), se reenvían tal cual
/// y la WiFi se guarda.
#[tokio::test]
async fn wifi_con_wep_vacias_se_guarda() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let vacias: Vec<(&str, &str)> = (1..=4)
        .map(|i| match i {
            1 => ("wep_key1_data", ""),
            2 => ("wep_key2_data", ""),
            3 => ("wep_key3_data", ""),
            _ => ("wep_key4_data", ""),
        })
        .collect();
    escribir_punto(&p, "1", &vacias).await;
    let (status, body) = preparar(&p, &cookie, "wifi.red_5", json!({ "nombre": "otra-red" })).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    aplicar_ajuste(&p, &cookie, &g).await;
    let enviado = ultima_escritura(&p, "cbSetWifiAccPt");
    for (k, _) in &vacias {
        assert!(
            enviado.contains(&((*k).into(), String::new())),
            "{k}: {enviado:?}"
        );
    }
}

/// Una clave que el router da tapada (asteriscos o la máscara) tampoco se
/// reenvía al cambiar otro campo.
#[tokio::test]
async fn clave_tapada_no_se_reenvia() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for tapada in ["********", panel::ui::explorador::MASCARA] {
        escribir_punto(&p, "1", &[("key_phrase", tapada)]).await;
        let antes = p.sim.control.writes().len();
        let (status, body) =
            preparar(&p, &cookie, "wifi.red_5", json!({ "nombre": "otra-red" })).await;
        assert_eq!(status, 403, "{tapada}: {body}");
        assert_eq!(p.sim.control.writes().len(), antes, "{tapada}");
    }
    // Con una clave nueva escrita, sí (no se reenvía la tapada: va la nueva).
    let (status, body) = preparar(
        &p,
        &cookie,
        "wifi.red_5",
        json!({ "nombre": "otra-red", "contrasena": CLAVE_1 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
}

/// Un secreto que llega igual que lo leído no es un cambio: el resumen solo
/// enseña lo que cambia y, solo, no hay nada que guardar.
#[tokio::test]
async fn secreto_sin_cambiar_no_se_envia() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "wifi.red_5",
        json!({ "nombre": "otra-red", "contrasena": CLAVE_1 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let campos = cambios_del_grupo(&g);
    assert_eq!(campos.len(), 1, "{g}");
    assert_eq!(campos[0]["campo"], "ajuste-wifi-red_5-nombre");
    let (status, body) =
        preparar(&p, &cookie, "wifi.red_5", json!({ "contrasena": CLAVE_1 })).await;
    assert_eq!(status, 409, "{body}");
}
