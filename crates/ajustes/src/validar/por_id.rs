//! Regla de la lectura por posición frente a la escritura por id (`buscar`).

use crate::modelo::Ajuste;
use catalogo::Funcion;

/// Un ajuste que lee una lista por posición (`X_0i`) y escribe por id
/// (`X_id` en sus `fijos`) leería otra instancia si no coinciden, como el
/// pool DHCP del router real: tiene que buscarla (`buscar = "X_0i"`). Y una
/// `buscar` tiene que ser el parámetro de posición de la lista de su lectura
/// `l`, con el id en los `fijos` y fuera de `params`.
pub(super) fn comprobar(a: &Ajuste, l: &Funcion, fallos: &mut Vec<String>) {
    let id = &a.id;
    let con_id = |posicion: &str| {
        posicion
            .strip_suffix("_0i")
            .map(|base| format!("{base}_id"))
            .filter(|k| a.fijos.contains_key(k))
    };
    for (k, _) in &a.leer.params {
        if let Some(fijo) = con_id(k) {
            fallos.push(format!(
                "{id}: lee por la posición {k} y escribe por el id {fijo}: \
                 la lectura tiene que buscar la instancia (buscar = \"{k}\")"
            ));
        }
    }
    let Some(b) = a.leer.buscar.as_deref() else {
        return;
    };
    if a.lista {
        fallos.push(format!("{id}: una lista no busca una instancia"));
    }
    if l.lista.as_ref().is_none_or(|lista| lista.id_param != b) {
        fallos.push(format!(
            "{id}: {b} no es el parámetro de posición de la lista de {}",
            l.name
        ));
    }
    if a.leer.params.iter().any(|(k, _)| k == b) {
        fallos.push(format!("{id}: {b} va en buscar, no en los parámetros"));
    }
    if con_id(b).is_none() {
        fallos.push(format!("{id}: buscar = {b} sin su id (…_id) en los fijos"));
    }
}
