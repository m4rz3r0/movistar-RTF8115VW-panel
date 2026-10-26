//! Pruebas de la regla «sin ping» en el gestor: la exención del bloqueo, la baja y sus guardas.

use super::*;

fn req(valores: &[(&str, &str)], instancia: Option<&str>, borrar: bool) -> ChangeRequest {
    ChangeRequest {
        funcion: ALTA.into(),
        instancia: instancia.map(str::to_owned),
        valores: valores
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        borrar,
        identidad: BTreeMap::new(),
    }
}

const CARGA: [(&str, &str); 7] = [
    ("target", "rule"),
    ("belong", "0"),
    ("version", "0"),
    ("enabled", "1"),
    ("protocol", "1"),
    ("specificAction", "0"),
    ("icmpType", "25"),
];

/// Solo la carga exacta del alta o la baja por un RID, y solo en el
/// alta: ni `modify`, ni otro valor, ni un parámetro de más.
#[test]
fn exento_solo_con_la_carga_exacta() {
    assert!(exento_de_bloqueo(&req(&CARGA, None, false)));
    assert!(exento_de_bloqueo(&req(&[], Some("16"), true)));
    let mut otra = CARGA;
    otra[6] = ("icmpType", "1");
    assert!(!exento_de_bloqueo(&req(&otra, None, false)));
    let mut mas = CARGA.to_vec();
    mas.push(("srcaddr", "192.0.2.1"));
    assert!(!exento_de_bloqueo(&req(&mas, None, false)));
    assert!(!exento_de_bloqueo(&req(&CARGA[1..], None, false)));
    assert!(!exento_de_bloqueo(&req(&[], Some("0|16"), true)));
    assert!(!exento_de_bloqueo(&req(&CARGA, Some("16"), true)));
    let mut modify = req(&CARGA, None, false);
    modify.funcion = "scbidirectionalflt.cmd?action=modify".into();
    assert!(!exento_de_bloqueo(&modify));
    let mut remove = req(&[("item", "0|16")], None, false);
    remove.funcion = BAJA.into();
    assert!(!exento_de_bloqueo(&remove));
}

/// La baja solo reconoce la regla del panel en un filtro de entrada
/// IPv4: con la misma huella en el de IPv6 o en uno de salida, no.
#[test]
fn la_baja_exige_el_filtro_de_entrada_ipv4() {
    let vacio = String::new;
    let regla = cf::Regla {
        rid: "16".into(),
        enable: "1".into(),
        protocol: "1".into(),
        action: "0".into(),
        reject_type: vacio(),
        icmp_type: "25".into(),
        orig_addr: vacio(),
        orig_mask: vacio(),
        orig_start_port: vacio(),
        orig_end_port: vacio(),
        dest_addr: vacio(),
        dest_mask: vacio(),
        dest_start_port: vacio(),
        dest_end_port: vacio(),
    };
    let filtro = |fid: &str, tipo: &str, version: &str| Filtro {
        fid: fid.into(),
        name: String::new(),
        interface: String::new(),
        intf_id: String::new(),
        tipo: tipo.into(),
        version: version.into(),
        default: String::new(),
        reglas: vec![regla.clone()],
    };
    let v4 = [filtro("0", "0", "0")];
    assert_eq!(
        filtro_de_la_regla(&v4, "16").map(|f| f.fid.as_str()),
        Some("0")
    );
    assert!(filtro_de_la_regla(&v4, "15").is_none());
    for (tipo, version) in [("0", "1"), ("1", "0")] {
        let otro = [filtro("1", tipo, version)];
        assert!(
            filtro_de_la_regla(&otro, "16").is_none(),
            "tipo {tipo}, versión {version}"
        );
    }
}

/// La baja lleva siempre el filtro y la regla.
#[test]
fn baja_con_filtro_y_regla() {
    let v: BTreeMap<String, String> = [("belong".to_owned(), "0".to_owned())].into();
    let (f, p) = baja("16", &v);
    assert_eq!(f, BAJA);
    assert_eq!(
        p,
        [
            ("item".to_owned(), "0|16".to_owned()),
            ("escape".to_owned(), "true".to_owned())
        ]
    );
}

/// La exención salta solo el bloqueo del catálogo: las demás guardas
/// (aquí, que sea una lectura) se comprueban igual.
#[test]
fn exento_no_salta_las_demas_guardas() {
    let cat = catalogo::Catalogo::load().unwrap();
    let alta = cat.funcion(ALTA).unwrap();
    assert!(alta.bloqueada.is_some());
    let guardas = crate::changes::router::guardas;
    assert!(matches!(
        guardas(alta, false),
        Err(CambioError::Bloqueada(_))
    ));
    assert!(guardas(alta, true).is_ok());
    let mut lectura = alta.clone();
    lectura.tipo = Tipo::Lectura;
    assert!(matches!(
        guardas(&lectura, true),
        Err(CambioError::Invalida(_))
    ));
}

/// Un filtro de entrada IPv4 `fid` con la regla del panel (RID 16).
fn filtro_con_la_regla(fid: &str) -> Filtro {
    let regla = cf::Regla {
        rid: "16".into(),
        enable: "1".into(),
        protocol: "1".into(),
        action: "0".into(),
        reject_type: "0".into(),
        icmp_type: "25".into(),
        orig_addr: "0.0.0.0".into(),
        orig_mask: "255.255.255.255".into(),
        orig_start_port: "0".into(),
        orig_end_port: "0".into(),
        dest_addr: "0.0.0.0".into(),
        dest_mask: "255.255.255.255".into(),
        dest_start_port: "0".into(),
        dest_end_port: "0".into(),
    };
    Filtro {
        fid: fid.into(),
        name: String::new(),
        interface: String::new(),
        intf_id: "0.2".into(),
        tipo: "0".into(),
        version: "0".into(),
        default: "0".into(),
        reglas: vec![regla],
    }
}

/// La baja de la regla del panel va atada al filtro donde se creó (el
/// `belong` de la identidad, que pone el deshacer): en otro filtro, o sin
/// él, no se borra.
#[test]
fn la_baja_va_atada_a_su_filtro() {
    let filtros = [filtro_con_la_regla("0")];
    let con = |belong: Option<&str>| {
        let mut r = req(&[], Some("16"), true);
        if let Some(b) = belong {
            r.identidad.insert("belong".into(), b.into());
        }
        r
    };
    assert_eq!(
        filtro_para_la_baja(&filtros, &con(Some("0"))).map(|f| f.fid.as_str()),
        Ok("0")
    );
    for r in [con(Some("1")), con(None)] {
        assert!(
            matches!(
                filtro_para_la_baja(&filtros, &r),
                Err(CambioError::InstanciaCambiada(_))
            ),
            "{:?}",
            r.identidad
        );
    }
}

/// Un FID que no es un número no llega a la baja: con `|` borraría otra
/// cosa.
#[test]
fn la_baja_exige_un_fid_numerico() {
    let filtros = [filtro_con_la_regla("0|9")];
    let mut r = req(&[], Some("16"), true);
    r.identidad.insert("belong".into(), "0|9".into());
    assert!(matches!(
        filtro_para_la_baja(&filtros, &r),
        Err(CambioError::Invalida(_))
    ));
}

/// El deshacer del alta lleva el filtro donde se creó como identidad.
#[test]
fn el_deshacer_del_alta_lleva_su_filtro() {
    let cat = catalogo::Catalogo::load().unwrap();
    let plan = Plan {
        funcion: cat.funcion(ALTA).unwrap().clone(),
        operacion: Operacion::Crear,
        instancia: Some("16".into()),
        valores: CARGA
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        payload: Vec::new(),
        campos: Vec::new(),
        lectura: Lectura::Cortafuegos,
        ids_antes: None,
    };
    let r = crate::changes::plan::inversa(&plan).unwrap();
    assert!(r.borrar);
    assert!(r.valores.is_empty(), "{:?}", r.valores);
    assert_eq!(r.identidad.get("belong").map(String::as_str), Some("0"));
    assert!(exento_de_bloqueo(&r));
}
