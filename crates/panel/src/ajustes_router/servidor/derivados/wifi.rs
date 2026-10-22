//! Derivados de la WiFi: si emite cada red y cada radio, canal y ancho,
//! conectados, la nota de la radio y la franja de resumen.
//!
//! El estado viene de la Tarea W1: la insignia de cada red y la nota de la
//! radio encendida que no emite ninguna o que emite redes que el panel no
//! muestra.
//!
//! Comprobado en el router real (2026-10-22): `adm_state` de un punto de
//! acceso enciende o apaga la red, `opr_status` dice si de verdad emite, y
//! la radio lleva los suyos (`radio_adm_state`, `radio_opr_status`).

use super::super::lectura::{hojas, instancias};
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
fn encendida_y_emite(n: &XmlNode, adm: &str, opr: &str) -> (Option<bool>, Option<bool>) {
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
            .and_then(|n| encendida_y_emite(n, "radio_adm_state", "radio_opr_status").1)
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
        let (encendida, emite) = nodo.map_or((None, None), |n| {
            encendida_y_emite(n, "adm_state", "opr_status")
        });
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
        // Alguna de sus redes no se pudo leer y ninguna de las otras emite:
        // no se sabe si emite alguna.
        let sin_leer = redes.iter().any(Option::is_none) && !redes.contains(&Some(true));
        // Sin la lista no se sabe si emite otra red: solo esa nota.
        let Some(lista) = listas
            .iter()
            .find(|(r, _)| r == id_radio)
            .and_then(|(_, n)| *n)
        else {
            vistas[j].nota_radio = sin_leer.then_some(NotaRadio::RedesSinLeer);
            continue;
        };
        let vistos = puntos_de.get(id_radio).map_or(&[][..], Vec::as_slice);
        let ocultas = ocultas_que_emiten(lista, vistos);
        vistas[j].nota_radio = if ocultas > 0 {
            Some(NotaRadio::Ocultas(ocultas))
        } else if redes.iter().all(|r| *r == Some(false)) {
            Some(NotaRadio::SinRedes)
        } else if sin_leer {
            Some(NotaRadio::RedesSinLeer)
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

/// Lectura de las estaciones asociadas a la radio `id` (la misma que usa
/// Red local: comparten la caché).
const ASOCIADOS: &str = "cbListWifiAccPtAssociation";

/// Lecturas extra del tema WiFi: la lista de puntos de acceso y la de
/// estaciones asociadas de cada radio. Vacío fuera del tema.
pub(super) fn claves_extra(del_tema: &[&Ajuste]) -> Vec<Clave> {
    let listas = claves_de_listas(del_tema);
    let asociados: Vec<Clave> = listas
        .iter()
        .filter_map(|(radio, _)| id_de_radio(del_tema, radio))
        .map(|id| Clave::new(ASOCIADOS, &[(ID_RADIO, id)]))
        .collect();
    listas
        .into_iter()
        .map(|(_, c)| c)
        .chain(asociados)
        .collect()
}

/// Id de la radio cuyo ajuste es `radio`.
fn id_de_radio<'a>(del_tema: &[&'a Ajuste], radio: &str) -> Option<&'a str> {
    del_tema
        .iter()
        .find(|a| a.id == radio)
        .and_then(|a| a.fijos.get(ID_RADIO))
        .map(String::as_str)
}

/// Red y radio de un elemento que es una red WiFi.
fn red_de(e: &Elemento) -> Option<(&'static str, &'static str)> {
    RADIO_DE_LA_RED
        .iter()
        .copied()
        .find(|(red, _)| e.ajustes.iter().any(|a| a == red))
}

/// Ajuste de la radio de un elemento que es una radio.
fn radio_de(e: &Elemento) -> Option<&'static str> {
    RADIO_DE_LA_RED
        .iter()
        .map(|(_, radio)| *radio)
        .find(|radio| e.ajustes.iter().any(|a| a == radio))
}

/// El elemento es una red WiFi (no una radio).
pub(super) fn es_red(e: &Elemento) -> bool {
    red_de(e).is_some()
}

/// El elemento es una radio.
pub(super) fn es_radio(e: &Elemento) -> bool {
    red_de(e).is_none() && radio_de(e).is_some()
}

/// Estado de la tarjeta de una red o de una radio y la clave de traducción
/// de su texto. Pura.
/// - `ap`: si emite la red (la insignia de la Tarea W1), en la de una red;
/// - `radio`: si emite la radio, en la de una radio;
/// - `ocultos`: lo que no se ve en las redes de la radio (la nota de la
///   Tarea W1), en la de una radio.
pub(super) fn estado_red(
    ap: Option<EmisionRed>,
    radio: Option<bool>,
    ocultos: Option<NotaRadio>,
) -> (EstadoVisual, String) {
    use EstadoVisual as E;
    let (estado, clave) = match (ap, radio, ocultos) {
        (Some(EmisionRed::Encendida), ..) => (E::Emite, "ajustes-wifi-red-encendida"),
        (Some(EmisionRed::Apagada), ..) => (E::Apagada, "ajustes-wifi-red-apagada"),
        (Some(EmisionRed::SinRadio), ..) => (E::Atencion, "ajustes-wifi-red-sin-radio"),
        (Some(EmisionRed::NoEmite), ..) => (E::Atencion, "ajustes-wifi-red-no-emite"),
        (None, Some(false), _) => (E::Apagada, "ajustes-wifi-radio-apagada"),
        (None, Some(true), Some(NotaRadio::SinRedes)) => {
            (E::Atencion, "ajustes-wifi-radio-sin-redes-corto")
        }
        (None, Some(true), Some(NotaRadio::RedesSinLeer)) => {
            (E::Atencion, "ajustes-wifi-radio-redes-sin-leer-corto")
        }
        (None, Some(true), _) => (E::Emite, "ajustes-wifi-red-encendida"),
        (None, None, _) => (E::Atencion, "ajustes-estado-desconocido"),
    };
    (estado, clave.to_owned())
}

/// Si emite la radio según su lectura.
fn radio_emite(n: &XmlNode) -> Option<bool> {
    encendida_y_emite(n, "radio_adm_state", "radio_opr_status").1
}

/// `derivado:estado_red` de un elemento.
pub(super) fn estado(ctx: &Contexto, e: &Elemento) -> (EstadoVisual, String) {
    if let Some((red, _)) = red_de(e) {
        return estado_red(ctx.vista(red).and_then(|v| v.red_wifi), None, None);
    }
    match radio_de(e) {
        Some(radio) => estado_red(
            None,
            ctx.nodo(radio).and_then(radio_emite),
            ctx.vista(radio).and_then(|v| v.nota_radio),
        ),
        None => estado_red(None, None, None),
    }
}

/// `ssid` de la lectura del punto de acceso de una red.
pub(super) fn ssid(ctx: &Contexto, e: &Elemento) -> Option<String> {
    let (red, _) = red_de(e)?;
    let mut leidos = Leidos::new();
    hojas(ctx.nodo(red)?, &mut leidos);
    leidos.remove("ssid").filter(|s| !s.is_empty())
}

/// Ancho de banda en MHz de un código del router.
fn mhz(codigo: &str) -> Option<&'static str> {
    match codigo.trim() {
        "1" => Some("20"),
        "2" => Some("40"),
        "3" => Some("80"),
        "4" => Some("160"),
        _ => None,
    }
}

/// Valor efectivo (`opr_…`) o, si no lo trae o vale 0, el configurado
/// (`adm_…`).
fn efectivo<'l>(leidos: &'l Leidos, opr: &str, adm: &str) -> Option<&'l str> {
    let valido = |v: &&String| !v.trim().is_empty() && v.trim() != "0";
    leidos
        .get(opr)
        .filter(valido)
        .or_else(|| leidos.get(adm))
        .map(|v| v.trim())
}

/// `derivado:canal_y_ancho`: el canal y el ancho con que emite la radio de
/// una red («Canal 6 · 20 MHz», «Canal automático · 80 MHz»).
pub(super) fn canal_y_ancho(ctx: &Contexto, e: &Elemento) -> Option<ValorLegible> {
    let (_, radio) = red_de(e)?;
    let mut leidos = Leidos::new();
    hojas(ctx.nodo(radio)?, &mut leidos);
    let canal = match efectivo(&leidos, "opr_channel", "adm_channel") {
        None | Some("0") => t!("ajustes-dato-canal-auto"),
        Some(c) => t!("ajustes-dato-canal-numero", canal = c.to_owned()),
    };
    let texto = match efectivo(&leidos, "opr_bandwidth", "adm_bandwidth").and_then(mhz) {
        Some(ancho) => t!("ajustes-dato-canal-y-ancho", canal = canal, ancho = ancho),
        None => canal,
    };
    Some(ValorLegible::Texto(texto))
}

/// `derivado:conectados`: estaciones asociadas al punto de acceso de una
/// red, de la lista de su radio. `None` si no se pudo leer.
pub(super) fn conectados(ctx: &Contexto, e: &Elemento) -> Option<u32> {
    let (red, radio) = red_de(e)?;
    let punto = ctx.ajuste(red)?.fijos.get(ID_PUNTO)?;
    let id_radio = id_de_radio(ctx.del_tema, radio)?;
    let lista = ctx.leida(&Clave::new(ASOCIADOS, &[(ID_RADIO, id_radio)]))?;
    let n = hgu::models::WifiAssoc::parse_all(lista)
        .into_iter()
        .filter(|st| st.ap_id == *punto)
        .count();
    Some(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Nota del pie de una radio, ya traducida: no emite ninguna red o emite
/// algunas que el panel no muestra.
pub(super) fn nota(ctx: &Contexto, e: &Elemento) -> Option<String> {
    let radio = radio_de(e).filter(|_| es_radio(e))?;
    Some(match ctx.vista(radio)?.nota_radio? {
        NotaRadio::SinRedes => t!("ajustes-wifi-radio-sin-redes"),
        NotaRadio::Ocultas(n) => t!("ajustes-wifi-radio-ocultas", n = n),
        NotaRadio::RedesSinLeer => t!("ajustes-wifi-radio-redes-sin-leer"),
    })
}

/// `apagada desde` de una red: si está apagada y lo último que hizo el
/// panel con ella fue apagarla, cuándo.
pub(super) fn apagada_desde(ctx: &Contexto, e: &Elemento) -> Option<i64> {
    let (red, _) = red_de(e)?;
    if ctx.vista(red)?.red_wifi != Some(EmisionRed::Apagada) {
        return None;
    }
    let punto = ctx.ajuste(red)?.fijos.get(ID_PUNTO)?;
    ctx.apagadas.get(punto).copied()
}

/// Un paso aplicado que escribe en un punto de acceso: los valores que
/// envió (con su id) y su diff (JSON), y cuándo se aplicó su cambio.
pub(super) struct PasoDeRed {
    pub valores: Leidos,
    pub diff: String,
    pub cuando: i64,
}

/// Desde cuándo está apagado cada punto de acceso según los pasos
/// aplicados por el panel, en orden: el último que tocó su `adm_state`
/// manda (si lo apagó, cuándo; si lo encendió, nada). Pura.
pub(super) fn ultimo_apagado(pasos: &[PasoDeRed]) -> HashMap<String, i64> {
    let mut out = HashMap::new();
    for p in pasos {
        let Some(punto) = p.valores.get(ID_PUNTO) else {
            continue;
        };
        let diff: Vec<crate::changes::CampoDiff> =
            serde_json::from_str(&p.diff).unwrap_or_default();
        let Some(d) = diff.iter().find(|d| d.nombre == "adm_state") else {
            continue;
        };
        if d.despues.trim() == "0" {
            out.insert(punto.clone(), p.cuando);
        } else {
            out.remove(punto);
        }
    }
    out
}

/// [`ultimo_apagado`] con los pasos del Historial (los de los cambios
/// aplicados o vigilándose). Sin Historial legible, nada.
pub(in crate::ajustes_router) async fn apagadas_desde(
    db: &sqlx::SqlitePool,
) -> HashMap<String, i64> {
    use sqlx::Row;
    let filas = sqlx::query(
        "SELECT p.valores_nuevos, p.solicitud, p.diff, c.updated FROM cambio_pasos p \
         JOIN changes c ON c.id = p.cambio_id \
         WHERE p.funcion = 'cbSetWifiAccPt' AND p.estado = 'aplicado' \
         AND c.estado IN ('aplicado', 'vigilando') \
         ORDER BY c.updated, p.cambio_id, p.orden",
    )
    .fetch_all(db)
    .await
    .inspect_err(|e| tracing::warn!("ajustes: no se puede leer el Historial: {e}"))
    .unwrap_or_default();
    let pasos: Vec<PasoDeRed> = filas
        .iter()
        .map(|f| {
            let mut valores: Leidos =
                serde_json::from_slice(&f.get::<Vec<u8>, _>("valores_nuevos")).unwrap_or_default();
            let pedidos: Leidos =
                serde_json::from_str(&f.get::<String, _>("solicitud")).unwrap_or_default();
            valores.extend(pedidos);
            PasoDeRed {
                valores,
                diff: f.get("diff"),
                cuando: f.get("updated"),
            }
        })
        .collect();
    ultimo_apagado(&pasos)
}

/// Franja de resumen de la WiFi: redes y radios encendidas, dispositivos
/// conectados y recomendaciones pendientes. `elementos` son los del
/// catálogo con su vista.
pub(super) fn resumen(
    ctx: &Contexto,
    elementos: &[(&Elemento, &ElementoVista)],
) -> Vec<DatoResumen> {
    let de = |n: usize, total: usize| t!("ajustes-resumen-de", n = n, total = total);
    // Una red cuenta si emite; una radio, si está encendida (aunque no
    // emita ninguna red: eso lo dice su tarjeta).
    let cuenta = |filtro: fn(&Elemento) -> bool, si: fn(&ElementoVista) -> bool| {
        let son: Vec<_> = elementos.iter().filter(|(e, _)| filtro(e)).collect();
        de(son.iter().filter(|(_, v)| si(v)).count(), son.len())
    };
    let emite = |v: &ElementoVista| v.estado == EstadoVisual::Emite;
    let encendida = |v: &ElementoVista| v.interruptor.as_ref().is_some_and(|i| i.encendido);
    let conectados: Option<u32> = elementos
        .iter()
        .filter(|(e, _)| es_red(e))
        .map(|(e, _)| conectados(ctx, e))
        .sum();
    let dato = |clave: &str, cifra: String, atencion: bool| {
        dato_resumen(clave, cifra, detalle_fijo(clave), atencion)
    };
    vec![
        dato("ajustes-resumen-wifi-redes", cuenta(es_red, emite), false),
        dato(
            "ajustes-resumen-wifi-radios",
            cuenta(es_radio, encendida),
            false,
        ),
        dato(
            "ajustes-resumen-wifi-conectados",
            conectados.map_or_else(sin_dato, |n| n.to_string()),
            false,
        ),
        recomendaciones(elementos),
    ]
}

#[cfg(test)]
mod tests;
