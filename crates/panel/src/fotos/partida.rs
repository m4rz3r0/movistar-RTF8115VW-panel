//! Lectura de la foto de partida ([`leer_partida`]): lo que lee el asistente
//! al instalar, y también el e2e y las pruebas, con el mismo código.
//!
//! Primero, cada lectura de partida del catálogo de funciones
//! ([`lecturas_de_partida`]: sin parámetros, o con `all=ALL` si la función lo
//! admite). Después, la lectura exacta de cada ajuste del catálogo de
//! ajustes (su `leer`, la misma que hace la página para el «ahora») que no
//! sea una de esas: un punto de acceso WiFi o una radio por su id, el pool
//! DHCP de casa (buscado por su id, como lo lee la página), sus IP fijas.
//! Son lecturas que el panel ya hace al enseñar esos ajustes. Sus
//! instancias se juntan en el nodo de su función ([`juntar`]), así que la
//! foto sigue siendo «función → XML».

use crate::instalar::lecturas_de_partida;
use crate::lecturas::Clave;
use crate::state::AppState;
use ajustes::Ajuste;
use hgu::xml::XmlNode;
use std::collections::BTreeMap;

/// Lo leído para la foto de partida. Su `Debug` no enseña el XML (lleva los
/// secretos), solo de qué funciones es.
#[derive(Default)]
pub struct FotoLeida {
    /// Función → XML de su nodo (`XmlNode::to_xml`).
    pub contenido: BTreeMap<String, String>,
    /// Funciones con alguna lectura fallida (sin repetir).
    pub fallidas: Vec<String>,
    /// Firmware del resumen del router (vacío si no llegó).
    pub firmware: String,
}

impl std::fmt::Debug for FotoLeida {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FotoLeida")
            .field("contenido", &self.contenido.keys().collect::<Vec<_>>())
            .field("fallidas", &self.fallidas)
            .field("firmware", &self.firmware)
            .finish()
    }
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

/// Id de la instancia `i` según el parámetro de instancia `id_param` de su
/// función (`X_0i` o `X_id`): el valor de `X_id` o, si no lo trae, el de
/// `X_0i` (leída por id, el router da `X_0i = -1`, que no cuenta).
fn id_de(i: &XmlNode, id_param: &str) -> Option<String> {
    let base = id_param
        .strip_suffix("_0i")
        .or_else(|| id_param.strip_suffix("_id"))?;
    let valor = |tag: String| {
        i.children
            .iter()
            .find(|c| c.children.is_empty() && c.tag.eq_ignore_ascii_case(&tag))
            .map(|c| c.text.clone())
            .filter(|v| !v.is_empty() && v != "-1")
    };
    valor(format!("{base}_id")).or_else(|| valor(format!("{base}_0i")))
}

/// Junta en `foto` la respuesta `n` de una lectura de `funcion`: si la foto
/// aún no la tiene, tal cual; si ya tiene otra lectura de la misma función,
/// le añade las instancias de esta que no estén ya. Con `id_param` (el
/// parámetro de instancia de la función), una instancia que ya está es la
/// del mismo id aunque llegue algo distinta (`X_0i = 1` sin parámetros,
/// `X_0i = -1` leída por id); sin él, la idéntica.
pub fn juntar(
    foto: &mut BTreeMap<String, XmlNode>,
    funcion: &str,
    n: XmlNode,
    id_param: Option<&str>,
) {
    match foto.get_mut(funcion) {
        None => {
            foto.insert(funcion.to_owned(), n);
        }
        Some(actual) => {
            for i in instancias(n) {
                let id = id_param.and_then(|p| id_de(&i, p));
                let ya = actual.children.iter().any(|c| match (&id, id_param) {
                    (Some(id), Some(p)) => id_de(c, p).as_ref() == Some(id),
                    _ => *c == i,
                });
                if !ya {
                    actual.children.push(i);
                }
            }
        }
    }
}

/// Parámetro de instancia de la función `nombre`, si es una lista.
fn id_param<'a>(state: &'a AppState, nombre: &str) -> Option<&'a str> {
    state
        .catalogo
        .funcion(nombre)
        .and_then(|f| f.lista.as_ref())
        .map(|l| l.id_param.as_str())
}

/// ¿Se deja `a` fuera de la foto? La foto guarda un nodo por función, así
/// que una lista que lee con un filtro la misma función que otra lista lee
/// sin él (la DMZ, con `dmz=1`, frente a las reglas de puertos) mezclaría
/// sus instancias con las de la otra: ni se lee para la foto ni se compara.
/// Es por ajuste; `instalar::FUERA_DE_LA_FOTO` deja fuera, por función, lo
/// que no lee el router (la sesión del propio panel).
pub fn fuera_de_la_foto(ajustes: &[Ajuste], a: &Ajuste) -> bool {
    a.lista
        && !a.leer.params.is_empty()
        && ajustes.iter().any(|b| {
            b.id != a.id && b.lista && b.leer.funcion == a.leer.funcion && b.leer.params.is_empty()
        })
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
                juntar(&mut nodos, nombre, n, id_param(state, nombre));
            }
            Err(e) => fallo(&mut out, nombre, &e),
        }
    }
    for a in state.ajustes.todos() {
        let clave = crate::ajustes_router::servidor::clave(a);
        if hechas.contains(&clave) || fuera_de_la_foto(state.ajustes.todos(), a) {
            continue;
        }
        // Nunca una función que el catálogo bloquea (o no conoce), aunque
        // un ajuste la lea.
        if state
            .catalogo
            .funcion(&a.leer.funcion)
            .is_none_or(|f| f.bloqueada.is_some())
        {
            tracing::warn!(
                "foto de partida: {} está bloqueada o no está en el catálogo: no se lee",
                a.leer.funcion
            );
            continue;
        }
        hechas.push(clave);
        let params: Vec<(&str, &str)> = a
            .leer
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let leida = match a.busqueda() {
            // El cortafuegos, de su página (como en Ajustes).
            None if a.leer.funcion == hgu::cortafuegos::PAGINA_LISTA => {
                hgu::cortafuegos::leer(&state.hgu)
                    .await
                    .map(|f| hgu::cortafuegos::a_xml(&f))
            }
            Some((posicion, ident)) => {
                state
                    .hgu
                    .call_por_id(&a.leer.funcion, &params, posicion, &ident)
                    .await
            }
            None => state.hgu.call(&a.leer.funcion, &params).await,
        };
        match leida {
            Ok(n) => juntar(
                &mut nodos,
                &a.leer.funcion,
                n,
                id_param(state, &a.leer.funcion),
            ),
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

    /// La DMZ lee con un filtro la función de las reglas de puertos: queda
    /// fuera de la foto. Las reglas, el pool de casa y la WiFi, no.
    #[test]
    fn la_dmz_queda_fuera_de_la_foto() {
        let cat = ajustes::CatalogoAjustes::load().unwrap();
        let todos = cat.todos();
        let fuera: Vec<&str> = todos
            .iter()
            .filter(|a| fuera_de_la_foto(todos, a))
            .map(|a| a.id.as_str())
            .collect();
        assert_eq!(fuera, ["puertos.dmz"]);
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
            None,
        );
        juntar(
            &mut foto,
            "cbX",
            nodo("cbX", vec![i1.clone(), i0.clone()]),
            None,
        );
        assert_eq!(foto["cbX"].children, vec![i0, hoja("Result", "0"), i1]);
        // Sueltos: agrupados con el nombre de la función, sin `Result`.
        juntar(
            &mut foto,
            "cbX",
            nodo("cbX", vec![hoja("s_id", "2"), hoja("Result", "0")]),
            None,
        );
        assert_eq!(
            foto["cbX"].children.last(),
            Some(&nodo("cbX", vec![hoja("s_id", "2")]))
        );
    }

    /// Con el parámetro de instancia, la misma instancia leída de dos
    /// maneras (sin parámetros, `s_0i = 1`; por id, `s_0i = -1`) no se
    /// repite.
    #[test]
    fn juntar_por_id_de_instancia() {
        let sin_params = nodo(
            "s",
            vec![hoja("s_0i", "1"), hoja("s_id", "1"), hoja("a", "x")],
        );
        let por_id = nodo(
            "s",
            vec![hoja("s_0i", "-1"), hoja("s_id", "1"), hoja("a", "x")],
        );
        let otra = nodo(
            "s",
            vec![hoja("s_0i", "-1"), hoja("s_id", "0"), hoja("a", "y")],
        );
        let mut foto = BTreeMap::new();
        juntar(
            &mut foto,
            "cbX",
            nodo("cbX", vec![sin_params.clone()]),
            Some("s_0i"),
        );
        juntar(
            &mut foto,
            "cbX",
            nodo("cbX", vec![por_id.clone()]),
            Some("s_0i"),
        );
        juntar(
            &mut foto,
            "cbX",
            nodo("cbX", vec![otra.clone()]),
            Some("s_0i"),
        );
        assert_eq!(foto["cbX"].children, vec![sin_params, otra.clone()]);
        // Sin parámetro de instancia, solo cuenta la idéntica.
        let mut foto = BTreeMap::new();
        juntar(&mut foto, "cbX", nodo("cbX", vec![otra.clone()]), None);
        juntar(&mut foto, "cbX", nodo("cbX", vec![por_id.clone()]), None);
        assert_eq!(foto["cbX"].children, vec![otra, por_id]);
    }
}
