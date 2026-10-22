//! Reservas DHCP por las server functions: crear, borrar, deshacer y errores.

use super::*;

/// «Fijar esta IP»: la reserva nueva con la MAC y la IP actuales del
/// dispositivo, activa. Aparece en las reservas y en el dispositivo; deshacerla la
/// borra.
#[tokio::test]
async fn fijar_ip_crea_reserva_y_deshacer_la_borra() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = reservas(&p, &cookie).await;
    let d = dispositivos(&p, &cookie).await;
    let host = por_mac(&d, "02:00:00:00:00:03");
    assert_eq!(host["reserva"], Value::Null, "{host}");
    let (mac, ipv4) = (host["mac"].clone(), host["ip"].clone());
    let (status, body) = preparar(
        &p,
        &cookie,
        None,
        false,
        json!({ "mac": mac, "ip": ipv4, "activa": "true" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["titulo"], "ajuste-red_local-reservas-titulo");
    let id = aplicar_ajuste(&p, &cookie, &g).await;

    let ahora = reservas(&p, &cookie).await;
    assert_eq!(ahora.len(), antes.len() + 1, "{ahora:?}");
    let nueva = ahora
        .iter()
        .find(|(_, m, _)| m == "02:00:00:00:00:03")
        .unwrap_or_else(|| panic!("sin la reserva nueva: {ahora:?}"));
    assert_eq!(nueva.2, "192.168.1.62");
    let d = dispositivos(&p, &cookie).await;
    assert_eq!(
        por_mac(&d, "02:00:00:00:00:03")["reserva"],
        json!(nueva.0),
        "{d:?}"
    );

    deshacer(&p, &cookie, id).await;
    assert_eq!(reservas(&p, &cookie).await, antes);
    let d = dispositivos(&p, &cookie).await;
    assert_eq!(por_mac(&d, "02:00:00:00:00:03")["reserva"], Value::Null);
}

#[tokio::test]
async fn borrar_reserva_y_deshacer_la_recrea() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = reservas(&p, &cookie).await;
    assert!(
        antes.contains(&(
            "1".into(),
            "02:00:00:00:00:01".into(),
            "192.168.1.69".into()
        )),
        "{antes:?}"
    );
    let (status, body) = preparar(&p, &cookie, Some("1"), true, json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    let sin = reservas(&p, &cookie).await;
    assert!(
        !sin.iter().any(|(_, m, _)| m == "02:00:00:00:00:01"),
        "{sin:?}"
    );
    assert_eq!(sin.len(), antes.len() - 1);

    deshacer(&p, &cookie, id).await;
    let otra_vez = reservas(&p, &cookie).await;
    assert_eq!(otra_vez.len(), antes.len(), "{otra_vez:?}");
    assert!(
        otra_vez
            .iter()
            .any(|(_, m, ip)| m == "02:00:00:00:00:01" && ip == "192.168.1.69"),
        "{otra_vez:?}"
    );
}

/// Una reserva que no vale no se prepara (400, junto al campo y en el
/// idioma) y no escribe nada.
#[tokio::test]
async fn reserva_invalida_da_400() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        None,
        false,
        json!({ "mac": "02:00:00:00:00:03", "ip": "192.0.2.10", "activa": "true" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("Dirección IP"), "{body}");
    assert!(body.contains("no es de tu red"), "{body}");
    let en = format!("{cookie}; rp_lang=en");
    let (status, body) = preparar(
        &p,
        &en,
        None,
        false,
        json!({ "mac": "02:00:00:00:00:03", "ip": "192.0.2.10", "activa": "true" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("isn't on your network"), "{body}");
    // La IP del router, una MAC que ya tiene reserva y, al editar, la IP de
    // otra reserva.
    for (instancia, valores, texto) in [
        (
            None,
            json!({ "mac": "02:00:00:00:00:03", "ip": "192.168.1.1", "activa": "true" }),
            "IP del router",
        ),
        (
            None,
            json!({ "mac": "02:00:00:00:00:04", "ip": "192.168.1.90", "activa": "true" }),
            "ya tiene una IP fija",
        ),
        (
            Some("0"),
            json!({ "ip": "192.168.1.69" }),
            "IP fija de otro dispositivo",
        ),
    ] {
        let (status, body) = preparar(&p, &cookie, instancia, false, valores).await;
        assert_eq!(status, 400, "{body}");
        assert!(body.contains(texto), "{texto}: {body}");
    }
    // Una nueva sin IP.
    let (status, body) = preparar(
        &p,
        &cookie,
        None,
        false,
        json!({ "mac": "02:00:00:00:00:03", "activa": "true" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("Dirección IP"), "{body}");
    assert!(p.sim.control.writes().is_empty());
    // Editar la misma reserva con su propia IP no es repetirla.
    let (status, body) =
        preparar(&p, &cookie, Some("0"), false, json!({ "activa": "false" })).await;
    assert_eq!(status, 200, "{body}");
}

/// Cada dispositivo con su nombre, su conexión (la banda por el punto de acceso
/// al que está asociado; si no, el cable) y su reserva.
#[tokio::test]
async fn dispositivos_con_banda() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("dispositivos-wifi"))
        .arrancar_con_gestor()
        .await;
    let d = dispositivos(&p, &cookie).await;
    assert_eq!(d.len(), 10, "{d:?}");
    let conexion = |mac: &str| por_mac(&d, mac)["conexion"].clone();
    assert_eq!(conexion("02:00:00:00:00:03"), json!("Wifi24"));
    assert_eq!(conexion("02:00:00:00:00:06"), json!("Wifi5"));
    assert_eq!(conexion("02:00:00:00:00:05"), json!("Invitados"));
    assert_eq!(conexion("02:00:00:00:00:01"), json!({ "Cable": "1:3" }));
    let apagado = por_mac(&d, "02:00:00:00:00:02");
    assert_eq!(apagado["activo"], false);
    assert_eq!(apagado["conexion"], json!("Desconocida"));
    assert_eq!(apagado["nombre"], "host-1");
    assert_eq!(apagado["ip"], "192.168.1.61");
    let reservado = por_mac(&d, "02:00:00:00:00:04");
    assert_eq!(reservado["reserva"], "0");
    assert_eq!(reservado["nombre"], "host-3");
    assert_eq!(por_mac(&d, "02:00:00:00:00:01")["reserva"], "1");
    // Los conectados primero, por IP.
    assert_eq!(d[0]["mac"], "02:00:00:00:00:01");
    assert_eq!(d.last().unwrap()["activo"], false);
}

/// Desactivar una reserva que ya no valdría (comparte IP con otra) no exige
/// corregir la IP; activarla, sí.
#[tokio::test]
async fn desactivar_sin_corregir_la_ip() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("ipv4_addr", "192.168.1.69"),
                ("adm_state", "1"),
            ],
        )
        .await
        .unwrap();
    let (status, body) =
        preparar(&p, &cookie, Some("0"), false, json!({ "activa": "false" })).await;
    assert_eq!(status, 200, "{body}");
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("adm_state", "0"),
            ],
        )
        .await
        .unwrap();
    let (status, body) = preparar(&p, &cookie, Some("0"), false, json!({ "activa": "true" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("IP fija de otro dispositivo"), "{body}");
}

/// Qué peticiones de una reserva se validan: una nueva, cambiar la MAC o la
/// IP y activarla; desactivarla o borrarla, no.
#[test]
fn que_se_valida_de_una_reserva() {
    let p = |instancia: Option<&str>, borrar: bool, valores: &[(&str, &str)]| PeticionAjuste {
        ajuste: "red_local.reservas".into(),
        instancia: instancia.map(str::to_owned),
        borrar,
        valores: valores
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        esperados: Default::default(),
    };
    assert!(valida_la_reserva(&p(
        None,
        false,
        &[("mac", "x"), ("ip", "y")]
    )));
    assert!(valida_la_reserva(&p(Some("0"), false, &[("ip", "y")])));
    assert!(valida_la_reserva(&p(Some("0"), false, &[("mac", "x")])));
    assert!(valida_la_reserva(&p(
        Some("0"),
        false,
        &[("activa", "true")]
    )));
    assert!(!valida_la_reserva(&p(
        Some("0"),
        false,
        &[("activa", "false")]
    )));
    assert!(!valida_la_reserva(&p(Some("0"), true, &[])));
}

/// El resumen de una reserva nombra el dispositivo, si se conoce.
#[tokio::test]
async fn el_resumen_nombra_el_dispositivo() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        None,
        false,
        json!({ "mac": "02:00:00:00:00:03", "ip": "192.168.1.62", "activa": "true" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["detalle"], "host-2", "{g}");
    let (status, body) = preparar(&p, &cookie, Some("0"), true, json!({})).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["detalle"], "host-3", "{g}");
}

/// `preparar_ajuste` con lo que la página espera de la instancia.
async fn preparar_esperando(
    p: &Panel,
    cookie: &str,
    instancia: &str,
    borrar: bool,
    valores: Value,
    mac: &str,
) -> (u16, String) {
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": {
            "ajuste": "red_local.reservas",
            "instancia": instancia,
            "borrar": borrar,
            "valores": valores,
            "esperados": { "mac": mac },
        } }),
        Some(cookie),
    )
    .await
}

/// Editar o borrar una reserva que otra sesión ha cambiado entretanto (se
/// borró la de id máximo y se creó otra, que el router numera igual): 409
/// y nada escrito. Con la que se esperaba, sí se prepara.
#[tokio::test]
async fn editar_o_borrar_otra_reserva_da_409() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let base = &[("rtm_cfg_dhcpv4_svr_pool_id", "0")];
    let escribir = |extra: &[(&'static str, &'static str)]| {
        let mut params = base.to_vec();
        params.extend_from_slice(extra);
        params
    };
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &escribir(&[("rtm_dhcpv4_svr_pool_static_id", "1"), ("del", "DEL")]),
        )
        .await
        .unwrap();
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &escribir(&[
                ("rtm_dhcpv4_svr_pool_static_id", "-1"),
                ("mac_addr", "02:00:00:00:00:0b"),
                ("ipv4_addr", "192.168.1.90"),
            ]),
        )
        .await
        .unwrap();
    assert!(
        reservas(&p, &cookie).await.contains(&(
            "1".into(),
            "02:00:00:00:00:0b".into(),
            "192.168.1.90".into()
        )),
        "el sim reutiliza el id"
    );
    let escritas = p.sim.control.writes().len();
    let (status, body) =
        preparar_esperando(&p, &cookie, "1", true, json!({}), "02:00:00:00:00:01").await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("ha cambiado"), "{body}");
    let (status, body) = preparar_esperando(
        &p,
        &cookie,
        "1",
        false,
        json!({ "activa": "false" }),
        "02-00-00-00-00-01",
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(p.sim.control.writes().len(), escritas);
    let (status, body) =
        preparar_esperando(&p, &cookie, "1", true, json!({}), "02-00-00-00-00-0B").await;
    assert_eq!(status, 200, "{body}");
}

/// Editar una reserva que no existe y crear una con la IP de difusión.
#[tokio::test]
async fn reserva_inexistente_o_de_difusion_da_400() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) =
        preparar(&p, &cookie, Some("77"), false, json!({ "activa": "false" })).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("no ha podido preparar"), "{body}");
    let (status, body) = preparar(
        &p,
        &cookie,
        None,
        false,
        json!({ "mac": "02:00:00:00:00:03", "ip": "192.168.1.0", "activa": "true" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("es de toda la red"), "{body}");
    assert!(p.sim.control.writes().is_empty());
}

/// Editar o borrar una reserva exige la MAC que la página espera: sin
/// ella, el servidor no puede saber si la reserva sigue siendo la misma
/// (400, nada escrito).
#[tokio::test]
async fn editar_o_borrar_sin_mac_esperada_da_400() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for (borrar, valores) in [(true, json!({})), (false, json!({ "activa": "false" }))] {
        let (status, body) = llamar(
            &p,
            "preparar_ajuste",
            json!({ "p": {
                "ajuste": "red_local.reservas",
                "instancia": "1",
                "borrar": borrar,
                "valores": valores,
            } }),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, 400, "borrar={borrar}: {body}");
        assert!(body.contains("no ha podido preparar"), "{body}");
    }
    assert!(p.sim.control.writes().is_empty());
}

/// La página trae las IP de las IP fijas (Puertos las usa para saber si el
/// destino de una regla sigue en la red).
#[tokio::test]
async fn ips_fijas_en_la_pagina() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let r = ok(&p, "leer_tema", json!({ "tema": "puertos" }), &cookie).await;
    assert_eq!(
        r["red"]["ips_fijas"],
        json!(["192.168.1.68", "192.168.1.69"])
    );
}
