//! Preparar un cambio de la Biblioteca pidiendo la contraseña del panel si el servidor la exige (crear o borrar usuarios del router).

use super::{Peticion, PreparadoVista, preparar_cambio};
use crate::ui::panel_ajustes::Reautenticacion;
use leptos::{prelude::*, task::spawn_local};

/// Prepara `peticion` y pasa lo preparado a `listo`. Si falta
/// reautenticarse, abre el diálogo de `reauth` y repite al acabar; otro
/// error queda en `error`. `fin` corre al acabar cada intento.
pub(super) fn preparar(
    peticion: Peticion,
    reauth: Reautenticacion,
    error: RwSignal<Option<String>>,
    listo: Callback<PreparadoVista>,
    fin: Callback<()>,
) {
    spawn_local(async move {
        let otra_vez = peticion.clone();
        match preparar_cambio(peticion).await {
            Ok(p) => listo.run(p),
            Err(e) => {
                let repetir = Callback::new(move |()| {
                    preparar(otra_vez.clone(), reauth, error, listo, fin);
                });
                reauth.tratar(&e, error, repetir);
            }
        }
        fin.run(());
    });
}
