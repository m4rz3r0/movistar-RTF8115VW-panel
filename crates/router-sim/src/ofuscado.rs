//! La ofuscación de usuario y contraseña del router, copia propia del simulador escrita desde su JavaScript (no la del panel, para que las pruebas no se den la razón a sí mismas).
//!
//! La web oficial pasa `loginUsername`, `loginPassword`, `rqUsername`,
//! `rqPasswd`, y en `js/user_profile.js` (líneas 181–183) `rqUser`,
//! `rqOPwd` y `rqNPwd`, por `mess_userpass`, que hace un XOR 0x1f a cada
//! unidad de `charCodeAt` y vuelve a montar la cadena con
//! `String.fromCharCode`. Como en JavaScript, va por unidades UTF-16 (un
//! carácter fuera del plano básico son dos). Es involutiva: con ella se lee
//! lo que llega.

/// `mess_userpass` del router sobre `s`: XOR 0x1f a cada unidad UTF-16.
pub(crate) fn deshacer(s: &str) -> String {
    let unidades: Vec<u16> = s.encode_utf16().map(|u| u ^ 0x1f).collect();
    String::from_utf16_lossy(&unidades)
}

#[cfg(test)]
mod tests {
    use super::deshacer;

    /// Vectores calculados a mano con la tabla ASCII: '1' (0x31) ^ 0x1f =
    /// 0x2e '.', '2' → 0x2d '-', '3' → 0x2c ',', '4' → 0x2b '+'; 'A' (0x41)
    /// → 0x5e '^', 'b' (0x62) → 0x7d '}', '9' (0x39) → 0x26 '&', '~' (0x7e)
    /// → 0x61 'a'; 's' (0x73) → 0x6c 'l', 'i' (0x69) → 0x76 'v', 'm' (0x6d)
    /// → 0x72 'r'.
    #[test]
    fn vectores_fijos() {
        assert_eq!(deshacer(".-,+"), "1234");
        assert_eq!(deshacer("^}&a"), "Ab9~");
        assert_eq!(deshacer("lvr"), "sim");
        assert_eq!(deshacer(""), "");
    }

    #[test]
    fn involutiva() {
        for s in ["1234", "clave-prueba-1", "ñ€ $&=+"] {
            assert_eq!(deshacer(&deshacer(s)), s);
        }
    }
}
