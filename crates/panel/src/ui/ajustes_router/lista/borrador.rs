//! Lo editado en el formulario de una fila: sobrevive a que la página vuelva a leer.

use super::super::detalle::{Borrador, Borradores, Registro};
use leptos::prelude::*;

/// Namespace de lo editado en el formulario de una fila (en [`Borradores`]).
const BORRADOR: &str = "fila";

/// Sigue lo editado en el formulario de la fila `fila`: `campos` son el
/// nombre de cada campo, su valor en edición y el leído. Lo que se editó
/// antes de que la página volviera a leer vuelve a su sitio; desde ahora,
/// se guarda mientras se edita, y `registro` dice si hay cambios sin
/// guardar. Devuelve con qué olvidarlo (al preparar el cambio).
pub(crate) fn seguir_lo_editado(
    fila: String,
    campos: Vec<(String, RwSignal<String>, String)>,
    registro: Registro,
) -> Callback<()> {
    let borradores = expect_context::<Borradores>();
    for ((ns, campo), v) in borradores.de(&fila) {
        if let Some((_, s, _)) = campos
            .iter()
            .find(|(k, _, _)| ns == BORRADOR && *k == campo)
        {
            s.set(v);
        }
    }
    let campos = StoredValue::new(campos);
    let sucio = Signal::derive(move || {
        campos.with_value(|cs| cs.iter().any(|(_, s, leido)| s.get() != *leido))
    });
    registro.set(Some((fila.clone(), sucio)));
    let id = fila.clone();
    Effect::new(move |_| {
        let cambios: Borrador = campos.with_value(|cs| {
            cs.iter()
                .filter_map(|(k, s, leido)| {
                    let v = s.get();
                    (v != *leido).then(|| ((BORRADOR.to_owned(), k.clone()), v))
                })
                .collect()
        });
        borradores.guardar(&id, cambios);
    });
    Callback::new(move |()| borradores.olvidar(&fila))
}
