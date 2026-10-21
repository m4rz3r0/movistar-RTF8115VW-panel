//! La foto de partida lleva la lectura de cada ajuste (cada punto de acceso
//! y cada radio, el DHCP, las IP fijas); una foto antigua, solo la de
//! partida; una que no se descifra lo dice.

use super::*;
use panel::crypto::HistoryKey;

/// Una foto como las de la versión anterior: solo las lecturas de partida,
/// sin la de cada ajuste.
async fn foto_antigua(p: &Panel) {
    let mut foto = std::collections::BTreeMap::new();
    for (nombre, params) in panel::instalar::lecturas_de_partida(&p.state.catalogo) {
        if let Ok(n) = p.state.hgu.call(nombre, &params).await {
            foto.insert(nombre.to_owned(), n.to_xml());
        }
    }
    fotos::guardar(&p.state.db, &p.state.history_key, TipoFoto::Partida, &foto)
        .await
        .unwrap();
}

#[tokio::test]
async fn la_foto_trae_cada_instancia() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    tomar_foto(&p).await;
    let c = ok(&p, "comparar_con_partida", json!({}), &cookie).await;
    let sin_dato = c["sin_dato"].as_array().unwrap();
    for ajuste in [
        "ajuste-wifi-red_24",
        "ajuste-wifi-red_5",
        "ajuste-wifi-invitados",
        "ajuste-wifi-radio_24",
        "ajuste-wifi-radio_5",
        "ajuste-red_local-dhcp",
        "ajuste-red_local-reservas",
    ] {
        assert!(!sin_dato.contains(&json!(ajuste)), "{ajuste}: {c}");
    }
    assert_eq!(c["diferencias"], json!([]), "{c}");

    // Cambiar la red de 2,4 GHz es una diferencia suya, no de la de 5 GHz.
    escribir_punto(&p, "0", &[("ssid", "casa-24")]).await;
    p.state.lecturas.invalidar();
    let c = ok(&p, "comparar_con_partida", json!({}), &cookie).await;
    assert_eq!(c["diferencias"].as_array().unwrap().len(), 1, "{c}");
    let d = &del_ajuste(&c, "wifi.red_24")[0]["cambio"]["Valor"];
    assert_eq!(d["partida"]["valor"], json!({ "Texto": "RED-2" }), "{c}");
    assert_eq!(d["actual"]["valor"], json!({ "Texto": "casa-24" }), "{c}");
    // Y se vuelve a él.
    let g = ok(
        &p,
        "preparar_volver_a_partida",
        json!({ "ajuste": "wifi.red_24", "campo": "nombre", "instancia": null }),
        &cookie,
    )
    .await;
    aplicar_ajuste(&p, &cookie, &g).await;
    let c = ok(&p, "comparar_con_partida", json!({}), &cookie).await;
    assert_eq!(c["diferencias"], json!([]), "{c}");
}

/// Una foto de la versión anterior (sin la lectura de cada ajuste): lo que
/// no trae es «sin dato», sin fallar, y no se puede volver a ello.
#[tokio::test]
async fn foto_antigua_sin_dato() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    foto_antigua(&p).await;
    let c = ok(&p, "comparar_con_partida", json!({}), &cookie).await;
    let sin_dato = c["sin_dato"].as_array().unwrap();
    for ajuste in ["ajuste-wifi-red_24", "ajuste-red_local-reservas"] {
        assert!(sin_dato.contains(&json!(ajuste)), "{ajuste}: {c}");
    }
    let (status, _) = volver(&p, Some(&cookie), "red_local.dhcp", Some("activo"), None).await;
    assert_eq!(status, 404);
}

/// Una foto que el panel ya no puede descifrar (otra clave maestra) no es
/// «no hay foto»: lo dice, sin detalles internos.
#[tokio::test]
async fn foto_ilegible_lo_dice() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let dir = tempfile::tempdir().unwrap();
    let otra = HistoryKey::load_or_create(&dir.path().join("otra.key")).unwrap();
    let foto = fotos::leer_partida(&p.state).await;
    fotos::guardar(&p.state.db, &otra, TipoFoto::Partida, &foto.contenido)
        .await
        .unwrap();
    let (status, body) = llamar(&p, "comparar_con_partida", json!({}), Some(&cookie)).await;
    assert_eq!(status, 500, "{body}");
    assert!(body.contains("ya no tiene la clave"), "{body}");
    assert!(!body.contains("RP_HISTORY_KEY"), "{body}");
    let (status, body) = volver(&p, Some(&cookie), "puertos.upnp", Some("activo"), None).await;
    assert_eq!(status, 500, "{body}");
}

/// La foto nunca lee una función que el catálogo bloquea, aunque un ajuste
/// la tenga como lectura.
#[tokio::test]
async fn la_foto_no_lee_funciones_bloqueadas() {
    let toml = r#"
[[ajuste]]
id = "sistema.prueba"
tema = "sistema"
nivel = "avanzado"
leer = { funcion = "cbCheckLogout" }

[[ajuste.campo]]
id = "dato"
lee = "dato"
tipo = "texto"
solo_lectura = "motivo"
"#;
    let p = PanelPrueba::nuevo().ajustes(toml).arrancar().await;
    let foto = fotos::leer_partida(&p.state).await;
    assert!(!foto.contenido.contains_key("cbCheckLogout"), "leída");
    assert!(
        !foto.fallidas.iter().any(|f| f == "cbCheckLogout"),
        "intentada"
    );
    // Lo demás sí se lee.
    assert!(foto.contenido.contains_key("cbGetUpnpCfg"));
}

/// Un cambio en una IP fija dice de cuál se trata (su MAC).
#[tokio::test]
async fn una_ip_fija_dice_cual_es() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    tomar_foto(&p).await;
    let tema = ok(&p, "leer_tema", json!({ "tema": "red_local" }), &cookie).await;
    let reservas = tema["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "red_local.reservas")
        .unwrap()
        .clone();
    let primera = &reservas["instancias"][0];
    let mac = primera["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "mac")
        .unwrap()["valor"]["Texto"]
        .as_str()
        .unwrap()
        .to_owned();
    let g = ok(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "red_local.reservas", "instancia": primera["id"], "borrar": false,
                       "valores": { "ip": "192.168.1.81" }, "esperados": { "mac": mac } } }),
        &cookie,
    )
    .await;
    aplicar_ajuste(&p, &cookie, &g).await;
    let c = ok(&p, "comparar_con_partida", json!({}), &cookie).await;
    let d = del_ajuste(&c, "red_local.reservas");
    assert_eq!(d.len(), 1, "{c}");
    assert_eq!(d[0]["cambio"]["Valor"]["partida"]["id"], "ip");
    let de = d[0]["de"].as_array().unwrap();
    assert!(
        de.iter()
            .any(|x| x["id"] == "mac" && x["valor"] == json!({ "Texto": mac })),
        "{c}"
    );
    assert!(
        !de.iter().any(|x| x["id"] == "ip"),
        "no repite lo que cambia"
    );
}
