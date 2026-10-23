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

/// Página del router desde la que se cambia la contraseña del usuario con
/// sesión iniciada.
pub const PAGINA_CAMBIO_CONTRASENA: &str = "user_profile.asp";
/// Función del router que cambia la contraseña del usuario con sesión
/// iniciada.
pub const CAMBIO_CONTRASENA: &str = "cbModifyPwd";

/// Codifica un valor de [`CAMBIO_CONTRASENA`] (`rqUser`, `rqOPwd` y `rqNPwd`).
/// Es el único sitio que decide cómo viajan: lo usan el panel al escribir y
/// el simulador al leer lo recibido.
///
/// Es [`mess_userpass`] (XOR 0x1f por carácter), como en `js/user_profile.js`
/// de la web oficial, que pasa los tres valores por `mess_userpass` (validado
/// con el router real en la fase 3: en claro, lo rechaza). Es involutiva:
/// el simulador la usa también para leer.
pub fn codificar_cambio_contrasena(valor: &str) -> String {
    mess_userpass(valor)
}

/// Parámetros de [`CAMBIO_CONTRASENA`] para pasar la contraseña de `usuario`
/// de `actual` a `nueva`, codificados con [`codificar_cambio_contrasena`].
pub fn parametros_cambio_contrasena(
    usuario: &str,
    actual: &str,
    nueva: &str,
) -> [(&'static str, String); 3] {
    [
        ("rqUser", codificar_cambio_contrasena(usuario)),
        ("rqOPwd", codificar_cambio_contrasena(actual)),
        ("rqNPwd", codificar_cambio_contrasena(nueva)),
    ]
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
    fn cambio_de_contrasena_con_xor_e_involutivo() {
        let p = parametros_cambio_contrasena("1234", "clave-de-prueba-1", "clave-nueva-de-prueba");
        assert_eq!(
            p.map(|(k, v)| format!("{k}={v}")),
            [
                "rqUser=.-,+",
                "rqOPwd=|s~iz2{z2omjz}~2.",
                "rqNPwd=|s~iz2qjzi~2{z2omjz}~"
            ]
        );
        for v in ["", "clave-de-prueba-1", "ñ€ $&=+"] {
            assert_eq!(
                codificar_cambio_contrasena(&codificar_cambio_contrasena(v)),
                v
            );
        }
    }

    #[test]
    fn mac_split() {
        // MAC del rango de documentación (RFC 7042), no de un equipo real.
        let p = split_mac("02:00:5e:10:00:01").unwrap();
        assert_eq!(p, ["02", "00", "5e", "10", "00", "01"]);
        assert_eq!(split_mac("02:00:5e:10:00"), None);
        assert_eq!(split_mac("02:00:5e:10:00:01:00"), None);
        assert_eq!(split_mac(""), None);
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
