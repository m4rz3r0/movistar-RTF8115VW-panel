//! Parámetros por ranuras ([`catalogo::Parametro::ranuras`]): las IP
//! reservadas de un pool DHCP, que el router guarda en 32 ranuras
//! (`reserved_ipv4_addr_<i>` y `reserved_ipv4_mask_<i>`) con 0.0.0.0 en las
//! libres. El primer parámetro de ranuras de la función (la IP) dice si una
//! ranura está libre.

use super::{BTreeMap, BTreeSet, Funcion, Operacion, Parametro, buscar, iguales};
use catalogo::RANURA_LIBRE;

/// Valor de los demás parámetros de una ranura nueva que llegan vacíos:
/// una sola dirección.
const UNA_SOLA: &str = "255.255.255.255";

/// Los parámetros de ranuras de `f`, en su orden.
fn de(f: &Funcion) -> Vec<&Parametro> {
    f.parametros
        .iter()
        .filter(|p| p.ranuras.is_some())
        .collect()
}

fn libre(v: &str) -> bool {
    let v = v.trim();
    v.is_empty() || v == RANURA_LIBRE
}

/// Nombre de la ranura `i` de `p`.
pub(super) fn nombre(p: &Parametro, i: u8) -> String {
    format!("{}_{i}", p.nombre)
}

/// Ranuras usadas de `f` según la lectura `actual`, en orden.
pub(crate) fn usadas(f: &Funcion, actual: &BTreeMap<String, String>) -> Vec<u8> {
    let Some(ip) = de(f).first().copied() else {
        return Vec::new();
    };
    (0..ip.ranuras.unwrap_or(0))
        .filter(|i| buscar(actual, &nombre(ip, *i)).is_some_and(|v| !libre(v)))
        .collect()
}

/// La ranura de una nueva, como la web: la siguiente a la última usada; si
/// la última está usada, la primera libre. `None` si no queda ninguna.
fn nueva(f: &Funcion, actual: &BTreeMap<String, String>) -> Option<u8> {
    let total = de(f).first()?.ranuras?;
    let usadas = usadas(f, actual);
    match usadas.last() {
        None => Some(0),
        Some(u) if u + 1 < total => Some(u + 1),
        Some(_) => (0..total).find(|i| !usadas.contains(i)),
    }
}

/// Pone en `valores` las ranuras como las manda la web, sobre la lectura
/// `actual`:
///
/// - Un parámetro de ranuras sin índice (`reserved_ipv4_addr`) es una
///   nueva: va a la ranura de [`nueva`]; vacío, no hay nueva. Los demás
///   parámetros vacíos de la nueva, [`UNA_SOLA`].
/// - Liberar una ranura usada (su IP a 0.0.0.0) pone también a 0.0.0.0 sus
///   demás parámetros, como la web al borrar.
///
/// Solo al editar. Con `explicito` (deshacer o reversión, con los valores
/// anteriores reales), las ranuras van tal cual.
pub(in crate::changes) fn preparar(
    f: &Funcion,
    op: Operacion,
    explicito: bool,
    actual: &BTreeMap<String, String>,
    valores: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let ps = de(f);
    let Some(ip) = ps.first().copied() else {
        return Ok(());
    };
    let nuevos: Vec<(&Parametro, String)> = ps
        .iter()
        .filter_map(|p| Some((*p, valores.remove(&p.nombre)?.trim().to_owned())))
        .collect();
    let hay_nueva = nuevos.iter().any(|(_, v)| !v.is_empty());
    let con_indice = valores.keys().any(|k| f.ranura(k).is_some());
    if op != Operacion::Editar && (hay_nueva || con_indice) {
        return Err(format!("{}: solo al editar", ip.nombre));
    }
    if explicito {
        return Ok(());
    }
    // Liberar una ranura usada.
    for i in usadas(f, actual) {
        if valores.get(&nombre(ip, i)).is_some_and(|v| libre(v)) {
            for p in &ps {
                valores.insert(nombre(p, i), RANURA_LIBRE.to_owned());
            }
        }
    }
    if !hay_nueva {
        return Ok(());
    }
    let ip_nueva = nuevos
        .iter()
        .find(|(p, _)| p.nombre == ip.nombre)
        .map(|(_, v)| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("{}: falta la IP de la nueva", ip.nombre))?;
    let i = nueva(f, actual).ok_or_else(|| {
        format!(
            "{}: no queda ninguna ranura libre ({} usadas)",
            ip.nombre,
            ip.ranuras.unwrap_or(0)
        )
    })?;
    valores.insert(nombre(ip, i), ip_nueva.to_owned());
    for p in ps.iter().skip(1) {
        let v = nuevos
            .iter()
            .find(|(q, _)| q.nombre == p.nombre)
            .map(|(_, v)| v.as_str())
            .filter(|v| !v.is_empty())
            .unwrap_or(UNA_SOLA);
        valores.insert(nombre(p, i), v.to_owned());
    }
    Ok(())
}

/// Ranuras de `p` que trae `m`, por su índice: `(nombre, valor)`.
pub(super) fn en<'a>(
    f: &Funcion,
    p: &Parametro,
    m: &'a BTreeMap<String, String>,
) -> Vec<(&'a str, &'a str)> {
    let mut v: Vec<(u8, &str, &str)> = m
        .iter()
        .filter_map(|(k, v)| {
            let (q, i) = f.ranura(k)?;
            (q.nombre == p.nombre).then_some((i, k.as_str(), v.as_str()))
        })
        .collect();
    v.sort_unstable_by_key(|(i, _, _)| *i);
    v.into_iter().map(|(_, k, v)| (k, v)).collect()
}

/// Índices de las ranuras que `efectivos` cambia respecto a la lectura
/// `actual` (alguno de sus parámetros es distinto).
pub(super) fn cambiadas(
    f: &Funcion,
    actual: &BTreeMap<String, String>,
    efectivos: &BTreeMap<String, String>,
) -> BTreeSet<u8> {
    efectivos
        .iter()
        .filter_map(|(k, v)| {
            let (p, i) = f.ranura(k)?;
            let igual = buscar(actual, k).is_some_and(|cur| iguales(Some(p), cur, v));
            (!igual).then_some(i)
        })
        .collect()
}
