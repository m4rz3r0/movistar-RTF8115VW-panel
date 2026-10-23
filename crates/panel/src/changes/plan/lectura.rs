//! La lectura pareada de una escritura: dónde se lee lo que escribe.

use super::super::CambioError;
use catalogo::Funcion;
use std::collections::{BTreeMap, BTreeSet};

/// Dónde leer lo que escribe una función.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::changes) enum Lectura {
    /// Llamada `cb*` de la UI avanzada. `clave_id`: parámetro de la lectura
    /// que lleva el id de la instancia (se quita para listarlas todas).
    /// `buscar`: la instancia se busca por su id recorriendo posiciones (ver
    /// [`BuscarId`]); su parámetro de posición no va en `params`.
    Xml {
        funcion: String,
        params: Vec<(String, String)>,
        clave_id: Option<String>,
        buscar: Option<BuscarId>,
    },
    /// Variables `var x='…'` de una página de la UI básica.
    Pagina(String),
    /// Sin lectura comprobable.
    Ninguna,
    /// La lista de usuarios de la web del router (`cbGetUserList`), con el
    /// nombre como id (ver `changes::especiales`).
    Usuarios,
    /// Las reglas del cortafuegos (`var filter` de su página de la lista),
    /// con el RID como id (ver `changes::especiales::cortafuegos`).
    Cortafuegos,
}

/// Instancia de una lista que se lee por su id: la lectura pareada pasa el
/// id de la escritura (`{X_id}`) como posición (`X_0i`), pero el router
/// elige por posición, que no tiene por qué ser el id (el pool DHCP de id 0
/// está en la posición 1 del router real). Se recorren las posiciones con
/// [`hgu::Client::call_por_id`] hasta la que tiene `campo = valor`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::changes) struct BuscarId {
    /// Parámetro de posición de la lectura (`X_0i`).
    pub(in crate::changes) posicion: String,
    /// Campo del id en la instancia (`X_id`).
    pub(in crate::changes) campo: String,
    pub(in crate::changes) valor: String,
}

/// ¿Pasa la lectura pareada el id `p` (`X_id`) como la posición `k`
/// (`X_0i`) de la misma lista?
fn posicion_por_id(k: &str, p: &str) -> bool {
    matches!((k.strip_suffix("_0i"), p.strip_suffix("_id")), (Some(a), Some(b)) if a == b)
}

/// Parámetros que identifican la instancia: los que usa la lectura pareada
/// (`{param}`) y el id de la lista.
pub(crate) fn identidad(f: &Funcion) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = f
        .lectura_pareada
        .iter()
        .flat_map(|lp| &lp.params)
        .filter_map(|(_, v)| v.strip_prefix('{').and_then(|v| v.strip_suffix('}')))
        .map(str::to_owned)
        .collect();
    if let Some(l) = &f.lista {
        out.insert(l.id_param.clone());
    }
    out
}

/// Lectura pareada con los `{param}` sustituidos por `valores` (o la
/// instancia, para el id de la lista). Un `X_0i = {X_id}` no lee la
/// posición del id: busca la instancia de ese id ([`BuscarId`]).
pub(in crate::changes) fn lectura_de(
    f: &Funcion,
    valores: &BTreeMap<String, String>,
    instancia: Option<&str>,
) -> Result<Lectura, CambioError> {
    // Antes que la regla de las páginas `.asp`: el cortafuegos se lee de
    // la suya, pero no son sus variables.
    if let Some(e) = super::super::especiales::Especial::de(f) {
        return Ok(e.lectura());
    }
    let Some(lp) = &f.lectura_pareada else {
        return Ok(Lectura::Ninguna);
    };
    if lp.funcion.ends_with(".asp") {
        return Ok(Lectura::Pagina(lp.funcion.clone()));
    }
    if !lp.funcion.starts_with("cb") {
        // Lecturas `.cmd` de la UI básica (texto sin formato conocido).
        return Ok(Lectura::Ninguna);
    }
    let id_param = f.lista.as_ref().map(|l| l.id_param.as_str());
    let mut params = Vec::new();
    let mut clave_id = None;
    let mut buscar = None;
    for (k, v) in &lp.params {
        let valor = match v.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
            Some(p) => {
                if Some(p) == id_param {
                    clave_id = Some(k.clone());
                }
                let valor = valores
                    .get(p)
                    .map(String::as_str)
                    .or(if Some(p) == id_param { instancia } else { None })
                    .ok_or_else(|| CambioError::Invalida(format!("falta el parámetro {p}")))?
                    .to_owned();
                if posicion_por_id(k, p) {
                    buscar = Some(BuscarId {
                        posicion: k.clone(),
                        campo: p.to_owned(),
                        valor,
                    });
                    continue;
                }
                valor
            }
            None if Some(k.as_str()) == id_param => {
                clave_id = Some(k.clone());
                instancia.unwrap_or(v).to_owned()
            }
            None => v.clone(),
        };
        params.push((k.clone(), valor));
    }
    Ok(Lectura::Xml {
        funcion: lp.funcion.clone(),
        params,
        clave_id,
        buscar,
    })
}
