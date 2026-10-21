//! Lectura de la foto de partida ([`leer_partida`]): lo que lee el asistente
//! al instalar, y también el e2e y las pruebas, con el mismo código.
//!
//! Primero, cada lectura de partida del catálogo de funciones
//! ([`lecturas_de_partida`]: sin parámetros, o con `all=ALL` si la función lo
//! admite). Después, la lectura exacta de cada ajuste del catálogo de
//! ajustes (su `leer`, la misma que hace la página para el «ahora») que no
//! sea una de esas: un punto de acceso WiFi o una radio por su id, el pool
//! DHCP de casa, sus IP fijas. Son lecturas que el panel ya hace al enseñar
//! esos ajustes. Sus instancias se juntan en el nodo de su función
//! ([`juntar`]), así que la foto sigue siendo «función → XML».

use crate::instalar::lecturas_de_partida;
use crate::lecturas::Clave;
use crate::state::AppState;
use hgu::xml::XmlNode;
use std::collections::BTreeMap;

/// Lo leído para la foto de partida.
#[derive(Debug, Default)]
pub struct FotoLeida {
    /// Función → XML de su nodo (`XmlNode::to_xml`).
    pub contenido: BTreeMap<String, String>,
    /// Funciones con alguna lectura fallida (sin repetir).
    pub fallidas: Vec<String>,
    /// Firmware del resumen del router (vacío si no llegó).
    pub firmware: String,
}

/// Instancias de una respuesta: sus elementos con hijos o, si la respuesta
/// trae los valores sueltos (una sola instancia), uno que los agrupa con el
/// nombre de la función.
fn instancias(n: XmlNode) -> Vec<XmlNode> {
    let propios = |c: &XmlNode| c.tag != "Result" && c.tag != "ErrorString";
    if n.children.iter().any(|c| !c.children.is_empty()) {
        return n
            .children
            .into_iter()
            .filter(|c| !c.children.is_empty())
            .collect();
    }
    let hojas: Vec<XmlNode> = n.children.into_iter().filter(propios).collect();
    if hojas.is_empty() {
        return Vec::new();
    }
    vec![XmlNode {
        tag: n.tag,
        text: String::new(),
        children: hojas,
    }]
}

/// Junta en `foto` la respuesta `n` de una lectura de `funcion`: si la foto
/// aún no la tiene, tal cual; si ya tiene otra lectura de la misma función,
/// le añade las instancias de esta que no estén ya.
pub fn juntar(foto: &mut BTreeMap<String, XmlNode>, funcion: &str, n: XmlNode) {
    match foto.get_mut(funcion) {
        None => {
            foto.insert(funcion.to_owned(), n);
        }
        Some(actual) => {
            for i in instancias(n) {
                if !actual.children.contains(&i) {
                    actual.children.push(i);
                }
            }
        }
    }
}

/// Lee la foto de partida del router de `state` (ver el módulo). Quien la
/// toma de verdad (el asistente) pausa antes el sondeo.
pub async fn leer_partida(state: &AppState) -> FotoLeida {
    let mut nodos: BTreeMap<String, XmlNode> = BTreeMap::new();
    let mut out = FotoLeida::default();
    let fallo = |out: &mut FotoLeida, nombre: &str, e: &dyn std::fmt::Display| {
        tracing::warn!("foto de partida: {nombre}: {e}");
        if !out.fallidas.iter().any(|f| f == nombre) {
            out.fallidas.push(nombre.to_owned());
        }
    };
    let mut hechas: Vec<Clave> = Vec::new();
    for (nombre, params) in lecturas_de_partida(&state.catalogo) {
        hechas.push(Clave::new(nombre, &params));
        match state.hgu.call(nombre, &params).await {
            Ok(n) => {
                if nombre == "cbGetSummary" {
                    out.firmware = hgu::models::Summary::from_node(&n).firmware;
                }
                juntar(&mut nodos, nombre, n);
            }
            Err(e) => fallo(&mut out, nombre, &e),
        }
    }
    for a in state.ajustes.todos() {
        let clave = Clave::new(&a.leer.funcion, &a.leer.params);
        if hechas.contains(&clave) {
            continue;
        }
        hechas.push(clave);
        let params: Vec<(&str, &str)> = a
            .leer
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        match state.hgu.call(&a.leer.funcion, &params).await {
            Ok(n) => juntar(&mut nodos, &a.leer.funcion, n),
            Err(e) => fallo(&mut out, &a.leer.funcion, &e),
        }
    }
    out.contenido = nodos.into_iter().map(|(k, n)| (k, n.to_xml())).collect();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hoja(tag: &str, text: &str) -> XmlNode {
        XmlNode {
            tag: tag.into(),
            text: text.into(),
            children: Vec::new(),
        }
    }

    fn nodo(tag: &str, children: Vec<XmlNode>) -> XmlNode {
        XmlNode {
            tag: tag.into(),
            text: String::new(),
            children,
        }
    }

    /// Una segunda lectura de la misma función añade sus instancias, sin
    /// repetir las que ya están; una respuesta de valores sueltos cuenta
    /// como una instancia.
    #[test]
    fn juntar_por_funcion() {
        let mut foto = BTreeMap::new();
        let i0 = nodo("s", vec![hoja("s_id", "0"), hoja("a", "1")]);
        let i1 = nodo("s", vec![hoja("s_id", "1"), hoja("a", "2")]);
        juntar(
            &mut foto,
            "cbX",
            nodo("cbX", vec![i0.clone(), hoja("Result", "0")]),
        );
        juntar(&mut foto, "cbX", nodo("cbX", vec![i1.clone(), i0.clone()]));
        assert_eq!(foto["cbX"].children, vec![i0, hoja("Result", "0"), i1]);
        // Sueltos: agrupados con el nombre de la función, sin `Result`.
        juntar(
            &mut foto,
            "cbX",
            nodo("cbX", vec![hoja("s_id", "2"), hoja("Result", "0")]),
        );
        assert_eq!(
            foto["cbX"].children.last(),
            Some(&nodo("cbX", vec![hoja("s_id", "2")]))
        );
    }
}
