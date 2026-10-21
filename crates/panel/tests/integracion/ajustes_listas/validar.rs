//! Validación de las reservas DHCP, sin servidor: IP, MAC y difusión.

use super::*;

#[test]
fn reserva_valida() {
    let r = validar_reserva(
        "02:00:00:00:00:03",
        ip("192.168.1.62"),
        LAN,
        &existentes(),
        None,
    );
    assert_eq!(r, Ok(()));
}

#[test]
fn fuera_de_la_red() {
    for otra in ["192.0.2.10", "198.51.100.62", "203.0.113.1"] {
        let r = validar_reserva("02:00:00:00:00:03", ip(otra), LAN, &existentes(), None);
        assert_eq!(r, Err(ErrorReserva::FueraDeLaRed), "{otra}");
    }
}

#[test]
fn ip_del_router() {
    let r = validar_reserva(
        "02:00:00:00:00:03",
        ip("192.168.1.1"),
        LAN,
        &existentes(),
        None,
    );
    assert_eq!(r, Err(ErrorReserva::IpDelRouter));
}

#[test]
fn ip_repetida() {
    let r = validar_reserva(
        "02:00:00:00:00:03",
        ip("192.168.1.69"),
        LAN,
        &existentes(),
        None,
    );
    assert_eq!(r, Err(ErrorReserva::IpRepetida));
}

/// La MAC se compara sin mirar mayúsculas ni el separador.
#[test]
fn mac_repetida() {
    for mac in [
        "02:00:00:00:00:04",
        "02-00-00-00-00-04",
        " 02-00-00-00-00-04 ",
    ] {
        let r = validar_reserva(mac, ip("192.168.1.90"), LAN, &existentes(), None);
        assert_eq!(r, Err(ErrorReserva::MacRepetida), "{mac}");
    }
    let mut mayusculas = existentes();
    mayusculas[0].1 = "02:00:00:00:00:0A".into();
    let r = validar_reserva(
        "02:00:00:00:00:0a",
        ip("192.168.1.90"),
        LAN,
        &mayusculas,
        None,
    );
    assert_eq!(r, Err(ErrorReserva::MacRepetida));
}

/// La primera y la última dirección de la red son de toda la red (en una
/// red de documentación, 192.0.2.0/24).
#[test]
fn difusion() {
    let lan = (Ipv4Addr::new(192, 0, 2, 1), Ipv4Addr::new(255, 255, 255, 0));
    for especial in ["192.0.2.255", "192.0.2.0"] {
        let r = validar_reserva("02:00:00:00:00:03", ip(especial), lan, &[], None);
        assert_eq!(r, Err(ErrorReserva::Difusion), "{especial}");
    }
    let r = validar_reserva("02:00:00:00:00:03", ip("192.0.2.254"), lan, &[], None);
    assert_eq!(r, Ok(()));
}

/// Al editar una reserva, su propia MAC e IP no cuentan como repetidas;
/// las de las demás, sí.
#[test]
fn editar_la_misma_no_es_repetida() {
    let e = existentes();
    let r = validar_reserva("02:00:00:00:00:04", ip("192.168.1.68"), LAN, &e, Some("0"));
    assert_eq!(r, Ok(()));
    let r = validar_reserva("02:00:00:00:00:04", ip("192.168.1.70"), LAN, &e, Some("0"));
    assert_eq!(r, Ok(()));
    let r = validar_reserva("02:00:00:00:00:04", ip("192.168.1.69"), LAN, &e, Some("0"));
    assert_eq!(r, Err(ErrorReserva::IpRepetida));
    let r = validar_reserva("02:00:00:00:00:01", ip("192.168.1.68"), LAN, &e, Some("0"));
    assert_eq!(r, Err(ErrorReserva::MacRepetida));
}
