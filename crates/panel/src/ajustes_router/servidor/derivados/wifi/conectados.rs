//! Los dispositivos conectados a cada red: cuántos y su nombre en Red local.

use super::*;
use crate::ajustes_router::reservas::mac_comparable;

/// Estaciones asociadas a la radio de la red `e` y el id de su punto de
/// acceso; `None` si `e` no es una red o no se pudo leer la lista.
fn asociados<'a>(ctx: &Contexto<'a>, e: &Elemento) -> Option<(&'a XmlNode, &'a str)> {
    let (red, radio) = red_de(e)?;
    let punto = ctx.ajuste(red)?.fijos.get(ID_PUNTO)?;
    let id_radio = id_de_radio(ctx.del_tema, radio)?;
    let lista = ctx.leida(&Clave::new(ASOCIADOS, &[(ID_RADIO, id_radio)]))?;
    Some((lista, punto.as_str()))
}

/// `derivado:conectados`: estaciones asociadas al punto de acceso de una
/// red, de la lista de su radio. `None` si no se pudo leer.
pub(in super::super) fn conectados(ctx: &Contexto, e: &Elemento) -> Option<u32> {
    let (lista, punto) = asociados(ctx, e)?;
    let n = hgu::models::WifiAssoc::parse_all(lista)
        .into_iter()
        .filter(|st| st.ap_id == punto)
        .count();
    Some(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Los dispositivos conectados a una red (el pie de su tarjeta), con su
/// nombre si Red local lo da. Vacío si no es una red o no se pudo leer.
pub(in super::super) fn dispositivos(ctx: &Contexto, e: &Elemento) -> Vec<Option<String>> {
    let Some((lista, punto)) = asociados(ctx, e) else {
        return Vec::new();
    };
    let hosts = ctx.leida(&super::super::super::red_local::clave_hosts());
    dispositivos_del_punto(lista, hosts, punto)
}

/// Dispositivos conectados al punto de acceso `punto` según las estaciones
/// de su radio (`asociados`), con el nombre que les da la lista de Red
/// local (`hosts`); `None` si no lo da o no se pudo leer. Pura.
pub(super) fn dispositivos_del_punto(
    asociados: &XmlNode,
    hosts: Option<&XmlNode>,
    punto: &str,
) -> Vec<Option<String>> {
    let nombres: HashMap<String, String> = hosts
        .map(hgu::models::LanHost::parse_all)
        .unwrap_or_default()
        .into_iter()
        .filter(|h| !h.hostname.trim().is_empty())
        .map(|h| (mac_comparable(&h.mac), h.hostname.trim().to_owned()))
        .collect();
    hgu::models::WifiAssoc::parse_all(asociados)
        .into_iter()
        .filter(|st| st.ap_id == punto)
        .map(|st| nombres.get(&mac_comparable(&st.mac)).cloned())
        .collect()
}
