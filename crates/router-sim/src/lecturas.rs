//! Lecturas avanzadas: fixtures con lo escrito aplicado.

use crate::escrituras::construir_lista;
use crate::estado::{FixtureKey, Inner, ListaViva, Params, SimState};
use crate::fixtures::fixture_key;
use crate::http::param;
use crate::xml::{failure, for_each_node, response, result_node, set_child};
use hgu::xml::XmlNode;
use std::collections::BTreeMap;

pub(crate) fn read(s: &Inner, st: &SimState, name: &str, params: &[(String, String)]) -> String {
    match read_node(s, st, name, params) {
        Ok(node) => response(&node),
        Err(e) => failure(name, &e),
    }
}

/// La respuesta de la lectura `name` como nodo, con lo escrito aplicado.
pub(crate) fn read_node(
    s: &Inner,
    st: &SimState,
    name: &str,
    params: &[(String, String)],
) -> Result<XmlNode, String> {
    if let Some(viva) = st.listas.get(name)
        && let Some(r) = read_lista(viva, name, params)
    {
        return r;
    }
    // Una lectura por posición (`X_0i`) de una lista sin fixture para esa
    // posición no cae en la de sin parámetros: la posición no existe.
    let por_posicion = params.iter().any(|(k, _)| k.ends_with("_0i"));
    let raw = s.fixtures.get(&fixture_key(name, params)).or_else(|| {
        (!(por_posicion && es_lectura_de_lista(s, name)))
            .then(|| s.fixtures.get(&(name.to_owned(), Vec::new())))
            .flatten()
    });
    // Una lista que se puede escribir y todavía no se ha tocado atiende
    // también las lecturas de una instancia que solo existe en `all=ALL`.
    if raw.is_none()
        && es_lectura_de_lista(s, name)
        && let Some(r) = construir_lista(s, name)
            .ok()
            .and_then(|viva| read_lista(&viva, name, params))
    {
        return r;
    }
    let mut node = if let Some(raw) = raw {
        hgu::xml::parse_response(name, raw).map_err(|e| format!("fixture no válida: {e}"))?
    } else if let Some(n) = instancia_de_la_lista(s, name, params) {
        n
    } else if params.is_empty() {
        all_instances(s, name).ok_or_else(|| "no fixture".to_owned())?
    } else {
        return Err("no fixture".to_owned());
    };
    aplicar_ediciones(st, name, &mut node);
    patch(s, st, name, &mut node);
    tapar_al_leer(name, &mut node);
    Ok(node)
}

/// Secretos que el router no deja leer (los marca `tapado` el catálogo de
/// funciones): se escriben, pero al leerlos salen vacíos, como en las
/// fixtures del router real.
const TAPADOS_AL_LEER: &[(&str, &str)] = &[("cbGetVoiceLine", "auth_password")];

fn tapar_al_leer(name: &str, n: &mut XmlNode) {
    for (lectura, campo) in TAPADOS_AL_LEER {
        if *lectura == name {
            vaciar(n, campo);
        }
    }
}

fn vaciar(n: &mut XmlNode, campo: &str) {
    if n.children.is_empty() && n.tag == campo {
        n.text.clear();
    }
    for c in &mut n.children {
        vaciar(c, campo);
    }
}

/// Valor de `campo` en el primer nodo (en preorden) que lo tiene.
pub(crate) fn valor_de(n: &XmlNode, campo: &str) -> Option<String> {
    n.text_of(campo)
        .map(str::to_owned)
        .or_else(|| n.children.iter().find_map(|c| valor_de(c, campo)))
}

/// Lecturas que comparten instancias con la lectura pareada de una
/// escritura: la lista por radio de puntos de acceso WiFi muestra los mismos
/// puntos que `cbGetWifiAccPt`.
fn lecturas_hermanas(name: &str) -> &'static [&'static str] {
    match name {
        "cbListWifiAccPt" => &["cbGetWifiAccPt"],
        _ => &[],
    }
}

/// Lista que contiene las mismas instancias que una lectura de una sola:
/// `cbGetWifiAccPt` (un punto de acceso) sale de `cbListWifiAccPt` (los de
/// una radio) cuando no hay fixture para ese punto de acceso.
fn lista_de_instancias(name: &str) -> Option<&'static str> {
    match name {
        "cbGetWifiAccPt" => Some("cbListWifiAccPt"),
        _ => None,
    }
}

/// Lectura de una instancia de `name` construida desde las fixtures de su
/// lista ([`lista_de_instancias`]): la instancia cuyos hijos casan con los
/// parámetros (`X_0i=v` elige la de `X_id=v`). Leída por `*_id`, devuelve
/// `*_0i = -1`, como el router. Las ediciones se aplican después, como a
/// cualquier lectura. `None` si no hay lista o ninguna instancia casa.
fn instancia_de_la_lista(s: &Inner, name: &str, params: &[(String, String)]) -> Option<XmlNode> {
    let lista = lista_de_instancias(name)?;
    let filtros: Vec<(String, &str, bool)> = params
        .iter()
        .filter(|(k, _)| k != "sessionKey")
        .map(|(k, v)| match k.strip_suffix("_0i") {
            Some(base) => (format!("{base}_id"), v.as_str(), true),
            None => (k.clone(), v.as_str(), false),
        })
        .collect();
    if filtros.is_empty() {
        return None;
    }
    let mut claves: Vec<&FixtureKey> = s.fixtures.keys().filter(|(n, _)| n == lista).collect();
    claves.sort();
    let mut nodo = claves.into_iter().find_map(|k| {
        let n = hgu::xml::parse_response(lista, &s.fixtures[k]).ok()?;
        n.children
            .into_iter()
            .find(|c| filtros.iter().all(|(k, v, _)| c.text_of(k) == Some(*v)))
    })?;
    for (hijo, _, indice) in &filtros {
        if let (false, Some(base)) = (indice, hijo.strip_suffix("_id")) {
            set_child(&mut nodo, &format!("{base}_0i"), "-1");
        }
    }
    Some(result_node(name, vec![nodo], "0", ""))
}

/// Aplica sobre la respuesta lo escrito en sus instancias: solo cambia hijos
/// que ya existen. Una edición con instancia busca esos nodos a cualquier
/// profundidad.
fn aplicar_ediciones(st: &SimState, name: &str, node: &mut XmlNode) {
    let hermanas = lecturas_hermanas(name);
    for e in &st.ediciones {
        if e.lectura == name || hermanas.contains(&e.lectura.as_str()) {
            if e.ident.is_empty() {
                // Sin instancia: solo la respuesta y sus hijos directos
                // (el contenedor de los campos), no nodos más profundos.
                aplicar_campos(node, &[], &e.campos);
                for c in &mut node.children {
                    aplicar_campos(c, &[], &e.campos);
                }
            } else {
                for_each_node(node, &mut |n| aplicar_campos(n, &e.ident, &e.campos));
            }
        }
    }
}

/// Escribe `campos` en `n` si es la instancia que `ident` identifica.
pub(crate) fn aplicar_campos(
    n: &mut XmlNode,
    ident: &[(String, String)],
    campos: &BTreeMap<String, String>,
) {
    if ident.iter().all(|(k, v)| n.text_of(k) == Some(v.as_str())) {
        for (k, v) in campos {
            set_child(n, k, v);
        }
    }
}

/// `true` si alguna escritura del catálogo con lista tiene `name` por lectura
/// pareada.
fn es_lectura_de_lista(s: &Inner, name: &str) -> bool {
    s.catalogo.todas().iter().any(|f| {
        f.lista.is_some()
            && f.lectura_pareada
                .as_ref()
                .is_some_and(|l| l.funcion == name)
    })
}

/// Lectura servida desde la lista materializada de `name`, si la hay y la
/// lectura pide todas (`all=ALL`, o sin parámetros), una posición o filtra
/// por un campo de la instancia; `None` si no aplica.
///
/// Un parámetro `X_0i=v` elige la instancia de la posición `v` de la lista,
/// como el router real: la posición no tiene por qué ser su id (`X_id`), y
/// cambia al borrar otra antes. La instancia leída así trae `X_0i = v`. Una
/// lectura por `*_id` devuelve `*_0i = -1`, como el router real; con
/// `all=ALL`, el `X_0i` de cada instancia.
fn read_lista(
    lista: &ListaViva,
    name: &str,
    params: &[(String, String)],
) -> Option<Result<XmlNode, String>> {
    // (hijo que se compara, valor)
    let mut filtros: Vec<(String, &str)> = Vec::new();
    // (parámetro de posición, posición)
    let mut posicion: Option<(&str, &str)> = None;
    for (k, v) in params
        .iter()
        .filter(|(k, _)| k != "all" && k != "sessionKey")
    {
        let de_posicion = k
            .strip_suffix("_0i")
            .is_some_and(|b| lista.plantilla.get(&format!("{b}_id")).is_some());
        if de_posicion {
            posicion = Some((k, v));
        } else if lista.plantilla.get(k).is_some() {
            filtros.push((k.clone(), v));
        }
    }
    let todas = param(params, "all") == Some("ALL");
    let sin_parametros = params.iter().all(|(k, _)| k == "sessionKey");
    if !(todas || sin_parametros || posicion.is_some() || !filtros.is_empty()) {
        return None;
    }
    let mut nodos: Vec<XmlNode> = match posicion {
        Some((k, v)) => {
            let Some(mut n) = v
                .parse::<usize>()
                .ok()
                .and_then(|i| lista.nodos.get(i))
                .cloned()
            else {
                return Some(Err("BAD_PARAM".to_owned()));
            };
            set_child(&mut n, k, v);
            vec![n]
        }
        None => lista.nodos.clone(),
    };
    nodos.retain(|n| filtros.iter().all(|(k, v)| n.text_of(k) == Some(*v)));
    if (posicion.is_some() || !filtros.is_empty()) && nodos.is_empty() {
        return Some(Err("no existe".to_owned()));
    }
    if !todas {
        for (hijo, _) in &filtros {
            if let Some(base) = hijo.strip_suffix("_id") {
                for n in &mut nodos {
                    set_child(n, &format!("{base}_0i"), "-1");
                }
            }
        }
    }
    Some(Ok(result_node(name, nodos, "0", "")))
}

/// Lista leída sin elegir instancia y sin `<name>.xml`: el router devuelve
/// todas las instancias (así responden las listas de las que hay fixture sin
/// parámetros, como `cbGetWanIPIntf`). Junta las de las fixtures de `name`
/// que eligen la instancia con el mismo parámetro (el que más fixtures
/// tenga; a igualdad, el primero por nombre), en orden de valor.
pub(crate) fn all_instances(s: &Inner, name: &str) -> Option<XmlNode> {
    let mut groups: BTreeMap<Vec<&str>, Vec<(&Params, &String)>> = BTreeMap::new();
    for ((n, p), raw) in &s.fixtures {
        if n == name && !p.is_empty() {
            let names = p.iter().map(|(k, _)| k.as_str()).collect();
            groups.entry(names).or_default().push((p, raw));
        }
    }
    let (_, mut group) = groups.into_iter().rev().max_by_key(|(_, g)| g.len())?;
    group.sort();
    let mut children = Vec::new();
    for (_, raw) in group {
        let n = hgu::xml::parse_response(name, raw).ok()?;
        children.extend(
            n.children
                .into_iter()
                .filter(|c| c.tag != "Result" && c.tag != "ErrorString"),
        );
    }
    Some(result_node(name, children, "0", ""))
}

/// Refleja el estado en memoria sobre la respuesta de la fixture.
fn patch(s: &Inner, st: &SimState, name: &str, node: &mut XmlNode) {
    match name {
        "cbGetSummary" => {
            if let Some(t) = st.rebooted {
                set_child(node, "SysUptime", &t.elapsed().as_secs().to_string());
            }
            if let Some(fw) = &st.firmware {
                set_child(node, "SWVersion", fw);
            }
        }
        "cbGetWanIPIntf" if !st.wan_up => for_each_node(node, &mut |n| {
            if s.wan
                .as_deref()
                .is_some_and(|w| n.text_of("name") == Some(w))
            {
                set_child(n, "opr_status", "0");
            }
        }),
        "cbGetStatistic" => for_each_node(node, &mut |n| {
            if let Some(&(rx, tx)) = n.text_of("name").and_then(|m| st.counters_extra.get(m)) {
                for (tag, extra) in [("rx_byte", rx), ("tx_byte", tx)] {
                    if let Some(v) = n.text_of(tag).and_then(|v| v.parse::<u64>().ok()) {
                        set_child(n, tag, &v.saturating_add(extra).to_string());
                    }
                }
            }
        }),
        _ => {}
    }
}
