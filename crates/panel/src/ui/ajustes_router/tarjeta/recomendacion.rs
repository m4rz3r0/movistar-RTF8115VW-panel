//! La recomendación principal de la tarjeta y su botón.

use super::super::ID_SOLO_LECTURA;
use super::super::aviso::Seguimiento;
use super::super::campo::partir_motivo;
use crate::ajustes_router::{ElementoVista, PeticionAjuste, preparar_ajuste};
use crate::t;
use crate::ui::explorador::texto_error;
use leptos::{prelude::*, task::spawn_local};
use std::collections::BTreeMap;

/// La recomendación pendiente más importante de `e`: para quién es y el
/// botón que prepara el cambio a lo recomendado (se revisa en el diálogo).
/// Fondo ámbar si es un riesgo de seguridad; neutro, si no. En solo
/// lectura, el botón se ve desactivado y dice por qué.
pub(super) fn recomendacion(
    e: &ElementoVista,
    solo_lectura: bool,
) -> Option<impl IntoView + use<>> {
    let r = e.recomendacion.clone()?;
    let s = expect_context::<Seguimiento>();
    let (para_quien, _) = partir_motivo(&r.texto, &t!("ajustes-recomendado-marca"));
    let id = e.id.clone();
    let preparando = RwSignal::new(false);
    let (ajuste, campo, valor) = (r.ajuste.clone(), r.campo.clone(), r.valor.clone());
    let pulsar = move |_| {
        if solo_lectura || preparando.get_untracked() {
            return;
        }
        preparando.set(true);
        let p = PeticionAjuste {
            ajuste: ajuste.clone(),
            instancia: None,
            borrar: false,
            valores: BTreeMap::from([(campo.clone(), valor.clone())]),
            esperados: BTreeMap::new(),
        };
        let id = id.clone();
        spawn_local(async move {
            match preparar_ajuste(p).await {
                Ok(g) => s.revisar(g, vec![id]),
                Err(e) => s.falla(texto_error(&e)),
            }
            preparando.try_set(false);
        });
    };
    Some(view! {
        <div class="tarjeta-rec" class:riesgo=r.riesgo>
            <p>{para_quien}</p>
            <button
                type="button"
                class="b-rec"
                aria-disabled=move || (solo_lectura || preparando.get()).then_some("true")
                aria-describedby=solo_lectura.then_some(ID_SOLO_LECTURA)
                on:click=pulsar
            >
                {r.boton}
            </button>
        </div>
    })
}
