//! Utilidades de formato que reproducen el JavaScript del router.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;

/// Ofusca usuario/contraseña como `mess_userpass` del router: XOR 0x1f por carácter.
/// Es involutiva (aplicarla dos veces devuelve el original).
pub fn mess_userpass(s: &str) -> String {
    s.chars()
        .map(|c| char::from_u32(c as u32 ^ 0x1f).unwrap_or(c))
        .collect()
}

/// Une los 4 campos de una IP con `.`.
pub fn join_ip(parts: [&str; 4]) -> String {
    parts.join(".")
}

/// Divide una IP en 4 campos; `None` si no tiene exactamente 4.
pub fn split_ip(s: &str) -> Option<[String; 4]> {
    split_exact(s, '.')
}

/// Une los 6 campos de una MAC con `:`.
pub fn join_mac(parts: [&str; 6]) -> String {
    parts.join(":")
}

/// Divide una MAC en 6 campos; `None` si no tiene exactamente 6.
pub fn split_mac(s: &str) -> Option<[String; 6]> {
    split_exact(s, ':')
}

fn split_exact<const N: usize>(s: &str, sep: char) -> Option<[String; N]> {
    let v: Vec<String> = s.split(sep).map(str::to_owned).collect();
    v.try_into().ok()
}

static VAR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"var\s+(\w+)\s*=\s*(?:'([^']*)'|"([^"]*)"|(-?\d+))\s*;"#).expect("regex válida")
});

/// Extrae las líneas `var nombre='valor';` (cadenas o enteros) de una página básica.
pub fn page_vars(html: &str) -> BTreeMap<String, String> {
    VAR_RE
        .captures_iter(html)
        .map(|c| {
            let val = (2..=4).find_map(|i| c.get(i)).map_or("", |m| m.as_str());
            (c[1].to_owned(), val.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mess_userpass_xor_e_involutivo() {
        assert_eq!(mess_userpass("1234"), ".-,+");
        assert_eq!(mess_userpass(&mess_userpass("admin:Pa$$")), "admin:Pa$$");
    }

    #[test]
    fn mac_split_y_join() {
        // MAC del rango de documentación (RFC 7042), no de un equipo real.
        let p = split_mac("02:00:5e:10:00:01").unwrap();
        assert_eq!(p, ["02", "00", "5e", "10", "00", "01"]);
        assert_eq!(
            join_mac(["02", "00", "5e", "10", "00", "01"]),
            "02:00:5e:10:00:01"
        );
        assert_eq!(split_mac("02:00:5e:10:00"), None);
        assert_eq!(split_mac("02:00:5e:10:00:01:00"), None);
        assert_eq!(split_mac(""), None);
    }

    #[test]
    fn ip_split_y_join() {
        assert_eq!(split_ip("192.168.1.68").unwrap(), ["192", "168", "1", "68"]);
        assert_eq!(join_ip(["192", "168", "1", "68"]), "192.168.1.68");
        assert_eq!(split_ip("192.168.1"), None);
        assert_eq!(split_ip("1.2.3.4.5"), None);
        assert_eq!(split_ip(""), None);
    }

    #[test]
    fn page_vars_extrae_cadenas_y_numeros() {
        let v =
            page_vars("var sessionKey='1816279949';\nvar n = 5;\nvar d = \"x y\";\nvar neg=-3;");
        assert_eq!(v["sessionKey"], "1816279949");
        assert_eq!(v["n"], "5");
        assert_eq!(v["d"], "x y");
        assert_eq!(v["neg"], "-3");
        assert_eq!(v.len(), 4);
    }
}
