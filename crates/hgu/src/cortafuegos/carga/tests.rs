//! Pruebas de las cargas del cortafuegos: las de la captura de la web y la forma de cada campo.

use super::*;

fn v(pares: &[(&str, &str)]) -> Valores {
    pares
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn texto(c: &[(&'static str, String)]) -> String {
    c.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// La regla ICMP de la captura (con direcciones y máscaras).
fn icmp_capturada() -> Valores {
    v(&[
        ("belong", "0"),
        ("version", "0"),
        ("enabled", "1"),
        ("protocol", "1"),
        ("specificAction", "0"),
        ("icmpType", "1"),
        ("srcaddr", "192.0.2.10"),
        ("srcmask", "255.255.255.0"),
        ("dstaddr", "192.0.2.20"),
        ("dstmask", "255.255.255.0"),
    ])
}

#[test]
fn alta_de_regla_como_la_web() {
    let r = normalizar_regla(&icmp_capturada(), "0").unwrap();
    assert_eq!(
        texto(&carga_regla(&r, None)),
        "escape=true&target=rule&belong=0&version=0&enabled=1&protocol=1&specificAction=0\
         &icmpType=1&srcaddr=192.0.2.10&srcmask=255.255.255.0&dstaddr=192.0.2.20\
         &dstmask=255.255.255.0"
    );
}

#[test]
fn edicion_de_regla_como_la_web() {
    let r = normalizar_regla(&icmp_capturada(), "0").unwrap();
    assert_eq!(
        texto(&carga_regla(&r, Some("16"))),
        "id=16&target=rule&belong=0&version=0&enabled=1&protocol=1&specificAction=0\
         &icmpType=1&srcaddr=192.0.2.10&srcmask=255.255.255.0&dstaddr=192.0.2.20\
         &dstmask=255.255.255.0"
    );
}

#[test]
fn alta_y_edicion_de_filtro_como_la_web() {
    let f = normalizar_filtro(&v(&[
        ("name", "test filter entr"),
        ("interface", "0.1"),
        ("type", "0"),
        ("version", "0"),
        ("default", "1"),
    ]))
    .unwrap();
    assert_eq!(
        texto(&carga_filtro(&f, None)),
        "target=entry&name=test filter entr&interface=0.1&type=0&version=0&default=1"
    );
    assert_eq!(
        texto(&carga_filtro(&f, Some("2"))),
        "id=2&target=entry&name=test filter entr&interface=0.1&type=0&version=0&default=1"
    );
}

#[test]
fn tcp_con_puertos_y_rechazo() {
    let r = normalizar_regla(
        &v(&[
            ("belong", "0"),
            ("enabled", "1"),
            ("protocol", "2"),
            ("specificAction", "1"),
            ("rejectType", "7"),
            ("dstportstart", "8080"),
        ]),
        "0",
    )
    .unwrap();
    // Sin versión, la del filtro; el puerto final, el inicial.
    assert_eq!(r["version"], "0");
    assert_eq!(r["dstportend"], "8080");
    assert_eq!(
        texto(&carga_regla(&r, None)),
        "escape=true&target=rule&belong=0&version=0&enabled=1&protocol=2&specificAction=1\
         &rejectType=7&dstportstart=8080&dstportend=8080"
    );
}

#[test]
fn lo_que_no_aplica_tiene_que_venir_vacio() {
    let base = icmp_capturada();
    let con = |k: &str, x: &str| {
        let mut b = base.clone();
        b.insert(k.to_owned(), x.to_owned());
        normalizar_regla(&b, "0")
    };
    // Puertos con ICMP, rechazo sin «rechazar», ICMP con TCP.
    assert_eq!(con("srcportstart", "80"), Err(CampoMal("srcportstart")));
    assert_eq!(con("rejectType", "1"), Err(CampoMal("rejectType")));
    let mut tcp = base.clone();
    tcp.insert("protocol".into(), "2".into());
    assert_eq!(normalizar_regla(&tcp, "0"), Err(CampoMal("icmpType")));
}

#[test]
fn formas_que_no_valen() {
    let base = icmp_capturada();
    let con = |k: &str, x: &str| {
        let mut b = base.clone();
        b.insert(k.to_owned(), x.to_owned());
        normalizar_regla(&b, "0")
    };
    assert_eq!(con("version", "1"), Err(CampoMal("version")));
    assert_eq!(con("srcaddr", "2001:db8::1"), Err(CampoMal("srcaddr")));
    assert_eq!(con("srcmask", "255.0.255.0"), Err(CampoMal("srcmask")));
    assert_eq!(con("icmpType", "55"), Err(CampoMal("icmpType")));
    assert_eq!(con("enabled", "si"), Err(CampoMal("enabled")));
    assert_eq!(con("protocol", "0"), Err(CampoMal("protocol")));
    assert_eq!(con("belong", "0|16"), Err(CampoMal("belong")));
    assert_eq!(con("name", "x"), Err(CampoMal("name")));
    assert_eq!(con("item", "0"), Err(CampoMal("?")));
    let mut tcp = base.clone();
    tcp.insert("protocol".into(), "3".into());
    tcp.remove("icmpType");
    tcp.insert("dstportstart".into(), "90".into());
    tcp.insert("dstportend".into(), "80".into());
    assert_eq!(normalizar_regla(&tcp, "0"), Err(CampoMal("dstportend")));
    // Nombres de filtro: ni `|` ni etiquetas ni más de 16 caracteres.
    let filtro = |nombre: &str| {
        normalizar_filtro(&v(&[
            ("name", nombre),
            ("interface", "0.2"),
            ("type", "0"),
            ("version", "0"),
            ("default", "0"),
        ]))
    };
    assert!(filtro("Entrada web").is_ok());
    for malo in ["a|b", "<b>", "diecisiete letras", " "] {
        assert_eq!(filtro(malo), Err(CampoMal("name")), "{malo}");
    }
}

#[test]
fn ipv6_con_prefijo_y_sus_tipos() {
    let r = normalizar_regla(
        &v(&[
            ("belong", "1"),
            ("enabled", "1"),
            ("protocol", "1"),
            ("specificAction", "2"),
            ("icmpType", "55"),
            ("srcaddr", "2001:db8::"),
            ("srcmask", "48"),
        ]),
        "1",
    )
    .unwrap();
    assert_eq!((r["srcmask"].as_str(), r["dstaddr"].as_str()), ("48", ""));
    assert_eq!(TIPOS_ICMP[55], "echo_request");
    assert_eq!(TIPOS_ICMP[25], "echo_request");
    assert!(icmp_de("1").all(|i| i == 1 || i >= 38));
    assert!(rechazo_de("0").all(|i| i <= 7));
}

#[test]
fn vista_de_la_lectura_y_vuelta() {
    let f = super::super::analizar(include_str!(
        "../../../fixtures/paginas/scbidirectionalflt_ls.asp"
    ))
    .unwrap();
    // RID 0: TCP al 22 desde una red; destino «cualquiera».
    let r0 = &f[0].reglas[0];
    let vista = vista_regla(&f[0], r0);
    assert_eq!(vista["dstaddr"], "");
    assert_eq!(vista["dstmask"], "");
    assert_eq!(vista["dstportstart"], "22");
    assert_eq!(vista["srcportstart"], "");
    assert_eq!(vista["icmpType"], "");
    assert_eq!(vista["rejectType"], "");
    // Lo leído vuelve a valer tal cual.
    assert_eq!(normalizar_regla(&vista, "0").unwrap(), vista);
    let vf = vista_filtro(&f[0]);
    assert_eq!(vf["interface"], "0.2");
    assert_eq!(normalizar_filtro(&vf).unwrap(), vf);
}

#[test]
fn coincide_con_direcciones_escritas_de_otra_forma() {
    let a = v(&[("srcaddr", "2001:DB8:0::1"), ("enabled", "1")]);
    let b = v(&[("srcaddr", "2001:db8::1"), ("enabled", "1")]);
    assert!(coincide(&a, &b, &["srcaddr", "enabled"]));
    let c = v(&[("srcaddr", "2001:db8::2"), ("enabled", "1")]);
    assert!(!coincide(&a, &c, &["srcaddr"]));
}

/// Las reglas y los filtros de la página se pueden volver a escribir igual;
/// una regla con el protocolo 0 («todos», que el editor no ofrece) o un
/// filtro con un nombre que la web no deja poner, no.
#[test]
fn reescribible_solo_lo_que_el_editor_sabe_escribir() {
    let mut f = super::super::analizar(include_str!(
        "../../../fixtures/paginas/scbidirectionalflt_ls.asp"
    ))
    .unwrap();
    assert!(f.iter().all(filtro_reescribible));
    assert!(
        f.iter()
            .all(|x| x.reglas.iter().all(|r| regla_reescribible(x, r)))
    );
    let mut todos = f[0].reglas[0].clone();
    todos.protocol = "0".into();
    assert!(!regla_reescribible(&f[0], &todos));
    f[0].name = "un nombre larguísimo de más".into();
    assert!(!filtro_reescribible(&f[0]));
}
