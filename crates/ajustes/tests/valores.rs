//! Traducción de valores router ↔ legibles y su validación.

use std::collections::BTreeMap;

use ajustes::{
    Campo, CatalogoAjustes, ErrorValor, ValorLegible, a_legible, a_router, falta_requerido,
};

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

[[ajuste.campo]]
id = "frase"
escribe = { funcion = "cbSetX", parametro = "key_phrase" }
lee = "key_phrase"
tipo = "secreto"
validar = { min = 8, max = 63, ascii = true }

[[ajuste.campo]]
id = "cifrado"
escribe = { funcion = "cbSetX", parametro = "authentication" }
lee = "authentication"
tipo = "opcion"
ademas = [{ parametro = "encryption", lee = "encryption" }]
opciones = [
  { valor = "3", clave = "wpa2", recomendada = true, ademas = { encryption = "4" } },
  { valor = "3", clave = "wpa2_tkip", ademas = { encryption = "2" } },
  { valor = "0", clave = "abierta", ademas = { encryption = "0" } },
]

[[ajuste.campo]]
id = "dns2"
escribe = { funcion = "cbSetX", parametro = "dns2" }
lee = "dns2"
tipo = "ip"
opcional = true

[[ajuste.campo]]
id = "estado"
escribe = { funcion = "cbSetX", parametro = "estado" }
lee = "estado"
tipo = "texto"
solo_lectura = "ajustes-solo-lectura-estado"

[[ajuste.campo]]
id = "version"
lee = "version"
tipo = "texto"
solo_lectura = "ajustes-solo-lectura-estado"
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

/// Lectura con los elementos `pares`.
fn leidos(pares: &[(&str, &str)]) -> BTreeMap<String, String> {
    pares
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// Valor legible de un campo cuya lectura solo trae su propio elemento.
fn legible(c: &Campo, crudo: &str) -> ValorLegible {
    a_legible(c, &leidos(&[(&c.lee, crudo)]))
}

/// Lo que se envía al router para un campo de un solo parámetro.
fn al_router(c: &Campo, texto: &str) -> Result<String, ErrorValor> {
    let pares = a_router(c, texto)?;
    let parametro = &c.escribe.as_ref().unwrap().parametro;
    assert_eq!(pares.len(), 1, "{pares:?}");
    assert_eq!(&pares[0].0, parametro);
    Ok(pares[0].1.clone())
}

/// `crudo` → legible → router da otra vez `crudo`.
fn ida_y_vuelta(id: &str, crudo: &str, valor: ValorLegible, texto: &str) {
    let c = campo(id);
    assert_eq!(legible(&c, crudo), valor, "{id}: {crudo}");
    assert_eq!(al_router(&c, texto).as_deref(), Ok(crudo), "{id}: {texto}");
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
        al_router(&mac, "02-00-00-00-00-0A").as_deref(),
        Ok("02:00:00:00:00:0a")
    );
    assert_eq!(
        al_router(&mac, "02:00:00:00:00:0a").as_deref(),
        Ok("02:00:00:00:00:0a")
    );
    assert_eq!(
        legible(&mac, "02:00:00:00:00:0a"),
        ValorLegible::Texto("02:00:00:00:00:0a".into())
    );

    // Los secretos se escriben tal cual, pero nunca se leen.
    let clave = campo("clave");
    assert_eq!(legible(&clave, "clave-de-prueba"), ValorLegible::Secreto);
    assert_eq!(legible(&clave, ""), ValorLegible::Secreto);
    assert_eq!(
        al_router(&clave, "clave-de-prueba").as_deref(),
        Ok("clave-de-prueba")
    );
}

#[test]
fn opcion_desconocida_es_desconocido() {
    assert_eq!(
        legible(&campo("seguridad"), "99"),
        ValorLegible::Desconocido("99".into())
    );
    assert_eq!(
        legible(&campo("activa"), "2"),
        ValorLegible::Desconocido("2".into())
    );
    assert_eq!(
        legible(&campo("maximo"), "muchos"),
        ValorLegible::Desconocido("muchos".into())
    );
    assert_eq!(
        al_router(&campo("seguridad"), "wpa9"),
        Err(ErrorValor::Opcion)
    );
    assert_eq!(
        al_router(&campo("activa"), "quizá"),
        Err(ErrorValor::Opcion)
    );
}

#[test]
fn limites_de_texto_en_bytes() {
    let nombre = campo("nombre");
    let justo = "ñ".repeat(16);
    assert_eq!(justo.len(), 32);
    assert_eq!(al_router(&nombre, &justo), Ok(justo.clone()));
    assert_eq!(
        al_router(&nombre, &"ñ".repeat(17)),
        Err(ErrorValor::Largo { max: 32 })
    );
    assert_eq!(al_router(&nombre, ""), Err(ErrorValor::Vacio));
    let clave = campo("clave");
    assert_eq!(
        al_router(&clave, "corta"),
        Err(ErrorValor::Corto { min: 8 })
    );
    // 4 caracteres, 8 bytes.
    assert!(al_router(&clave, "ññññ").is_ok());
}

#[test]
fn numeros_ip_y_mac_validados() {
    let maximo = campo("maximo");
    let rango = ErrorValor::FueraDeRango {
        min: Some(1),
        max: Some(64),
    };
    assert_eq!(al_router(&maximo, "65"), Err(rango.clone()));
    assert_eq!(al_router(&maximo, "0"), Err(rango.clone()));
    assert_eq!(al_router(&maximo, "diez"), Err(rango));
    assert_eq!(al_router(&maximo, " 7 ").as_deref(), Ok("7"));
    assert_eq!(al_router(&maximo, ""), Err(ErrorValor::Vacio));

    let puerta = campo("puerta");
    for mala in ["192.0.2", "192.0.2.256", "192.0.2.1.5", "::1", "192.0.2.x"] {
        assert_eq!(al_router(&puerta, mala), Err(ErrorValor::Ip), "{mala}");
    }
    let mac = campo("mac");
    for mala in [
        "02:00:00:00:00",
        "02:00:00-00:00:0a",
        "02:00:00:00:00:0g",
        "0200000000 0a",
    ] {
        assert_eq!(al_router(&mac, mala), Err(ErrorValor::Mac), "{mala}");
    }
}

#[test]
fn caracteres_de_control_rechazados() {
    for id in ["nombre", "clave", "seguridad", "maximo", "puerta", "mac"] {
        assert_eq!(
            al_router(&campo(id), "a\u{7}b"),
            Err(ErrorValor::Control),
            "{id}"
        );
    }
    assert_eq!(
        al_router(&campo("nombre"), "a\nb"),
        Err(ErrorValor::Control)
    );
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
    let rango = |min, max| ErrorValor::FueraDeRango { min, max }.clave_i18n();
    assert_eq!(rango(Some(1), Some(9)), "ajustes-error-fuera-de-rango");
    assert_eq!(rango(Some(1), None), "ajustes-error-fuera-de-rango-desde");
    assert_eq!(rango(None, Some(9)), "ajustes-error-fuera-de-rango-hasta");
    assert_eq!(rango(None, None), "ajustes-error-numero");
    assert_eq!(ErrorValor::Ip.clave_i18n(), "ajustes-error-ip");
    assert_eq!(ErrorValor::Mac.clave_i18n(), "ajustes-error-mac");
    assert_eq!(ErrorValor::Opcion.clave_i18n(), "ajustes-error-opcion");
    assert_eq!(ErrorValor::Control.clave_i18n(), "ajustes-error-control");
    assert_eq!(ErrorValor::Ascii.clave_i18n(), "ajustes-error-ascii");
}

#[test]
fn solo_ascii_imprimible_si_el_campo_lo_pide() {
    let frase = campo("frase");
    // Del espacio a la tilde (32–126), todo vale.
    let todo: String = (b' '..=b'~').map(char::from).take(63).collect();
    assert_eq!(al_router(&frase, &todo), Ok(todo.clone()));
    assert_eq!(
        al_router(&frase, "una frase larga"),
        Ok("una frase larga".into())
    );
    for mala in ["contraseña", "café con leche", "clave😀segura"] {
        assert_eq!(al_router(&frase, mala), Err(ErrorValor::Ascii), "{mala}");
    }
    // Los límites se siguen aplicando, y el control se rechaza antes.
    assert_eq!(
        al_router(&frase, "corta"),
        Err(ErrorValor::Corto { min: 8 })
    );
    assert_eq!(al_router(&frase, "abc\tdefgh"), Err(ErrorValor::Control));
    // Sin `ascii`, la ñ sigue valiendo.
    assert!(al_router(&campo("clave"), "contraseña").is_ok());
}

#[test]
fn lee_el_elemento_sin_distinguir_mayusculas() {
    let c = campo("nombre");
    assert_eq!(
        a_legible(&c, &leidos(&[("Nombre", "Casa")])),
        ValorLegible::Texto("Casa".into())
    );
    // Sin el elemento en la lectura, desconocido.
    assert_eq!(
        a_legible(&campo("seguridad"), &leidos(&[])),
        ValorLegible::Desconocido(String::new())
    );
}

#[test]
fn opcion_que_escribe_varios_parametros() {
    let c = campo("cifrado");
    // Se reconoce solo si coinciden los dos elementos leídos.
    assert_eq!(
        a_legible(&c, &leidos(&[("authentication", "3"), ("encryption", "4")])),
        ValorLegible::Opcion("wpa2".into())
    );
    assert_eq!(
        a_legible(&c, &leidos(&[("authentication", "3"), ("encryption", "2")])),
        ValorLegible::Opcion("wpa2_tkip".into())
    );
    assert_eq!(
        a_legible(&c, &leidos(&[("authentication", "3"), ("encryption", "9")])),
        ValorLegible::Desconocido("3".into())
    );
    assert_eq!(
        a_legible(&c, &leidos(&[("authentication", "3")])),
        ValorLegible::Desconocido("3".into())
    );
    // Y se escriben los dos.
    let par = |p: &str, v: &str| (p.to_owned(), v.to_owned());
    assert_eq!(
        a_router(&c, "wpa2_tkip"),
        Ok(vec![par("authentication", "3"), par("encryption", "2")])
    );
    assert_eq!(
        a_router(&c, "abierta"),
        Ok(vec![par("authentication", "0"), par("encryption", "0")])
    );
    assert_eq!(a_router(&c, "wep"), Err(ErrorValor::Opcion));
}

#[test]
fn vacio_solo_si_es_opcional() {
    let dns2 = campo("dns2");
    assert_eq!(al_router(&dns2, "").as_deref(), Ok(""));
    assert_eq!(al_router(&dns2, "  ").as_deref(), Ok(""));
    assert_eq!(legible(&dns2, ""), ValorLegible::Texto(String::new()));
    assert_eq!(al_router(&dns2, "192.0.2.53").as_deref(), Ok("192.0.2.53"));
    // Sin `opcional`, vacío es un error en cualquier tipo.
    for id in ["puerta", "mac", "maximo", "seguridad", "activa"] {
        assert_eq!(al_router(&campo(id), ""), Err(ErrorValor::Vacio), "{id}");
    }
    // También en un texto sin mínimo.
    let mut sin_minimo = campo("nombre");
    sin_minimo.validar = None;
    assert_eq!(al_router(&sin_minimo, ""), Err(ErrorValor::Vacio));
    sin_minimo.opcional = true;
    assert_eq!(al_router(&sin_minimo, "").as_deref(), Ok(""));
}

#[test]
fn a_legible_normaliza_mac_e_ip() {
    let mac = campo("mac");
    for crudo in [
        "02:00:00:00:00:0A",
        "02-00-00-00-00-0a",
        " 02:00:00:00:00:0a ",
    ] {
        assert_eq!(
            legible(&mac, crudo),
            ValorLegible::Texto("02:00:00:00:00:0a".into()),
            "{crudo}"
        );
    }
    let puerta = campo("puerta");
    assert_eq!(
        legible(&puerta, " 192.0.2.1 "),
        ValorLegible::Texto("192.0.2.1".into())
    );
    // Lo que no se interpreta se muestra tal cual.
    assert_eq!(
        legible(&mac, "no-es-mac"),
        ValorLegible::Texto("no-es-mac".into())
    );
    assert_eq!(
        legible(&puerta, "192.0.2"),
        ValorLegible::Texto("192.0.2".into())
    );
}

#[test]
fn solo_lectura_no_se_escribe() {
    for id in ["estado", "version"] {
        let c = campo(id);
        assert_eq!(a_router(&c, "x"), Err(ErrorValor::SoloLectura), "{id}");
    }
    assert_eq!(
        legible(&campo("version"), "1.0"),
        ValorLegible::Texto("1.0".into())
    );
    assert_eq!(
        ErrorValor::SoloLectura.clave_i18n(),
        "ajustes-error-solo-lectura"
    );
}

#[test]
fn mensaje_de_fuera_de_rango_legible() {
    let e = |min, max| ErrorValor::FueraDeRango { min, max }.to_string();
    assert_eq!(e(Some(1), Some(64)), "número fuera de rango: de 1 a 64");
    assert_eq!(e(Some(1), None), "número fuera de rango: al menos 1");
    assert_eq!(e(None, Some(64)), "número fuera de rango: como mucho 64");
    assert_eq!(e(None, None), "no es un número");
}

/// Cada error tiene su texto en los dos idiomas, y los de longitud y rango
/// dicen el límite (con sus valores) sin hablar de bytes.
#[test]
fn textos_de_error_con_sus_limites() {
    use i18n::{FluentArgs, Idioma, existe, tr};
    let errores = [
        ErrorValor::Vacio,
        ErrorValor::Corto { min: 8 },
        ErrorValor::Largo { max: 32 },
        ErrorValor::FueraDeRango {
            min: Some(1),
            max: Some(65535),
        },
        ErrorValor::FueraDeRango {
            min: Some(1),
            max: None,
        },
        ErrorValor::FueraDeRango {
            min: None,
            max: Some(65535),
        },
        ErrorValor::FueraDeRango {
            min: None,
            max: None,
        },
        ErrorValor::Ip,
        ErrorValor::Mac,
        ErrorValor::Opcion,
        ErrorValor::Control,
        ErrorValor::Ascii,
        ErrorValor::SoloLectura,
    ];
    for idioma in Idioma::TODOS {
        for e in &errores {
            let clave = e.clave_i18n();
            assert!(existe(idioma, clave), "{idioma:?}: falta {clave}");
            let mut args = FluentArgs::new();
            let (min, max) = match e {
                ErrorValor::Corto { min } => (Some(*min), None),
                ErrorValor::Largo { max } => (None, Some(*max)),
                ErrorValor::FueraDeRango { min, max } => (*min, *max),
                _ => (None, None),
            };
            if let Some(min) = min {
                args.set("min", min);
            }
            if let Some(max) = max {
                args.set("max", max);
            }
            let texto = tr(idioma, clave, Some(&args));
            for limite in min.iter().chain(max.iter()) {
                assert!(
                    texto.contains(&limite.to_string()),
                    "{idioma:?} {clave}: «{texto}» no dice {limite}"
                );
            }
            assert!(!texto.contains('{'), "{idioma:?} {clave}: «{texto}»");
            assert!(
                !texto.to_lowercase().contains("byte"),
                "{idioma:?} {clave}: «{texto}»"
            );
            if matches!(e, ErrorValor::Corto { .. } | ErrorValor::Largo { .. }) {
                // Explica cuánto ocupan las letras con tilde y los emoji.
                assert!(texto.contains("emoji"), "{idioma:?} {clave}: «{texto}»");
            }
        }
    }
}

/// `falta_requerido`: al poner el campo que lo pide en su valor, el campo
/// exigido tiene que venir y no vacío; con otro valor, o sin tocar el que lo
/// pide, no se exige nada.
#[test]
fn falta_requerido_segun_el_valor_pedido() {
    let toml = r#"
[[ajuste]]
id = "sistema.prueba"
tema = "sistema"
nivel = "oculto"
leer = { funcion = "cbGetX" }

[[ajuste.campo]]
id = "espejo"
escribe = { funcion = "cbSetX", parametro = "espejo" }
lee = "espejo"
tipo = "booleano"
requiere = { campo = "puerto", cuando = "true" }

[[ajuste.campo]]
id = "puerto"
escribe = { funcion = "cbSetX", parametro = "puerto" }
lee = "puerto"
tipo = "opcion"
opciones = [{ valor = "1", clave = "puerto_1" }, { valor = "2", clave = "puerto_2" }]
"#;
    let c = CatalogoAjustes::from_toml(&[("sistema", toml)], "").unwrap();
    let a = c.ajuste("sistema.prueba").unwrap();
    let pedir = |pares: &[(&str, &str)]| {
        let v: BTreeMap<String, String> = pares
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        falta_requerido(a, &v).map(|(pide, falta)| (pide.id.clone(), falta.id.clone()))
    };
    assert_eq!(
        pedir(&[("espejo", "true")]),
        Some(("espejo".into(), "puerto".into()))
    );
    assert_eq!(
        pedir(&[("espejo", "true"), ("puerto", " ")]),
        Some(("espejo".into(), "puerto".into()))
    );
    assert_eq!(pedir(&[("espejo", "true"), ("puerto", "puerto_2")]), None);
    assert_eq!(pedir(&[("espejo", "false")]), None);
    assert_eq!(pedir(&[("puerto", "puerto_1")]), None);
}
