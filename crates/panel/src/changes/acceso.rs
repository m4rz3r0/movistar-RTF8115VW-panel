//! Si un cambio se vigila: según lo que escribe (el `acceso` de cada
//! función en el catálogo, [`catalogo::Acceso`]) y, en la WiFi, según por
//! dónde llega el panel al router.
//!
//! - Algo que puede cortar el acceso del panel al router o la conexión a
//!   internet (o una función sin clasificar): se vigila, como siempre.
//! - Solo cosas que no lo cortan (cuentas, voz, nombres…): sin vigilancia;
//!   el cambio queda aplicado en cuanto la relectura muestra lo nuevo.
//! - La WiFi: sin vigilancia solo si el panel comprueba que llega al router
//!   por cable (su IP está en la lista de dispositivos del router y su MAC
//!   no está asociada a ninguna red WiFi). Por WiFi, o si no se puede
//!   saber, se vigila: una clave o un canal nuevos podrían dejarlo fuera.

use super::*;
use catalogo::Acceso;
use hgu::models::LanHost;
use std::{collections::HashSet, net::IpAddr};

/// Cuánto vale lo averiguado sobre el enlace del panel.
const VIDA_ENLACE: Duration = Duration::from_secs(60);

/// Por dónde llega el panel al router.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Enlace {
    Cable,
    Wifi,
    /// No se pudo saber (el panel no está en la lista del router, no se
    /// pudo leer…).
    Desconocido,
}

/// Lo que piden las funciones de un cambio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Necesidad {
    Siempre,
    Nunca,
    /// Solo si el panel no va por cable.
    SegunEnlace,
}

/// Lo que piden las funciones con estos `accesos`: basta una que pueda
/// cortar (o sin clasificar) para vigilar; sin ninguna, también.
pub(crate) fn necesidad(accesos: impl IntoIterator<Item = Option<Acceso>>) -> Necesidad {
    let mut n = None;
    for a in accesos {
        n = Some(match (a, n) {
            (None | Some(Acceso::PuedeCortar), _) => return Necesidad::Siempre,
            (Some(Acceso::Wifi), _) | (_, Some(Necesidad::SegunEnlace)) => Necesidad::SegunEnlace,
            (Some(Acceso::NoCorta), _) => Necesidad::Nunca,
        });
    }
    n.unwrap_or(Necesidad::Siempre)
}

/// ¿Se vigila, con esta necesidad y este enlace?
pub(crate) fn vigilar(n: Necesidad, enlace: Enlace) -> bool {
    match n {
        Necesidad::Siempre => true,
        Necesidad::Nunca => false,
        Necesidad::SegunEnlace => enlace != Enlace::Cable,
    }
}

/// El enlace del panel: `ip`, su IP hacia el router; `hosts`, la lista de
/// dispositivos del router; `wifi`, las MAC asociadas a sus redes WiFi (en
/// minúsculas). Lo que falte, `Desconocido`.
pub(crate) fn enlace_de(
    ip: Option<IpAddr>,
    hosts: Option<&[LanHost]>,
    wifi: Option<&HashSet<String>>,
) -> Enlace {
    let (Some(ip), Some(hosts), Some(wifi)) = (ip, hosts, wifi) else {
        return Enlace::Desconocido;
    };
    let ip = ip.to_string();
    match hosts.iter().find(|h| h.active && h.ip == ip) {
        None => Enlace::Desconocido,
        Some(h) if wifi.contains(&h.mac.to_ascii_lowercase()) => Enlace::Wifi,
        Some(_) => Enlace::Cable,
    }
}

impl ChangeManager {
    /// ¿Se vigila un cambio que escribe las funciones `nombres`? Solo lee
    /// el router si hace falta saber el enlace.
    pub(crate) async fn se_vigila<'a>(&self, nombres: impl IntoIterator<Item = &'a str>) -> bool {
        if self.siempre.load(std::sync::atomic::Ordering::SeqCst) {
            return true;
        }
        let n = necesidad(
            nombres
                .into_iter()
                .map(|f| self.state.catalogo.funcion(f).and_then(|f| f.acceso)),
        );
        match n {
            Necesidad::SegunEnlace => vigilar(n, self.enlace().await),
            n => vigilar(n, Enlace::Desconocido),
        }
    }

    /// Por dónde llega el panel al router (lo averiguado vale
    /// [`VIDA_ENLACE`]).
    pub async fn enlace(&self) -> Enlace {
        if let Some((cuando, e)) = *lock(&self.enlace)
            && cuando.elapsed() < VIDA_ENLACE
        {
            return e;
        }
        let e = self.averiguar_enlace().await;
        tracing::debug!("enlace del panel con el router: {e:?}");
        *lock(&self.enlace) = Some((Instant::now(), e));
        e
    }

    async fn averiguar_enlace(&self) -> Enlace {
        let c = &self.state.hgu;
        let Some(ip) = c.ip_local().await else {
            return Enlace::Desconocido;
        };
        let hosts = read::lan_hosts(c).await.ok();
        let mut wifi = Some(HashSet::new());
        for radio in read::RADIOS_POR_DEFECTO {
            match read::wifi_stations(c, radio).await {
                Ok(st) => {
                    if let Some(w) = wifi.as_mut() {
                        w.extend(st.into_iter().map(|s| s.mac.to_ascii_lowercase()));
                    }
                }
                Err(_) => wifi = None,
            }
        }
        enlace_de(Some(ip), hosts.as_deref(), wifi.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn necesidad_de_un_grupo() {
        use Acceso::*;
        assert_eq!(necesidad([Some(NoCorta)]), Necesidad::Nunca);
        assert_eq!(
            necesidad([Some(NoCorta), Some(Wifi)]),
            Necesidad::SegunEnlace
        );
        assert_eq!(
            necesidad([Some(Wifi), Some(NoCorta)]),
            Necesidad::SegunEnlace
        );
        assert_eq!(
            necesidad([Some(NoCorta), Some(Wifi), Some(PuedeCortar)]),
            Necesidad::Siempre
        );
        // Sin clasificar o sin funciones: ante la duda, se vigila.
        assert_eq!(necesidad([Some(NoCorta), None]), Necesidad::Siempre);
        assert_eq!(necesidad([]), Necesidad::Siempre);
    }

    #[test]
    fn la_wifi_solo_sin_vigilancia_por_cable() {
        for e in [Enlace::Cable, Enlace::Wifi, Enlace::Desconocido] {
            assert!(vigilar(Necesidad::Siempre, e));
            assert!(!vigilar(Necesidad::Nunca, e));
        }
        assert!(!vigilar(Necesidad::SegunEnlace, Enlace::Cable));
        assert!(vigilar(Necesidad::SegunEnlace, Enlace::Wifi));
        assert!(vigilar(Necesidad::SegunEnlace, Enlace::Desconocido));
    }

    #[test]
    fn enlace_por_la_lista_del_router() {
        let host = |ip: &str, mac: &str, active| LanHost {
            ip: ip.into(),
            mac: mac.into(),
            hostname: String::new(),
            iface: String::new(),
            active,
        };
        let hosts = vec![
            host("192.168.1.61", "02:00:00:00:00:0A", true),
            host("192.168.1.62", "02:00:00:00:00:0B", true),
            host("192.168.1.63", "02:00:00:00:00:0C", false),
        ];
        let wifi: HashSet<String> = ["02:00:00:00:00:0b".to_owned()].into();
        let ip = |s: &str| Some(s.parse::<IpAddr>().unwrap());
        let e = |i| enlace_de(i, Some(&hosts), Some(&wifi));
        assert_eq!(e(ip("192.168.1.61")), Enlace::Cable);
        assert_eq!(e(ip("192.168.1.62")), Enlace::Wifi);
        // Desconectado o fuera de la lista (127.0.0.1 con el simulador).
        assert_eq!(e(ip("192.168.1.63")), Enlace::Desconocido);
        assert_eq!(e(ip("127.0.0.1")), Enlace::Desconocido);
        assert_eq!(e(None), Enlace::Desconocido);
        // Sin la lista de la WiFi no se sabe si va por cable.
        assert_eq!(
            enlace_de(ip("192.168.1.61"), Some(&hosts), None),
            Enlace::Desconocido
        );
        assert_eq!(
            enlace_de(ip("192.168.1.61"), None, Some(&wifi)),
            Enlace::Desconocido
        );
    }

    /// Cada escritura y acción del catálogo tiene su clasificación, y las
    /// del día a día quedan donde deben.
    #[test]
    fn todas_las_escrituras_clasificadas() {
        let c = Catalogo::load().unwrap();
        let sin: Vec<&str> = c
            .todas()
            .iter()
            .filter(|f| f.tipo != Tipo::Lectura && f.acceso.is_none())
            .map(|f| f.name.as_str())
            .collect();
        assert!(sin.is_empty(), "sin clasificar: {sin:?}");
        let n = |nombres: &[&str]| {
            necesidad(nombres.iter().map(|f| c.funcion(f).and_then(|f| f.acceso)))
        };
        assert_eq!(n(&["cbAddUser"]), Necesidad::Nunca);
        assert_eq!(n(&["cbDeleteUser"]), Necesidad::Nunca);
        assert_eq!(n(&["cbSetVoiceLine"]), Necesidad::Nunca);
        assert_eq!(n(&["cbSetTr69Config"]), Necesidad::Nunca);
        assert_eq!(n(&["cbSetWifiAccPt"]), Necesidad::SegunEnlace);
        assert_eq!(n(&["te_wifi.wl"]), Necesidad::SegunEnlace);
        for f in [
            "cbSetLanIPv4Addr",
            "cbSetDhcpv4ServerPool",
            "cbDhcpV4SvrPoolSetStatic",
            "cbReboot",
            "cbSetWanIPIntf",
            "scbidirectionalflt.cmd?action=add",
        ] {
            assert_eq!(n(&[f]), Necesidad::Siempre, "{f}");
        }
        assert_eq!(n(&["cbAddUser", "cbSetWifiAccPt"]), Necesidad::SegunEnlace);
        assert_eq!(n(&["cbAddUser", "cbSetDnsRelayServer"]), Necesidad::Siempre);
    }
}
