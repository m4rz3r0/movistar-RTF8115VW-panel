//! Validación de las reglas de puertos, sin servidor: rangos y solapes.

use super::*;

fn regla(id: Option<&str>, protocolo: Protocolo, externo: (u16, u16)) -> ReglaPuertos {
    ReglaPuertos {
        id: id.map(str::to_owned),
        nombre: "Consola".into(),
        protocolo,
        externo,
        interno: externo,
        ip: ip("192.168.1.62"),
        activa: true,
    }
}

/// Las reglas del router de las pruebas: 8080 TCP y 3074–3075 UDP.
fn reglas_existentes() -> Vec<ReglaPuertos> {
    vec![
        regla(Some("0"), Protocolo::Tcp, (8080, 8080)),
        regla(Some("1"), Protocolo::Udp, (3074, 3075)),
    ]
}

#[test]
fn regla_valida() {
    let e = reglas_existentes();
    for r in [
        regla(None, Protocolo::Tcp, (9000, 9000)),
        regla(None, Protocolo::Udp, (8080, 8080)),
        regla(None, Protocolo::Tcp, (3074, 3075)),
        regla(None, Protocolo::Ambos, (9000, 9010)),
    ] {
        assert_eq!(validar_regla(&r, &e, LAN), Ok(()), "{r:?}");
    }
    // Otro puerto interno del mismo tamaño.
    let mut r = regla(None, Protocolo::Tcp, (9000, 9001));
    r.interno = (80, 81);
    assert_eq!(validar_regla(&r, &e, LAN), Ok(()));
}

#[test]
fn puerto_cero() {
    let r = regla(None, Protocolo::Tcp, (0, 10));
    assert_eq!(validar_regla(&r, &[], LAN), Err(ErrorRegla::PuertoCero));
    let mut r = regla(None, Protocolo::Tcp, (10, 10));
    r.interno = (0, 0);
    assert_eq!(validar_regla(&r, &[], LAN), Err(ErrorRegla::PuertoCero));
}

#[test]
fn rango_invertido() {
    let r = regla(None, Protocolo::Tcp, (9010, 9000));
    assert_eq!(validar_regla(&r, &[], LAN), Err(ErrorRegla::RangoInvertido));
    let mut r = regla(None, Protocolo::Tcp, (9000, 9000));
    r.interno = (90, 80);
    assert_eq!(validar_regla(&r, &[], LAN), Err(ErrorRegla::RangoInvertido));
}

#[test]
fn rangos_distintos() {
    let mut r = regla(None, Protocolo::Tcp, (9000, 9010));
    r.interno = (9000, 9005);
    assert_eq!(
        validar_regla(&r, &[], LAN),
        Err(ErrorRegla::RangosDistintos)
    );
}

#[test]
fn solapa() {
    let e = reglas_existentes();
    for r in [
        regla(None, Protocolo::Tcp, (8080, 8080)),
        regla(None, Protocolo::Tcp, (8000, 8080)),
        regla(None, Protocolo::Udp, (3075, 3080)),
        regla(None, Protocolo::Ambos, (8080, 8080)),
        regla(None, Protocolo::Ambos, (3070, 3074)),
        // Editar la 1 con los puertos de la 0.
        regla(Some("1"), Protocolo::Tcp, (8080, 8080)),
    ] {
        assert_eq!(validar_regla(&r, &e, LAN), Err(ErrorRegla::Solapa), "{r:?}");
    }
    // Editar la 0 sin cambiar sus puertos no se solapa consigo misma.
    let r = regla(Some("0"), Protocolo::Tcp, (8080, 8080));
    assert_eq!(validar_regla(&r, &e, LAN), Ok(()));
}

#[test]
fn fuera_de_la_red_regla() {
    for otra in ["192.0.2.10", "192.168.1.1", "192.168.1.0"] {
        let mut r = regla(None, Protocolo::Tcp, (9000, 9000));
        r.ip = ip(otra);
        assert_eq!(
            validar_regla(&r, &[], LAN),
            Err(ErrorRegla::FueraDeLaRed),
            "{otra}"
        );
    }
    // La última dirección es de toda la red (en una red de documentación).
    let doc = (Ipv4Addr::new(192, 0, 2, 1), Ipv4Addr::new(255, 255, 255, 0));
    let mut r = regla(None, Protocolo::Tcp, (9000, 9000));
    r.ip = ip("192.0.2.255");
    assert_eq!(validar_regla(&r, &[], doc), Err(ErrorRegla::FueraDeLaRed));
    r.ip = ip("192.0.2.254");
    assert_eq!(validar_regla(&r, &[], doc), Ok(()));
}
