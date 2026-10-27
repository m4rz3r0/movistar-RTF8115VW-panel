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
/// Es el único sitio del panel que decide cómo viajan; el simulador los lee
/// con su propia copia del JavaScript del router, para no darse la razón.
///
/// Es [`mess_userpass`] (XOR 0x1f por carácter), como en `js/user_profile.js`
/// de la web oficial, que pasa los tres valores por `mess_userpass`. Del
/// router real solo se sabe que en claro lo rechaza; que acepte el XOR está
/// sin comprobar. Es involutiva.
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

/// Página del router desde la que se gestionan los usuarios de su web
/// (alta, baja y cambio de otro usuario).
pub const PAGINA_USUARIOS: &str = "user_profile.asp";
/// Función del router que crea un usuario (`rqUser`, `rqPwd` y `rqGroup`,
/// en claro, como en `js/user_profile.js`).
pub const ALTA_USUARIO: &str = "cbAddUser";
/// Función del router que borra usuarios (`rqDelList`, ver
/// [`lista_de_baja`]).
pub const BAJA_USUARIO: &str = "cbDeleteUser";
/// Función del router que cambia la contraseña y el grupo de otro usuario
/// (ventana `user_profile_modifyOthers.asp`).
pub const CAMBIO_OTRO: &str = "cbModifyPwdOther";

/// Parámetros de [`CAMBIO_OTRO`] para dejar a `usuario` con la contraseña
/// `nueva` y el grupo `grupo`. Van en claro, como en `js/user_profile.js`:
/// a diferencia de [`CAMBIO_CONTRASENA`], la web oficial no los pasa por
/// [`mess_userpass`]. `rqNPwd` va siempre: el JavaScript lo exige también
/// para cambiar solo el grupo.
pub fn parametros_cambio_otro(
    usuario: &str,
    nueva: &str,
    grupo: &str,
) -> [(&'static str, String); 3] {
    [
        ("rqUser", usuario.to_owned()),
        ("rqNPwd", nueva.to_owned()),
        ("rqGroup", grupo.to_owned()),
    ]
}

/// `rqDelList` de [`BAJA_USUARIO`]: los nombres seguidos, cada uno con `:`
/// detrás (`ana:pepe:`), como `deleteUser` en `js/user_profile.js`.
pub fn lista_de_baja(usuarios: &[&str]) -> String {
    usuarios.iter().map(|u| format!("{u}:")).collect()
}

/// Los nombres de un `rqDelList` ([`lista_de_baja`] al revés); los huecos
/// vacíos (el `:` final, dos seguidos) no cuentan.
pub fn nombres_de_baja(lista: &str) -> Vec<&str> {
    lista.split(':').filter(|n| !n.is_empty()).collect()
}

/// ¿Se puede pedir la baja de `nombre`? No vacío, sin `:` (el separador de
/// [`lista_de_baja`]: borraría otros) y sin caracteres de control. Es más
/// laxo que [`usuario_valido`]: un usuario creado fuera del panel puede no
/// cumplir su regla y aun así borrarse.
pub fn nombre_de_baja_valido(nombre: &str) -> bool {
    !nombre.is_empty() && !nombre.contains(':') && !nombre.chars().any(char::is_control)
}

/// ¿Vale `nombre` como usuario nuevo? La regla de `js/user_profile.js`,
/// `/^\w{4,20}$/`: de 4 a 20 letras ASCII, cifras o `_`.
pub fn usuario_valido(nombre: &str) -> bool {
    (4..=20).contains(&nombre.len())
        && nombre
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
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
    fn lista_de_baja_con_dos_puntos() {
        assert_eq!(lista_de_baja(&["ana", "pepe"]), "ana:pepe:");
        assert_eq!(nombres_de_baja("ana:pepe:"), ["ana", "pepe"]);
        assert_eq!(nombres_de_baja(":ana::"), ["ana"]);
        assert!(nombres_de_baja("").is_empty());
    }

    #[test]
    fn usuario_valido_como_el_javascript() {
        assert!(usuario_valido("prueba1"));
        assert!(usuario_valido("con_guion_bajo"));
        assert!(!usuario_valido("ab"));
        assert!(!usuario_valido("con espacio"));
        assert!(!usuario_valido(&"a".repeat(21)));
        assert!(usuario_valido(&"a".repeat(20)));
        assert!(!usuario_valido("año1"));
    }

    #[test]
    fn cambio_de_otro_en_claro() {
        let p = parametros_cambio_otro("invitado1", "clave-nueva-1", "user");
        assert_eq!(
            p.map(|(k, v)| format!("{k}={v}")),
            ["rqUser=invitado1", "rqNPwd=clave-nueva-1", "rqGroup=user"]
        );
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

    #[test]
    fn nombre_de_baja_sin_dos_puntos_ni_control() {
        assert!(nombre_de_baja_valido("invitado1"));
        // Creado fuera del panel, con otra forma: se puede borrar.
        assert!(nombre_de_baja_valido("ana.o-1"));
        for n in ["", "a:b", "a\u{7}b", "a\nb"] {
            assert!(!nombre_de_baja_valido(n), "{n:?}");
        }
    }
}
