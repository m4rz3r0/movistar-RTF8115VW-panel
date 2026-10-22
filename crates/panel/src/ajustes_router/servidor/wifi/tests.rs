//! Pruebas de la insignia de cada red y la nota de su radio.

use super::*;
use ajustes::CatalogoAjustes;

/// Lectura de un punto de acceso (o de una radio, con `radio_`).
fn nodo(prefijo: &str, adm: &str, opr: Option<&str>) -> XmlNode {
    let opr = opr.map_or(String::new(), |v| {
        format!("<{prefijo}opr_status>{v}</{prefijo}opr_status>")
    });
    let raw = format!(
        "<Response><cbX><i><{prefijo}adm_state>{adm}</{prefijo}adm_state>{opr}</i>\
         <Result>0</Result></cbX></Response>"
    );
    hgu::xml::parse_response("cbX", &raw).unwrap()
}

/// Lista de los puntos de acceso de una radio: (id, adm_state, opr_status).
fn lista(puntos: &[(&str, &str, &str)]) -> XmlNode {
    let puntos: String = puntos
        .iter()
        .map(|(id, adm, opr)| {
            format!(
                "<dpm_cfg_wifi_acc_pt><adm_state>{adm}</adm_state><opr_status>{opr}</opr_status>\
                 <dpm_cfg_wifi_acc_pt_id>{id}</dpm_cfg_wifi_acc_pt_id></dpm_cfg_wifi_acc_pt>"
            )
        })
        .collect();
    let raw = format!(
        "<Response><cbListWifiAccPt>{puntos}<Result>0</Result></cbListWifiAccPt></Response>"
    );
    hgu::xml::parse_response("cbListWifiAccPt", &raw).unwrap()
}

/// Las del router real: cuatro puntos por radio, todos apagados.
fn listas_apagadas() -> Vec<(&'static str, XmlNode)> {
    let apagados = |ids: [&'static str; 4]| lista(&ids.map(|id| (id, "0", "0")));
    vec![
        ("wifi.radio_24", apagados(["0", "2", "4", "6"])),
        ("wifi.radio_5", apagados(["1", "3", "5", "7"])),
    ]
}

/// Las vistas del tema WiFi del catálogo con estas lecturas (id del
/// ajuste → lectura; el resto, sin leer) y estas listas de puntos de
/// acceso (id del ajuste de la radio → lista) ya marcadas.
fn marcar_con(lecturas: &[(&str, XmlNode)], listas: &[(&str, XmlNode)]) -> Vec<AjusteVista> {
    let f = Catalogo::load().unwrap();
    let cat = CatalogoAjustes::load().unwrap();
    let del_tema: Vec<&Ajuste> = cat.por_tema(Tema::from_slug("wifi").unwrap()).collect();
    let nodos: Vec<Option<&XmlNode>> = del_tema
        .iter()
        .map(|a| lecturas.iter().find(|(id, _)| *id == a.id).map(|(_, n)| n))
        .collect();
    let mut vistas: Vec<AjusteVista> = del_tema
        .iter()
        .zip(&nodos)
        .map(|(a, n)| vista(&f, a, *n))
        .collect();
    let listas: Vec<(String, Option<&XmlNode>)> = claves_de_listas(&del_tema)
        .into_iter()
        .map(|(radio, _)| {
            let n = listas.iter().find(|(r, _)| *r == radio).map(|(_, n)| n);
            (radio, n)
        })
        .collect();
    marcar_emision(&del_tema, &mut vistas, &nodos, &listas);
    vistas
}

/// Como [`marcar_con`], con las listas del router real.
fn marcar(lecturas: &[(&str, XmlNode)]) -> Vec<AjusteVista> {
    marcar_con(lecturas, &listas_apagadas())
}

fn de<'v>(vistas: &'v [AjusteVista], id: &str) -> &'v AjusteVista {
    vistas.iter().find(|v| v.id == id).unwrap()
}

/// Las lecturas de las redes y las radios con todas las redes apagadas y
/// las radios encendidas, como el router real.
fn todo_apagado() -> Vec<(&'static str, XmlNode)> {
    vec![
        ("wifi.red_24", nodo("", "0", Some("0"))),
        ("wifi.invitados", nodo("", "0", Some("0"))),
        ("wifi.red_5", nodo("", "0", Some("0"))),
        ("wifi.radio_24", nodo("radio_", "1", Some("1"))),
        ("wifi.radio_5", nodo("radio_", "1", Some("1"))),
    ]
}

#[test]
fn redes_apagadas_con_la_radio_encendida() {
    let mut lecturas = todo_apagado();
    lecturas[2] = ("wifi.red_5", nodo("", "1", Some("1")));
    let v = marcar(&lecturas);
    assert_eq!(de(&v, "wifi.red_24").red_wifi, Some(EmisionRed::Apagada));
    assert_eq!(de(&v, "wifi.red_5").red_wifi, Some(EmisionRed::Encendida));
    assert_eq!(
        de(&v, "wifi.radio_24").nota_radio,
        Some(NotaRadio::SinRedes)
    );
    assert_eq!(de(&v, "wifi.radio_5").nota_radio, None, "una red emite");
    assert_eq!(de(&v, "wifi.radio_24").red_wifi, None);
    assert_eq!(de(&v, "wifi.red_24_avanzado").red_wifi, None);
}

/// Un punto de acceso que el panel no muestra (el 4, en la radio de
/// 2,4 GHz) está encendido: la radio no dice que no emite ninguna WiFi,
/// sino cuántas emite que no se ven aquí. Los que muestra el panel no
/// cuentan aunque emitan (el 1, en la de 5 GHz).
#[test]
fn un_punto_oculto_encendido_se_cuenta() {
    let mut lecturas = todo_apagado();
    lecturas[2] = ("wifi.red_5", nodo("", "1", Some("1")));
    let listas = vec![
        (
            "wifi.radio_24",
            lista(&[
                ("0", "0", "0"),
                ("2", "0", "0"),
                ("4", "1", "1"),
                ("6", "0", "0"),
            ]),
        ),
        (
            "wifi.radio_5",
            lista(&[
                ("1", "1", "1"),
                ("3", "0", "0"),
                ("5", "0", "0"),
                ("7", "0", "0"),
            ]),
        ),
    ];
    let v = marcar_con(&lecturas, &listas);
    assert_eq!(
        de(&v, "wifi.radio_24").nota_radio,
        Some(NotaRadio::Ocultas(1))
    );
    assert_eq!(de(&v, "wifi.radio_5").nota_radio, None);
    assert_eq!(de(&v, "wifi.red_24").red_wifi, Some(EmisionRed::Apagada));
}

/// Sin la lista de una radio no se sabe si emite otra red: no hay nota.
#[test]
fn sin_la_lista_no_hay_nota() {
    let v = marcar_con(&todo_apagado(), &[]);
    assert_eq!(de(&v, "wifi.radio_24").nota_radio, None);
    assert_eq!(de(&v, "wifi.radio_5").nota_radio, None);
    assert_eq!(de(&v, "wifi.red_24").red_wifi, Some(EmisionRed::Apagada));
}

/// Encendida (`adm_state=1`) con la radio encendida, pero sin emitir
/// (`opr_status=0`): ni «Apagada» ni «Encendida». La radio lleva la nota
/// de que ninguna de sus redes emite (no de que estén apagadas: la
/// insignia dice que esta está encendida).
#[test]
fn encendida_que_no_emite_lo_dice() {
    let mut lecturas = todo_apagado();
    lecturas[0] = ("wifi.red_24", nodo("", "1", Some("0")));
    let v = marcar(&lecturas);
    assert_eq!(de(&v, "wifi.red_24").red_wifi, Some(EmisionRed::NoEmite));
    assert_eq!(
        de(&v, "wifi.radio_24").nota_radio,
        Some(NotaRadio::SinRedes),
        "no emite ninguna"
    );
    // El texto de la nota habla de emitir, no de estar encendidas.
    for idioma in i18n::Idioma::TODOS {
        let nota = i18n::tr(idioma, "ajustes-wifi-radio-sin-redes", None);
        assert!(
            !nota.contains("activa") && !nota.contains("active"),
            "{nota}"
        );
    }
}

/// Si emite manda sobre lo encendida; sin él, vale lo encendida.
#[test]
fn sin_si_emite_vale_lo_encendida() {
    let v = marcar(&[
        ("wifi.red_5", nodo("", "1", None)),
        ("wifi.red_24", nodo("", "0", None)),
        ("wifi.radio_24", nodo("radio_", "1", None)),
        ("wifi.radio_5", nodo("radio_", "1", None)),
    ]);
    assert_eq!(de(&v, "wifi.red_5").red_wifi, Some(EmisionRed::Encendida));
    assert_eq!(de(&v, "wifi.red_24").red_wifi, Some(EmisionRed::Apagada));
    assert_eq!(
        de(&v, "wifi.radio_24").nota_radio,
        None,
        "la de invitados no se ha podido leer"
    );
}

#[test]
fn red_encendida_con_la_radio_apagada_no_emite() {
    let v = marcar(&[
        ("wifi.red_5", nodo("", "1", Some("0"))),
        ("wifi.radio_5", nodo("radio_", "0", Some("0"))),
    ]);
    assert_eq!(de(&v, "wifi.red_5").red_wifi, Some(EmisionRed::SinRadio));
    assert_eq!(
        de(&v, "wifi.radio_5").nota_radio,
        None,
        "la radio está apagada"
    );
}

#[test]
fn sin_lectura_no_hay_insignia() {
    let v = marcar(&[]);
    assert!(
        v.iter()
            .all(|a| a.red_wifi.is_none() && a.nota_radio.is_none())
    );
}

/// Una lista por radio, con su id.
#[test]
fn una_lista_por_radio() {
    let cat = CatalogoAjustes::load().unwrap();
    let del_tema: Vec<&Ajuste> = cat.por_tema(Tema::from_slug("wifi").unwrap()).collect();
    let listas = claves_de_listas(&del_tema);
    let esperadas = [("wifi.radio_24", "0"), ("wifi.radio_5", "1")];
    assert_eq!(listas.len(), esperadas.len(), "{listas:?}");
    for ((radio, c), (r, id)) in listas.iter().zip(esperadas) {
        assert_eq!(radio, r);
        assert_eq!(c, &Clave::new(LISTA, &[(ID_RADIO, id)]));
    }
}

/// Cada red básica del catálogo (las que se leen por punto de acceso)
/// tiene su radio en la tabla.
#[test]
fn cada_red_del_catalogo_tiene_su_radio() {
    let cat = CatalogoAjustes::load().unwrap();
    for a in cat.por_tema(Tema::from_slug("wifi").unwrap()) {
        if a.nivel == ajustes::Nivel::Basico && a.leer.funcion == "cbGetWifiAccPt" {
            let radio = RADIO_DE_LA_RED.iter().find(|(r, _)| *r == a.id);
            let radio = radio.unwrap_or_else(|| panic!("{} sin radio", a.id)).1;
            assert!(cat.ajuste(radio).is_some(), "{radio}");
        }
    }
}
