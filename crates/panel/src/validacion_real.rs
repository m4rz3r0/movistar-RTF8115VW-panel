//! Interruptores de las funciones de la fase 3 que dependen de una prueba en el router real.
//!
//! Con un interruptor en `Some(motivo)` ([`sin_validar`]), la interfaz deja
//! los botones de su función desactivados con ese motivo y el servidor
//! responde 409 sin escribir: la tarjeta y, para todo lo que entra por el
//! gestor (Biblioteca o petición fabricada), `planificar_especial`. El
//! deshacer y la reversión de lo ya hecho siguen funcionando. Se usa en el
//! servidor y en el navegador (es `const`).
//!
//! Estado (Ruling Q): en la candidata v0.9.0 las tres están habilitadas
//! (`None`) para poder hacer la E3 y la E5 desde el panel, que solo usa el
//! usuario con el controlador delante. Tras cada validación se fija la suya:
//! validada, sigue habilitada; fallida, pasa a `Some(motivo(f))`. La Tarea 9
//! añade un test que fija el valor de cada una en la 1.0.

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

/// Clave del motivo por el que `f` está desactivada, o `None` si está
/// habilitada. Hoy las tres dan `None` sin estar validadas: así lo pide la
/// candidata para hacer la E3 y la E5 (Ruling Q, ver el módulo). La que no
/// valide pasará a `Some(motivo(f))`.
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
