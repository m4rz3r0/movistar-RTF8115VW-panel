//! DNS propios: un valor que no es del router ni del operador.

use crate::modelo::Campo;
use std::net::Ipv4Addr;

/// ¿Es `valor` un DNS propio en el campo `c` (que declara los resolutores
/// del operador en `del_operador`)? No lo es el propio router (vacío,
/// `0.0.0.0` o su IP de casa, `ip_router`) ni uno del operador; cualquier
/// otro (un resolutor de casa, uno público que no está en la lista), sí.
/// `None` si el campo no declara `del_operador` o, sin la IP del router, no
/// se puede saber.
pub fn dns_propio(c: &Campo, valor: &str, ip_router: Option<Ipv4Addr>) -> Option<bool> {
    if c.del_operador.is_empty() {
        return None;
    }
    let v = valor.trim();
    let ip = v.parse::<Ipv4Addr>().ok();
    if v.is_empty()
        || ip.is_some_and(|ip| ip.is_unspecified())
        || c.del_operador.iter().any(|d| d.trim() == v)
    {
        return Some(false);
    }
    let router = ip_router?;
    Some(ip != Some(router))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CatalogoAjustes;

    #[test]
    fn propio_si_no_es_del_router_ni_del_operador() {
        let cat = CatalogoAjustes::load().unwrap();
        let a = cat.ajuste("red_local.dns_casa").unwrap();
        let principal = a.campo("principal").unwrap();
        let router = Some(Ipv4Addr::new(192, 168, 1, 1));
        let propio = |v: &str| dns_propio(principal, v, router);
        for v in ["192.168.1.70", "192.0.2.53", "9.9.9.9"] {
            assert_eq!(propio(v), Some(true), "{v}");
        }
        for v in [
            "",
            " ",
            "0.0.0.0",
            "192.168.1.1",
            "80.58.61.250",
            "80.58.61.254",
        ] {
            assert_eq!(propio(v), Some(false), "{v}");
        }
        // Sin la IP del router no se sabe, salvo lo que nunca es propio.
        assert_eq!(dns_propio(principal, "192.168.1.70", None), None);
        assert_eq!(dns_propio(principal, "80.58.61.250", None), Some(false));
        // Un campo sin `del_operador`.
        let secundario = a.campo("secundario").unwrap();
        assert_eq!(dns_propio(secundario, "192.168.1.70", router), None);
    }
}
