//! Cuándo se vuelve a probar la contraseña nueva del router si aún no entra: pocos intentos con espera creciente.

use std::time::Duration;

/// Primera espera antes de reintentar; cada una dobla la anterior.
const PRIMERA: Duration = Duration::from_secs(1);
/// Lo más que se reintenta: cada intento fallido acerca el bloqueo por
/// intentos del router.
const MAX_INTENTOS: usize = 4;

/// Las esperas antes de cada reintento dentro de `plazo`: 1, 2, 4… s, la
/// última recortada para acabar justo en el plazo, y como mucho
/// [`MAX_INTENTOS`]. Con el plazo de serie (10 s), a 1, 3, 7 y 10 s.
pub(crate) fn pausas(plazo: Duration) -> Vec<Duration> {
    let mut out = Vec::new();
    let (mut usado, mut siguiente) = (Duration::ZERO, PRIMERA);
    while usado < plazo && out.len() < MAX_INTENTOS {
        let p = if out.len() + 1 == MAX_INTENTOS {
            plazo - usado
        } else {
            siguiente.min(plazo - usado)
        };
        out.push(p);
        usado += p;
        siguiente *= 2;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: f64) -> Duration {
        Duration::from_secs_f64(x)
    }

    #[test]
    fn pocos_reintentos_con_espera_creciente() {
        assert_eq!(pausas(s(10.0)), [s(1.0), s(2.0), s(4.0), s(3.0)]);
        assert_eq!(pausas(s(30.0)), [s(1.0), s(2.0), s(4.0), s(23.0)]);
        assert_eq!(pausas(s(4.5)), [s(1.0), s(2.0), s(1.5)]);
        assert_eq!(pausas(s(0.3)), [s(0.3)]);
        assert!(pausas(Duration::ZERO).is_empty());
    }
}
