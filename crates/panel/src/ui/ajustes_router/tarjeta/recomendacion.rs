//! La recomendación principal de la tarjeta y su botón.

use super::super::ID_SOLO_LECTURA;
use super::super::aviso::Seguimiento;
use super::super::campo::partir_motivo;
use crate::ajustes_router::{ElementoVista, PeticionAjuste, preparar_ajuste};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::i18n::traducir;
use leptos::{prelude::*, task::spawn_local};
use std::collections::BTreeMap;

/// De qué lotes de Mejoras forma parte el campo de la recomendación
/// («Forma parte de «Seguridad recomendada»»), o nada si de ninguno.
fn texto_lotes(lotes: &[String]) -> Option<String> {
    match lotes {
        [] => None,
        [lote] => Some(t!(
            "ajustes-recomendacion-lote",
            lote = traducir(lote, None)
        )),
        [primero, segundo, ..] => Some(t!(
            "ajustes-recomendacion-lotes",
            primero = traducir(primero, None),
            segundo = traducir(segundo, None),
        )),
    }
}

/// La recomendación pendiente más importante de `e`: qué pasa ahora, para
/// quién es lo recomendado, de qué lotes de Mejoras forma parte (con un
/// enlace a Mejoras) y el
/// botón que prepara el cambio a lo recomendado (se revisa en el diálogo).
/// Fondo ámbar si es un riesgo de seguridad; neutro, si no. En solo
/// lectura, el botón se ve desactivado y dice por qué. Una recomendación
/// informativa no tiene botón (se aplica desde el detalle del elemento).
/// Debajo, plegado, de dónde sale.
pub(super) fn recomendacion(
    e: &ElementoVista,
    solo_lectura: bool,
) -> Option<impl IntoView + use<>> {
    let r = e.recomendacion.clone()?;
    let s = expect_context::<Seguimiento>();
    let (para_quien, _) = partir_motivo(&r.texto, &t!("ajustes-recomendado-marca"));
    let lotes = texto_lotes(&r.lotes).map(|texto| {
        view! {
            <span class="tarjeta-rec-lotes">
                <span class="tarjeta-rec-lotes-texto">{texto}</span>
                " "
                <a class="b-txt" href="/mejoras">
                    {t!("ajustes-recomendacion-ver-mejoras")}
                </a>
            </span>
        }
    });
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
    let boton = (!r.informativa).then(|| {
        view! {
            <button
                type="button"
                class="b-rec"
                aria-disabled=move || (solo_lectura || preparando.get()).then_some("true")
                aria-describedby=solo_lectura.then_some(ID_SOLO_LECTURA)
                on:click=pulsar
            >
                {r.boton}
            </button>
        }
    });
    Some(view! {
        <div class="tarjeta-rec" class:riesgo=r.riesgo>
            <p>
                {r.problema.clone()}
                " "
                <span class="tarjeta-rec-quien">{para_quien}</span>
                {lotes}
            </p>
            {boton}
            {super::super::de_donde(r.fuentes, r.respaldo)}
        </div>
    })
}
