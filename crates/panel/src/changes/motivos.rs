//! Motivos del gestor con texto propio: los de los usuarios del router, la regla «sin ping» ya puesta y las funciones que esperan su validación real.

use super::{BLOQUEO_USUARIO_PROTEGIDO, CambioError, USUARIO_YA_EXISTE};

/// Motivo de [`CambioError::Invalida`] al pedir borrar varios usuarios del
/// router de una vez. Es la clave i18n del texto.
pub const USUARIO_UNO_CADA_VEZ: &str = "usuarios-router-error-uno-cada-vez";
/// Motivo de [`CambioError::InstanciaCambiada`] al borrar un usuario del
/// router que ya no está en su lista. Es la clave i18n del texto.
pub const USUARIO_NO_EXISTE: &str = "usuarios-router-error-no-existe";
/// Motivo de [`CambioError::Invalida`] al pedir el grupo `system`, que es
/// del operador. Es la clave i18n del texto.
pub const USUARIO_GRUPO_SISTEMA: &str = "usuarios-router-error-grupo-sistema";
/// Motivo de [`CambioError::Invalida`] con un grupo que no es `user` ni
/// `admin`. Es la clave i18n del texto.
pub const USUARIO_GRUPO_NO_VALIDO: &str = "usuarios-router-error-grupo-no-valido";
/// Motivo de [`CambioError::Invalida`] con un nombre de usuario que no es de
/// 4 a 20 letras, cifras o «_». Es la clave i18n del texto.
pub const USUARIO_NOMBRE_NO_VALIDO: &str = "usuarios-router-error-nombre";

/// Código HTTP y clave i18n de los motivos del gestor que las pantallas
/// enseñan con su propio texto: los de los usuarios del router, la regla
/// «sin ping» ya puesta y las funciones que esperan su validación real.
/// `None`: cada pantalla da su texto genérico.
pub fn motivo_con_texto(e: &CambioError) -> Option<(u16, &'static str)> {
    use crate::validacion_real::{Funcion3, motivo};
    const DE_USUARIOS: [&str; 4] = [
        USUARIO_UNO_CADA_VEZ,
        USUARIO_GRUPO_SISTEMA,
        USUARIO_GRUPO_NO_VALIDO,
        USUARIO_NOMBRE_NO_VALIDO,
    ];
    let pendientes = [
        Funcion3::Usuarios,
        Funcion3::ContrasenaOtro,
        Funcion3::SinPing,
    ]
    .map(motivo);
    let entre = |m: &str, claves: &[&'static str]| claves.iter().copied().find(|c| *c == m);
    match e {
        CambioError::Bloqueada(m) if m == BLOQUEO_USUARIO_PROTEGIDO => {
            Some((403, BLOQUEO_USUARIO_PROTEGIDO))
        }
        CambioError::Bloqueada(m) => entre(m, &pendientes).map(|c| (409, c)),
        CambioError::InstanciaCambiada(m) => entre(
            m,
            &[
                USUARIO_YA_EXISTE,
                USUARIO_NO_EXISTE,
                super::SIN_PING_YA_PUESTA,
            ],
        )
        .map(|c| (409, c)),
        CambioError::Invalida(m) => entre(m, &DE_USUARIOS).map(|c| (400, c)),
        _ => None,
    }
}
