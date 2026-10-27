//! El campo `router` de `/salud`: cómo acabó la última comunicación real del
//! panel con el router ([`hgu::Client::contacto`]), sin hacer ninguna
//! petición propia.

use hgu::client::{Contacto, EstadoContacto};
use std::time::Duration;

/// Sin ninguna comunicación en este plazo, no se sabe si el router responde
/// (el sondeo le pregunta cada 10 s; tras errores, como mucho cada 60 s).
pub const RECIENTE: Duration = Duration::from_secs(120);

/// `sin_configurar` sin router configurado; `pendiente` si aún no ha habido
/// ninguna comunicación (justo al arrancar); `ok` si la última respondió y
/// hace menos de [`RECIENTE`]; `error` si no respondió, si rechazó las
/// credenciales o si hace demasiado de la última.
pub fn router_en_salud(hgu: &hgu::Client) -> &'static str {
    if !hgu.configurado() {
        return "sin_configurar";
    }
    estado(hgu.contacto())
}

fn estado(c: Option<Contacto>) -> &'static str {
    match c {
        None => "pendiente",
        Some(Contacto {
            cuando,
            estado: EstadoContacto::Responde,
        }) if cuando.elapsed() < RECIENTE => "ok",
        Some(_) => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn hace(d: Duration, estado: EstadoContacto) -> Option<Contacto> {
        Some(Contacto {
            cuando: Instant::now().checked_sub(d).unwrap(),
            estado,
        })
    }

    #[test]
    fn la_ultima_comunicacion_decide() {
        let s = Duration::from_secs(1);
        assert_eq!(estado(None), "pendiente");
        assert_eq!(estado(hace(s, EstadoContacto::Responde)), "ok");
        assert_eq!(estado(hace(s, EstadoContacto::SinRespuesta)), "error");
        assert_eq!(estado(hace(s, EstadoContacto::Rechaza)), "error");
        assert_eq!(
            estado(hace(RECIENTE + s, EstadoContacto::Responde)),
            "error"
        );
    }

    #[test]
    fn sin_configurar() {
        assert_eq!(
            router_en_salud(&hgu::Client::sin_configurar()),
            "sin_configurar"
        );
    }
}
