//! Cortafuegos en el detalle de su elemento: el editor de filtros y reglas ([`editor::Editor`]) y «No responder al ping desde internet» ([`Cortafuegos`]).
//!
//! El editor enseña los filtros con sus reglas en orden y prepara cada
//! alta, cambio o baja en el diálogo de revisión; mientras lee (o si no
//! puede), queda la tabla de reglas de siempre. La acción propia del
//! elemento prepara la regla del panel «sin ping» y la abre en el mismo
//! diálogo; se deshace desde el aviso del cambio o el Historial. Si no está
//! disponible (el cortafuegos sin leer, la regla ya puesta o el panel en
//! solo lectura), el botón sale desactivado con el motivo. Si el servidor
//! tiene «sin ping» desactivada en este router
//! ([`crate::validacion_real`]), no sale: solo el editor y una nota de una
//! línea con el motivo. Va dentro del formulario del detalle: botones
//! `type="button"`.

use super::ID_SOLO_LECTURA;
use super::aviso::Seguimiento;
use super::rejilla::Pagina;
use crate::ajustes_router::{AccionPropia, AjusteVista, preparar_sin_ping};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::i18n::traducir;
use crate::validacion_real::{Desactivadas, Funcion3};
use leptos::{prelude::*, task::spawn_local};

mod editor;

/// Componente del ajuste de las reglas.
pub(crate) const CORTAFUEGOS: &str = "cortafuegos";
/// Id del motivo del botón (lo describe).
const ID_MOTIVO: &str = "cortafuegos-sin-ping-motivo";

/// El motivo por el que «No responder al ping» no se puede pulsar (el de
/// la acción o, en solo lectura, `None` con el botón desactivado igual).
fn bloqueo(accion: Option<&AccionPropia>, solo_lectura: bool) -> (bool, Option<String>) {
    let motivo = accion.and_then(|a| a.motivo.clone());
    (solo_lectura || accion.is_none() || motivo.is_some(), motivo)
}

/// Las reglas de `a` y «No responder al ping desde internet» (`accion`, la
/// acción propia de su elemento `elemento`), si el servidor no la tiene
/// entre las `desactivadas`.
#[component]
pub(crate) fn Cortafuegos(
    a: AjusteVista,
    accion: Option<AccionPropia>,
    elemento: String,
    desactivadas: Desactivadas,
) -> impl IntoView {
    let reglas = view! {
        <div class="cortafuegos-reglas">
            <editor::Editor a=a elemento=elemento.clone() />
        </div>
    };
    if let Some(m) = desactivadas.motivo(Funcion3::SinPing) {
        return view! {
            {reglas}
            <p class="nota">{traducir(m, None)}</p>
        }
        .into_any();
    }
    let pagina = expect_context::<Pagina>();
    let s = expect_context::<Seguimiento>();
    let ocupado = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let (bloqueado, motivo) = bloqueo(accion.as_ref(), pagina.solo_lectura);
    let describe = if motivo.is_some() {
        Some(ID_MOTIVO)
    } else if pagina.solo_lectura {
        Some(ID_SOLO_LECTURA)
    } else {
        None
    };
    let elemento = StoredValue::new(elemento);
    let pulsar = move |_| {
        if bloqueado || ocupado.get_untracked() {
            return;
        }
        error.set(None);
        ocupado.set(true);
        spawn_local(async move {
            match preparar_sin_ping().await {
                Ok(g) => s.revisar(g, vec![elemento.get_value()]),
                Err(e) => {
                    error.try_set(Some(texto_error(&e)));
                }
            }
            ocupado.try_set(false);
        });
    };
    view! {
        {reglas}
        <section class="formulario cortafuegos-sin-ping" aria-labelledby="cortafuegos-sin-ping-titulo">
            <h4 id="cortafuegos-sin-ping-titulo">{t!("ajuste-seguridad-cortafuegos-sin-ping")}</h4>
            <p class="ayuda">{t!("ajuste-seguridad-cortafuegos-sin-ping-ayuda")}</p>
            {motivo.map(|m| view! { <p class="nota-solo-lectura" id=ID_MOTIVO>{m}</p> })}
            <p class="error-formulario" role="alert">{move || error.get()}</p>
            <div class="botones">
                <button
                    type="button"
                    class="b-sec"
                    aria-disabled=move || (bloqueado || ocupado.get()).then_some("true")
                    aria-describedby=describe
                    on:click=pulsar
                >
                    {t!("ajuste-seguridad-cortafuegos-sin-ping")}
                </button>
            </div>
        </section>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::bloqueo;
    use crate::ajustes_router::AccionPropia;

    /// Se pulsa solo con la acción disponible y sin solo lectura.
    #[test]
    fn el_boton_solo_sin_motivo() {
        let libre = AccionPropia {
            id: "sin_ping".into(),
            motivo: None,
        };
        assert_eq!(bloqueo(Some(&libre), false), (false, None));
        assert_eq!(bloqueo(Some(&libre), true), (true, None));
        assert_eq!(bloqueo(None, false), (true, None));
        let puesta = AccionPropia {
            motivo: Some("ya puesta".into()),
            ..libre
        };
        assert_eq!(
            bloqueo(Some(&puesta), false),
            (true, Some("ya puesta".into()))
        );
    }
}
