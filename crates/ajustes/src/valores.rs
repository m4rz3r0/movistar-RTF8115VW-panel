//! Traducción de valores entre el formato del router y el legible.
//!
//! Lo legible es lo que manejan las vistas: la clave de una opción
//! (`wpa2_wpa3`), `true`/`false` para un booleano (ya invertido si el campo lo
//! pide) y el texto tal cual para el resto.

use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use crate::legible::{ErrorValor, Limites, admite_opcional, mac, validar_legible};
use crate::modelo::{Ajuste, Campo, Opcion, SIN_ELECCION, TipoCampo, ValorLegible};

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
        TipoCampo::Opcion if c.sin_eleccion.as_deref() == Some(crudo) => {
            ValorLegible::Opcion(SIN_ELECCION.to_owned())
        }
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
    let limites = Limites {
        min: c.validar.and_then(|v| v.min),
        max: c.validar.and_then(|v| v.max),
        ascii: c.validar.is_some_and(|v| v.ascii),
        opcional: c.opcional,
    };
    validar_legible(
        c.tipo,
        limites,
        |k| c.opciones.iter().any(|o| o.clave == k),
        legible,
    )?;
    let vacio = match c.tipo {
        TipoCampo::Texto | TipoCampo::Secreto => legible.is_empty(),
        _ => legible.trim().is_empty(),
    };
    if vacio {
        return Ok((String::new(), None));
    }
    // Ya validado: las conversiones no fallan.
    let valor = match c.tipo {
        TipoCampo::Texto | TipoCampo::Secreto => legible.to_owned(),
        TipoCampo::Opcion => {
            let o = c
                .opciones
                .iter()
                .find(|o| o.clave == legible)
                .ok_or(ErrorValor::Opcion)?;
            return Ok((o.valor.clone(), Some(o)));
        }
        TipoCampo::Booleano => {
            let (v_si, v_no) = c.si_no();
            if (legible == "true") != c.invertido {
                v_si
            } else {
                v_no
            }
            .to_owned()
        }
        TipoCampo::Numero => legible
            .trim()
            .parse::<i64>()
            .map_err(|_| ErrorValor::FueraDeRango {
                min: limites.min,
                max: limites.max,
            })?
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

/// Primer campo de `a` que, con los `valores` pedidos (id → legible), exige
/// otro que no viene, viene vacío o viene «sin elegir» ([`SIN_ELECCION`]):
/// `(el que lo pide, el que falta)`. Un campo que no se toca no exige nada.
pub fn falta_requerido<'a>(
    a: &'a Ajuste,
    valores: &BTreeMap<String, String>,
) -> Option<(&'a Campo, &'a Campo)> {
    a.campos.iter().find_map(|c| {
        let r = c.requiere.as_ref()?;
        if valores.get(&c.id) != Some(&r.cuando) {
            return None;
        }
        let exigido = a.campo(&r.campo)?;
        valores
            .get(&exigido.id)
            .is_none_or(|v| {
                v.trim().is_empty() || (exigido.sin_eleccion.is_some() && v == SIN_ELECCION)
            })
            .then_some((c, exigido))
    })
}
