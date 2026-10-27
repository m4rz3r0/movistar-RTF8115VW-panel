//! Reglas de puertos: validación, abrir, editar, borrar y deshacer.

use super::*;

// ------------------------------------------------- puertos en el simulador

/// Reglas de `leer_tema` como `(id, nombre, protocolo, externo_desde,
/// destino)`.
async fn reglas_del_tema(p: &Panel, cookie: &str) -> Vec<[String; 5]> {
    let tema = ok(p, "leer_tema", json!({ "tema": "puertos" }), cookie).await;
    let a = crate::comun::ajustes_de(&tema)
        .into_iter()
        .find(|a| a["id"] == "puertos.reglas")
        .unwrap()
        .clone();
    assert_eq!(a["estado"], "Leido", "{a}");
    let valor = |i: &Value, campo: &str| {
        let c = i["campos"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == campo)
            .unwrap();
        let v = &c["valor"];
        v["Texto"]
            .as_str()
            .or(v["Opcion"].as_str())
            .unwrap_or_default()
            .to_owned()
    };
    a["instancias"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            [
                i["id"].as_str().unwrap().to_owned(),
                valor(i, "nombre"),
                valor(i, "protocolo"),
                valor(i, "externo_desde"),
                valor(i, "destino"),
            ]
        })
        .collect()
}

async fn preparar_regla(
    p: &Panel,
    cookie: &str,
    instancia: Option<&str>,
    borrar: bool,
    valores: Value,
    esperados: Value,
) -> (u16, String) {
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": {
            "ajuste": "puertos.reglas",
            "instancia": instancia,
            "borrar": borrar,
            "valores": valores,
            "esperados": esperados,
        } }),
        Some(cookie),
    )
    .await
}

/// Una regla nueva completa hacia host-2 (192.168.1.62).
fn nueva(protocolo: &str, puerto: &str) -> Value {
    json!({
        "nombre": "Consola",
        "protocolo": protocolo,
        "externo_desde": puerto,
        "externo_hasta": puerto,
        "destino": "192.168.1.62",
        "interno_desde": puerto,
        "interno_hasta": puerto,
        "activa": "true",
    })
}

/// Lo que identifica la regla 0 de las fixtures (8080 TCP a .75).
fn esperados_0() -> Value {
    json!({ "protocolo": "tcp", "externo_desde": "8080", "destino": "192.168.1.75" })
}

#[tokio::test]
async fn abrir_puerto_y_deshacer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = reglas_del_tema(&p, &cookie).await;
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("tcp", "9000"), json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["titulo"], "ajuste-puertos-reglas-titulo");
    assert_eq!(g["detalle"], "Consola", "{g}");
    assert_eq!(g["pasos"].as_array().unwrap().len(), 1, "{g}");
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    let ahora = reglas_del_tema(&p, &cookie).await;
    assert_eq!(ahora.len(), antes.len() + 1, "{ahora:?}");
    let n = ahora
        .iter()
        .find(|r| r[3] == "9000")
        .expect("la regla nueva");
    assert_eq!(n[1..], ["Consola", "tcp", "9000", "192.168.1.62"]);
    deshacer(&p, &cookie, id).await;
    assert_eq!(reglas_del_tema(&p, &cookie).await, antes);
}

/// «Ambos»: dos reglas (TCP y UDP) en un único grupo, con un solo diálogo y
/// un solo deshacer que borra las dos.
#[tokio::test]
async fn abrir_puerto_en_ambos_es_un_grupo_y_un_deshacer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = reglas_del_tema(&p, &cookie).await;
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("ambos", "9000"), json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(pasos.len(), 2, "{g}");
    let protocolo = |paso: &Value| {
        paso["campos"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["campo"] == "ajuste-puertos-reglas-protocolo")
            .map(|c| c["despues"].clone())
    };
    assert_eq!(protocolo(&pasos[0]), Some(json!("tcp")), "{g}");
    assert_eq!(protocolo(&pasos[1]), Some(json!("udp")), "{g}");
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    let ahora = reglas_del_tema(&p, &cookie).await;
    let nuevas: Vec<&[String; 5]> = ahora.iter().filter(|r| r[3] == "9000").collect();
    assert_eq!(nuevas.len(), 2, "{ahora:?}");
    assert!(nuevas.iter().any(|r| r[2] == "tcp") && nuevas.iter().any(|r| r[2] == "udp"));
    assert_ne!(nuevas[0][0], nuevas[1][0], "dos instancias distintas");
    deshacer(&p, &cookie, id).await;
    assert_eq!(reglas_del_tema(&p, &cookie).await, antes);
}

/// Una regla que abre un puerto que ya abre otra con el mismo protocolo no
/// se prepara (400, junto al campo y en el idioma) y no escribe nada. Con
/// el otro protocolo, sí.
#[tokio::test]
async fn regla_solapada_da_400() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for protocolo in ["tcp", "ambos"] {
        let (status, body) = preparar_regla(
            &p,
            &cookie,
            None,
            false,
            nueva(protocolo, "8080"),
            json!({}),
        )
        .await;
        assert_eq!(status, 400, "{protocolo}: {body}");
        assert!(body.contains("ya está abierto"), "{body}");
    }
    let en = format!("{cookie}; rp_lang=en");
    let (status, body) =
        preparar_regla(&p, &en, None, false, nueva("tcp", "8080"), json!({})).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("already open"), "{body}");
    // Fuera de la red y rango al revés.
    let mut fuera = nueva("tcp", "9000");
    fuera["destino"] = json!("192.0.2.10");
    let (status, body) = preparar_regla(&p, &cookie, None, false, fuera, json!({})).await;
    assert_eq!(status, 400, "{body}");
    assert!(
        body.contains("no es la de un dispositivo de tu red"),
        "{body}"
    );
    let mut al_reves = nueva("tcp", "9000");
    al_reves["externo_hasta"] = json!("8990");
    al_reves["interno_hasta"] = json!("8990");
    let (status, body) = preparar_regla(&p, &cookie, None, false, al_reves, json!({})).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("menor"), "{body}");
    // A una nueva no le puede faltar el destino.
    let mut sin_destino = nueva("tcp", "9000");
    sin_destino.as_object_mut().unwrap().remove("destino");
    let (status, body) = preparar_regla(&p, &cookie, None, false, sin_destino, json!({})).await;
    assert_eq!(status, 400, "{body}");
    assert!(p.sim.control.writes().is_empty());
    // Con UDP no choca.
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("udp", "8080"), json!({})).await;
    assert_eq!(status, 200, "{body}");
}

/// Editar o borrar una regla exige lo que la identifica (protocolo, puerto
/// de internet y destino): sin ello, 400; si el router ya tiene otra cosa,
/// 409; nada escrito. «Ambos» solo vale al crear.
#[tokio::test]
async fn editar_o_borrar_regla_con_esperados() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for borrar in [true, false] {
        let valores = if borrar {
            json!({})
        } else {
            json!({ "activa": "false" })
        };
        let (status, body) =
            preparar_regla(&p, &cookie, Some("0"), borrar, valores.clone(), json!({})).await;
        assert_eq!(status, 400, "sin esperados, borrar={borrar}: {body}");
        let (status, body) = preparar_regla(
            &p,
            &cookie,
            Some("0"),
            borrar,
            valores.clone(),
            json!({ "protocolo": "tcp", "externo_desde": "8080" }),
        )
        .await;
        assert_eq!(status, 400, "a medias, borrar={borrar}: {body}");
        let (status, body) = preparar_regla(
            &p,
            &cookie,
            Some("0"),
            borrar,
            valores.clone(),
            json!({ "protocolo": "tcp", "externo_desde": "8081", "destino": "192.168.1.75" }),
        )
        .await;
        assert_eq!(status, 409, "otra, borrar={borrar}: {body}");
        assert!(body.contains("ha cambiado"), "{body}");
    }
    let (status, body) = preparar_regla(
        &p,
        &cookie,
        Some("0"),
        false,
        json!({ "protocolo": "ambos" }),
        esperados_0(),
    )
    .await;
    assert_eq!(status, 400, "ambos al editar: {body}");
    let (status, body) =
        preparar_regla(&p, &cookie, Some("77"), true, json!({}), esperados_0()).await;
    assert_eq!(status, 400, "no existe: {body}");
    assert!(p.sim.control.writes().is_empty());
    // Con lo que tiene, sí; y desactivarla no la valida (su destino ya no
    // está, pero eso no impide apagarla).
    let (status, body) =
        preparar_regla(&p, &cookie, Some("0"), true, json!({}), esperados_0()).await;
    assert_eq!(status, 200, "{body}");
    let (status, body) = preparar_regla(
        &p,
        &cookie,
        Some("0"),
        false,
        json!({ "activa": "false" }),
        esperados_0(),
    )
    .await;
    assert_eq!(status, 200, "{body}");
}

// ------------------------------------------- ronda de corrección 1

/// «TCP y UDP» con la segunda escritura fallida: el gestor revierte la
/// primera (borra la regla TCP recién creada) y el grupo queda fallido; la
/// lista vuelve a ser la de antes.
#[tokio::test]
async fn ambos_con_la_segunda_escritura_fallida_no_deja_nada() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = reglas_del_tema(&p, &cookie).await;
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("ambos", "9000"), json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    p.sim.control.fail_write_n(2);
    let (status, body) = llamar(
        &p,
        "confirmar_ajuste",
        json!({ "token": g["token"], "desmarcados": [] }),
        Some(&cookie),
    )
    .await;
    assert_ne!(status, 200, "{body}");
    esperar_libre(&p).await;
    let historial = p.state.cambios().unwrap();
    let ultimo = historial
        .historial(5)
        .await
        .into_iter()
        .next()
        .expect("el cambio");
    assert_eq!(ultimo.estado, panel::changes::Estado::Fallido, "{ultimo:?}");
    assert_eq!(reglas_del_tema(&p, &cookie).await, antes);
}

/// Editar una regla valida lo que resulta de mezclar lo pedido con lo que
/// tiene el router: chocar con otra regla o salir de la red da 400; un
/// cambio que vale se aplica.
#[tokio::test]
async fn editar_regla_valida_con_lo_del_router() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("tcp", "9000"), json!({})).await;
    assert_eq!(status, 200, "{body}");
    aplicar_ajuste(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let escritas = p.sim.control.writes().len();
    // La 0 al puerto 9000 de internet (el de dentro sigue en 8080).
    let (status, body) = preparar_regla(
        &p,
        &cookie,
        Some("0"),
        false,
        json!({ "externo_desde": "9000", "externo_hasta": "9000" }),
        esperados_0(),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("ya está abierto"), "{body}");
    let (status, body) = preparar_regla(
        &p,
        &cookie,
        Some("0"),
        false,
        json!({ "destino": "192.0.2.10" }),
        esperados_0(),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(
        body.contains("no es la de un dispositivo de tu red"),
        "{body}"
    );
    assert_eq!(p.sim.control.writes().len(), escritas);
    let (status, body) = preparar_regla(
        &p,
        &cookie,
        Some("0"),
        false,
        json!({ "destino": "192.168.1.62" }),
        esperados_0(),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    aplicar_ajuste(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let ahora = reglas_del_tema(&p, &cookie).await;
    let r0 = ahora.iter().find(|r| r[0] == "0").unwrap();
    assert_eq!(r0[2..], ["tcp", "8080", "192.168.1.62"]);
}

/// Con un protocolo desconocido no se sabe si choca: cuenta como TCP y UDP
/// a la vez (8080 UDP choca con ella).
#[tokio::test]
async fn protocolo_desconocido_solapa_con_los_dos() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("puertos-protocolo-desconocido"))
        .arrancar_con_gestor()
        .await;
    for protocolo in ["tcp", "udp"] {
        let (status, body) = preparar_regla(
            &p,
            &cookie,
            None,
            false,
            nueva(protocolo, "8080"),
            json!({}),
        )
        .await;
        assert_eq!(status, 400, "{protocolo}: {body}");
        assert!(body.contains("ya está abierto"), "{body}");
    }
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("udp", "8081"), json!({})).await;
    assert_eq!(status, 200, "{body}");
}

/// «TCP y UDP»: cada paso lleva su título («TCP», «UDP») y el segundo solo
/// lo que cambia respecto al primero (el resto, en su nota).
#[tokio::test]
async fn ambos_con_un_titulo_por_paso() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) =
        preparar_regla(&p, &cookie, None, false, nueva("ambos", "9000"), json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(
        pasos[0]["titulo"], "ajuste-puertos-reglas-protocolo-tcp",
        "{g}"
    );
    assert_eq!(
        pasos[1]["titulo"], "ajuste-puertos-reglas-protocolo-udp",
        "{g}"
    );
    let campos = |i: usize| -> Vec<String> {
        pasos[i]["campos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["campo"].as_str().unwrap().to_owned())
            .collect()
    };
    assert!(campos(0).len() >= 7, "{g}");
    assert_eq!(campos(1), ["ajuste-puertos-reglas-protocolo"], "{g}");
    assert_eq!(pasos[1]["nota"], "ajuste-puertos-reglas-paso-igual", "{g}");
    // Una regla de un solo protocolo no lleva títulos.
    let (_, body) = preparar_regla(&p, &cookie, None, false, nueva("tcp", "9000"), json!({})).await;
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["pasos"][0]["titulo"], Value::Null, "{g}");
}
