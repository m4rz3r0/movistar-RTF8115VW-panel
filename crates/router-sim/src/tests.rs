//! Pruebas internas del simulador.

use super::*;
use crate::escrituras::escribir_lista;
use crate::estado::Params;
use crate::fixtures::fixture_key;
use crate::http::form;
use catalogo::Lista;
use std::collections::HashMap;

#[test]
fn form_decoding() {
    assert_eq!(
        form(b"a=b%20c&mac=02%3A00&e=&x+y=1&p=%zz"),
        vec![
            ("a".to_owned(), "b c".to_owned()),
            ("mac".to_owned(), "02:00".to_owned()),
            ("e".to_owned(), String::new()),
            ("x y".to_owned(), "1".to_owned()),
            ("p".to_owned(), "%zz".to_owned())
        ]
    );
}

fn inner_de_prueba() -> Inner {
    let lista = "<?xml version=\"1.0\" ?><Response><cbGetX><x><x_0i>0</x_0i><x_id>0</x_id>\
                 <v>a</v></x><Result>0</Result></cbGetX></Response>";
    Inner {
        user: "u".into(),
        usuarios: Mutex::new(Vec::new()),
        fixtures: HashMap::from([(
            fixture_key("cbGetX", &[("all".into(), "ALL".into())]),
            lista.to_owned(),
        )]),
        catalogo: Catalogo::load().unwrap(),
        wan: None,
        unresponsive: AtomicBool::new(false),
        retardo_ms: AtomicU64::new(0),
        state: Mutex::new(SimState::default()),
    }
}

fn p(kv: &[(&str, &str)]) -> Params {
    kv.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect()
}

#[test]
fn lista_sin_del_rechaza_borrar() {
    let s = inner_de_prueba();
    let mut st = SimState::default();
    let sin_del = Lista {
        id_param: "x_id".into(),
        all: false,
        del: false,
    };
    let r = escribir_lista(
        &s,
        &mut st,
        "cbGetX",
        &sin_del,
        &p(&[("x_id", "0"), ("del", "DEL")]),
    );
    assert!(r.is_err(), "{r:?}");
    assert_eq!(st.listas["cbGetX"].nodos.len(), 1, "no se borra nada");
    let con_del = Lista {
        del: true,
        ..sin_del
    };
    escribir_lista(
        &s,
        &mut st,
        "cbGetX",
        &con_del,
        &p(&[("x_id", "0"), ("del", "DEL")]),
    )
    .unwrap();
    assert!(st.listas["cbGetX"].nodos.is_empty());
}
