//! Pruebas de la foto de partida: la instancia de un ajuste en la foto.

use super::*;
use ajustes::CatalogoAjustes;

/// Fixture `nombre` de `crates/hgu/fixtures`.
fn fixture(nombre: &str) -> String {
    let ruta = concat!(env!("CARGO_MANIFEST_DIR"), "/../hgu/fixtures/");
    std::fs::read_to_string(format!("{ruta}{nombre}.xml")).unwrap()
}

/// Una foto de `cbGetWifiAccPt` con los dos puntos de acceso principales
/// (el 0 de la lista de la radio de 2,4 GHz y el 1 leído por su id),
/// como la daría el router leído sin parámetros.
fn foto_de_los_dos_puntos() -> String {
    let instancia = |raw: &str, funcion: &str, id: &str| {
        let n = hgu::xml::parse_response(funcion, raw).unwrap();
        n.children
            .into_iter()
            .find(|c| c.text_of("dpm_cfg_wifi_acc_pt_id") == Some(id))
            .unwrap()
    };
    let p0 = instancia(
        &fixture("cbListWifiAccPt__dpm_cfg_wifi_id=0"),
        "cbListWifiAccPt",
        "0",
    );
    let p1 = instancia(
        &fixture("cbGetWifiAccPt__dpm_cfg_wifi_acc_pt_id=1"),
        "cbGetWifiAccPt",
        "1",
    );
    XmlNode {
        tag: "cbGetWifiAccPt".into(),
        text: String::new(),
        children: vec![p0, p1],
    }
    .to_xml()
}

#[test]
fn valor_en_foto_por_instancia() {
    let f = Catalogo::load().unwrap();
    let cat = CatalogoAjustes::load().unwrap();
    let xml = foto_de_los_dos_puntos();
    let nombre = |ajuste: &str| {
        let a = cat.ajuste(ajuste).unwrap();
        valor_en_foto(&f, a, a.campo("nombre").unwrap(), &xml, None)
    };
    assert_eq!(nombre("wifi.red_24").as_deref(), Some("RED-2"));
    assert_eq!(nombre("wifi.red_5").as_deref(), Some("RED-1"));
    // El de invitados (2) no está en esta foto.
    assert_eq!(nombre("wifi.invitados"), None);
    // Las radios, por `dpm_cfg_wifi_0i` (su `leer`) y `dpm_cfg_wifi_id`.
    let radios = XmlNode {
        tag: "cbGetWifi".into(),
        text: String::new(),
        children: ["0", "1"]
            .into_iter()
            .flat_map(|i| {
                let raw = fixture(&format!("cbGetWifi__dpm_cfg_wifi_0i={i}"));
                hgu::xml::parse_response("cbGetWifi", &raw)
                    .unwrap()
                    .children
                    .into_iter()
                    .filter(|c| !c.children.is_empty())
            })
            .collect(),
    }
    .to_xml();
    for (ajuste, id) in [("wifi.radio_24", "0"), ("wifi.radio_5", "1")] {
        let a = cat.ajuste(ajuste).unwrap();
        let encendida = a.campo("encendida").unwrap();
        assert!(
            valor_en_foto(&f, a, encendida, &radios, None).is_some(),
            "{ajuste}"
        );
        let n = nodo(a, &radios).unwrap();
        let l = instancia_en_foto(a, &n).unwrap();
        assert_eq!(leido(&l, "dpm_cfg_wifi_id"), Some(id), "{ajuste}");
    }
    // Un ajuste sin filtros toma la respuesta entera.
    let upnp = cat.ajuste("puertos.upnp").unwrap();
    let xml = "<cbGetUpnpCfg><upnp_cfg><adm_state>1</adm_state></upnp_cfg></cbGetUpnpCfg>";
    assert_eq!(
        valor_en_foto(&f, upnp, &upnp.campos[0], xml, None).as_deref(),
        Some("1")
    );
    // Sin ninguno de sus campos, no hay dato.
    let vacio = "<cbGetUpnpCfg><Result>0</Result></cbGetUpnpCfg>";
    assert_eq!(valor_en_foto(&f, upnp, &upnp.campos[0], vacio, None), None);
}
