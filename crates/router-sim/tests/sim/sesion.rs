//! Sesión con el simulador: claves, contraseñas, reinicios, retardo y sin respuesta.

use super::*;

#[tokio::test]
async fn bad_session_key_rejected() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let stale = c.session_key("upnp.asp").await.unwrap();
    let fresh = c.session_key("upnp.asp").await.unwrap();
    assert_ne!(stale, fresh, "el sessionKey rota en cada carga");
    for key in ["0", stale.as_str()] {
        let r = c
            .call("cbSetUpnpCfg", &[("adm_state", "0"), ("sessionKey", key)])
            .await;
        assert!(
            matches!(&r, Err(Error::CallFailed { result, .. }) if result == "1"),
            "{r:?}"
        );
    }
    assert!(sim.control.writes().is_empty());
    let n = c.call("cbGetUpnpCfg", &[]).await.unwrap();
    let before = n
        .get("upnp_cfg")
        .unwrap()
        .text_of("adm_state")
        .unwrap()
        .to_owned();
    let flipped = if before == "1" { "0" } else { "1" };
    c.write("upnp.asp", "cbSetUpnpCfg", &[("adm_state", flipped)])
        .await
        .unwrap();
    let n = c.call("cbGetUpnpCfg", &[]).await.unwrap();
    assert_eq!(
        n.get("upnp_cfg").unwrap().text_of("adm_state"),
        Some(flipped)
    );
}

#[tokio::test]
async fn reboot_and_invalidate_sessions_force_relogin() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.write("reboot.asp", "cbReboot", &[]).await.unwrap();
    assert_eq!(sim.control.writes()[0].0, "cbReboot");
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
    sim.control.invalidate_sessions();
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
}

#[tokio::test]
async fn unresponsive_times_out() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(1));
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
    sim.control.set_unresponsive(true);
    let r = c.call("cbGetUpnpCfg", &[]).await;
    assert!(matches!(r, Err(Error::Network(_))), "{r:?}");
    sim.control.set_unresponsive(false);
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
}

#[tokio::test]
async fn reboot_resets_uptime() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let uptime = |n: &XmlNode| n.text_of("SysUptime").unwrap().parse::<u64>().unwrap();
    let before = uptime(&c.call("cbGetSummary", &[]).await.unwrap());
    assert!(before > 1000);
    c.write("reboot.asp", "cbReboot", &[]).await.unwrap();
    let after = uptime(&c.call("cbGetSummary", &[]).await.unwrap());
    assert!(after < 5, "{after}");
}

#[tokio::test]
async fn contrasena_cambiable() {
    let sim = start().await;
    sim.control.set_password("otra-clave-del-router");
    let r = client(&sim, Duration::from_secs(2))
        .call("cbGetSummary", &[])
        .await;
    assert!(matches!(r, Err(Error::Auth(_))), "{r:?}");
    let mut cfg = ClientConfig::new(&sim.base_url, USER, "otra-clave-del-router");
    cfg.min_gap = Duration::from_millis(1);
    Client::new(cfg).call("cbGetSummary", &[]).await.unwrap();
}

// ------------------------------------------------ cbModifyPwd y login

fn cliente_con(sim: &SimHandle, password: &str) -> Client {
    Client::new(ClientConfig {
        base_url: sim.base_url.clone(),
        user: USER.into(),
        password: password.into(),
        timeout: Duration::from_secs(5),
        min_gap: Duration::from_millis(1),
    })
}

/// `cbModifyPwd` de `usuario`, de `actual` a `nueva`, codificado como lo
/// manda el panel (XOR, como la web oficial).
async fn modificar(c: &Client, usuario: &str, actual: &str, nueva: &str) -> Result<(), Error> {
    let p = hgu::fmt::parametros_cambio_contrasena(usuario, actual, nueva);
    let p: Vec<(&str, &str)> = p.iter().map(|(k, v)| (*k, v.as_str())).collect();
    c.write("user_profile.asp", "cbModifyPwd", &p)
        .await
        .map(|_| ())
}

#[tokio::test]
async fn modify_pwd_cambia_y_el_login_nuevo_entra() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.call("cbGetSummary", &[]).await.unwrap();
    modificar(&c, USER, PASS, "clave-nueva").await.unwrap();
    // La sesión abierta sigue valiendo.
    c.call("cbGetSummary", &[]).await.unwrap();
    // La nueva entra; la vieja ya no.
    cliente_con(&sim, "clave-nueva")
        .call("cbGetSummary", &[])
        .await
        .unwrap();
    assert!(
        cliente_con(&sim, PASS)
            .call("cbGetSummary", &[])
            .await
            .is_err()
    );
}

#[tokio::test]
async fn modify_pwd_con_actual_mala_falla() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let e = modificar(&c, USER, "otra", "clave-nueva")
        .await
        .unwrap_err();
    assert!(
        matches!(&e, Error::CallFailed { result, .. } if result != "0"),
        "{e:?}"
    );
    let e = modificar(&c, "otro", PASS, "x").await.unwrap_err();
    assert!(matches!(e, Error::CallFailed { .. }));
    cliente_con(&sim, PASS)
        .call("cbGetSummary", &[])
        .await
        .unwrap();
}

#[tokio::test]
async fn modify_pwd_sin_efecto_responde_ok_y_no_cambia() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    sim.control.modify_pwd_sin_efecto();
    modificar(&c, USER, PASS, "clave-nueva").await.unwrap();
    cliente_con(&sim, PASS)
        .call("cbGetSummary", &[])
        .await
        .unwrap();
    // Solo la próxima: la siguiente sí cambia.
    modificar(&c, USER, PASS, "clave-nueva").await.unwrap();
    cliente_con(&sim, "clave-nueva")
        .call("cbGetSummary", &[])
        .await
        .unwrap();
}

/// Con retraso, el cambio responde OK y queda pendiente: hasta que pasa, la
/// anterior sigue entrando y la nueva no; el primer inicio de sesión
/// después lo aplica.
#[tokio::test]
async fn modify_pwd_con_retraso_se_aplica_al_entrar_despues() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    sim.control
        .modify_pwd_con_retraso(Duration::from_millis(300));
    modificar(&c, USER, PASS, "clave-nueva").await.unwrap();
    assert!(
        cliente_con(&sim, "clave-nueva")
            .call("cbGetSummary", &[])
            .await
            .is_err()
    );
    cliente_con(&sim, PASS)
        .call("cbGetSummary", &[])
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(350)).await;
    cliente_con(&sim, "clave-nueva")
        .call("cbGetSummary", &[])
        .await
        .unwrap();
    assert!(
        cliente_con(&sim, PASS)
            .call("cbGetSummary", &[])
            .await
            .is_err()
    );
    // Solo el próximo: el siguiente cambio se aplica enseguida.
    modificar(&c, USER, "clave-nueva", "otra-clave")
        .await
        .unwrap();
    cliente_con(&sim, "otra-clave")
        .call("cbGetSummary", &[])
        .await
        .unwrap();
}

/// Tras unos inicios: los `n` inicios de sesión siguientes al cambio aún
/// entran con la anterior (los fallidos también cuentan) y el siguiente ya
/// la aplica.
#[tokio::test]
async fn modify_pwd_tras_inicios_se_aplica_en_el_siguiente() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    sim.control.modify_pwd_tras_inicios(2);
    modificar(&c, USER, PASS, "clave-nueva").await.unwrap();
    assert!(
        cliente_con(&sim, "clave-nueva")
            .call("cbGetSummary", &[])
            .await
            .is_err()
    );
    cliente_con(&sim, PASS)
        .call("cbGetSummary", &[])
        .await
        .unwrap();
    assert!(
        cliente_con(&sim, PASS)
            .call("cbGetSummary", &[])
            .await
            .is_err()
    );
    cliente_con(&sim, "clave-nueva")
        .call("cbGetSummary", &[])
        .await
        .unwrap();
}

#[tokio::test]
async fn login_falla_con_aunque_sea_la_vigente() {
    let sim = start().await;
    sim.control.login_falla_con(PASS);
    assert!(
        client(&sim, Duration::from_secs(5))
            .call("cbGetSummary", &[])
            .await
            .is_err()
    );
    sim.control.set_password("otra-clave");
    cliente_con(&sim, "otra-clave")
        .call("cbGetSummary", &[])
        .await
        .unwrap();
}

/// Petición HTTP mínima (sin cliente): devuelve el cuerpo.
fn http(base: &str, metodo: &str, ruta: &str, cookie: &str, cuerpo: &str) -> (String, String) {
    use std::io::{Read, Write};
    let dir = base.trim_start_matches("http://");
    let mut s = std::net::TcpStream::connect(dir).unwrap();
    let peticion = format!(
        "{metodo} {ruta} HTTP/1.1\r\nHost: {dir}\r\nCookie: {cookie}\r\nContent-Type: \
         application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}",
        cuerpo.len()
    );
    s.write_all(peticion.as_bytes()).unwrap();
    let mut r = String::new();
    s.read_to_string(&mut r).unwrap();
    let cabeceras = r.split("\r\n\r\n").next().unwrap().to_owned();
    (
        cabeceras,
        r.split_once("\r\n\r\n")
            .map_or(String::new(), |x| x.1.to_owned()),
    )
}

fn codificar(s: &str) -> String {
    hgu::fmt::mess_userpass(s)
        .bytes()
        .map(|b| format!("%{b:02X}"))
        .collect()
}

#[tokio::test]
async fn login_falla_con_tambien_en_el_avanzado_aunque_el_basico_pase() {
    let sim = start().await;
    let base = sim.base_url.clone();
    let (usuario, vigente) = (codificar(USER), codificar(PASS));
    let b2 = base.clone();
    let (cabeceras, _) = tokio::task::spawn_blocking(move || http(&b2, "GET", "/", "", ""))
        .await
        .unwrap();
    let cookie = cabeceras
        .lines()
        .find_map(|l| l.strip_prefix("set-cookie: "))
        .and_then(|c| c.split(';').next())
        .unwrap()
        .to_owned();
    let (c1, u1, v1) = (cookie.clone(), usuario.clone(), vigente.clone());
    let b2 = base.clone();
    // El login básico entra antes de armar el fallo.
    let (_, cuerpo) = tokio::task::spawn_blocking(move || {
        http(
            &b2,
            "POST",
            "/cgi-bin/te_acceso_router.cgi",
            &c1,
            &format!("loginUsername={u1}&loginPassword={v1}"),
        )
    })
    .await
    .unwrap();
    assert!(cuerpo.contains("te_wifi.asp"), "el básico entra: {cuerpo}");
    let avanzado = move |base: String, cookie: String| {
        let cuerpo = format!("rqUsername={usuario}&rqPasswd={vigente}");
        tokio::task::spawn_blocking(move || {
            http(&base, "POST", "/cgi-bin/cbCheckLogin.xml", &cookie, &cuerpo).1
        })
    };
    let ok = avanzado(base.clone(), cookie.clone()).await.unwrap();
    assert!(
        ok.contains("<SessionID>"),
        "sin armar, el avanzado entra: {ok}"
    );
    sim.control.login_falla_con(PASS);
    let mal = avanzado(base, cookie).await.unwrap();
    assert!(
        !mal.contains("<SessionID>") && mal.contains("<Result>1</Result>"),
        "{mal}"
    );
}

#[tokio::test]
async fn retardo_de_cada_respuesta() {
    let mut cfg = SimConfig::new(USER, PASS, fixtures_dir());
    cfg.retardo = Duration::from_millis(150);
    let sim = Sim::start(cfg).await;
    let c = client(&sim, Duration::from_secs(5));
    c.call("cbGetSummary", &[]).await.unwrap();
    let t = std::time::Instant::now();
    c.call("cbGetSummary", &[]).await.unwrap();
    assert!(
        t.elapsed() >= Duration::from_millis(150),
        "{:?}",
        t.elapsed()
    );
    // Sin retardo, con el control del simulador…
    sim.control.set_retardo(Duration::ZERO);
    let t = std::time::Instant::now();
    c.call("cbGetSummary", &[]).await.unwrap();
    assert!(
        t.elapsed() < Duration::from_millis(150),
        "{:?}",
        t.elapsed()
    );
    // …y con el de HTTP (el del e2e).
    let addr = sim.base_url.trim_start_matches("http://").to_owned();
    let resp = tokio::task::spawn_blocking(move || {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(addr).unwrap();
        s.write_all(
            b"POST /__sim/retardo HTTP/1.1\r\nHost: x\r\nContent-Length: 3\r\nConnection: close\r\n\r\n200",
        )
        .unwrap();
        let mut resp = String::new();
        s.read_to_string(&mut resp).unwrap();
        resp
    })
    .await
    .unwrap();
    assert!(resp.starts_with("HTTP/1.1 204"), "{resp}");
    let t = std::time::Instant::now();
    c.call("cbGetSummary", &[]).await.unwrap();
    assert!(
        t.elapsed() >= Duration::from_millis(200),
        "{:?}",
        t.elapsed()
    );
}
