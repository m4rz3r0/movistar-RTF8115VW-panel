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
