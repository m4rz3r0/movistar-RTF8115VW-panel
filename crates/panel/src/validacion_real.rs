//! Interruptores de las funciones de la fase 3 que dependen de una prueba en el router real.
//!
//! Cada función nueva se habilita cuando su validación (§3 del spec) la ha
//! confirmado en el router del usuario. Mientras no, [`sin_validar`] da la
//! clave del motivo: la interfaz deja sus botones desactivados con él y el
//! servidor responde 409 sin escribir. Se usa en el servidor y en el
//! navegador (es `const`).

/// Funciones de la fase 3 que esperan su validación real.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Funcion3 {
    /// Crear y borrar usuarios de la web del router (E3).
    Usuarios,
    /// Cambiar la contraseña o el grupo de otro usuario (E3).
    ContrasenaOtro,
    /// «No responder al ping desde internet» (E5, Tarea 3).
    SinPing,
}

/// Clave del motivo por el que `f` sigue desactivada, o `None` si ya está
/// validada. Hoy, las tres esperan la validación del controlador: si la E3
/// o la E5 no la confirman, la suya pasa a `Some(...)`.
pub const fn sin_validar(f: Funcion3) -> Option<&'static str> {
    match f {
        Funcion3::Usuarios | Funcion3::ContrasenaOtro | Funcion3::SinPing => None,
    }
}

/// Clave del motivo de cada función, para cuando haya que desactivarla.
pub const fn motivo(f: Funcion3) -> &'static str {
    match f {
        Funcion3::Usuarios => "validacion-real-pendiente-usuarios",
        Funcion3::ContrasenaOtro => "validacion-real-pendiente-contrasena-otro",
        Funcion3::SinPing => "validacion-real-pendiente-sin-ping",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Las claves de los motivos existen en los dos idiomas.
    #[test]
    fn motivos_con_texto() {
        for f in [
            Funcion3::Usuarios,
            Funcion3::ContrasenaOtro,
            Funcion3::SinPing,
        ] {
            for idioma in i18n::Idioma::TODOS {
                assert!(i18n::existe(idioma, motivo(f)), "{f:?} {idioma:?}");
            }
        }
    }
}
