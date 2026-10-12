//! Redes IP de la configuración (`RP_TRUSTED_PROXY`, `RP_INSTALL_NETS`): una
//! red en notación CIDR (`192.0.2.0/24`, `fd00::/8`) o una IP sola, que vale
//! como red de una sola dirección (/32 o /128).

use std::{net::IpAddr, str::FromStr};

/// Red IP: dirección base (con los bits de equipo a cero) y longitud del
/// prefijo. Una red IPv4 escrita como IPv4 mapeada en IPv6 se guarda como
/// IPv4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RedIp {
    base: IpAddr,
    prefijo: u8,
}

impl RedIp {
    /// ¿Está `ip` en la red? Una IPv4 mapeada en IPv6 se compara como IPv4.
    pub fn contiene(&self, ip: IpAddr) -> bool {
        match (self.base, ip.to_canonical()) {
            (IpAddr::V4(base), IpAddr::V4(ip)) => {
                u32::from(ip) & mascara_v4(self.prefijo) == u32::from(base)
            }
            (IpAddr::V6(base), IpAddr::V6(ip)) => {
                u128::from(ip) & mascara_v6(self.prefijo) == u128::from(base)
            }
            _ => false,
        }
    }
}

fn mascara_v4(prefijo: u8) -> u32 {
    u32::MAX.checked_shl(32 - u32::from(prefijo)).unwrap_or(0)
}

fn mascara_v6(prefijo: u8) -> u128 {
    u128::MAX.checked_shl(128 - u32::from(prefijo)).unwrap_or(0)
}

impl FromStr for RedIp {
    type Err = String;

    /// `192.0.2.0/24`, `fd00::/8` o una IP sola (`127.0.0.1`, `::1`), que es
    /// una red de una dirección. El error no repite el texto.
    fn from_str(s: &str) -> Result<RedIp, String> {
        let (ip, prefijo) = match s.split_once('/') {
            Some((ip, p)) => (ip, Some(p)),
            None => (s, None),
        };
        let base: IpAddr = ip
            .parse()
            .map_err(|_| "no es una IP ni una red (p. ej. 192.0.2.1 o 192.0.2.0/24)".to_owned())?;
        let max = if base.is_ipv4() { 32 } else { 128 };
        let prefijo = match prefijo {
            None => max,
            Some(p) => p
                .parse::<u8>()
                .ok()
                .filter(|p| *p <= max)
                .ok_or_else(|| format!("el prefijo de la red debe ir de 0 a {max}"))?,
        };
        // Una red IPv4 escrita como mapeada (`::ffff:a.b.c.d/n`, n ≥ 96).
        let (base, prefijo) = match (base, base.to_canonical()) {
            (IpAddr::V6(_), IpAddr::V4(v4)) if prefijo >= 96 => (IpAddr::V4(v4), prefijo - 96),
            _ => (base, prefijo),
        };
        let base = match base {
            IpAddr::V4(b) => IpAddr::V4((u32::from(b) & mascara_v4(prefijo)).into()),
            IpAddr::V6(b) => IpAddr::V6((u128::from(b) & mascara_v6(prefijo)).into()),
        };
        Ok(RedIp { base, prefijo })
    }
}

/// Lista de redes separadas por comas (con espacios opcionales). Ningún
/// elemento puede estar vacío. El error dice qué elemento falla (contando
/// desde 1) sin repetir su valor.
pub fn lista(texto: &str) -> Result<Vec<RedIp>, String> {
    texto
        .split(',')
        .map(str::trim)
        .enumerate()
        .map(|(i, s)| {
            if s.is_empty() {
                return Err(format!("el elemento {} está vacío", i + 1));
            }
            s.parse().map_err(|e| format!("el elemento {}: {e}", i + 1))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red(s: &str) -> RedIp {
        s.parse().unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn red_ip_contiene_v4_v6_y_mapeadas() {
        let ocho = red("127.0.0.0/8");
        assert!(ocho.contiene(ip("127.0.0.1")));
        assert!(ocho.contiene(ip("127.255.255.255")));
        assert!(!ocho.contiene(ip("192.0.2.1")));
        let doc = red("198.51.100.0/24");
        assert!(doc.contiene(ip("198.51.100.1")));
        assert!(doc.contiene(ip("198.51.100.255")));
        assert!(!doc.contiene(ip("203.0.113.1")));
        // IPv4 mapeada en IPv6 cuenta como IPv4.
        assert!(doc.contiene(ip("::ffff:198.51.100.3")));
        assert!(!doc.contiene(ip("::1")));

        // Una IP sola es una red de una dirección.
        let local = red("127.0.0.1");
        assert!(local.contiene(ip("127.0.0.1")));
        assert!(local.contiene(ip("::ffff:127.0.0.1")));
        assert!(!local.contiene(ip("127.0.0.2")));
        let uno = red("::1");
        assert!(uno.contiene(ip("::1")));
        assert!(!uno.contiene(ip("::2")));
        assert!(!uno.contiene(ip("127.0.0.1")));

        let ula = red("fd00::/8");
        assert!(ula.contiene(ip("fdff::1")));
        assert!(!ula.contiene(ip("fe80::1")));
        assert!(!ula.contiene(ip("192.0.2.1")));
        let doc6 = red("2001:db8::/32");
        assert!(doc6.contiene(ip("2001:db8:ffff::1")));
        assert!(!doc6.contiene(ip("fd00::1")));

        // Los bits de equipo de la base no cuentan; /0 lo abarca todo.
        assert!(red("192.0.2.77/24").contiene(ip("192.0.2.1")));
        assert!(red("0.0.0.0/0").contiene(ip("203.0.113.9")));
        // Una red IPv4 escrita como mapeada se compara como IPv4.
        assert!(red("::ffff:192.0.2.0/120").contiene(ip("192.0.2.9")));
    }

    #[test]
    fn red_ip_no_valida() {
        for s in [
            "",
            "nada",
            "192.0.2.0/",
            "192.0.2.0/33",
            "fd00::/129",
            "192.0.2.0/x",
            "/8",
        ] {
            assert!(s.parse::<RedIp>().is_err(), "{s}");
        }
    }

    #[test]
    fn lista_separada_por_comas() {
        let l = lista("127.0.0.1, 198.51.100.0/24,::1").unwrap();
        assert_eq!(l, [red("127.0.0.1"), red("198.51.100.0/24"), red("::1")]);
        for s in ["", " ", "127.0.0.1,", ",127.0.0.1", "127.0.0.1,,::1"] {
            let e = lista(s).unwrap_err();
            assert!(e.contains("vacío"), "{s}: {e}");
        }
        // El error dice qué elemento falla, sin repetir su valor.
        let e = lista("127.0.0.1, 192.0.2.300").unwrap_err();
        assert!(e.contains("elemento 2"), "{e}");
        assert!(!e.contains("192.0.2.300"), "{e}");
    }
}
