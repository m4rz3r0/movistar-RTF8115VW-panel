//! Detección de la red del router: qué conexión WAN lleva internet, de qué
//! tipo es, la subred LAN, si hay telefonía (VoIP) o televisión (IPTV) y qué
//! radios WiFi tiene.

use crate::models::{MASCARA_IPTV, MASCARA_VOIP, TipoConexion, WanConn, WifiRadio};
use crate::{Client, Error, read};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Ids de radio que se prueban con `cbGetWifi`.
const RADIOS_A_PROBAR: [&str; 4] = ["0", "1", "2", "3"];

/// Lo que se sabe de la red del router tras [`detectar`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedDetectada {
    /// Nombre de la conexión a internet (`name` de `cbGetWanIPIntf`, p. ej.
    /// `ip2`); `None` si no se pudo decidir.
    pub wan: Option<String>,
    /// Tipo de la conexión a internet.
    pub tipo: Option<TipoConexion>,
    /// Subred LAN, `"a.b.c.0/24"`.
    pub lan: Option<String>,
    /// Dirección del router en la LAN.
    pub router_lan: Option<Ipv4Addr>,
    /// Hay una conexión de telefonía.
    pub voip: bool,
    /// Hay una conexión de televisión.
    pub iptv: bool,
    /// Ids de los radios WiFi (para `cbGetWifi`).
    pub radios: Vec<String>,
    /// Hora (segundos Unix) de la detección.
    pub momento: i64,
}

/// ¿Es una dirección que puede ser de internet (ni privada RFC 1918, ni de
/// bucle local, ni de enlace local)? La red compartida de los operadores
/// (CGNAT, RFC 6598) cuenta.
fn ipv4_de_internet(ip: Ipv4Addr) -> bool {
    !(ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified())
}

/// Conexión a internet entre `conns`. Por orden:
/// 1. la que tiene el `ip_intf_id` de la ruta por defecto (`ruta`);
/// 2. si no, la única conexión de datos (ni IPTV ni VoIP) con una IPv4 que
///    no es privada;
/// 3. si no, ninguna.
pub fn elegir_wan<'a>(conns: &'a [WanConn], ruta: Option<&str>) -> Option<&'a WanConn> {
    if let Some(r) = ruta
        && let Some(c) = conns.iter().find(|c| c.ip_intf_id == r)
    {
        return Some(c);
    }
    let mut candidatas = conns
        .iter()
        .filter(|c| c.es_de_datos() && c.ipv4.is_some_and(ipv4_de_internet));
    let primera = candidatas.next()?;
    candidatas.next().is_none().then_some(primera)
}

/// `"a.b.c.0/24"` de una dirección y su máscara.
fn subred(ip: Ipv4Addr, mascara: Ipv4Addr) -> String {
    let m = u32::from(mascara);
    format!("{}/{}", Ipv4Addr::from(u32::from(ip) & m), m.leading_ones())
}

/// Radios que responden a `cbGetWifi` (sin repetir el mismo radio con
/// dos ids); `None` si no responde ninguno.
async fn radios(c: &Client) -> Option<Vec<String>> {
    let mut ids = Vec::new();
    let mut vistos = Vec::new();
    for id in RADIOS_A_PROBAR {
        let Ok(n) = c.call("cbGetWifi", &[("dpm_cfg_wifi_0i", id)]).await else {
            continue;
        };
        if let Some(r) = WifiRadio::from_node(&n)
            && !vistos.contains(&r.id)
        {
            vistos.push(r.id);
            ids.push(id.to_owned());
        }
    }
    (!ids.is_empty()).then_some(ids)
}

/// Detecta la red del router. Falla si no se pueden leer las conexiones WAN
/// (o la ruta por defecto, salvo que el router rechace la llamada: entonces
/// se elige sin ella). Un fallo al leer la LAN o los radios no hace fallar:
/// la LAN queda sin detectar y los radios son [`read::RADIOS_POR_DEFECTO`].
pub async fn detectar(c: &Client, ahora: i64) -> Result<RedDetectada, Error> {
    let conns = read::wan(c).await?;
    let ruta = match read::ruta_por_defecto(c).await {
        Ok(r) => r,
        Err(Error::CallFailed { .. }) => None,
        Err(e) => return Err(e),
    };
    let wan = elegir_wan(&conns, ruta.as_deref());
    let lan = read::lan_ipv4(c).await.ok().flatten();
    let radios = radios(c)
        .await
        .unwrap_or_else(|| read::RADIOS_POR_DEFECTO.map(String::from).to_vec());
    Ok(RedDetectada {
        wan: wan.map(|w| w.name.clone()),
        tipo: wan.map(WanConn::tipo),
        lan: lan.map(|(ip, m)| subred(ip, m)),
        router_lan: lan.map(|(ip, _)| ip),
        voip: conns.iter().any(|w| w.app_mask & MASCARA_VOIP != 0),
        iptv: conns.iter().any(|w| w.app_mask & MASCARA_IPTV != 0),
        radios,
        momento: ahora,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientConfig;
    use crate::xml::parse_response;
    use router_sim::{Sim, SimConfig, SimHandle};

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
    const DHCP_SIN_VOIP: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures-variantes/dhcp-sin-voip"
    );

    fn conns() -> Vec<WanConn> {
        let raw = std::fs::read_to_string(format!("{FIXTURES}/cbGetWanIPIntf.xml")).unwrap();
        WanConn::parse_all(&parse_response("cbGetWanIPIntf", &raw).unwrap())
    }

    fn cliente(sim: &SimHandle) -> Client {
        let mut cfg = ClientConfig::new(&sim.base_url, "u", "clave-de-prueba");
        cfg.min_gap = std::time::Duration::from_millis(1);
        Client::new(cfg)
    }

    #[test]
    fn elegir_wan_por_ruta_por_defecto() {
        let conns = conns();
        assert_eq!(elegir_wan(&conns, Some("2")).unwrap().name, "ip2");
        // La ruta manda aunque haya otra candidata.
        assert_eq!(elegir_wan(&conns, Some("4")).unwrap().name, "ip4");
    }

    #[test]
    fn elegir_wan_sin_ruta_usa_ip_no_privada() {
        let conns = conns();
        // ip3 (VoIP, 10.255.0.x) e ip4 (IPTV) no cuentan.
        assert_eq!(elegir_wan(&conns, None).unwrap().name, "ip2");
        // Una ruta a una interfaz que no es WAN tampoco.
        assert_eq!(elegir_wan(&conns, Some("1")).unwrap().name, "ip2");
        // CGNAT (100.64/10) cuenta como dirección de internet.
        let mut cgnat = conns.clone();
        cgnat[0].ipv4 = Some(Ipv4Addr::new(100, 64, 0, 9));
        assert_eq!(elegir_wan(&cgnat, None).unwrap().name, "ip2");
        // Una privada no.
        let mut privada = conns.clone();
        privada[0].ipv4 = Some(Ipv4Addr::new(192, 168, 144, 9));
        assert!(elegir_wan(&privada, None).is_none());
    }

    #[test]
    fn elegir_wan_ambigua_es_none() {
        let mut conns = conns();
        let mut otra = conns[0].clone();
        otra.name = "ip5".into();
        otra.ip_intf_id = "5".into();
        otra.ipv4 = Some(Ipv4Addr::new(198, 51, 100, 20));
        conns.push(otra);
        assert!(elegir_wan(&conns, None).is_none());
        assert!(elegir_wan(&[], None).is_none());
    }

    #[test]
    fn subred_de_la_lan() {
        let ip = Ipv4Addr::new(192, 0, 2, 77);
        assert_eq!(subred(ip, Ipv4Addr::new(255, 255, 255, 0)), "192.0.2.0/24");
        let ip = Ipv4Addr::new(198, 51, 100, 20);
        assert_eq!(
            subred(ip, Ipv4Addr::new(255, 255, 254, 0)),
            "198.51.100.0/23"
        );
    }

    /// Subred LAN de las fixtures: la del router de fábrica con máscara /24.
    fn lan_de_las_fixtures() -> Option<String> {
        Some(subred(
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
        ))
    }

    #[tokio::test]
    async fn detectar_con_sim() {
        let sim = Sim::start(SimConfig::new("u", "clave-de-prueba", FIXTURES)).await;
        let red = detectar(&cliente(&sim), 1234).await.unwrap();
        assert_eq!(red.wan.as_deref(), Some("ip2"));
        assert_eq!(red.tipo, Some(TipoConexion::Pppoe));
        assert_eq!(red.lan, lan_de_las_fixtures());
        assert_eq!(red.router_lan, Some(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(red.voip && red.iptv);
        assert_eq!(red.radios, ["0", "1"]);
        assert_eq!(red.momento, 1234);
    }

    #[tokio::test]
    async fn detectar_variante_dhcp() {
        let sim =
            Sim::start(SimConfig::new("u", "clave-de-prueba", FIXTURES).con_capa(DHCP_SIN_VOIP))
                .await;
        let red = detectar(&cliente(&sim), 0).await.unwrap();
        assert_eq!(red.wan.as_deref(), Some("ip5"));
        assert_eq!(red.tipo, Some(TipoConexion::Dhcp));
        assert!(!red.voip && !red.iptv);
        assert_eq!(red.lan, lan_de_las_fixtures());
    }

    #[tokio::test]
    async fn detectar_sin_lan_ni_radios_no_falla() {
        let sim = Sim::start(SimConfig::new("u", "clave-de-prueba", FIXTURES)).await;
        sim.control.fail_reads("cbGetLanIPv4Addr", true);
        sim.control.fail_reads("cbGetWifi", true);
        let red = detectar(&cliente(&sim), 0).await.unwrap();
        assert_eq!(red.wan.as_deref(), Some("ip2"));
        assert_eq!(red.lan, None);
        assert_eq!(red.radios, read::RADIOS_POR_DEFECTO);
    }
}
