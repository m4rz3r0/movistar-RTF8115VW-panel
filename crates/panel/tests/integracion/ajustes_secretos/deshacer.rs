//! Deshacer y revertir cambios con claves tapadas.

use super::*;

/// El deshacer de un cambio de la línea no se bloquea: omite la contraseña
/// (su valor anterior es el vacío que dio el router) y vuelve lo demás.
#[tokio::test]
async fn deshacer_la_voz_omite_la_clave_tapada() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "telefono.linea",
        json!({ "activa": "false", "contrasena": CLAVE_3 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let id = aplicar_ajuste(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let (status, body) = llamar(
        &p,
        "deshacer_ajuste",
        json!({ "cambio": id }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    aplicar_ajuste(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let enviado = ultima_escritura(&p, "cbSetVoiceLine");
    assert!(
        enviado.iter().all(|(k, _)| k != "auth_password"),
        "{enviado:?}"
    );
    assert_eq!(de_la_voz(&p, "adm_state").await, "1");
    // El simulador, como el router, la da vacía al leerla: lo que cuenta es
    // que el deshacer no ha escrito ninguna (ni vacía).
    assert_eq!(de_la_voz(&p, "auth_password").await, "");
}

/// La reversión automática (la WAN cae mientras se vigila) tampoco se
/// bloquea: omite la contraseña y deja la línea como estaba.
#[tokio::test]
async fn revertir_la_voz_omite_la_clave_tapada() {
    let mut t = crate::comun::tiempos_rapidos();
    t.bajo = Duration::from_secs(5);
    t.medio = Duration::from_secs(5);
    let (p, cookie) = PanelPrueba::nuevo().tiempos(t).arrancar_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "telefono.linea",
        json!({ "activa": "false", "contrasena": CLAVE_3 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let id = ok(
        &p,
        "confirmar_ajuste",
        json!({ "token": g["token"], "desmarcados": [] }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    esperar_hasta(Duration::from_secs(5), "no llega a vigilarse", async || {
        ok(&p, "estado_cambio", json!({ "id": id }), &cookie).await["estado"] == "Vigilando"
    })
    .await;
    p.sim.control.set_wan_up(false);
    let c = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Revertido", "{c}");
    let enviado = ultima_escritura(&p, "cbSetVoiceLine");
    assert!(
        enviado.iter().all(|(k, _)| k != "auth_password"),
        "{enviado:?}"
    );
    assert_eq!(de_la_voz(&p, "adm_state").await, "1");
}

/// Un cambio que solo tocó la clave de la voz no se puede deshacer: su valor
/// anterior es el vacío que dio el router. No se ofrece «Deshacer», y la
/// fila dice por qué (en los dos idiomas); el deshacer, pedido igualmente,
/// da 400 con ese motivo. Su detalle no enseña un «antes».
#[tokio::test]
async fn deshacer_solo_la_clave_tapada_no_se_ofrece() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "telefono.linea",
        json!({ "contrasena": CLAVE_3 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let id = aplicar_ajuste(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    for (idioma, motivo) in [
        ("es", "no se puede volver atrás"),
        ("en", "can't be put back"),
    ] {
        let con_idioma = format!("{cookie}; rp_lang={idioma}");
        let h = ok(&p, "historial", json!({ "pagina": 1 }), &con_idioma).await;
        let c = h["entradas"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|e| e.get("Cambio").filter(|c| c["id"] == id))
            .unwrap();
        assert_eq!(c["puede_deshacer"], false, "{c}");
        assert!(
            c["sin_deshacer"]
                .as_str()
                .unwrap_or_default()
                .contains(motivo),
            "{idioma}: {c}"
        );
    }
    let (status, body) = llamar(
        &p,
        "deshacer_ajuste",
        json!({ "cambio": id }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("no se puede volver atrás"), "{body}");
    let (status, _) = llamar(&p, "deshacer", json!({ "change_id": id }), Some(&cookie)).await;
    assert_eq!(status, 400);
    let v = ok(&p, "secretos_del_cambio", json!({ "cambio": id }), &cookie).await;
    assert_eq!(
        v,
        json!([{ "orden": 0, "nombre": "auth_password", "antes": null, "despues": CLAVE_3,
                 "tapado": true }])
    );
}

/// En un cambio mixto (la línea y su clave), deshacer vuelve la línea y
/// avisa de que la clave se queda como se escribió.
#[tokio::test]
async fn deshacer_mixto_avisa_de_la_clave_tapada() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "telefono.linea",
        json!({ "activa": "false", "contrasena": CLAVE_3 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let id = aplicar_ajuste(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let h = ok(&p, "historial", json!({ "pagina": 1 }), &cookie).await;
    let c = h["entradas"][0]["Cambio"].clone();
    assert_eq!(c["puede_deshacer"], true, "{c}");
    assert_eq!(c["sin_deshacer"], Value::Null, "{c}");
    let g = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    let aviso = g["aviso"].as_str().unwrap_or_default();
    assert!(
        aviso.contains("Contraseña de la línea") && aviso.contains("se queda"),
        "{g}"
    );
    let prep = ok(&p, "deshacer", json!({ "change_id": id }), &cookie).await;
    let aviso = prep["aviso"].as_str().unwrap_or_default();
    assert!(
        aviso.contains("Contraseña de la línea") && aviso.contains("se queda"),
        "{prep}"
    );
}

/// Si el cambio aún se vigila, deshacerlo da el motivo de siempre, aunque
/// solo cambiara una clave que el router no deja leer: el estado va
/// primero.
#[tokio::test]
async fn deshacer_en_vigilancia_da_su_motivo_aunque_sea_una_clave_tapada() {
    let mut t = crate::comun::tiempos_rapidos();
    t.bajo = Duration::from_secs(10);
    t.medio = Duration::from_secs(10);
    let (p, cookie) = PanelPrueba::nuevo().tiempos(t).arrancar_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "telefono.linea",
        json!({ "contrasena": CLAVE_3 }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let id = ok(
        &p,
        "confirmar_ajuste",
        json!({ "token": g["token"], "desmarcados": [] }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    esperar_hasta(Duration::from_secs(5), "no llega a vigilarse", async || {
        ok(&p, "estado_cambio", json!({ "id": id }), &cookie).await["estado"] == "Vigilando"
    })
    .await;
    let (status, body) = llamar(
        &p,
        "deshacer_ajuste",
        json!({ "cambio": id }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(!body.contains("no se puede volver atrás"), "{body}");
}
