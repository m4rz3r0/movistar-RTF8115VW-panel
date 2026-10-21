//! Volver a un valor de Teléfono: la contraseña de la línea, que el router
//! no deja leer, va en la misma escritura y hay que pedirla.

use super::*;

/// Escribe en la línea de voz 0 del simulador.
async fn escribir_voz(p: &Panel, pares: &[(&str, &str)]) {
    let mut params = vec![("interwork_id", "0")];
    params.extend_from_slice(pares);
    p.state
        .hgu
        .write("voiceadvance.asp", "cbSetVoiceLine", &params)
        .await
        .unwrap();
    p.state.lecturas.invalidar();
}

/// Lo que el simulador tiene en la línea de voz 0.
async fn de_la_voz(p: &Panel, nombre: &str) -> String {
    let n = p
        .state
        .hgu
        .call("cbGetVoiceLine", &[("interwork_id", "0")])
        .await
        .unwrap();
    elemento(&n, nombre).unwrap_or_default()
}

const CLAVE_VOZ: &str = "clave-de-prueba-3";

#[tokio::test]
async fn volver_con_la_clave_tapada_la_pide() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    tomar_foto(&p).await;
    escribir_voz(&p, &[("vad_state", "1")]).await;
    let c = ok(&p, "comparar_con_partida", json!({}), &cookie).await;
    let d = del_ajuste(&c, "telefono.linea");
    assert_eq!(d.len(), 1, "{c}");
    let v = &d[0]["cambio"]["Valor"];
    assert_eq!(v["partida"]["id"], "silencio");
    assert_eq!(v["restaurable"], true);
    // La página pide la contraseña de la línea (vacía: el router no da su valor).
    assert_eq!(v["pide"]["id"], "contrasena", "{c}");
    assert_eq!(v["pide"]["tapado"], true);
    assert_eq!(v["pide"]["valor_secreto"], Value::Null);

    let pedir = |clave: Option<&str>| {
        json!({ "ajuste": "telefono.linea", "campo": "silencio", "instancia": null,
                "clave": clave })
    };
    // Sin ella (o vacía, o solo espacios), no se prepara: no se reenvía
    // vacía.
    for sin in [None, Some(""), Some("   ")] {
        let (status, body) =
            llamar(&p, "preparar_volver_a_partida", pedir(sin), Some(&cookie)).await;
        assert_eq!(status, 400, "{body}");
        assert!(body.contains("escríbela para guardar"), "{body}");
    }
    // Con ella, se aplica y la clave queda escrita.
    let g = ok(
        &p,
        "preparar_volver_a_partida",
        pedir(Some(CLAVE_VOZ)),
        &cookie,
    )
    .await;
    assert!(
        !g.to_string().contains(CLAVE_VOZ),
        "resumen enmascarado: {g}"
    );
    aplicar_ajuste(&p, &cookie, &g).await;
    assert_eq!(de_la_voz(&p, "vad_state").await, "0");
    assert_eq!(
        de_la_voz(&p, "auth_password").await,
        "",
        "el sim la tapa al leer"
    );
    let escrita = p
        .sim
        .control
        .writes()
        .into_iter()
        .rev()
        .find(|(f, _)| f == "cbSetVoiceLine")
        .unwrap()
        .1;
    assert!(
        escrita
            .iter()
            .any(|(k, v)| k == "auth_password" && v == CLAVE_VOZ),
        "la clave va en la escritura"
    );

    // Lo que no va con una clave tapada no la pide.
    cambiar_upnp(&p, &cookie, true).await;
    let c = ok(&p, "comparar_con_partida", json!({}), &cookie).await;
    let v = &del_ajuste(&c, "puertos.upnp")[0]["cambio"]["Valor"];
    assert_eq!(v["pide"], Value::Null);
}
