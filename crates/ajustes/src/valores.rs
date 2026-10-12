//! Traducción de valores entre el formato del router y el legible.
//!
//! Lo legible es lo que manejan las vistas: la clave de una opción
//! (`wpa2_wpa3`), `true`/`false` para un booleano (ya invertido si el campo lo
//! pide) y el texto tal cual para el resto.

use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use thiserror::Error;

use crate::modelo::{Campo, Opcion, TipoCampo, ValorLegible};

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
    #[error("{}", rango(*.min, *.max))]
    FueraDeRango { min: Option<i64>, max: Option<i64> },
    #[error("no es una dirección IPv4")]
    Ip,
    #[error("no es una dirección MAC")]
    Mac,
    #[error("no es una de las opciones")]
    Opcion,
    #[error("lleva caracteres de control")]
    Control,
    #[error("solo admite letras sin tilde, cifras y signos ASCII")]
    Ascii,
    #[error("el campo es de solo lectura")]
    SoloLectura,
}

/// Mensaje de [`ErrorValor::FueraDeRango`] según los límites que haya.
fn rango(min: Option<i64>, max: Option<i64>) -> String {
    match (min, max) {
        (Some(min), Some(max)) => format!("número fuera de rango: de {min} a {max}"),
        (Some(min), None) => format!("número fuera de rango: al menos {min}"),
        (None, Some(max)) => format!("número fuera de rango: como mucho {max}"),
        (None, None) => "no es un número".to_owned(),
    }
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
            ErrorValor::Ascii => "ajustes-error-ascii",
            ErrorValor::SoloLectura => "ajustes-error-solo-lectura",
        }
    }
}

/// Elemento `nombre` de una lectura: primero exacto, luego sin distinguir
/// mayúsculas.
fn leido<'a>(leidos: &'a BTreeMap<String, String>, nombre: &str) -> Option<&'a str> {
    leidos
        .get(nombre)
        .or_else(|| {
            leidos
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nombre))
                .map(|(_, v)| v)
        })
        .map(String::as_str)
}

/// Admite `opcional`: un campo que se puede dejar vacío.
pub(crate) fn admite_opcional(t: TipoCampo) -> bool {
    !matches!(t, TipoCampo::Opcion | TipoCampo::Booleano)
}

/// Valor legible de un campo a partir de los elementos de su lectura
/// (`leidos`, tal como los devuelve el router). Lo que no casa con el campo
/// es [`ValorLegible::Desconocido`] con el valor de `lee` (vacío si falta);
/// una opción con `ademas` solo se reconoce si casan todos sus elementos. Un
/// secreto es siempre [`ValorLegible::Secreto`]. MAC e IP salen
/// normalizadas cuando se pueden interpretar.
pub fn a_legible(c: &Campo, leidos: &BTreeMap<String, String>) -> ValorLegible {
    if c.tipo == TipoCampo::Secreto {
        return ValorLegible::Secreto;
    }
    let crudo = leido(leidos, &c.lee).unwrap_or("");
    let desconocido = || ValorLegible::Desconocido(crudo.to_owned());
    let texto = |t: String| ValorLegible::Texto(t);
    if c.opcional && admite_opcional(c.tipo) && crudo.trim().is_empty() {
        return texto(String::new());
    }
    match c.tipo {
        TipoCampo::Secreto => ValorLegible::Secreto,
        TipoCampo::Opcion => c
            .opciones
            .iter()
            .find(|o| {
                o.valor == crudo
                    && c.ademas.iter().all(|ad| {
                        leido(leidos, &ad.lee) == o.ademas.get(&ad.parametro).map(String::as_str)
                    })
            })
            .map_or_else(desconocido, |o| ValorLegible::Opcion(o.clave.clone())),
        TipoCampo::Booleano => {
            let (si, no) = c.si_no();
            let valor = if crudo == si {
                true
            } else if crudo == no {
                false
            } else {
                return desconocido();
            };
            ValorLegible::Booleano(valor != c.invertido)
        }
        TipoCampo::Numero => crudo
            .trim()
            .parse::<i64>()
            .map_or_else(|_| desconocido(), |n| texto(n.to_string())),
        TipoCampo::Ip => texto(
            crudo
                .trim()
                .parse::<Ipv4Addr>()
                .map_or_else(|_| crudo.to_owned(), |ip| ip.to_string()),
        ),
        TipoCampo::Mac => texto(mac(crudo.trim()).unwrap_or_else(|| crudo.to_owned())),
        TipoCampo::Texto => texto(crudo.to_owned()),
    }
}

/// Pares `(parámetro, valor)` que se envían a la función de escritura del
/// campo para el legible `legible`, validado: primero el del campo y luego
/// los de su `ademas`, en su orden. Un campo de solo lectura no se escribe.
pub fn a_router(c: &Campo, legible: &str) -> Result<Vec<(String, String)>, ErrorValor> {
    let e = c.escribible().ok_or(ErrorValor::SoloLectura)?;
    let (valor, opcion) = traducir(c, legible)?;
    let mut pares = vec![(e.parametro.clone(), valor)];
    for ad in &c.ademas {
        let v = opcion
            .and_then(|o| o.ademas.get(&ad.parametro))
            .ok_or(ErrorValor::Opcion)?;
        pares.push((ad.parametro.clone(), v.clone()));
    }
    Ok(pares)
}

/// Valor del router para el parámetro del campo, validado, y la opción
/// elegida si es un campo `opcion`. No mira si el campo es de solo lectura
/// (así se comprueba también su `recomendado`).
pub(crate) fn traducir<'c>(
    c: &'c Campo,
    legible: &str,
) -> Result<(String, Option<&'c Opcion>), ErrorValor> {
    if legible.chars().any(char::is_control) {
        return Err(ErrorValor::Control);
    }
    let vacio = match c.tipo {
        TipoCampo::Texto | TipoCampo::Secreto => legible.is_empty(),
        _ => legible.trim().is_empty(),
    };
    if vacio {
        return if c.opcional && admite_opcional(c.tipo) {
            Ok((String::new(), None))
        } else {
            Err(ErrorValor::Vacio)
        };
    }
    let (min, max) = c.validar.map_or((None, None), |v| (v.min, v.max));
    let ascii = c.validar.is_some_and(|v| v.ascii);
    let valor = match c.tipo {
        TipoCampo::Texto | TipoCampo::Secreto => {
            if ascii && !legible.bytes().all(|b| (b' '..=b'~').contains(&b)) {
                return Err(ErrorValor::Ascii);
            }
            let bytes = legible.len() as i64;
            if let Some(min) = min.filter(|m| bytes < *m) {
                return Err(ErrorValor::Corto { min });
            }
            if let Some(max) = max.filter(|m| bytes > *m) {
                return Err(ErrorValor::Largo { max });
            }
            legible.to_owned()
        }
        TipoCampo::Opcion => {
            let o = c
                .opciones
                .iter()
                .find(|o| o.clave == legible)
                .ok_or(ErrorValor::Opcion)?;
            return Ok((o.valor.clone(), Some(o)));
        }
        TipoCampo::Booleano => {
            let si = match legible {
                "true" => true,
                "false" => false,
                _ => return Err(ErrorValor::Opcion),
            };
            let (v_si, v_no) = c.si_no();
            if si != c.invertido { v_si } else { v_no }.to_owned()
        }
        TipoCampo::Numero => legible
            .trim()
            .parse::<i64>()
            .ok()
            .filter(|n| min.is_none_or(|m| *n >= m) && max.is_none_or(|m| *n <= m))
            .ok_or(ErrorValor::FueraDeRango { min, max })?
            .to_string(),
        TipoCampo::Ip => legible
            .trim()
            .parse::<Ipv4Addr>()
            .map_err(|_| ErrorValor::Ip)?
            .to_string(),
        TipoCampo::Mac => mac(legible.trim()).ok_or(ErrorValor::Mac)?,
    };
    Ok((valor, None))
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
