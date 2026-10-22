//! Lectura de los temas (`leer_tema`): valores legibles, listas, caché y errores.

use super::*;

#[tokio::test]
async fn leer_tema_exige_sesion() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let (status, cache, body) = peticion(&p, "wifi", None).await;
    assert_eq!(status, 401, "{body}");
    assert_eq!(cache, "no-store");
}

#[tokio::test]
async fn leer_tema_wifi_con_valores_legibles() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let ssid = elemento(
        &p.state
            .hgu
            .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "0")])
            .await
            .unwrap(),
        "ssid",
    )
    .unwrap();
    let tema = leer(&p, &cookie, "wifi").await;
    assert_eq!(tema["tema"], "wifi");
    let red = ajuste(&tema, "wifi.red_24");
    assert_eq!(red["estado"], "Leido", "{red}");
    assert_eq!(red["nivel"], "basico");
    assert_eq!(red["lista"], false);
    assert_eq!(
        campo(red, "seguridad")["valor"],
        json!({ "Opcion": "wpa2" })
    );
    assert_eq!(campo(red, "nombre")["valor"], json!({ "Texto": ssid }));
    // Lo que la página necesita para pintar sin el catálogo.
    let seguridad = campo(red, "seguridad");
    assert_eq!(seguridad["tipo"], "opcion");
    assert_eq!(seguridad["recomendado"], "wpa2");
    let abierta = seguridad["opciones"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["clave"] == "abierta")
        .unwrap();
    assert_eq!(abierta["recomendada"], false);
    assert_eq!(abierta["aviso"], "aviso-wifi_abierta");
    assert_eq!(campo(red, "visible")["recomendado"], "true");
    assert_eq!(campo(red, "contrasena")["ascii"], true);
    assert_eq!(campo(red, "contrasena")["min"], 8);
    assert_eq!(campo(red, "contrasena")["max"], 63);
    assert_eq!(red["efecto"], "efecto-wifi_red");
    // Los nombres de la API solo van en los detalles técnicos.
    assert_eq!(red["tecnico"]["escrituras"], json!(["cbSetWifiAccPt"]));
    assert!(
        red["tecnico"]["parametros"]
            .as_array()
            .unwrap()
            .contains(&json!(["seguridad", "authentication"])),
        "{red}"
    );
    let mut sin_tecnico = red.clone();
    sin_tecnico.as_object_mut().unwrap().remove("tecnico");
    let texto = sin_tecnico.to_string();
    assert!(
        !texto.contains("cbGet") && !texto.contains("cbSet") && !texto.contains("authentication"),
        "{texto}"
    );
}

#[tokio::test]
async fn secretos_solo_en_su_campo() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let clave = "clave-conocida-del-sim-7319";
    escribir_red_24(&p, &[("key_phrase", clave)]).await;
    let (status, body) = llamar(&p, "leer_tema", json!({ "tema": "wifi" }), Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    // Desde la Tarea 11, el valor de un secreto viaja con la página, pero
    // solo en su campo aparte (para el campo censurado con el ojo).
    assert_eq!(body.matches(clave).count(), 1, "{body}");
    let tema: Value = serde_json::from_str(&body).unwrap();
    let red = ajuste(&tema, "wifi.red_24");
    assert_eq!(campo(red, "contrasena")["valor"], "Secreto");
    assert_eq!(campo(red, "contrasena")["valor_secreto"], clave);
}

#[tokio::test]
async fn usuarios_del_router_por_pares_y_en_claro() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let tema = leer(&p, &cookie, "seguridad").await;
    let usuarios = ajuste(&tema, "seguridad.usuarios_router");
    assert_eq!(usuarios["lista"], true);
    let instancias = usuarios["instancias"].as_array().unwrap();
    let papeles: Vec<&Value> = instancias
        .iter()
        .map(|i| &campo(i, "papel")["valor"])
        .collect();
    assert_eq!(
        papeles,
        [
            &json!({ "Opcion": "operador" }),
            &json!({ "Opcion": "administrador" }),
            &json!({ "Opcion": "usuario" }),
        ]
    );
    for i in instancias {
        // El nombre de usuario es un dato sensible, no un secreto: en claro,
        // sin ojo.
        assert_eq!(
            campo(i, "nombre")["valor"],
            json!({ "Texto": "X-REDACTED-X" }),
            "{i}"
        );
        assert_eq!(
            campo(i, "nombre")["solo_lectura"],
            "ajuste-seguridad-usuarios_router-motivo"
        );
        assert!(campo(i, "nombre")["valor_secreto"].is_null(), "{i}");
    }
    // TR-069 es una opción, con la apagada recomendada.
    let remota = ajuste(&tema, "seguridad.gestion_remota");
    assert_eq!(campo(remota, "activa")["recomendado"], "apagada");
}

#[tokio::test]
async fn listas_con_sus_instancias() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let tema = leer(&p, &cookie, "puertos").await;
    let reglas = ajuste(&tema, "puertos.reglas");
    assert_eq!(reglas["componente"], "puertos");
    // En una lista, `campos` es la plantilla de una instancia nueva.
    let plantilla: Vec<&Value> = reglas["campos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["id"])
        .collect();
    let instancias = reglas["instancias"].as_array().unwrap();
    assert_eq!(instancias.len(), 1, "{reglas}");
    assert_eq!(instancias[0]["id"], "0");
    let de_la_instancia: Vec<&Value> = instancias[0]["campos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["id"])
        .collect();
    assert_eq!(plantilla, de_la_instancia);
    assert_eq!(
        campo(reglas, "protocolo")["opciones"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        campo(&instancias[0], "protocolo")["valor"],
        json!({ "Opcion": "tcp" })
    );
    assert_eq!(
        campo(&instancias[0], "externo_desde")["valor"],
        json!({ "Texto": "8080" })
    );
    let tema = leer(&p, &cookie, "red_local").await;
    let reservas = ajuste(&tema, "red_local.reservas");
    let ids: Vec<&Value> = reservas["instancias"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| &i["id"])
        .collect();
    assert_eq!(ids, [&json!("0"), &json!("1")]);
}

#[tokio::test]
async fn lectura_fallida_deja_solo_ese_ajuste_sin_leer() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    p.sim.control.fail_reads("cbGetUpnpCfg", true);
    let tema = leer(&p, &cookie, "puertos").await;
    let upnp = ajuste(&tema, "puertos.upnp");
    let motivo = upnp["estado"]["SinLeer"]
        .as_str()
        .unwrap_or_else(|| panic!("{upnp}"));
    assert!(!motivo.is_empty() && !motivo.contains("cbGet"), "{motivo}");
    assert!(upnp["campos"].as_array().unwrap().is_empty(), "{upnp}");
    assert_eq!(ajuste(&tema, "puertos.reglas")["estado"], "Leido");
    assert_eq!(ajuste(&tema, "puertos.alg")["estado"], "Leido");
}

#[tokio::test]
async fn opcion_desconocida_se_muestra_y_se_conserva() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    escribir_red_24(&p, &[("authentication", "99")]).await;
    let tema = leer(&p, &cookie, "wifi").await;
    let red = ajuste(&tema, "wifi.red_24");
    assert_eq!(
        campo(red, "seguridad")["valor"],
        json!({ "Desconocido": "99" })
    );
}

#[tokio::test]
async fn tema_desconocido_404() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let (status, cache, body) = peticion(&p, "nada", Some(&cookie)).await;
    assert_eq!(status, 404, "{body}");
    assert_eq!(cache, "no-store");
    assert!(body.contains("no existe"), "{body}");
}

#[tokio::test]
async fn respuesta_no_store() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let (status, cache, body) = peticion(&p, "puertos", Some(&cookie)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
}

/// Cada lectura distinta (función y parámetros) se hace una sola vez por
/// petición, aunque la compartan varios ajustes (`wifi.red_24` y
/// `wifi.red_24_avanzado`).
#[tokio::test]
async fn cada_lectura_una_sola_vez() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    for tema in ["wifi", "seguridad"] {
        let antes = p.sim.control.reads().len();
        leer(&p, &cookie, tema).await;
        let leidas: Vec<_> = p.sim.control.reads().into_iter().skip(antes).collect();
        assert!(!leidas.is_empty(), "{tema}: ninguna lectura");
        let mut vistas = Vec::new();
        for l in &leidas {
            assert!(!vistas.contains(l), "{tema}: {l:?} repetida");
            vistas.push(l.clone());
        }
        let distintas: std::collections::BTreeSet<_> = p
            .state
            .ajustes
            .por_tema(ajustes::Tema::from_slug(tema).unwrap())
            .map(|a| (a.leer.funcion.clone(), a.leer.params.clone()))
            .collect();
        assert_eq!(leidas.len(), distintas.len(), "{tema}: {leidas:#?}");
    }
    let red_24 = (
        "cbGetWifiAccPt".to_owned(),
        vec![("dpm_cfg_wifi_acc_pt_id".to_owned(), "0".to_owned())],
    );
    let veces = p
        .sim
        .control
        .reads()
        .iter()
        .filter(|l| **l == red_24)
        .count();
    assert_eq!(veces, 1);
}

/// El id de una instancia es el del router, no su posición: borrada la
/// primera reserva, la que queda conserva el suyo.
#[tokio::test]
async fn ids_de_instancia_no_son_la_posicion() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("del", "DEL"),
            ],
        )
        .await
        .unwrap();
    let tema = leer(&p, &cookie, "red_local").await;
    let reservas = ajuste(&tema, "red_local.reservas");
    let ids: Vec<&Value> = reservas["instancias"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| &i["id"])
        .collect();
    assert_eq!(ids, [&json!("1")], "{reservas}");
}

/// En una lista que se escribe, una respuesta sin el id de las instancias
/// deja el ajuste sin leer: editar o borrar por posición podría tocar otra
/// instancia. El resto del tema se lee.
#[tokio::test]
async fn lista_escrita_sin_id_queda_sin_leer() {
    let p = PanelPrueba::nuevo()
        .capa(&variante("listas-irregulares"))
        .arrancar()
        .await;
    let cookie = sesion(&p).await;
    let tema = leer(&p, &cookie, "red_local").await;
    let reservas = ajuste(&tema, "red_local.reservas");
    assert!(reservas["estado"]["SinLeer"].is_string(), "{reservas}");
    assert!(reservas["instancias"].as_array().unwrap().is_empty());
    assert!(reservas["campos"].as_array().unwrap().is_empty());
    assert_eq!(ajuste(&tema, "red_local.dhcp")["estado"], "Leido");
}

/// Los usuarios del router se emparejan solo con los elementos de sus
/// campos: una hoja suelta en la respuesta no crea un usuario de más.
#[tokio::test]
async fn usuarios_emparejados_solo_por_sus_elementos() {
    let p = PanelPrueba::nuevo()
        .capa(&variante("listas-irregulares"))
        .arrancar()
        .await;
    let cookie = sesion(&p).await;
    let tema = leer(&p, &cookie, "seguridad").await;
    let usuarios = ajuste(&tema, "seguridad.usuarios_router");
    let instancias = usuarios["instancias"].as_array().unwrap();
    let papeles: Vec<&Value> = instancias
        .iter()
        .map(|i| &campo(i, "papel")["valor"])
        .collect();
    assert_eq!(
        papeles,
        [
            &json!({ "Opcion": "operador" }),
            &json!({ "Opcion": "administrador" }),
            &json!({ "Opcion": "usuario" }),
        ],
        "{usuarios}"
    );
    let ids: Vec<&Value> = instancias.iter().map(|i| &i["id"]).collect();
    assert_eq!(ids, [&json!("0"), &json!("1"), &json!("2")]);
}
