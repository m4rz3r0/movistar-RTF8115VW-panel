//! La última comunicación con el router y la cadena de un error de red.
//!
//! El cliente anota cómo acabó cada petición ([`Client::contacto`]): el
//! `/salud` del panel lo dice sin hacer ninguna petición propia, que
//! competiría con la sesión del panel.

use super::Client;
use std::time::Instant;

/// Cómo acabó la última comunicación con el router.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoContacto {
    /// Respondió (cualquier respuesta HTTP).
    Responde,
    /// No respondió: error de red o tiempo agotado.
    SinRespuesta,
    /// Respondió, pero rechazó el usuario o la contraseña.
    Rechaza,
}

/// La última comunicación con el router: cuándo terminó y cómo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contacto {
    pub cuando: Instant,
    pub estado: EstadoContacto,
}

impl Client {
    /// La última comunicación con el router por este cliente; `None` si aún
    /// no ha habido ninguna desde que se creó o se reconfiguró.
    pub fn contacto(&self) -> Option<Contacto> {
        *self
            .shared
            .contacto
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub(super) fn anotar_contacto(&self, estado: Option<EstadoContacto>) {
        *self
            .shared
            .contacto
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = estado.map(|estado| Contacto {
            cuando: Instant::now(),
            estado,
        });
    }
}

/// El error de red con todas sus causas («error sending request: client
/// error (SendRequest): connection closed before message completed»), para
/// que el registro diga qué pasó. Sin la URL: en la web básica lleva el
/// `sessionKey` y los valores escritos.
pub(super) fn cadena(e: reqwest::Error) -> String {
    let e = e.without_url();
    let mut texto = e.to_string();
    let mut causa = std::error::Error::source(&e);
    while let Some(c) = causa {
        let t = c.to_string();
        if !texto.contains(&t) {
            texto.push_str(": ");
            texto.push_str(&t);
        }
        causa = c.source();
    }
    texto
}
