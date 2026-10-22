//! Pruebas internas del lado del servidor de Ajustes.

use super::guardar::{cambio_legible, grupo_de, instancias_fijas};
use super::*;
use ajustes::CatalogoAjustes;

fn peticion(ajuste: &str, valores: &[(&str, &str)], borrar: bool) -> PeticionAjuste {
    PeticionAjuste {
        ajuste: ajuste.into(),
        instancia: Some("7".into()),
        borrar,
        valores: valores
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        esperados: BTreeMap::new(),
    }
}

fn mapa(pares: &[(&str, &str)]) -> BTreeMap<String, String> {
    pares
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn grupo_de_un_ajuste_de_varios_campos() {
    let cat = CatalogoAjustes::load().unwrap();
    let a = cat.ajuste("wifi.red_5").unwrap();
    let p = peticion(
        "wifi.red_5",
        &[("nombre", "casa-5"), ("seguridad", "abierta")],
        false,
    );
    let g = grupo_de(a, &p).unwrap();
    assert_eq!(g.titulo.as_deref(), Some("ajuste-wifi-red_5-titulo"));
    assert_eq!(g.pasos.len(), 1, "{g:?}");
    let paso = &g.pasos[0];
    assert_eq!(paso.etiqueta.as_deref(), Some("ajuste-wifi-red_5-titulo"));
    assert_eq!(paso.req.funcion, "cbSetWifiAccPt");
    assert_eq!(paso.req.instancia, None, "no es una lista");
    assert!(!paso.req.borrar);
    assert_eq!(
        paso.req.valores,
        mapa(&[
            ("dpm_cfg_wifi_acc_pt_id", "1"),
            ("ssid", "casa-5"),
            ("authentication", "0"),
            ("encryption", "0"),
        ])
    );
}

#[test]
fn grupo_de_sin_secreto_sin_tocar_ni_campos_sin_tocar() {
    let cat = CatalogoAjustes::load().unwrap();
    let a = cat.ajuste("wifi.red_24").unwrap();
    let p = peticion(
        "wifi.red_24",
        &[("nombre", "casa"), ("contrasena", MASCARA)],
        false,
    );
    let g = grupo_de(a, &p).unwrap();
    assert_eq!(
        g.pasos[0].req.valores,
        mapa(&[("dpm_cfg_wifi_acc_pt_id", "0"), ("ssid", "casa")])
    );
    // Nada tocado: ningún paso.
    let g = grupo_de(a, &peticion("wifi.red_24", &[], false)).unwrap();
    assert!(g.pasos.is_empty());
}

#[test]
fn grupo_de_valida_cada_valor() {
    let cat = CatalogoAjustes::load().unwrap();
    let a = cat.ajuste("wifi.red_24").unwrap();
    let largo = "x".repeat(33);
    let e = grupo_de(a, &peticion("wifi.red_24", &[("nombre", &largo)], false));
    assert_eq!(e.unwrap_err(), ErrorValor::Largo { max: 32 });
    let e = grupo_de(a, &peticion("wifi.red_24", &[("seguridad", "wep")], false));
    assert_eq!(e.unwrap_err(), ErrorValor::Opcion);
    // Un campo de solo lectura no se escribe.
    let u = cat.ajuste("seguridad.usuarios_router").unwrap();
    let e = grupo_de(u, &peticion("x", &[("papel", "x")], false));
    assert_eq!(e.unwrap_err(), ErrorValor::SoloLectura);
}

#[test]
fn grupo_de_una_lista_con_su_instancia_y_borrar() {
    let cat = CatalogoAjustes::load().unwrap();
    let a = cat.ajuste("red_local.reservas").unwrap();
    let g = grupo_de(a, &peticion("x", &[("ip", "192.0.2.9")], false)).unwrap();
    assert_eq!(g.pasos.len(), 1);
    assert_eq!(g.pasos[0].req.instancia.as_deref(), Some("7"));
    assert_eq!(
        g.pasos[0].req.valores,
        mapa(&[
            ("ipv4_addr", "192.0.2.9"),
            ("rtm_cfg_dhcpv4_svr_pool_id", "0")
        ])
    );
    // Borrar: cada función de escritura, solo con los fijos.
    let g = grupo_de(a, &peticion("x", &[("ip", "192.0.2.9")], true)).unwrap();
    assert_eq!(g.pasos.len(), 1);
    assert!(g.pasos[0].req.borrar);
    assert_eq!(
        g.pasos[0].req.valores,
        mapa(&[("rtm_cfg_dhcpv4_svr_pool_id", "0")])
    );
}

#[test]
fn instancia_fija_de_un_ajuste_sobre_una_funcion_de_lista() {
    let f = Catalogo::load().unwrap();
    let cat = CatalogoAjustes::load().unwrap();
    let a = cat.ajuste("red_local.dhcp").unwrap();
    let mut g = grupo_de(a, &peticion("x", &[("activo", "false")], false)).unwrap();
    assert_eq!(g.pasos[0].req.instancia, None);
    instancias_fijas(&f, a, &mut g).unwrap();
    assert_eq!(g.pasos[0].req.instancia.as_deref(), Some("0"));
    // Sin el id en los fijos, no se adivina (crearía otra instancia).
    let mut sin_id = a.clone();
    sin_id.fijos.clear();
    let mut g = grupo_de(&sin_id, &peticion("x", &[("activo", "false")], false)).unwrap();
    assert_eq!(
        instancias_fijas(&f, &sin_id, &mut g).unwrap_err(),
        "cbSetDhcpv4ServerPool"
    );
}

#[test]
fn resumen_legible_de_una_opcion_con_ademas() {
    let cat = CatalogoAjustes::load().unwrap();
    let a = cat.ajuste("wifi.red_24").unwrap();
    let c = a.campo("seguridad").unwrap();
    // Solo cambia `authentication`; `encryption` (igual) lo completa
    // lo pedido.
    let diff = Diff {
        campos: vec![crate::changes::CampoDiff {
            nombre: "authentication".into(),
            antes: "99".into(),
            despues: "3".into(),
            secreto: false,
            sensible: false,
        }],
    };
    let pedido = mapa(&[("authentication", "3"), ("encryption", "4")]);
    let completar = |p: &str| buscar(&pedido, p).map(str::to_owned);
    let l = cambio_legible("ajuste-wifi-red_24", c, &diff, &completar).unwrap();
    assert_eq!(l.campo, "ajuste-wifi-red_24-seguridad");
    assert_eq!(l.antes, "99", "desconocido: tal cual");
    assert_eq!(l.despues, "wpa2");
    assert_eq!(l.tipo, TipoCampo::Opcion);
    // Un campo cuyo parámetro no cambia no sale.
    let nombre = a.campo("nombre").unwrap();
    assert_eq!(
        cambio_legible("ajuste-wifi-red_24", nombre, &diff, &completar),
        None
    );
}
