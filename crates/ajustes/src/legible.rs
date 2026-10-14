//! Validación de un valor legible sin el catálogo: la usan `a_router` en el
//! servidor y la página de Ajustes en el navegador, así los dos aplican las
//! mismas reglas y los mismos límites (en bytes UTF-8 para los textos).

use std::fmt;
use std::net::Ipv4Addr;

use crate::modelo::TipoCampo;

/// Valor legible que no se puede escribir. Los mensajes no llevan el valor
/// (puede ser un secreto); el texto para la interfaz es [`ErrorValor::clave_i18n`]
/// con los argumentos de [`ErrorValor::limites`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorValor {
    Vacio,
    Corto { min: i64 },
    Largo { max: i64 },
    FueraDeRango { min: Option<i64>, max: Option<i64> },
    Ip,
    Mac,
    Opcion,
    Control,
    Ascii,
    SoloLectura,
}

impl fmt::Display for ErrorValor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorValor::Vacio => f.write_str("el valor no puede estar vacío"),
            ErrorValor::Corto { min } => write!(f, "demasiado corto: al menos {min} bytes"),
            ErrorValor::Largo { max } => write!(f, "demasiado largo: como mucho {max} bytes"),
            ErrorValor::FueraDeRango { min, max } => match (min, max) {
                (Some(min), Some(max)) => write!(f, "número fuera de rango: de {min} a {max}"),
                (Some(min), None) => write!(f, "número fuera de rango: al menos {min}"),
                (None, Some(max)) => write!(f, "número fuera de rango: como mucho {max}"),
                (None, None) => f.write_str("no es un número"),
            },
            ErrorValor::Ip => f.write_str("no es una dirección IPv4"),
            ErrorValor::Mac => f.write_str("no es una dirección MAC"),
            ErrorValor::Opcion => f.write_str("no es una de las opciones"),
            ErrorValor::Control => f.write_str("lleva caracteres de control"),
            ErrorValor::Ascii => f.write_str("solo admite letras sin tilde, cifras y signos ASCII"),
            ErrorValor::SoloLectura => f.write_str("el campo es de solo lectura"),
        }
    }
}

impl std::error::Error for ErrorValor {}

impl ErrorValor {
    /// Clave de traducción del mensaje (`ajustes-error-…`). Sus argumentos
    /// son [`ErrorValor::limites`] (`min`, `max`); fuera de rango tiene una
    /// clave por cada combinación de límites, con solo los que hay.
    pub fn clave_i18n(&self) -> &'static str {
        match self {
            ErrorValor::Vacio => "ajustes-error-vacio",
            ErrorValor::Corto { .. } => "ajustes-error-corto",
            ErrorValor::Largo { .. } => "ajustes-error-largo",
            ErrorValor::FueraDeRango { min, max } => match (min, max) {
                (Some(_), Some(_)) => "ajustes-error-fuera-de-rango",
                (Some(_), None) => "ajustes-error-fuera-de-rango-desde",
                (None, Some(_)) => "ajustes-error-fuera-de-rango-hasta",
                (None, None) => "ajustes-error-numero",
            },
            ErrorValor::Ip => "ajustes-error-ip",
            ErrorValor::Mac => "ajustes-error-mac",
            ErrorValor::Opcion => "ajustes-error-opcion",
            ErrorValor::Control => "ajustes-error-control",
            ErrorValor::Ascii => "ajustes-error-ascii",
            ErrorValor::SoloLectura => "ajustes-error-solo-lectura",
        }
    }

    /// Argumentos `min` y `max` del mensaje: solo los que tiene el error.
    pub fn limites(&self) -> (Option<i64>, Option<i64>) {
        match *self {
            ErrorValor::Corto { min } => (Some(min), None),
            ErrorValor::Largo { max } => (None, Some(max)),
            ErrorValor::FueraDeRango { min, max } => (min, max),
            _ => (None, None),
        }
    }
}

/// Límites de un campo: en bytes UTF-8 para textos y secretos, en valor para
/// números; solo ASCII imprimible; se puede dejar vacío.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Limites {
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub ascii: bool,
    pub opcional: bool,
}

/// Admite `opcional`: un campo que se puede dejar vacío.
pub(crate) fn admite_opcional(t: TipoCampo) -> bool {
    !matches!(t, TipoCampo::Opcion | TipoCampo::Booleano)
}

/// Comprueba el valor legible `legible` de un campo de tipo `tipo` con sus
/// límites; `es_opcion` dice si una clave es una de sus opciones. No mira
/// si el campo es de solo lectura.
pub fn validar_legible(
    tipo: TipoCampo,
    l: Limites,
    es_opcion: impl Fn(&str) -> bool,
    legible: &str,
) -> Result<(), ErrorValor> {
    if legible.chars().any(char::is_control) {
        return Err(ErrorValor::Control);
    }
    let vacio = match tipo {
        TipoCampo::Texto | TipoCampo::Secreto => legible.is_empty(),
        _ => legible.trim().is_empty(),
    };
    if vacio {
        return if l.opcional && admite_opcional(tipo) {
            Ok(())
        } else {
            Err(ErrorValor::Vacio)
        };
    }
    match tipo {
        TipoCampo::Texto | TipoCampo::Secreto => {
            if l.ascii && !legible.bytes().all(|b| (b' '..=b'~').contains(&b)) {
                return Err(ErrorValor::Ascii);
            }
            let bytes = legible.len() as i64;
            if let Some(min) = l.min.filter(|m| bytes < *m) {
                return Err(ErrorValor::Corto { min });
            }
            if let Some(max) = l.max.filter(|m| bytes > *m) {
                return Err(ErrorValor::Largo { max });
            }
        }
        TipoCampo::Opcion => {
            if !es_opcion(legible) {
                return Err(ErrorValor::Opcion);
            }
        }
        TipoCampo::Booleano => {
            if !matches!(legible, "true" | "false") {
                return Err(ErrorValor::Opcion);
            }
        }
        TipoCampo::Numero => {
            legible
                .trim()
                .parse::<i64>()
                .ok()
                .filter(|n| l.min.is_none_or(|m| *n >= m) && l.max.is_none_or(|m| *n <= m))
                .ok_or(ErrorValor::FueraDeRango {
                    min: l.min,
                    max: l.max,
                })?;
        }
        TipoCampo::Ip => {
            legible
                .trim()
                .parse::<Ipv4Addr>()
                .map_err(|_| ErrorValor::Ip)?;
        }
        TipoCampo::Mac => {
            mac(legible.trim()).ok_or(ErrorValor::Mac)?;
        }
    }
    Ok(())
}

/// MAC con `:` o con `-` (el mismo en toda ella), en minúsculas con `:`.
pub fn mac(v: &str) -> Option<String> {
    let sep = if v.contains('-') { '-' } else { ':' };
    let partes: Vec<&str> = v.split(sep).collect();
    let valida = partes.len() == 6
        && partes
            .iter()
            .all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_hexdigit()));
    valida.then(|| partes.join(":").to_ascii_lowercase())
}
