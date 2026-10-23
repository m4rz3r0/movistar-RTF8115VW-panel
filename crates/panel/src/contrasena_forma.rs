//! Forma de las contraseñas del router: las reglas de `js/user_profile.js`.
//!
//! La web oficial exige `/^[\x20-\x7e]{8,32}$/` y rechaza
//! `/(^ |  | $)/`. El panel añade una: sin «`», que en XOR 0x1f (como
//! viaja: [`hgu::fmt::codificar_cambio_contrasena`]) daría 0x7f, un
//! carácter de control. Con ASCII, bytes y caracteres coinciden.

/// Longitud mínima de la contraseña nueva, en bytes.
pub const MIN_BYTES: usize = 8;
/// Longitud máxima de la contraseña nueva, en bytes (la de la web oficial).
pub const MAX_BYTES: usize = 32;

/// Por qué no vale la contraseña nueva. Sus claves de texto quedan
/// guardadas en el Historial: las variantes no se renombran ni se quitan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorNueva {
    Corta,
    Larga,
    /// Lleva caracteres de control (saltos de línea, tabuladores…).
    Control,
    /// Es la que ya tiene el router.
    Igual,
    /// Lleva algo fuera del ASCII imprimible (ñ, tildes…) o «`».
    Caracter,
    /// Empieza o acaba con un espacio, o lleva dos seguidos.
    Espacios,
}

/// Comprueba la forma de la contraseña nueva con el máximo de la web
/// oficial ([`MAX_BYTES`]).
pub fn validar_forma(nueva: &str) -> Result<(), ErrorNueva> {
    validar_forma_hasta(nueva, MAX_BYTES)
}

/// Comprueba la forma de la contraseña nueva, de [`MIN_BYTES`] a `max`
/// bytes, en este orden: la longitud, los caracteres de control (0x00–0x1f
/// y 0x7f), los que no son ASCII imprimible o son «`» y los espacios.
pub fn validar_forma_hasta(nueva: &str, max: usize) -> Result<(), ErrorNueva> {
    if nueva.len() < MIN_BYTES {
        Err(ErrorNueva::Corta)
    } else if nueva.len() > max {
        Err(ErrorNueva::Larga)
    } else if nueva.chars().any(|c| c.is_ascii_control()) {
        Err(ErrorNueva::Control)
    } else if nueva.chars().any(|c| !c.is_ascii() || c == '`') {
        Err(ErrorNueva::Caracter)
    } else if nueva.starts_with(' ') || nueva.ends_with(' ') || nueva.contains("  ") {
        Err(ErrorNueva::Espacios)
    } else {
        Ok(())
    }
}

/// Como [`validar_forma`], y además distinta de la actual.
pub fn validar_nueva(nueva: &str, actual: &str) -> Result<(), ErrorNueva> {
    validar_forma(nueva)?;
    if nueva == actual {
        return Err(ErrorNueva::Igual);
    }
    Ok(())
}

/// Texto de un [`ErrorNueva`] en el idioma de la página.
pub fn texto_error_nueva(e: ErrorNueva) -> String {
    match e {
        ErrorNueva::Corta => crate::t!("contrasena-router-error-corta", n = MIN_BYTES),
        ErrorNueva::Larga => crate::t!("contrasena-router-error-larga", n = MAX_BYTES),
        ErrorNueva::Control => crate::t!("contrasena-router-error-control"),
        ErrorNueva::Igual => crate::t!("contrasena-router-error-igual"),
        ErrorNueva::Caracter => crate::t!("contrasena-router-error-caracter"),
        ErrorNueva::Espacios => crate::t!("contrasena-router-error-espacios"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valida_la_contrasena_nueva() {
        assert_eq!(validar_forma("1234567"), Err(ErrorNueva::Corta));
        assert_eq!(validar_forma(""), Err(ErrorNueva::Corta));
        assert_eq!(validar_forma("12345678"), Ok(()));
        assert_eq!(validar_forma(&"a".repeat(32)), Ok(()));
        assert_eq!(validar_forma(&"a".repeat(33)), Err(ErrorNueva::Larga));
        assert_eq!(validar_forma("clave con espacios"), Ok(()));
        // Solo ASCII imprimible, y sin «`», que en XOR daría 0x7f.
        for c in ["ññññ", "clave`de-prueba", "clave-de-prueba-€"] {
            assert_eq!(validar_forma(c), Err(ErrorNueva::Caracter), "{c:?}");
        }
        for c in ["clave\nde-prueba", "clave\tde-prueba", "clave\u{7f}prueba"] {
            assert_eq!(validar_forma(c), Err(ErrorNueva::Control), "{c:?}");
        }
        // Sin espacio al principio ni al final, ni dos seguidos.
        for c in [" clave-de-prueba", "clave-de-prueba ", "clave  de-prueba"] {
            assert_eq!(validar_forma(c), Err(ErrorNueva::Espacios), "{c:?}");
        }
        // Con otro máximo (la Tarea 2, con 16).
        assert_eq!(validar_forma_hasta(&"a".repeat(16), 16), Ok(()));
        assert_eq!(
            validar_forma_hasta(&"a".repeat(17), 16),
            Err(ErrorNueva::Larga)
        );
        assert_eq!(
            validar_nueva("clave-de-prueba-1", "clave-de-prueba-1"),
            Err(ErrorNueva::Igual)
        );
        assert_eq!(
            validar_nueva("clave-nueva-de-prueba", "clave-de-prueba-1"),
            Ok(())
        );
    }
}
