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
fn dispositivos_del_punto(
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Estaciones asociadas a la radio: (punto de acceso, MACs).
    fn asociadas(grupos: &[(&str, &[&str])]) -> XmlNode {
        let grupos: String = grupos
            .iter()
            .map(|(punto, macs)| {
                let estaciones: String = macs
                    .iter()
                    .map(|m| format!("<wifi_association><mac_addr>{m}</mac_addr></wifi_association>"))
                    .collect();
                format!(
                    "<wifi_association_group><dpm_cfg_wifi_acc_pt_id>{punto}</dpm_cfg_wifi_acc_pt_id>\
                     {estaciones}</wifi_association_group>"
                )
            })
            .collect();
        let raw = format!(
            "<Response><cbListWifiAccPtAssociation>{grupos}<Result>0</Result>\
             </cbListWifiAccPtAssociation></Response>"
        );
        hgu::xml::parse_response("cbListWifiAccPtAssociation", &raw).unwrap()
    }

    /// Lista de dispositivos de Red local: (MAC, nombre).
    fn equipos(hosts: &[(&str, &str)]) -> XmlNode {
        let hosts: String = hosts
            .iter()
            .map(|(mac, nombre)| {
                format!(
                    "<rtm_cfg_host><mac_addr>{mac}</mac_addr><hostname>{nombre}</hostname>\
                     <ipv4_addr>192.168.1.70</ipv4_addr><is_active>1</is_active></rtm_cfg_host>"
                )
            })
            .collect();
        let raw = format!(
            "<Response><cbGetLanHostList>{hosts}<Result>0</Result></cbGetLanHostList></Response>"
        );
        hgu::xml::parse_response("cbGetLanHostList", &raw).unwrap()
    }

    /// Los dispositivos de una red son las estaciones de su punto de acceso,
    /// con el nombre que les da Red local (sin distinguir mayúsculas en la
    /// MAC); sin nombre, o sin la lista de Red local, `None`. Las de otro
    /// punto de acceso de la misma radio no cuentan.
    #[test]
    fn dispositivos_de_una_red_con_su_nombre() {
        let radio = asociadas(&[
            (
                "0",
                &[
                    "02:00:00:00:01:01",
                    "02:00:00:00:01:02",
                    "02:00:00:00:01:03",
                ],
            ),
            ("2", &["02:00:00:00:02:01"]),
        ]);
        let hosts = equipos(&[
            ("02:00:00:00:01:01", "Movil de Ana"),
            ("02:00:00:00:01:0a", "otro"),
            ("02:00:00:00:01:02", ""),
            ("02:00:00:00:02:01", "Invitado"),
        ]);
        let hosts_mayus = equipos(&[("02:00:00:00:01:01".to_uppercase().as_str(), "Movil de Ana")]);
        assert_eq!(
            dispositivos_del_punto(&radio, Some(&hosts), "0"),
            [Some("Movil de Ana".to_owned()), None, None]
        );
        assert_eq!(
            dispositivos_del_punto(&radio, Some(&hosts_mayus), "0")[0],
            Some("Movil de Ana".to_owned())
        );
        assert_eq!(
            dispositivos_del_punto(&radio, Some(&hosts), "2"),
            [Some("Invitado".to_owned())]
        );
        assert_eq!(dispositivos_del_punto(&radio, None, "2"), [None]);
        assert!(dispositivos_del_punto(&radio, Some(&hosts), "1").is_empty());
    }
}
