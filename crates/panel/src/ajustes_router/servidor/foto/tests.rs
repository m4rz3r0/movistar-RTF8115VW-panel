//! Pruebas de la foto de partida: la instancia de un ajuste en la foto.

use super::*;
use ajustes::CatalogoAjustes;

/// Fixture `nombre` de `crates/hgu/fixtures`.
fn fixture(nombre: &str) -> String {
    let ruta = concat!(env!("CARGO_MANIFEST_DIR"), "/../hgu/fixtures/");
    std::fs::read_to_string(format!("{ruta}{nombre}.xml")).unwrap()
}

/// Respuesta de la fixture `nombre` a una lectura de `funcion`.
fn respuesta(funcion: &str, nombre: &str) -> XmlNode {
    hgu::xml::parse_response(funcion, &fixture(nombre)).unwrap()
}

/// La foto de `funcion` tras juntar `respuestas` con el mismo código que
/// usa la lectura de la foto de partida (`fotos::juntar`).
fn juntadas(funcion: &str, respuestas: Vec<XmlNode>) -> String {
    let cat = Catalogo::load().unwrap();
    let id_param = cat
        .funcion(funcion)
        .and_then(|f| f.lista.as_ref())
        .map(|l| l.id_param.clone());
    let mut foto = BTreeMap::new();
    for r in respuestas {
        crate::fotos::juntar(&mut foto, funcion, r, id_param.as_deref());
    }
    foto.remove(funcion).unwrap().to_xml()
}

/// La respuesta del punto de acceso `id` leído por su id, como la da el
/// router: la instancia de la lista de su radio, en un `cbGetWifiAccPt`.
fn punto(id: &str) -> XmlNode {
    let lista = respuesta("cbListWifiAccPt", "cbListWifiAccPt__dpm_cfg_wifi_id=0");
    XmlNode {
        tag: "cbGetWifiAccPt".into(),
        text: String::new(),
        children: lista
            .children
            .into_iter()
            .filter(|c| c.text_of("dpm_cfg_wifi_acc_pt_id") == Some(id))
            .collect(),
    }
}

#[test]
fn valor_en_foto_por_instancia() {
    let f = Catalogo::load().unwrap();
    let cat = CatalogoAjustes::load().unwrap();
    // La de partida (sin parámetros: el 1) y las de cada ajuste (el 0 y,
    // otra vez, el 1, que no se repite).
    let xml = juntadas(
        "cbGetWifiAccPt",
        vec![
            respuesta("cbGetWifiAccPt", "cbGetWifiAccPt__dpm_cfg_wifi_acc_pt_id=1"),
            punto("0"),
            respuesta("cbGetWifiAccPt", "cbGetWifiAccPt__dpm_cfg_wifi_acc_pt_0i=1"),
        ],
    );
    assert_eq!(xml.matches("<ssid>RED-1</ssid>").count(), 1, "sin repetir");
    let nombre = |ajuste: &str| {
        let a = cat.ajuste(ajuste).unwrap();
        valor_en_foto(&f, a, a.campo("nombre").unwrap(), &xml, None)
    };
    assert_eq!(nombre("wifi.red_24").as_deref(), Some("RED-2"));
    assert_eq!(nombre("wifi.red_5").as_deref(), Some("RED-1"));
    // El de invitados (2) no está en esta foto.
    assert_eq!(nombre("wifi.invitados"), None);
    // Las radios, por su `dpm_cfg_wifi_id` (sus `fijos`).
    let radios = juntadas(
        "cbGetWifi",
        ["0", "1"]
            .into_iter()
            .map(|i| respuesta("cbGetWifi", &format!("cbGetWifi__dpm_cfg_wifi_0i={i}")))
            .collect(),
    );
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

/// Con los dos pools DHCP en la foto (el de id 1 en la posición 0, como en
/// el router real), los ajustes del pool de casa toman el de id 0.
#[test]
fn valor_en_foto_del_pool_de_casa() {
    let f = Catalogo::load().unwrap();
    let cat = CatalogoAjustes::load().unwrap();
    let funcion = "cbGetDhcpv4ServerPool";
    let xml = juntadas(
        funcion,
        ["0", "1"]
            .into_iter()
            .map(|i| {
                respuesta(
                    funcion,
                    &format!("{funcion}__rtm_cfg_dhcpv4_svr_pool_0i={i}"),
                )
            })
            .collect(),
    );
    let a = cat.ajuste("red_local.dhcp").unwrap();
    assert_eq!(
        valor_en_foto(&f, a, a.campo("inicio").unwrap(), &xml, None).as_deref(),
        Some("192.168.1.100")
    );
    let a = cat.ajuste("red_local.dns_casa").unwrap();
    assert_eq!(
        valor_en_foto(&f, a, a.campo("principal").unwrap(), &xml, None).as_deref(),
        Some("192.168.1.1")
    );
}
