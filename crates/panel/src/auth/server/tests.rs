//! Pruebas internas de la autenticación (contraseñas, freno y sesiones).

use super::*;

#[test]
fn retardo_exponencial_con_tope() {
    let esperados = [(0, 0), (1, 1), (2, 2), (3, 4), (4, 8), (5, 16), (6, 32)];
    for (fallos, secs) in esperados {
        assert_eq!(delay_secs(fallos), secs, "{fallos}");
    }
    for fallos in [7, 8, 100, i64::MAX] {
        assert_eq!(delay_secs(fallos), 60, "{fallos}");
    }
}

fn xff(values: &[&'static str]) -> HeaderMap {
    let mut h = HeaderMap::new();
    for v in values {
        h.append("x-forwarded-for", v.parse().unwrap());
    }
    h
}

#[test]
fn ip_del_cliente() {
    let proxy: IpAddr = "192.0.2.100".parse().unwrap();
    let proxies = ["192.0.2.100".parse::<RedIp>().unwrap()];
    let proxies = &proxies[..];
    let otro: IpAddr = "192.0.2.101".parse().unwrap();
    let ip = |s: &str| s.parse::<IpAddr>().unwrap();
    // Fuera del proxy, X-Forwarded-For no cuenta.
    assert_eq!(client_ip(otro, &xff(&["198.51.100.4"]), proxies), otro);
    // Desde el proxy, el último valor.
    assert_eq!(
        client_ip(proxy, &xff(&["9.9.9.9, 198.51.100.4"]), proxies),
        ip("198.51.100.4")
    );
    assert_eq!(
        client_ip(proxy, &xff(&["9.9.9.9", "198.51.100.8"]), proxies),
        ip("198.51.100.8")
    );
    // Sin cabecera o con basura, la del proxy.
    assert_eq!(client_ip(proxy, &xff(&[]), proxies), proxy);
    assert_eq!(
        client_ip(proxy, &xff(&["198.51.100.4, nada"]), proxies),
        proxy
    );
    // IPv4 mapeada en IPv6 se compara como IPv4.
    assert_eq!(
        client_ip(ip("::ffff:192.0.2.100"), &xff(&["198.51.100.4"]), proxies),
        ip("198.51.100.4")
    );
}

#[test]
fn ip_del_cliente_tras_una_cadena_de_proxies() {
    let ip = |s: &str| s.parse::<IpAddr>().unwrap();
    // A (192.0.2.10) y B (192.0.2.20), de confianza: cliente → A → B.
    let proxies = [
        "192.0.2.10".parse::<RedIp>().unwrap(),
        "192.0.2.20".parse::<RedIp>().unwrap(),
    ];
    let proxies = &proxies[..];
    let b = ip("192.0.2.20");
    assert_eq!(
        client_ip(b, &xff(&["198.51.100.4, 192.0.2.10"]), proxies),
        ip("198.51.100.4")
    );
    assert_eq!(
        client_ip(b, &xff(&["198.51.100.4", "192.0.2.10"]), proxies),
        ip("198.51.100.4")
    );
    // Lo que el cliente pone a la izquierda no cuenta.
    assert_eq!(
        client_ip(
            b,
            &xff(&["203.0.113.66, 198.51.100.4, 192.0.2.10"]),
            proxies
        ),
        ip("198.51.100.4")
    );
    // Un valor de confianza falsificado a la izquierda, tampoco.
    assert_eq!(
        client_ip(b, &xff(&["192.0.2.10, 198.51.100.4, 192.0.2.10"]), proxies),
        ip("198.51.100.4")
    );
    // Basura en medio: la última dirección de confianza vista.
    assert_eq!(
        client_ip(b, &xff(&["198.51.100.4, nada, 192.0.2.10"]), proxies),
        ip("192.0.2.10")
    );
    assert_eq!(client_ip(b, &xff(&["198.51.100.4, nada"]), proxies), b);
    // Todos de confianza: el de más a la izquierda.
    assert_eq!(
        client_ip(b, &xff(&["192.0.2.20, 192.0.2.10"]), proxies),
        ip("192.0.2.20")
    );
    // IPv4 mapeada, tanto en el socket como en la cabecera.
    assert_eq!(
        client_ip(
            ip("::ffff:192.0.2.20"),
            &xff(&["::ffff:198.51.100.4, ::ffff:192.0.2.10"]),
            proxies
        ),
        ip("198.51.100.4")
    );
    // Un socket que no es de confianza manda, aunque traiga la cadena.
    assert_eq!(
        client_ip(
            ip("203.0.113.9"),
            &xff(&["198.51.100.4, 192.0.2.10"]),
            proxies
        ),
        ip("203.0.113.9")
    );
}

#[test]
fn hash_y_verificacion() {
    let h = hash_password("una contraseña larga");
    assert!(h.starts_with("$argon2id$"), "{h}");
    assert!(verify("una contraseña larga", &h));
    assert!(!verify("otra contraseña larga", &h));
    assert!(!verify("una contraseña larga", "no es un hash"));
    assert_ne!(h, hash_password("una contraseña larga"), "sal aleatoria");
}

#[test]
fn cookie_de_sesion() {
    let mut h = HeaderMap::new();
    h.append(
        "cookie",
        "a=1; __Host-rp_session=abc ; b=2".parse().unwrap(),
    );
    assert_eq!(cookie_session_id(&h), Some("abc"));
    let mut h = HeaderMap::new();
    h.append("cookie", "x=1".parse().unwrap());
    h.append("cookie", "__Host-rp_session=def".parse().unwrap());
    assert_eq!(cookie_session_id(&h), Some("def"));
    let mut h = HeaderMap::new();
    h.append("cookie", "__Host-rp_session=".parse().unwrap());
    assert_eq!(cookie_session_id(&h), None);
}

#[test]
fn ids_de_sesion_distintos_y_hash_distinto_del_id() {
    let a = new_session_id();
    assert_eq!(a.len(), 43);
    assert_ne!(a, new_session_id());
    assert_ne!(session_key(&a), a);
    assert_eq!(session_key(&a), session_key(&a));
}
