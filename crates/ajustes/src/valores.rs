//! Traducción de valores entre el formato del router y el legible.
//!
//! Lo legible es lo que manejan las vistas: la clave de una opción
//! (`wpa2_wpa3`), `true`/`false` para un booleano (ya invertido si el campo lo
//! pide) y el texto tal cual para el resto.

use std::net::Ipv4Addr;
use thiserror::Error;

use crate::modelo::{Campo, TipoCampo, ValorLegible};

/// Valor legible que no se puede escribir. Los mensajes no llevan el valor
/// (puede ser un secreto); el texto para la interfaz es [`ErrorValor::clave_i18n`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorValor {
    #[error("el valor no puede estar vacío")]
    Vacio,
    #[error("demasiado corto: al menos {min} bytes")]
    Corto { min: i64 },
    #[error("demasiado largo: como mucho {max} bytes")]
    Largo { max: i64 },
    #[error("número fuera de rango ({min:?}..={max:?})")]
    FueraDeRango { min: Option<i64>, max: Option<i64> },
    #[error("no es una dirección IPv4")]
    Ip,
    #[error("no es una dirección MAC")]
    Mac,
    #[error("no es una de las opciones")]
    Opcion,
    #[error("lleva caracteres de control")]
    Control,
}

impl ErrorValor {
    /// Clave de traducción del mensaje (`ajustes-error-…`).
    pub fn clave_i18n(&self) -> &'static str {
        match self {
            ErrorValor::Vacio => "ajustes-error-vacio",
            ErrorValor::Corto { .. } => "ajustes-error-corto",
            ErrorValor::Largo { .. } => "ajustes-error-largo",
            ErrorValor::FueraDeRango { .. } => "ajustes-error-fuera-de-rango",
            ErrorValor::Ip => "ajustes-error-ip",
            ErrorValor::Mac => "ajustes-error-mac",
            ErrorValor::Opcion => "ajustes-error-opcion",
            ErrorValor::Control => "ajustes-error-control",
        }
    }
}

/// Valor legible de `crudo`, tal como lo devuelve el router. Lo que no
/// casa con el campo es [`ValorLegible::Desconocido`]; un secreto es
/// siempre [`ValorLegible::Secreto`].
pub fn a_legible(c: &Campo, crudo: &str) -> ValorLegible {
    let desconocido = || ValorLegible::Desconocido(crudo.to_owned());
    match c.tipo {
        TipoCampo::Secreto => ValorLegible::Secreto,
        TipoCampo::Opcion => c
            .opciones
            .iter()
            .find(|o| o.valor == crudo)
            .map_or_else(desconocido, |o| ValorLegible::Opcion(o.clave.clone())),
        TipoCampo::Booleano => {
            let si = if crudo == c.booleano.si {
                true
            } else if crudo == c.booleano.no {
                false
            } else {
                return desconocido();
            };
            ValorLegible::Booleano(si != c.invertido)
        }
        TipoCampo::Numero if crudo.trim().parse::<i64>().is_err() => desconocido(),
        TipoCampo::Texto | TipoCampo::Numero | TipoCampo::Ip | TipoCampo::Mac => {
            ValorLegible::Texto(crudo.to_owned())
        }
    }
}

/// Valor que se envía al router para el legible `legible`, validado.
pub fn a_router(c: &Campo, legible: &str) -> Result<String, ErrorValor> {
    if legible.chars().any(char::is_control) {
        return Err(ErrorValor::Control);
    }
    let (min, max) = c.validar.map_or((None, None), |v| (v.min, v.max));
    match c.tipo {
        TipoCampo::Texto | TipoCampo::Secreto => {
            let bytes = legible.len() as i64;
            if bytes == 0 && min.is_some_and(|m| m > 0) {
                return Err(ErrorValor::Vacio);
            }
            if let Some(min) = min.filter(|m| bytes < *m) {
                return Err(ErrorValor::Corto { min });
            }
            if let Some(max) = max.filter(|m| bytes > *m) {
                return Err(ErrorValor::Largo { max });
            }
            Ok(legible.to_owned())
        }
        TipoCampo::Opcion => c
            .opciones
            .iter()
            .find(|o| o.clave == legible)
            .map(|o| o.valor.clone())
            .ok_or(ErrorValor::Opcion),
        TipoCampo::Booleano => {
            let si = match legible {
                "true" => true,
                "false" => false,
                _ => return Err(ErrorValor::Opcion),
            };
            let b = &c.booleano;
            Ok(if si != c.invertido { &b.si } else { &b.no }.clone())
        }
        TipoCampo::Numero => {
            let v = no_vacio(legible)?;
            let n = v
                .parse::<i64>()
                .ok()
                .filter(|n| min.is_none_or(|m| *n >= m) && max.is_none_or(|m| *n <= m))
                .ok_or(ErrorValor::FueraDeRango { min, max })?;
            Ok(n.to_string())
        }
        TipoCampo::Ip => no_vacio(legible)?
            .parse::<Ipv4Addr>()
            .map(|ip| ip.to_string())
            .map_err(|_| ErrorValor::Ip),
        TipoCampo::Mac => mac(no_vacio(legible)?).ok_or(ErrorValor::Mac),
    }
}

/// `v` sin espacios alrededor; vacío es un error.
fn no_vacio(v: &str) -> Result<&str, ErrorValor> {
    let v = v.trim();
    if v.is_empty() {
        Err(ErrorValor::Vacio)
    } else {
        Ok(v)
    }
}

/// MAC con `:` o con `-` (el mismo en toda ella), en minúsculas con `:`.
fn mac(v: &str) -> Option<String> {
    let sep = if v.contains('-') { '-' } else { ':' };
    let partes: Vec<&str> = v.split(sep).collect();
    let valida = partes.len() == 6
        && partes
            .iter()
            .all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_hexdigit()));
    valida.then(|| partes.join(":").to_ascii_lowercase())
}
