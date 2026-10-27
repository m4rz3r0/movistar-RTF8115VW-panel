//! Freno de los intentos fallidos y la IP del cliente tras el proxy.

use super::*;

#[tokio::test]
async fn login_mal_incrementa_retardo() {
    let p = PanelPrueba::nuevo().arrancar().await;
    crear_usuario(&p).await;

    let (status, body) = login_desde(&p, PANEL_USER, "contraseña equivocada", "127.0.0.1").await;
    assert_eq!(status, 401, "{body}");
    assert!(body.contains("Usuario o contraseña incorrectos"), "{body}");
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, Some(1));
    // No hay freno por usuario: desde otro equipo nadie puede bloquearlo.
    assert_eq!(fallos(&p, "user", PANEL_USER).await, None);

    // Durante la espera se rechaza incluso la contraseña buena, sin contar
    // un fallo más.
    let (status, body) = login_desde(&p, PANEL_USER, PANEL_PASS, "127.0.0.1").await;
    assert_eq!(status, 429, "{body}");
    assert!(body.contains("Demasiados intentos, espera 1 s"), "{body}");
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, Some(1));

    retrasar_fallos(&p, 1).await;
    let (status, _) = login_desde(&p, PANEL_USER, "otra mala", "127.0.0.1").await;
    assert_eq!(status, 401);
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, Some(2));
    let (status, body) = login_desde(&p, PANEL_USER, PANEL_PASS, "127.0.0.1").await;
    assert_eq!(status, 429);
    assert!(body.contains("espera 2 s"), "{body}");

    // Tras esperar, el login correcto entra y limpia los contadores.
    retrasar_fallos(&p, 2).await;
    let (status, body) = login_desde(&p, PANEL_USER, PANEL_PASS, "127.0.0.1").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, None);
}

/// Los fallos de un equipo no frenan al mismo usuario en otro: nadie de la
/// red puede dejar al administrador sin entrar.
#[tokio::test]
async fn fallos_de_otra_ip_no_bloquean_al_usuario() {
    let p = PanelPrueba::nuevo().arrancar().await;
    crear_usuario(&p).await;
    for _ in 0..3 {
        retrasar_fallos(&p, 60).await;
        let (status, _) = login_desde(&p, PANEL_USER, "mala", "192.0.2.2").await;
        assert_eq!(status, 401);
    }
    let (status, _) = login_desde(&p, PANEL_USER, PANEL_PASS, "192.0.2.2").await;
    assert_eq!(status, 429, "el equipo que falla sí queda frenado");
    let (status, body) = login_desde(&p, PANEL_USER, PANEL_PASS, "127.0.0.1").await;
    assert_eq!(status, 200, "{body}");
    // El login bueno de un equipo no limpia el freno del otro.
    assert_eq!(fallos(&p, "ip", "192.0.2.2").await, Some(3));
}

/// En IPv6 cuenta el prefijo /64: cambiar de dirección dentro de la misma
/// red (SLAAC, direcciones temporales) no se salta el freno.
#[tokio::test]
async fn ipv6_cuenta_por_prefijo_64() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let (status, _) = login_desde(&p, "x", "mala", "[2001:db8:0:1::a]").await;
    assert_eq!(status, 401);
    let (status, _) = login_desde(&p, "y", "mala", "[2001:db8:0:1::b]").await;
    assert_eq!(status, 429);
    assert_eq!(fallos(&p, "ip", "2001:db8:0:1::/64").await, Some(1));
    let (status, _) = login_desde(&p, "x", "mala", "[2001:db8:0:2::a]").await;
    assert_eq!(status, 401, "otra /64 tiene su propio contador");
}

#[tokio::test]
async fn retardo_maximo_60_s() {
    let p = PanelPrueba::nuevo().arrancar().await;
    // El último fallo, ahora mismo (en milisegundos: `strftime('%s')`
    // redondeaba al segundo y, con la máquina cargada, la espera que quedaba
    // ya era de 59 s).
    let ahora_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    sqlx::query("INSERT INTO login_attempts VALUES ('ip', '127.0.0.1', 40, ?)")
        .bind(ahora_ms)
        .execute(&p.state.db)
        .await
        .unwrap();
    let (status, body) = login_desde(&p, "x", "lo que sea", "127.0.0.1").await;
    assert_eq!(status, 429);
    // 40 fallos esperan lo máximo, 60 s, menos lo que haya tardado la
    // petición: nunca más de 60.
    let segundos: i64 = body
        .split("espera ")
        .nth(1)
        .and_then(|r| r.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{body}"));
    assert!((55..=60).contains(&segundos), "{body}");
}

#[tokio::test]
async fn xff_ignorado_si_no_viene_del_proxy() {
    let p = PanelPrueba::nuevo().proxy("192.0.2.100").arrancar().await;
    let mut req = post_form("/api/login", &login_body("x", "mala"), "127.0.0.1");
    req.headers_mut()
        .insert("x-forwarded-for", "198.51.100.4".parse().unwrap());
    assert_eq!(send(&p.app, req).await.status(), 401);
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, Some(1));
    assert_eq!(fallos(&p, "ip", "198.51.100.4").await, None);
}

#[tokio::test]
async fn xff_usado_si_viene_del_proxy() {
    let p = PanelPrueba::nuevo().arrancar().await; // proxy de confianza: 127.0.0.1
    let mut req = post_form("/api/login", &login_body("x", "mala"), "127.0.0.1");
    req.headers_mut()
        .insert("x-forwarded-for", "9.9.9.9, 198.51.100.4".parse().unwrap());
    assert_eq!(send(&p.app, req).await.status(), 401);
    assert_eq!(fallos(&p, "ip", "198.51.100.4").await, Some(1));
    assert_eq!(fallos(&p, "ip", "9.9.9.9").await, None);
    assert_eq!(fallos(&p, "ip", "127.0.0.1").await, None);
}

/// Con una red de proxies (p. ej. la de los contenedores), cuenta
/// `X-Forwarded-For` si el par está en la red, y no si está fuera.
#[tokio::test]
async fn proxy_de_confianza_por_red() {
    let p = PanelPrueba::nuevo()
        .proxy("198.51.100.0/24")
        .arrancar()
        .await;
    // Un usuario distinto en cada intento, para que su freno no cuente.
    let intento = |peer: &str, cliente: &'static str| {
        let mut req = post_form("/api/login", &login_body(peer, "mala"), peer);
        req.headers_mut()
            .insert("x-forwarded-for", cliente.parse().unwrap());
        req
    };
    assert_eq!(
        send(&p.app, intento("198.51.100.5", "203.0.113.4"))
            .await
            .status(),
        401
    );
    assert_eq!(fallos(&p, "ip", "203.0.113.4").await, Some(1));
    assert_eq!(fallos(&p, "ip", "198.51.100.5").await, None);
    assert_eq!(
        send(&p.app, intento("192.0.2.5", "203.0.113.8"))
            .await
            .status(),
        401
    );
    assert_eq!(fallos(&p, "ip", "192.0.2.5").await, Some(1));
    assert_eq!(fallos(&p, "ip", "203.0.113.8").await, None);
}
