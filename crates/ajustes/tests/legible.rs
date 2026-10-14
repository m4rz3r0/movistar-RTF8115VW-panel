//! Validación de un valor legible sin el catálogo (la que usa también la
//! página en el navegador): las mismas reglas y límites que `a_router`.

use ajustes::{ErrorValor, Limites, TipoCampo, validar_legible};

fn validar(tipo: TipoCampo, l: Limites, v: &str) -> Result<(), ErrorValor> {
    validar_legible(tipo, l, |k| ["wpa2", "abierta"].contains(&k), v)
}

const SIN: Limites = Limites {
    min: None,
    max: None,
    ascii: false,
    opcional: false,
};

#[test]
fn textos_en_bytes_y_ascii() {
    let l = Limites {
        min: Some(8),
        max: Some(10),
        ascii: true,
        ..SIN
    };
    assert_eq!(
        validar(TipoCampo::Secreto, l, "1234567"),
        Err(ErrorValor::Corto { min: 8 })
    );
    assert_eq!(validar(TipoCampo::Secreto, l, "12345678"), Ok(()));
    assert_eq!(
        validar(TipoCampo::Secreto, l, "12345678901"),
        Err(ErrorValor::Largo { max: 10 })
    );
    assert_eq!(
        validar(TipoCampo::Secreto, l, "contraseña"),
        Err(ErrorValor::Ascii)
    );
    // Sin `ascii`, la «ñ» ocupa dos bytes: 10 letras son 11 bytes.
    let l = Limites { ascii: false, ..l };
    assert_eq!(
        validar(TipoCampo::Texto, l, "contraseña"),
        Err(ErrorValor::Largo { max: 10 })
    );
    assert_eq!(
        validar(TipoCampo::Texto, l, "línea\nrota"),
        Err(ErrorValor::Control)
    );
}

#[test]
fn vacio_solo_si_es_opcional() {
    assert_eq!(validar(TipoCampo::Texto, SIN, ""), Err(ErrorValor::Vacio));
    assert_eq!(validar(TipoCampo::Ip, SIN, "  "), Err(ErrorValor::Vacio));
    let opcional = Limites {
        opcional: true,
        ..SIN
    };
    assert_eq!(validar(TipoCampo::Texto, opcional, ""), Ok(()));
    assert_eq!(validar(TipoCampo::Ip, opcional, ""), Ok(()));
    // Una opción o un sí/no nunca quedan vacíos.
    assert_eq!(
        validar(TipoCampo::Opcion, opcional, ""),
        Err(ErrorValor::Vacio)
    );
}

#[test]
fn numeros_ip_mac_opciones_y_si_no() {
    let l = Limites {
        min: Some(1),
        max: Some(64),
        ..SIN
    };
    assert_eq!(validar(TipoCampo::Numero, l, "64"), Ok(()));
    let rango = Err(ErrorValor::FueraDeRango {
        min: Some(1),
        max: Some(64),
    });
    assert_eq!(validar(TipoCampo::Numero, l, "65"), rango);
    assert_eq!(validar(TipoCampo::Numero, l, "1.5"), rango);
    assert_eq!(
        validar(TipoCampo::Ip, SIN, "192.0.2.300"),
        Err(ErrorValor::Ip)
    );
    assert_eq!(validar(TipoCampo::Ip, SIN, " 192.0.2.30 "), Ok(()));
    assert_eq!(validar(TipoCampo::Mac, SIN, "02-00-00-00-00-0A"), Ok(()));
    assert_eq!(
        validar(TipoCampo::Mac, SIN, "02:00:00:00:00"),
        Err(ErrorValor::Mac)
    );
    assert_eq!(validar(TipoCampo::Opcion, SIN, "wpa2"), Ok(()));
    assert_eq!(
        validar(TipoCampo::Opcion, SIN, "wep"),
        Err(ErrorValor::Opcion)
    );
    assert_eq!(validar(TipoCampo::Booleano, SIN, "true"), Ok(()));
    assert_eq!(
        validar(TipoCampo::Booleano, SIN, "sí"),
        Err(ErrorValor::Opcion)
    );
}

#[test]
fn argumentos_de_cada_error() {
    assert_eq!(ErrorValor::Corto { min: 8 }.limites(), (Some(8), None));
    assert_eq!(ErrorValor::Largo { max: 63 }.limites(), (None, Some(63)));
    let rango = ErrorValor::FueraDeRango {
        min: None,
        max: Some(9),
    };
    assert_eq!(rango.limites(), (None, Some(9)));
    assert_eq!(ErrorValor::Ip.limites(), (None, None));
}
