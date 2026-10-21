//! Pruebas internas de la planificación de escrituras.

use super::*;
use catalogo::Catalogo;

fn cat() -> Catalogo {
    Catalogo::load().unwrap()
}

fn mapa(pares: &[(&str, &str)]) -> BTreeMap<String, String> {
    pares
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn carga_con_la_instancia_completa_y_secretos_sin_tocar() {
    let c = cat();
    let m = Mascara::load().unwrap();
    let f = c.funcion("cbSetWifiAccPt").unwrap();
    let actual = mapa(&[
        ("dpm_cfg_wifi_acc_pt_id", "1"),
        ("ssid", "Casa"),
        ("key_phrase", "clave-actual"),
        ("WMM", "1"),
        ("bssid", "02:00:00:00:00:01"),
    ]);
    let valores = mapa(&[
        ("dpm_cfg_wifi_acc_pt_id", "1"),
        ("ssid", "Otra"),
        ("key_phrase", MASCARA),
        ("wep_key1_data", ""),
    ]);
    let Compuesto {
        efectivos,
        payload,
        campos,
        secretos_tapados,
    } = componer(f, &m, Operacion::Editar, &actual, &valores, false);
    assert!(secretos_tapados.is_empty());
    let p: BTreeMap<_, _> = payload.into_iter().collect();
    assert_eq!(p["ssid"], "Otra");
    assert_eq!(p["key_phrase"], "clave-actual", "secreto sin tocar");
    // Sin distinguir mayúsculas cuando no hay coincidencia exacta.
    assert_eq!(p["wmm"], "1");
    assert_eq!(p["dpm_cfg_wifi_acc_pt_id"], "1");
    // Ni lo que no es parámetro ni lo que no tiene valor.
    assert!(!p.contains_key("bssid"));
    assert!(!p.contains_key("wep_key1_data"));
    assert!(!p.contains_key("validation_only"));
    assert!(p.values().all(|v| v != MASCARA));
    assert_eq!(efectivos["key_phrase"], "clave-actual");
    assert_eq!(campos.len(), 1);
    assert_eq!(campos[0].nombre, "ssid");

    // Un campo que no es secreto pero llegó enmascarado (p. ej. por la
    // regla de sufijo numérico) tampoco se escribe enmascarado.
    let actual = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("wep_key_idx", "2")]);
    let valores = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("wep_key_idx", MASCARA)]);
    let r = componer(f, &m, Operacion::Editar, &actual, &valores, false);
    assert!(r.campos.is_empty());
    assert!(r.payload.contains(&("wep_key_idx".into(), "2".into())));
    let r = componer(f, &m, Operacion::Editar, &mapa(&[]), &valores, false);
    assert!(r.payload.iter().all(|(_, v)| v != MASCARA));

    // Explícito (reversión): un secreto vacío es el valor a escribir.
    let actual = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "nueva")]);
    let valores = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "")]);
    let r = componer(f, &m, Operacion::Editar, &actual, &valores, true);
    assert!(r.payload.contains(&("key_phrase".into(), String::new())));
    assert_eq!(r.campos.len(), 1);
    let r = componer(f, &m, Operacion::Editar, &actual, &valores, false);
    assert!(r.campos.is_empty(), "sin explícito, vacío = sin tocar");
}

/// Un secreto que el catálogo marca como tapado al leer (la contraseña
/// de la línea de voz) no se reenvía desde la lectura ni vacío ni
/// enmascarado: `componer` lo señala y el gestor no escribe. Una clave
/// nueva en la petición sí vale.
#[test]
fn secreto_marcado_leido_vacio_o_tapado_se_senala() {
    let c = cat();
    let m = Mascara::load().unwrap();
    let f = c.funcion("cbSetVoiceLine").unwrap();
    assert!(f.parametro("auth_password").unwrap().tapado);
    for tapada in ["", "  ", "********", MASCARA, "●●●●"] {
        let actual = mapa(&[
            ("interwork_id", "0"),
            ("adm_state", "1"),
            ("auth_username", "usuario-de-prueba"),
            ("auth_password", tapada),
        ]);
        let otro = mapa(&[("interwork_id", "0"), ("adm_state", "0")]);
        let r = componer(f, &m, Operacion::Editar, &actual, &otro, false);
        assert_eq!(r.secretos_tapados, ["auth_password"], "«{tapada}»");
        for sin_tocar in [MASCARA, ""] {
            let v = mapa(&[("interwork_id", "0"), ("auth_password", sin_tocar)]);
            let r = componer(f, &m, Operacion::Editar, &actual, &v, false);
            assert_eq!(r.secretos_tapados, ["auth_password"], "«{tapada}»");
        }
        let nueva = mapa(&[
            ("interwork_id", "0"),
            ("auth_password", "clave-de-prueba-1"),
        ]);
        let r = componer(f, &m, Operacion::Editar, &actual, &nueva, false);
        assert!(r.secretos_tapados.is_empty(), "«{tapada}»");
        assert!(
            r.payload
                .contains(&("auth_password".into(), "clave-de-prueba-1".into()))
        );
    }
}

/// Un secreto sin marca (una clave WEP sin usar) se reenvía tal como se
/// leyó, también vacío; solo una máscara no se reenvía nunca.
#[test]
fn secreto_sin_marca_se_reenvia_salvo_una_mascara() {
    let c = cat();
    let m = Mascara::load().unwrap();
    let f = c.funcion("cbSetWifiAccPt").unwrap();
    let con = |wep: &str| {
        mapa(&[
            ("dpm_cfg_wifi_acc_pt_id", "1"),
            ("ssid", "Casa"),
            ("key_phrase", "clave-de-prueba-1"),
            ("wep_key1_data", wep),
        ])
    };
    let valores = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("ssid", "Otra")]);
    let r = componer(f, &m, Operacion::Editar, &con(""), &valores, false);
    assert!(r.secretos_tapados.is_empty());
    assert!(r.payload.contains(&("wep_key1_data".into(), String::new())));
    for mascara in ["*****", MASCARA] {
        let r = componer(f, &m, Operacion::Editar, &con(mascara), &valores, false);
        assert_eq!(r.secretos_tapados, ["wep_key1_data"], "«{mascara}»");
    }
    assert!(enmascarado("****") && !enmascarado("") && !enmascarado("a***"));
}

/// Un secreto tapado al leer no se puede comprobar: tras escribirlo, el
/// router lo sigue dando vacío, y eso no es una diferencia.
#[test]
fn comprobar_no_mira_un_secreto_tapado() {
    let c = cat();
    let f = c.funcion("cbSetVoiceLine").unwrap();
    let campo = |n: &str, a: &str, d: &str| Campo {
        nombre: n.into(),
        antes: a.into(),
        despues: d.into(),
        secreto: true,
        leido: true,
    };
    let campos = [
        campo("auth_password", "", "clave-de-prueba-1"),
        Campo {
            secreto: false,
            ..campo("adm_state", "1", "0")
        },
    ];
    let actual = mapa(&[("auth_password", ""), ("adm_state", "0")]);
    assert_eq!(
        comparar(f, &campos, &actual, Lado::Nuevo),
        Comprobacion::Coincide(None)
    );
    // Solo ella: no hay nada que comprobar.
    assert_eq!(
        comparar(f, &campos[..1], &actual, Lado::Nuevo),
        Comprobacion::Limitada
    );
}

/// La reversión y el deshacer (`explicito`) no se bloquean nunca por un
/// secreto que no sirve para reenviarlo: lo omiten de la carga y del
/// diff, venga de la lectura de ahora o del valor anterior guardado.
#[test]
fn explicito_omite_lo_que_no_se_puede_reenviar() {
    let c = cat();
    let m = Mascara::load().unwrap();
    let f = c.funcion("cbSetVoiceLine").unwrap();
    let actual = mapa(&[
        ("interwork_id", "0"),
        ("adm_state", "0"),
        ("auth_password", ""),
    ]);
    // La inversa: el valor anterior (vacío, el de la lectura de entonces).
    let inversa = mapa(&[
        ("interwork_id", "0"),
        ("adm_state", "1"),
        ("auth_password", ""),
    ]);
    let r = componer(f, &m, Operacion::Editar, &actual, &inversa, true);
    assert!(r.secretos_tapados.is_empty());
    assert!(
        r.payload.iter().all(|(k, _)| k != "auth_password"),
        "{:?}",
        r.payload
    );
    assert!(r.campos.iter().all(|c| c.nombre != "auth_password"));
    assert!(r.payload.contains(&("adm_state".into(), "1".into())));
    // Sin tocarla y leída vacía, igual.
    let otro = mapa(&[("interwork_id", "0"), ("adm_state", "1")]);
    let r = componer(f, &m, Operacion::Editar, &actual, &otro, true);
    assert!(r.secretos_tapados.is_empty());
    assert!(r.payload.iter().all(|(k, _)| k != "auth_password"));
}

#[test]
fn secreto_cambiado_se_marca() {
    let c = cat();
    let m = Mascara::load().unwrap();
    let f = c.funcion("cbSetWifiAccPt").unwrap();
    let actual = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "vieja")]);
    let valores = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "nueva")]);
    let campos = componer(f, &m, Operacion::Editar, &actual, &valores, false).campos;
    assert_eq!(campos.len(), 1);
    assert!(campos[0].secreto && campos[0].leido);
    let d = visible(&campos);
    assert_eq!(
        (d.campos[0].antes.as_str(), d.campos[0].despues.as_str()),
        (MASCARA, MASCARA)
    );
    assert!(!resumen(&campos).contains("nueva"));
    assert!(!resumen(&campos).contains("vieja"));
}

#[test]
fn crear_y_borrar() {
    let c = cat();
    let m = Mascara::load().unwrap();
    let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
    let valores = mapa(&[
        ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
        ("rtm_dhcpv4_svr_pool_static_id", "-1"),
        ("mac_addr", " 02:00:00:00:00:AA "),
        ("ipv4_addr", "192.168.1.60"),
    ]);
    let Compuesto {
        payload, campos, ..
    } = componer(f, &m, Operacion::Crear, &BTreeMap::new(), &valores, false);
    let p: BTreeMap<_, _> = payload.into_iter().collect();
    assert_eq!(p["rtm_dhcpv4_svr_pool_static_id"], "-1");
    assert_eq!(p["mac_addr"], "02:00:00:00:00:AA", "IP/MAC sin espacios");
    let nombres: Vec<_> = campos.iter().map(|c| c.nombre.as_str()).collect();
    assert_eq!(nombres, ["ipv4_addr", "mac_addr"], "sin los ids");

    let actual = mapa(&[
        ("rtm_dhcpv4_svr_pool_static_id", "3"),
        ("adm_state", "1"),
        ("mac_addr", "02:00:00:00:00:aa"),
        ("ipv4_addr", "192.168.1.60"),
    ]);
    let valores = mapa(&[
        ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
        ("rtm_dhcpv4_svr_pool_static_id", "3"),
    ]);
    let Compuesto {
        payload, campos, ..
    } = componer(f, &m, Operacion::Borrar, &actual, &valores, false);
    assert_eq!(
        payload,
        vec![
            ("rtm_dhcpv4_svr_pool_static_id".to_owned(), "3".to_owned()),
            ("rtm_cfg_dhcpv4_svr_pool_id".to_owned(), "0".to_owned()),
            ("del".to_owned(), "DEL".to_owned()),
        ]
    );
    assert_eq!(campos.len(), 3);
    assert!(campos.iter().all(|c| c.despues.is_empty() && c.leido));
}

#[test]
fn inversa_de_cada_operacion() {
    let c = cat();
    let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap().clone();
    let campo = |n: &str, a: &str, d: &str, leido| Campo {
        nombre: n.into(),
        antes: a.into(),
        despues: d.into(),
        secreto: false,
        leido,
    };
    let mut plan = Plan {
        funcion: f,
        operacion: Operacion::Editar,
        instancia: Some("0".into()),
        valores: mapa(&[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("rtm_dhcpv4_svr_pool_static_id", "0"),
            ("ipv4_addr", "192.168.1.80"),
        ]),
        payload: Vec::new(),
        campos: vec![campo("ipv4_addr", "192.168.1.68", "192.168.1.80", true)],
        lectura: Lectura::Ninguna,
        ids_antes: None,
    };
    let r = inversa(&plan).unwrap();
    assert_eq!(r.instancia.as_deref(), Some("0"));
    assert!(!r.borrar);
    assert_eq!(
        r.valores,
        mapa(&[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("ipv4_addr", "192.168.1.68")
        ])
    );
    // Sin valores anteriores conocidos no hay inversa.
    plan.campos = vec![campo("ipv4_addr", "", "192.168.1.80", false)];
    assert_eq!(inversa(&plan), None);

    plan.operacion = Operacion::Crear;
    plan.instancia = None;
    assert_eq!(inversa(&plan), None, "sin el id de la instancia creada");
    plan.instancia = Some("5".into());
    let r = inversa(&plan).unwrap();
    assert!(r.borrar);
    assert_eq!(r.instancia.as_deref(), Some("5"));
    assert_eq!(r.valores, mapa(&[("rtm_cfg_dhcpv4_svr_pool_id", "0")]));

    plan.operacion = Operacion::Borrar;
    plan.campos = vec![campo("ipv4_addr", "192.168.1.60", "", true)];
    let r = inversa(&plan).unwrap();
    assert!(!r.borrar);
    assert_eq!(r.instancia, None);
    assert_eq!(r.valores["ipv4_addr"], "192.168.1.60");

    plan.operacion = Operacion::Accion;
    assert_eq!(inversa(&plan), None);
}

#[test]
fn lectura_pareada_sustituye_parametros() {
    let c = cat();
    let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
    let v = mapa(&[
        ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
        ("rtm_dhcpv4_svr_pool_static_id", "4"),
    ]);
    let l = lectura_de(f, &v, Some("4")).unwrap();
    assert_eq!(
        l,
        Lectura::Xml {
            funcion: "cbDhcpV4SvrPoolGetStatic".into(),
            params: vec![
                ("rtm_cfg_dhcpv4_svr_pool_id".into(), "0".into()),
                ("rtm_dhcpv4_svr_pool_static_id".into(), "4".into()),
            ],
            clave_id: Some("rtm_dhcpv4_svr_pool_static_id".into()),
        }
    );
    let falta = lectura_de(f, &mapa(&[]), Some("4"));
    assert!(matches!(falta, Err(CambioError::Invalida(_))), "{falta:?}");
    let wl = c.funcion("te_wifi.wl").unwrap();
    assert_eq!(
        lectura_de(wl, &mapa(&[]), None).unwrap(),
        Lectura::Pagina("te_wifi.asp".into())
    );
}

#[test]
fn validacion_de_valores() {
    let c = cat();
    let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
    let p = |n| f.parametro(n).unwrap();
    assert!(validar(p("ipv4_addr"), "192.168.1.65").is_ok());
    assert!(validar(p("ipv4_addr"), "192.168.1.256").is_err());
    assert!(validar(p("mac_addr"), "02:00:00:00:00:aB").is_ok());
    assert!(validar(p("mac_addr"), "02:00:00:00:00").is_err());
    assert!(validar(p("mac_addr"), "02:00:00:00:00:zz").is_err());
    assert!(validar(p("adm_state"), "1").is_ok());
    assert!(validar(p("adm_state"), "2").is_err());
    assert!(validar(p("rtm_cfg_dhcpv4_svr_pool_id"), "x").is_err());
    let w = c.funcion("cbSetWifiAccPt").unwrap();
    assert!(validar(w.parametro("ssid").unwrap(), "a\nb").is_err());
    let e = validar(w.parametro("key_phrase").unwrap(), "a\0b").unwrap_err();
    assert!(!e.contains("a\0b"), "el error no muestra el secreto");
}

#[test]
fn comparar_normaliza_y_enmascara() {
    let c = cat();
    let f = c.funcion("cbSetWifiAccPt").unwrap();
    let campos = vec![
        Campo {
            nombre: "key_phrase".into(),
            antes: "vieja".into(),
            despues: "nueva".into(),
            secreto: true,
            leido: true,
        },
        Campo {
            nombre: "ssid".into(),
            antes: "A".into(),
            despues: "B".into(),
            secreto: false,
            leido: true,
        },
    ];
    let actual = mapa(&[("key_phrase", "otra"), ("ssid", " B ")]);
    let Comprobacion::Difiere(v) = comparar(f, &campos, &actual, Lado::Nuevo) else {
        panic!()
    };
    assert_eq!(v.len(), 1);
    assert!(v[0].contains(MASCARA) && !v[0].contains("otra") && !v[0].contains("nueva"));
    assert_eq!(
        comparar(f, &campos, &mapa(&[("bssid", "x")]), Lado::Nuevo),
        Comprobacion::Limitada
    );
    let anterior = mapa(&[("key_phrase", "vieja"), ("SSID", "A")]);
    assert_eq!(
        comparar(f, &campos, &anterior, Lado::Anterior),
        Comprobacion::Coincide(None)
    );
    let s = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
    assert!(iguales(
        s.parametro("mac_addr"),
        "02:AA:00:00:00:00",
        " 02:aa:00:00:00:00"
    ));
    assert!(!iguales(None, "Casa", "casa"));
}

#[test]
fn debug_sin_valores_ni_secretos() {
    let c = cat();
    let f = c.funcion("cbSetWifiAccPt").unwrap().clone();
    let plan = Plan {
        funcion: f,
        operacion: Operacion::Editar,
        instancia: None,
        valores: mapa(&[
            ("dpm_cfg_wifi_acc_pt_id", "1"),
            ("key_phrase", "clave-en-claro"),
        ]),
        payload: vec![("key_phrase".into(), "clave-en-claro".into())],
        campos: vec![
            Campo {
                nombre: "key_phrase".into(),
                antes: "clave-de-antes".into(),
                despues: "clave-en-claro".into(),
                secreto: true,
                leido: true,
            },
            Campo {
                nombre: "ssid".into(),
                antes: "RED-1".into(),
                despues: "RED-2".into(),
                secreto: false,
                leido: true,
            },
        ],
        lectura: Lectura::Ninguna,
        ids_antes: None,
    };
    let d = format!("{plan:?}");
    assert!(
        !d.contains("clave-en-claro") && !d.contains("clave-de-antes"),
        "{d}"
    );
    assert!(d.contains("key_phrase") && d.contains("RED-2"), "{d}");
}
