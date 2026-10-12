//! Traducción de valores router ↔ legibles y su validación.

use ajustes::{Campo, CatalogoAjustes, ErrorValor, ValorLegible, a_legible, a_router};

const CAMPOS: &str = r#"
[[ajuste]]
id = "wifi.prueba"
tema = "wifi"
nivel = "basico"
leer = { funcion = "cbGetX" }

[[ajuste.campo]]
id = "nombre"
escribe = { funcion = "cbSetX", parametro = "nombre" }
lee = "nombre"
tipo = "texto"
validar = { min = 1, max = 32 }

[[ajuste.campo]]
id = "seguridad"
escribe = { funcion = "cbSetX", parametro = "seguridad" }
lee = "seguridad"
tipo = "opcion"
opciones = [
  { valor = "11", clave = "wpa2_wpa3", recomendada = true },
  { valor = "0", clave = "abierta", aviso = "inseguro" },
]

[[ajuste.campo]]
id = "activa"
escribe = { funcion = "cbSetX", parametro = "activa" }
lee = "activa"
tipo = "booleano"
booleano = { si = "enable", no = "disable" }

[[ajuste.campo]]
id = "visible"
escribe = { funcion = "cbSetX", parametro = "oculta" }
lee = "oculta"
tipo = "booleano"
invertido = true

[[ajuste.campo]]
id = "maximo"
escribe = { funcion = "cbSetX", parametro = "max_clientes" }
lee = "max_clientes"
tipo = "numero"
validar = { min = 1, max = 64 }

[[ajuste.campo]]
id = "puerta"
escribe = { funcion = "cbSetX", parametro = "puerta" }
lee = "puerta"
tipo = "ip"

[[ajuste.campo]]
id = "mac"
escribe = { funcion = "cbSetX", parametro = "mac" }
lee = "mac"
tipo = "mac"

[[ajuste.campo]]
id = "clave"
escribe = { funcion = "cbSetX", parametro = "key_phrase" }
lee = "key_phrase"
tipo = "secreto"
validar = { min = 8, max = 63 }
"#;

fn campo(id: &str) -> Campo {
    let c = CatalogoAjustes::from_toml(&[("wifi", CAMPOS)], "").expect("campos legibles");
    c.ajuste("wifi.prueba")
        .unwrap()
        .campos
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("falta el campo {id}"))
        .clone()
}

/// `crudo` → legible → router da otra vez `crudo`.
fn ida_y_vuelta(id: &str, crudo: &str, legible: ValorLegible, texto: &str) {
    let c = campo(id);
    assert_eq!(a_legible(&c, crudo), legible, "{id}: {crudo}");
    assert_eq!(a_router(&c, texto).as_deref(), Ok(crudo), "{id}: {texto}");
}

#[test]
fn a_legible_y_a_router_ida_y_vuelta() {
    ida_y_vuelta(
        "seguridad",
        "11",
        ValorLegible::Opcion("wpa2_wpa3".into()),
        "wpa2_wpa3",
    );
    ida_y_vuelta(
        "seguridad",
        "0",
        ValorLegible::Opcion("abierta".into()),
        "abierta",
    );
    ida_y_vuelta("activa", "enable", ValorLegible::Booleano(true), "true");
    ida_y_vuelta("activa", "disable", ValorLegible::Booleano(false), "false");
    // «Visible» sobre `oculta`: invertido.
    ida_y_vuelta("visible", "1", ValorLegible::Booleano(false), "false");
    ida_y_vuelta("visible", "0", ValorLegible::Booleano(true), "true");
    ida_y_vuelta("maximo", "32", ValorLegible::Texto("32".into()), "32");
    ida_y_vuelta(
        "puerta",
        "192.0.2.1",
        ValorLegible::Texto("192.0.2.1".into()),
        "192.0.2.1",
    );
    ida_y_vuelta(
        "nombre",
        "Casa ñ",
        ValorLegible::Texto("Casa ñ".into()),
        "Casa ñ",
    );

    let mac = campo("mac");
    assert_eq!(
        a_router(&mac, "02-00-00-00-00-0A").as_deref(),
        Ok("02:00:00:00:00:0a")
    );
    assert_eq!(
        a_router(&mac, "02:00:00:00:00:0a").as_deref(),
        Ok("02:00:00:00:00:0a")
    );
    assert_eq!(
        a_legible(&mac, "02:00:00:00:00:0a"),
        ValorLegible::Texto("02:00:00:00:00:0a".into())
    );

    // Los secretos se escriben tal cual, pero nunca se leen.
    let clave = campo("clave");
    assert_eq!(a_legible(&clave, "clave-de-prueba"), ValorLegible::Secreto);
    assert_eq!(a_legible(&clave, ""), ValorLegible::Secreto);
    assert_eq!(
        a_router(&clave, "clave-de-prueba").as_deref(),
        Ok("clave-de-prueba")
    );
}

#[test]
fn opcion_desconocida_es_desconocido() {
    assert_eq!(
        a_legible(&campo("seguridad"), "99"),
        ValorLegible::Desconocido("99".into())
    );
    assert_eq!(
        a_legible(&campo("activa"), "2"),
        ValorLegible::Desconocido("2".into())
    );
    assert_eq!(
        a_legible(&campo("maximo"), "muchos"),
        ValorLegible::Desconocido("muchos".into())
    );
    assert_eq!(
        a_router(&campo("seguridad"), "wpa9"),
        Err(ErrorValor::Opcion)
    );
    assert_eq!(a_router(&campo("activa"), "quizá"), Err(ErrorValor::Opcion));
}

#[test]
fn limites_de_texto_en_bytes() {
    let nombre = campo("nombre");
    let justo = "ñ".repeat(16);
    assert_eq!(justo.len(), 32);
    assert_eq!(a_router(&nombre, &justo), Ok(justo.clone()));
    assert_eq!(
        a_router(&nombre, &"ñ".repeat(17)),
        Err(ErrorValor::Largo { max: 32 })
    );
    assert_eq!(a_router(&nombre, ""), Err(ErrorValor::Vacio));
    let clave = campo("clave");
    assert_eq!(a_router(&clave, "corta"), Err(ErrorValor::Corto { min: 8 }));
    // 4 caracteres, 8 bytes.
    assert!(a_router(&clave, "ññññ").is_ok());
}

#[test]
fn numeros_ip_y_mac_validados() {
    let maximo = campo("maximo");
    let rango = ErrorValor::FueraDeRango {
        min: Some(1),
        max: Some(64),
    };
    assert_eq!(a_router(&maximo, "65"), Err(rango.clone()));
    assert_eq!(a_router(&maximo, "0"), Err(rango.clone()));
    assert_eq!(a_router(&maximo, "diez"), Err(rango));
    assert_eq!(a_router(&maximo, " 7 ").as_deref(), Ok("7"));
    assert_eq!(a_router(&maximo, ""), Err(ErrorValor::Vacio));

    let puerta = campo("puerta");
    for mala in ["192.0.2", "192.0.2.256", "192.0.2.1.5", "::1", "192.0.2.x"] {
        assert_eq!(a_router(&puerta, mala), Err(ErrorValor::Ip), "{mala}");
    }
    let mac = campo("mac");
    for mala in [
        "02:00:00:00:00",
        "02:00:00-00:00:0a",
        "02:00:00:00:00:0g",
        "0200000000 0a",
    ] {
        assert_eq!(a_router(&mac, mala), Err(ErrorValor::Mac), "{mala}");
    }
}

#[test]
fn caracteres_de_control_rechazados() {
    for id in ["nombre", "clave", "seguridad", "maximo", "puerta", "mac"] {
        assert_eq!(
            a_router(&campo(id), "a\u{7}b"),
            Err(ErrorValor::Control),
            "{id}"
        );
    }
    assert_eq!(a_router(&campo("nombre"), "a\nb"), Err(ErrorValor::Control));
}

#[test]
fn claves_de_error() {
    assert_eq!(ErrorValor::Vacio.clave_i18n(), "ajustes-error-vacio");
    assert_eq!(
        ErrorValor::Corto { min: 1 }.clave_i18n(),
        "ajustes-error-corto"
    );
    assert_eq!(
        ErrorValor::Largo { max: 1 }.clave_i18n(),
        "ajustes-error-largo"
    );
    assert_eq!(
        ErrorValor::FueraDeRango {
            min: None,
            max: None
        }
        .clave_i18n(),
        "ajustes-error-fuera-de-rango"
    );
    assert_eq!(ErrorValor::Ip.clave_i18n(), "ajustes-error-ip");
    assert_eq!(ErrorValor::Mac.clave_i18n(), "ajustes-error-mac");
    assert_eq!(ErrorValor::Opcion.clave_i18n(), "ajustes-error-opcion");
    assert_eq!(ErrorValor::Control.clave_i18n(), "ajustes-error-control");
}
