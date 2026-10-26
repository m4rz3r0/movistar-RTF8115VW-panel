//! Interruptores de las funciones de la fase 3 que dependían de una prueba en el router real.
//!
//! Con una función desactivada ([`Desactivadas::motivo`] da `Some(clave)`),
//! la interfaz no la ofrece (solo enseña el valor y una nota con el
//! motivo) y el servidor responde 409 sin escribir: la tarjeta y, para todo lo que entra por el
//! gestor (Biblioteca o petición fabricada), `planificar_especial`. El
//! deshacer y la reversión de lo ya hecho siguen funcionando.
//!
//! Estado de la 1.0, tras la validación en el router real ([`en_la_version`]):
//!
//! - **Usuarios** (crear y borrar cuentas de la web del router): validada,
//!   habilitada.
//! - **ContrasenaOtro** (cambiar la contraseña o el grupo de otra cuenta):
//!   desactivada. En este router, el inicio de sesión avanzado
//!   (`cbCheckLogin`) responde `Result=4` (EXECUTE_FORBIDDEN) a cualquier
//!   cuenta que no sea el usuario fijo de la web (`1234`), así que el panel
//!   no puede comprobar que la contraseña nueva funciona.
//! - **SinPing** («No responder al ping desde internet»): desactivada. Con
//!   la regla puesta, el router sigue respondiendo al ping desde internet:
//!   otra regla del cortafuegos del operador lo permite.
//!
//! Es aparte de la lista blanca de escrituras del catálogo
//! (`comprobada` en `funciones.toml`): `scbidirectionalflt.cmd` está
//! comprobada (la regla se pone y se quita), pero «sin ping» no sirve en
//! este router.
//!
//! Lo que vale en un proceso lo decide su gestor de cambios
//! (`ChangeManager::desactivadas`): los valores de la versión, salvo en las
//! pruebas contra router-sim, que lo habilitan todo para recorrer sus
//! caminos felices (`Config::habilitar_todo`, `RP_E2E_HABILITAR_TODO=1`, el
//! mismo interruptor que da por comprobado el catálogo y que solo atiende
//! un binario de depuración). La interfaz no los calcula: los recibe del
//! servidor con la página ([`Desactivadas`] en cada elemento).

use serde::{Deserialize, Serialize};

/// Clave del motivo de un 409 al pedir una escritura que no está
/// comprobada en el router real (`comprobada` en el catálogo).
pub const SIN_COMPROBAR: &str = "funcion-sin-comprobar";

/// Funciones de la fase 3 que dependían de la validación real.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Funcion3 {
    /// Crear y borrar usuarios de la web del router (E3).
    Usuarios,
    /// Cambiar la contraseña o el grupo de otro usuario (E3).
    ContrasenaOtro,
    /// «No responder al ping desde internet» (E5, Tarea 3).
    SinPing,
}

impl Funcion3 {
    /// Las tres.
    pub const TODAS: [Funcion3; 3] = [
        Funcion3::Usuarios,
        Funcion3::ContrasenaOtro,
        Funcion3::SinPing,
    ];
}

/// Clave del motivo por el que `f` está desactivada en esta versión, o
/// `None` si está habilitada (ver el módulo). Es lo que vale en producción;
/// el servidor y la interfaz no la consultan directamente, sino a través
/// del gestor ([`Desactivadas`]).
pub const fn en_la_version(f: Funcion3) -> Option<&'static str> {
    match f {
        Funcion3::Usuarios => None,
        Funcion3::ContrasenaOtro | Funcion3::SinPing => Some(motivo(f)),
    }
}

/// Clave del motivo de cada función, para cuando está desactivada.
pub const fn motivo(f: Funcion3) -> &'static str {
    match f {
        Funcion3::Usuarios => "funcion-no-disponible-usuarios",
        Funcion3::ContrasenaOtro => "funcion-no-disponible-contrasena-otro",
        Funcion3::SinPing => "funcion-no-disponible-sin-ping",
    }
}

/// Qué funciones de la fase 3 tiene desactivadas un proceso del panel. La
/// calcula el servidor y viaja a la interfaz; sin ella (`Default`), las de
/// la versión.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Desactivadas {
    usuarios: bool,
    contrasena_otro: bool,
    sin_ping: bool,
}

impl Desactivadas {
    /// Las desactivadas según `desactivada`.
    pub fn segun(desactivada: impl Fn(Funcion3) -> bool) -> Desactivadas {
        Desactivadas {
            usuarios: desactivada(Funcion3::Usuarios),
            contrasena_otro: desactivada(Funcion3::ContrasenaOtro),
            sin_ping: desactivada(Funcion3::SinPing),
        }
    }

    /// Las de esta versión ([`en_la_version`]).
    pub fn de_la_version() -> Desactivadas {
        Desactivadas::segun(|f| en_la_version(f).is_some())
    }

    /// Ninguna: todas habilitadas.
    pub fn ninguna() -> Desactivadas {
        Desactivadas::segun(|_| false)
    }

    /// Clave del motivo por el que `f` está desactivada, o `None` si está
    /// habilitada.
    pub fn motivo(self, f: Funcion3) -> Option<&'static str> {
        let desactivada = match f {
            Funcion3::Usuarios => self.usuarios,
            Funcion3::ContrasenaOtro => self.contrasena_otro,
            Funcion3::SinPing => self.sin_ping,
        };
        desactivada.then(|| motivo(f))
    }
}

impl Default for Desactivadas {
    fn default() -> Self {
        Desactivadas::de_la_version()
    }
}

/// ¿Se escribe `a`? No si alguno de sus campos lo escribe una función sin
/// comprobar en el router real (el servidor les pone [`SIN_COMPROBAR`]): en
/// una lista, la plantilla de una fila nueva, que usa la misma función que
/// edita y borra las filas.
pub fn escribible(a: &crate::ajustes_router::AjusteVista) -> bool {
    !a.campos
        .iter()
        .any(|c| c.solo_lectura.as_deref() == Some(SIN_COMPROBAR))
}

/// Las desactivadas del servidor de `state`: las de su gestor de cambios
/// o, sin él, las de la versión.
#[cfg(feature = "ssr")]
pub(crate) fn del_servidor(state: &crate::state::AppState) -> Desactivadas {
    state
        .cambios()
        .map_or_else(Desactivadas::de_la_version, |g| g.desactivadas())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Las claves de los motivos existen en los dos idiomas.
    #[test]
    fn motivos_con_texto() {
        for f in Funcion3::TODAS {
            for idioma in i18n::Idioma::TODOS {
                assert!(i18n::existe(idioma, motivo(f)), "{f:?} {idioma:?}");
            }
        }
    }

    /// Los valores de la 1.0, tras la validación en el router real: crear y
    /// borrar usuarios, habilitada; cambiar otra cuenta y «sin ping»,
    /// desactivadas con su motivo.
    #[test]
    fn valores_de_la_1_0() {
        assert_eq!(en_la_version(Funcion3::Usuarios), None);
        assert_eq!(
            en_la_version(Funcion3::ContrasenaOtro),
            Some("funcion-no-disponible-contrasena-otro")
        );
        assert_eq!(
            en_la_version(Funcion3::SinPing),
            Some("funcion-no-disponible-sin-ping")
        );
        let d = Desactivadas::default();
        for f in Funcion3::TODAS {
            assert_eq!(d.motivo(f), en_la_version(f), "{f:?}");
        }
    }

    /// `ninguna` las habilita todas; `segun` desactiva solo lo pedido.
    #[test]
    fn ninguna_y_segun() {
        for f in Funcion3::TODAS {
            assert_eq!(Desactivadas::ninguna().motivo(f), None);
            let solo = Desactivadas::segun(|g| g == f);
            for g in Funcion3::TODAS {
                assert_eq!(solo.motivo(g), (g == f).then(|| motivo(g)), "{f:?} {g:?}");
            }
        }
    }
}
