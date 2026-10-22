//! Si cada red WiFi emite: la insignia de la tarjeta de cada red y la nota
//! de la radio encendida que no emite ninguna o que emite redes que el
//! panel no muestra.
//!
//! Comprobado en el router real (2026-10-22): `adm_state` de un punto de
//! acceso enciende o apaga la red, `opr_status` dice si de verdad emite, y
//! la radio lleva los suyos (`radio_adm_state`, `radio_opr_status`).

use super::lectura::{hojas, instancias};
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

/// Lista de los puntos de acceso de una radio: los que el panel muestra y
/// los demás (el router tiene cuatro por radio).
const LISTA: &str = "cbListWifiAccPt";
/// Id de un punto de acceso en su lectura y en los `fijos` de su ajuste.
const ID_PUNTO: &str = "dpm_cfg_wifi_acc_pt_id";
/// Id de una radio en los `fijos` de su ajuste y en la lista de sus puntos.
const ID_RADIO: &str = "dpm_cfg_wifi_id";

/// Lecturas que hacen falta además de las del tema: la lista de puntos de
/// acceso de cada radio (id del ajuste de la radio y su clave). Vacío fuera
/// del tema WiFi.
pub(super) fn claves_de_listas(del_tema: &[&Ajuste]) -> Vec<(String, Clave)> {
    let mut out: Vec<(String, Clave)> = Vec::new();
    for &(_, radio) in RADIO_DE_LA_RED {
        let Some(a) = del_tema.iter().find(|a| a.id == radio) else {
            continue;
        };
        let Some(id) = a.fijos.get(ID_RADIO) else {
            continue;
        };
        if !out.iter().any(|(r, _)| r == radio) {
            out.push((radio.to_owned(), Clave::new(LISTA, &[(ID_RADIO, id)])));
        }
    }
    out
}

/// Marca en `vistas` (las de `del_tema`, con la lectura de cada una en
/// `nodos`, `None` si falló) si emite cada red y, en cada radio encendida,
/// la nota de lo que no se ve: que no emite ninguna red o cuántas emite que
/// el panel no muestra. `listas` trae la lista de puntos de acceso de cada
/// radio (id de su ajuste; `None` si falló). Fuera del tema WiFi no hace
/// nada.
pub(super) fn marcar_emision(
    del_tema: &[&Ajuste],
    vistas: &mut [AjusteVista],
    nodos: &[Option<&XmlNode>],
    listas: &[(String, Option<&XmlNode>)],
) {
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
    // Por radio, si emite cada una de sus redes y sus puntos de acceso.
    let mut redes_de: BTreeMap<&str, Vec<Option<bool>>> = BTreeMap::new();
    let mut puntos_de: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
    for &(red, id_radio) in RADIO_DE_LA_RED {
        let Some((i, nodo)) = lectura(red) else {
            continue;
        };
        if let Some(p) = del_tema
            .iter()
            .find(|a| a.id == red)
            .and_then(|a| a.fijos.get(ID_PUNTO))
        {
            puntos_de.entry(id_radio).or_default().push(p);
        }
        let (encendida, emite) =
            nodo.map_or((None, None), |n| estado(n, "adm_state", "opr_status"));
        redes_de.entry(id_radio).or_default().push(emite);
        let marca = match (encendida.or(emite), radio(id_radio), emite) {
            (Some(true), Some(false), _) => EmisionRed::SinRadio,
            (_, _, Some(true)) => EmisionRed::Encendida,
            (Some(true), _, Some(false)) => EmisionRed::NoEmite,
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
        if radio(id_radio) != Some(true) {
            continue;
        }
        // Sin la lista no se sabe si emite otra red: ninguna nota.
        let Some(lista) = listas
            .iter()
            .find(|(r, _)| r == id_radio)
            .and_then(|(_, n)| *n)
        else {
            continue;
        };
        let vistos = puntos_de.get(id_radio).map_or(&[][..], Vec::as_slice);
        let ocultas = ocultas_que_emiten(lista, vistos);
        vistas[j].nota_radio = if ocultas > 0 {
            Some(NotaRadio::Ocultas(ocultas))
        } else if redes.iter().all(|r| *r == Some(false)) {
            Some(NotaRadio::SinRedes)
        } else {
            None
        };
    }
}

/// Puntos de acceso de `lista` que emiten y no están en `vistos` (los ids
/// de los que muestra el panel).
fn ocultas_que_emiten(lista: &XmlNode, vistos: &[&String]) -> u32 {
    let elementos = [ID_PUNTO, "adm_state", "opr_status"];
    let emiten = instancias(lista, &elementos).into_iter().filter(|l| {
        let visto = l.get(ID_PUNTO).is_some_and(|id| vistos.contains(&id));
        !visto && si_no(l, "opr_status").or(si_no(l, "adm_state")) == Some(true)
    });
    u32::try_from(emiten.count()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
