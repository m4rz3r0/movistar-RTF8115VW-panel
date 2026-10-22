//! Pruebas de la insignia de cada red y la nota de su radio.

use super::historial::{PasoDeRed, ultimo_apagado};
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
    // La de invitados no se ha podido leer: no se sabe si la radio emite.
    assert_eq!(
        de(&v, "wifi.radio_24").nota_radio,
        Some(NotaRadio::RedesSinLeer)
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

/// Una red de la radio no se pudo leer y ninguna de las otras emite: la
/// radio no dice que emite (ni con su lista ni sin ella), sino que sus
/// redes no se han podido leer.
#[test]
fn radio_con_una_red_sin_leer_no_dice_que_emite() {
    let sin_red_24: Vec<_> = todo_apagado()
        .into_iter()
        .filter(|(id, _)| *id != "wifi.red_24")
        .collect();
    for listas in [listas_apagadas(), Vec::new()] {
        let v = marcar_con(&sin_red_24, &listas);
        let nota = de(&v, "wifi.radio_24").nota_radio;
        assert_eq!(nota, Some(NotaRadio::RedesSinLeer), "{}", listas.len());
        let (estado, clave) = estado_red(None, Some(true), nota);
        assert_ne!(estado, EstadoVisual::Emite);
        assert_eq!(clave, "ajustes-wifi-radio-redes-sin-leer-corto");
    }
    // Si otra de sus redes emite, sí se sabe que emite.
    let mut con_invitados = sin_red_24.clone();
    con_invitados[0] = ("wifi.invitados", nodo("", "1", Some("1")));
    let v = marcar_con(&con_invitados, &listas_apagadas());
    assert_eq!(de(&v, "wifi.radio_24").nota_radio, None);
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

/// La tabla de estados de la tarjeta: la insignia de la red manda; en una
/// radio, si emite y lo que no se ve en sus redes.
#[test]
fn estado_red_de_redes_y_radios() {
    use EstadoVisual as E;
    let casos = [
        (
            (Some(EmisionRed::Encendida), None, None),
            (E::Emite, "ajustes-wifi-red-encendida"),
        ),
        (
            (Some(EmisionRed::Apagada), Some(true), None),
            (E::Apagada, "ajustes-wifi-red-apagada"),
        ),
        (
            (Some(EmisionRed::SinRadio), None, None),
            (E::Atencion, "ajustes-wifi-red-sin-radio"),
        ),
        (
            (Some(EmisionRed::NoEmite), None, None),
            (E::Atencion, "ajustes-wifi-red-no-emite"),
        ),
        (
            (None, Some(false), None),
            (E::Apagada, "ajustes-wifi-radio-apagada"),
        ),
        (
            (None, Some(true), Some(NotaRadio::SinRedes)),
            (E::Atencion, "ajustes-wifi-radio-sin-redes-corto"),
        ),
        (
            (None, Some(true), Some(NotaRadio::Ocultas(2))),
            (E::Emite, "ajustes-wifi-red-encendida"),
        ),
        (
            (None, Some(true), None),
            (E::Emite, "ajustes-wifi-red-encendida"),
        ),
        (
            (None, Some(true), Some(NotaRadio::RedesSinLeer)),
            (E::Atencion, "ajustes-wifi-radio-redes-sin-leer-corto"),
        ),
        (
            (None, None, None),
            (E::Atencion, "ajustes-estado-desconocido"),
        ),
    ];
    for ((ap, radio, ocultos), (estado, clave)) in casos {
        assert_eq!(
            estado_red(ap, radio, ocultos),
            (estado, clave.to_owned()),
            "{ap:?} {radio:?} {ocultos:?}"
        );
        for idioma in i18n::Idioma::TODOS {
            assert!(i18n::existe(idioma, clave), "{clave}");
        }
    }
    let es = |k: &str| i18n::tr(i18n::Idioma::Es, k, None);
    assert_eq!(
        es("ajustes-wifi-radio-sin-redes-corto"),
        "Encendida, sin redes"
    );
}

/// Ancho de banda de cada código del router.
#[test]
fn ancho_en_mhz() {
    assert_eq!(mhz("1"), Some("20"));
    assert_eq!(mhz("3"), Some("80"));
    assert_eq!(mhz("9"), None);
}

/// «Apagada desde»: manda el último paso que tocó el `adm_state` de cada
/// punto de acceso; encenderla lo borra, y otro punto no cuenta.
#[test]
fn apagada_desde_el_ultimo_paso() {
    let paso = |punto: &str, campo: &str, despues: &str, cuando: i64| PasoDeRed {
        valores: [(ID_PUNTO.to_owned(), punto.to_owned())].into(),
        diff: format!(
            r#"[{{"nombre":"{campo}","antes":"x","despues":"{despues}","secreto":false}}]"#
        ),
        cuando,
    };
    let pasos = [
        paso("0", "adm_state", "0", 10),
        paso("1", "adm_state", "0", 20),
        paso("1", "adm_state", "1", 30),
        paso("0", "ssid", "RED-9", 40),
        paso("2", "adm_state", "0", 50),
        paso("2", "adm_state", "0", 60),
    ];
    let m = ultimo_apagado(&pasos);
    assert_eq!(m.get("0"), Some(&10), "el nombre no cuenta");
    assert_eq!(m.get("1"), None, "se volvió a encender");
    assert_eq!(m.get("2"), Some(&60));
}
