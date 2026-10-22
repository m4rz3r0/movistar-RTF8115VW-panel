//! Si cada red WiFi emite: la insignia de la tarjeta de cada red y la nota
//! de la radio encendida que no emite ninguna.
//!
//! Comprobado en el router real (2026-10-22): `adm_state` de un punto de
//! acceso enciende o apaga la red, `opr_status` dice si de verdad emite, y
//! la radio lleva los suyos (`radio_adm_state`, `radio_opr_status`).

use super::lectura::hojas;
use super::*;

/// Radio de cada red, por los ids de sus ajustes: la de invitados va en la
/// de 2,4 GHz (punto de acceso 2, en la lista de la radio 0).
pub(super) const RADIO_DE_LA_RED: &[(&str, &str)] = &[
    ("wifi.red_24", "wifi.radio_24"),
    ("wifi.red_5", "wifi.radio_5"),
    ("wifi.invitados", "wifi.radio_24"),
];

/// Un sí/no de la lectura (`1`/`0`); `None` si no lo trae o vale otra cosa.
fn si_no(leidos: &Leidos, elemento: &str) -> Option<bool> {
    match leidos.get(elemento).map(|v| v.trim()) {
        Some("1") => Some(true),
        Some("0") => Some(false),
        _ => None,
    }
}

/// Encendida según la lectura (`adm`) y si emite (`opr`, o `adm` si la
/// lectura no lo trae).
fn estado(n: &XmlNode, adm: &str, opr: &str) -> (Option<bool>, Option<bool>) {
    let mut leidos = Leidos::new();
    hojas(n, &mut leidos);
    let adm = si_no(&leidos, adm);
    (adm, si_no(&leidos, opr).or(adm))
}

/// Marca en `vistas` (las del tema, con la lectura de cada una en `nodos`,
/// `None` si falló) si emite cada red y qué radio encendida no emite
/// ninguna. Fuera del tema WiFi no hace nada.
pub(super) fn marcar_emision(vistas: &mut [AjusteVista], nodos: &[Option<&XmlNode>]) {
    let ids: Vec<String> = vistas.iter().map(|v| v.id.clone()).collect();
    let lectura = |id: &str| {
        let i = ids.iter().position(|v| v == id)?;
        Some((i, nodos.get(i).copied().flatten()))
    };
    // Si emite cada radio; `None` si no se sabe.
    let radio = |id: &str| {
        lectura(id)
            .and_then(|(_, n)| n)
            .and_then(|n| estado(n, "radio_adm_state", "radio_opr_status").1)
    };
    let mut marcas = Vec::new();
    // Por radio, si emite cada una de sus redes.
    let mut redes_de: BTreeMap<&str, Vec<Option<bool>>> = BTreeMap::new();
    for &(red, id_radio) in RADIO_DE_LA_RED {
        let Some((i, nodo)) = lectura(red) else {
            continue;
        };
        let (encendida, emite) =
            nodo.map_or((None, None), |n| estado(n, "adm_state", "opr_status"));
        redes_de.entry(id_radio).or_default().push(emite);
        let marca = match (encendida.or(emite), radio(id_radio), emite) {
            (Some(true), Some(false), _) => EmisionRed::SinRadio,
            (_, _, Some(true)) => EmisionRed::Encendida,
            (_, _, Some(false)) => EmisionRed::Apagada,
            (_, _, None) => continue,
        };
        marcas.push((i, marca));
    }
    for (i, marca) in marcas {
        vistas[i].red_wifi = Some(marca);
    }
    for (id_radio, redes) in redes_de {
        let Some((j, _)) = lectura(id_radio) else {
            continue;
        };
        vistas[j].radio_sin_redes =
            radio(id_radio) == Some(true) && redes.iter().all(|r| *r == Some(false));
    }
}

#[cfg(test)]
mod tests {
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

    /// Las vistas del tema WiFi del catálogo con estas lecturas (id del
    /// ajuste → lectura; el resto, sin leer) ya marcadas.
    fn marcar(lecturas: &[(&str, XmlNode)]) -> Vec<AjusteVista> {
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
        marcar_emision(&mut vistas, &nodos);
        vistas
    }

    fn de<'v>(vistas: &'v [AjusteVista], id: &str) -> &'v AjusteVista {
        vistas.iter().find(|v| v.id == id).unwrap()
    }

    #[test]
    fn redes_apagadas_con_la_radio_encendida() {
        let v = marcar(&[
            ("wifi.red_24", nodo("", "0", Some("0"))),
            ("wifi.invitados", nodo("", "0", Some("0"))),
            ("wifi.red_5", nodo("", "1", Some("1"))),
            ("wifi.radio_24", nodo("radio_", "1", Some("1"))),
            ("wifi.radio_5", nodo("radio_", "1", Some("1"))),
        ]);
        assert_eq!(de(&v, "wifi.red_24").red_wifi, Some(EmisionRed::Apagada));
        assert_eq!(de(&v, "wifi.red_5").red_wifi, Some(EmisionRed::Encendida));
        assert!(de(&v, "wifi.radio_24").radio_sin_redes);
        assert!(!de(&v, "wifi.radio_5").radio_sin_redes, "una red emite");
        assert_eq!(de(&v, "wifi.radio_24").red_wifi, None);
        assert_eq!(de(&v, "wifi.red_24_avanzado").red_wifi, None);
    }

    /// Lo que dice si emite manda sobre lo encendida; sin él, lo encendida.
    #[test]
    fn si_emite_manda_y_sin_el_vale_lo_encendida() {
        let v = marcar(&[
            ("wifi.red_24", nodo("", "1", Some("0"))),
            ("wifi.red_5", nodo("", "1", None)),
            ("wifi.radio_24", nodo("radio_", "1", None)),
            ("wifi.radio_5", nodo("radio_", "1", None)),
        ]);
        assert_eq!(de(&v, "wifi.red_24").red_wifi, Some(EmisionRed::Apagada));
        assert_eq!(de(&v, "wifi.red_5").red_wifi, Some(EmisionRed::Encendida));
        assert!(
            !de(&v, "wifi.radio_24").radio_sin_redes,
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
        assert!(
            !de(&v, "wifi.radio_5").radio_sin_redes,
            "la radio está apagada"
        );
    }

    #[test]
    fn sin_lectura_no_hay_insignia() {
        let v = marcar(&[]);
        assert!(v.iter().all(|a| a.red_wifi.is_none() && !a.radio_sin_redes));
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
}
