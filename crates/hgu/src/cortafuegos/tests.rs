//! Pruebas del cortafuegos con la copia saneada de la página real.

use super::*;

const PAGINA: &str = include_str!("../../fixtures/paginas/scbidirectionalflt_ls.asp");

fn filtros() -> Vec<Filtro> {
    analizar(PAGINA).expect("la página saneada se analiza")
}

fn regla(protocol: &str, action: &str, icmp: &str) -> Regla {
    Regla {
        rid: "16".into(),
        enable: "1".into(),
        protocol: protocol.into(),
        action: action.into(),
        reject_type: "0".into(),
        icmp_type: icmp.into(),
        orig_addr: "0.0.0.0".into(),
        orig_mask: "255.255.255.255".into(),
        orig_start_port: "0".into(),
        orig_end_port: "0".into(),
        dest_addr: "0.0.0.0".into(),
        dest_mask: "255.255.255.255".into(),
        dest_start_port: "0".into(),
        dest_end_port: "0".into(),
    }
}

#[test]
fn analiza_la_pagina_real_saneada() {
    let f = filtros();
    assert_eq!(f.len(), 2);
    assert_eq!(
        (
            f[0].name.as_str(),
            f[0].interface.as_str(),
            f[0].intf_id.as_str()
        ),
        ("IPv4PppIn", "ppp0", "0.2")
    );
    assert_eq!(f[0].reglas.len(), 10);
    let r8 = f[0].reglas.iter().find(|r| r.rid == "8").unwrap();
    assert_eq!((r8.protocol.as_str(), r8.icmp_type.as_str()), ("1", "1"));
    assert_eq!(f[1].version, "1");
    assert_eq!(f[1].reglas.len(), 6);
}

#[test]
fn pagina_sin_filtro_es_error_sin_html() {
    let e = analizar("<html>x</html>").unwrap_err();
    assert!(!e.to_string().contains("<html"), "{e}");
    // Un JSON roto tampoco enseña lo que trae.
    let e = analizar("var filter = [{\"FID\":\"<b>secreto</b>\",").unwrap_err();
    assert!(!e.to_string().contains("secreto"), "{e}");
}

#[test]
fn carga_sin_ping_es_la_del_editor() {
    let f = filtros();
    let mut carga = carga_sin_ping(&f[0]);
    carga.sort();
    let carga: Vec<(&str, &str)> = carga.iter().map(|(k, v)| (*k, v.as_str())).collect();
    assert_eq!(
        carga,
        [
            ("belong", "0"),
            ("enabled", "1"),
            ("icmpType", "25"),
            ("protocol", "1"),
            ("specificAction", "0"),
            ("target", "rule"),
            ("version", "0"),
        ]
    );
}

#[test]
fn huella_sin_ping() {
    assert!(es_sin_ping(&regla("1", "0", "25")));
    assert!(!es_sin_ping(&regla("1", "0", "1")));
    assert!(!es_sin_ping(&regla("1", "2", "25")));
    // Con un origen concreto ya no es la del panel.
    let mut r = regla("1", "0", "25");
    r.orig_addr = "192.0.2.1".into();
    assert!(!es_sin_ping(&r));
}

#[test]
fn abiertos_de_la_pagina_real() {
    let a = abiertos(&filtros());
    let de = |s: Servicio| a.iter().find(|x| x.servicio == s).copied();
    assert_eq!(
        de(Servicio::Ping),
        Some(Abierto {
            servicio: Servicio::Ping,
            desde_cualquiera: true
        })
    );
    for s in [Servicio::Ssh, Servicio::Web, Servicio::GestionOperador] {
        assert_eq!(
            de(s),
            Some(Abierto {
                servicio: s,
                desde_cualquiera: false
            }),
            "{s:?}"
        );
    }
}

#[test]
fn item_lleva_filtro_y_regla() {
    assert_eq!(item("0", "16"), "0|16");
}

#[test]
fn a_xml_una_regla_por_hijo() {
    let f = filtros();
    let n = a_xml(&f);
    assert_eq!(n.tag, PAGINA_LISTA);
    assert_eq!(n.children.len(), 16);
    let r8 = n
        .children
        .iter()
        .find(|c| c.text_of("RID") == Some("8"))
        .unwrap();
    assert_eq!(r8.text_of("Name"), Some("IPv4PppIn"));
    assert_eq!(r8.text_of("IntfID"), Some("0.2"));
    assert_eq!(r8.text_of("IcmpType"), Some("1"));
    // Y vuelve a dar los mismos filtros.
    assert_eq!(de_xml(&n), f);
}

#[test]
fn filtro_de_entrada_de_la_conexion() {
    let f = filtros();
    assert_eq!(
        filtro_de_entrada_v4(&f, "2").map(|x| x.fid.as_str()),
        Some("0")
    );
    assert_eq!(filtro_de_entrada_v4(&f, "3"), None);
}

/// Un filtro de entrada IPv4 con solo `reglas`.
fn entrada(reglas: Vec<Regla>) -> Vec<Filtro> {
    let mut f = filtros();
    f.truncate(1);
    f[0].reglas = reglas;
    f
}

fn abiertos_desde_cualquiera(f: &[Filtro]) -> Vec<Servicio> {
    abiertos(f)
        .into_iter()
        .filter(|a| a.desde_cualquiera)
        .map(|a| a.servicio)
        .collect()
}

/// El router trata el puerto 0 como «cualquiera» (`dsport==="0"` sale
/// vacío en la lista): una regla TCP que permite con puertos 0..0 abre
/// todo.
#[test]
fn tcp_con_puerto_0_abre_todos() {
    let f = entrada(vec![regla("2", "2", "0")]);
    assert_eq!(
        abiertos_desde_cualquiera(&f),
        [Servicio::Web, Servicio::Ssh, Servicio::GestionOperador]
    );
    // Un rango concreto, solo lo suyo.
    let mut r = regla("2", "2", "0");
    r.dest_start_port = "20".into();
    r.dest_end_port = "25".into();
    assert_eq!(
        abiertos_desde_cualquiera(&entrada(vec![r])),
        [Servicio::Ssh]
    );
}

/// El protocolo 0 («NONE» en `PROTOCOL` de la página; el editor no lo
/// ofrece) no filtra por protocolo: cuenta como todos, el ping incluido.
#[test]
fn protocolo_0_cuenta_como_todos() {
    let f = entrada(vec![regla("0", "2", "0")]);
    assert_eq!(
        abiertos_desde_cualquiera(&f),
        [
            Servicio::Ping,
            Servicio::Web,
            Servicio::Ssh,
            Servicio::GestionOperador
        ]
    );
    // Si descarta, no abre nada.
    assert!(abiertos(&entrada(vec![regla("0", "0", "0")])).is_empty());
}
