//! Interruptores de la validación real en el gestor: qué funciones de la validación en el router real deja hacer este proceso.
//!
//! La base son los valores de la versión
//! ([`crate::validacion_real::en_la_version`]). Las pruebas contra
//! router-sim (integración y e2e) recorren los caminos felices de las
//! funciones que la versión desactiva: las habilitan con
//! [`crate::config::Config::habilitar_todo`], el mismo interruptor que da
//! por comprobadas las escrituras del catálogo, y que solo atiende un
//! binario de depuración. Encima, una prueba puede desactivar más con
//! [`ChangeManager::forzar_interruptores`], nunca habilitar.

use super::ChangeManager;
use crate::validacion_real::{Desactivadas, Funcion3};
use std::sync::OnceLock;

/// Lo que cambia los valores de la versión en un gestor.
pub(super) struct Interruptores {
    /// Todas habilitadas como base ([`crate::config::Config::todo_habilitado`]).
    en_pruebas: bool,
    /// Desactivadas a la fuerza por una prueba
    /// ([`ChangeManager::forzar_interruptores`]).
    forzados: OnceLock<fn(Funcion3) -> Option<&'static str>>,
}

impl Interruptores {
    /// Con la base de la versión o, `en_pruebas`, todas habilitadas.
    pub(super) fn new(en_pruebas: bool) -> Interruptores {
        Interruptores {
            en_pruebas,
            forzados: OnceLock::new(),
        }
    }
}

impl ChangeManager {
    /// Fuerza los interruptores de la validación real de este gestor. Solo
    /// para las pruebas. Solo desactiva: lo que ya está desactivado sigue
    /// así. Vale la primera vez.
    #[doc(hidden)]
    pub fn forzar_interruptores(&self, f: fn(Funcion3) -> Option<&'static str>) {
        let _ = self.interruptores.forzados.set(f);
    }

    /// Las funciones de la validación real que deja hacer este gestor: lo que manda
    /// el servidor a la interfaz y lo que comprueba antes de escribir.
    pub fn desactivadas(&self) -> Desactivadas {
        combinar(
            self.interruptores.en_pruebas,
            self.interruptores.forzados.get().copied(),
        )
    }

    /// Clave del motivo por el que `f` está desactivada, o `None`.
    pub(crate) fn sin_validar(&self, f: Funcion3) -> Option<&'static str> {
        self.desactivadas().motivo(f)
    }
}

/// Desactivadas con la base (`en_pruebas`: ninguna; si no, las de la
/// versión) y lo forzado encima, que solo desactiva más.
fn combinar(
    en_pruebas: bool,
    forzados: Option<fn(Funcion3) -> Option<&'static str>>,
) -> Desactivadas {
    let base = if en_pruebas {
        Desactivadas::ninguna()
    } else {
        Desactivadas::de_la_version()
    };
    Desactivadas::segun(|f| base.motivo(f).is_some() || forzados.and_then(|g| g(f)).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validacion_real::{en_la_version, motivo};

    /// Sin nada, los valores de la versión.
    #[test]
    fn sin_nada_los_de_la_version() {
        assert_eq!(combinar(false, None), Desactivadas::de_la_version());
        for f in Funcion3::TODAS {
            assert_eq!(combinar(false, None).motivo(f), en_la_version(f));
        }
    }

    /// Habilitadas en pruebas, ninguna desactivada; forzar `Some` desactiva
    /// encima y forzar `None` no habilita lo que la versión desactiva.
    #[test]
    fn forzar_solo_desactiva() {
        let todas: fn(Funcion3) -> Option<&'static str> = |f| Some(motivo(f));
        let ninguna: fn(Funcion3) -> Option<&'static str> = |_| None;
        assert_eq!(combinar(true, None), Desactivadas::ninguna());
        assert_eq!(combinar(true, Some(ninguna)), Desactivadas::ninguna());
        for en_pruebas in [false, true] {
            assert_eq!(
                combinar(en_pruebas, Some(todas)),
                Desactivadas::segun(|_| true)
            );
        }
        assert_eq!(
            combinar(false, Some(ninguna)),
            Desactivadas::de_la_version()
        );
    }
}
