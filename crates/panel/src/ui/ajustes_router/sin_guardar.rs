//! La pregunta de los cambios sin guardar, al dejar un detalle con algo editado.

use crate::t;
use leptos::{html, prelude::*};

/// La pregunta de los cambios sin guardar: seguir editando o descartarlos.
/// Esc sigue editando.
#[component]
pub(crate) fn SinGuardar(
    nombre: String,
    seguir: Callback<()>,
    descartar: Callback<()>,
) -> impl IntoView {
    let dlg = NodeRef::<html::Dialog>::new();
    Effect::new(move |_| {
        if let Some(d) = dlg.get()
            && !d.open()
        {
            let _ = d.show_modal();
        }
    });
    view! {
        <dialog
            class="dialogo dialogo-sin-guardar"
            node_ref=dlg
            aria-labelledby="sin-guardar-titulo"
            aria-modal="true"
            on:cancel=move |ev: leptos::ev::Event| {
                ev.prevent_default();
                seguir.run(());
            }
        >
            <div class="dialogo-cuerpo">
                <h2 id="sin-guardar-titulo">{t!("ajustes-sin-guardar-titulo")}</h2>
                <p>{t!("ajustes-sin-guardar-texto", nombre = nombre)}</p>
                <div class="dialogo-pie">
                    <button type="button" class="b-pel" on:click=move |_| descartar.run(())>
                        {t!("ajustes-sin-guardar-descartar")}
                    </button>
                    <button type="button" class="b-pri" autofocus on:click=move |_| seguir.run(())>
                        {t!("ajustes-sin-guardar-seguir")}
                    </button>
                </div>
            </div>
        </dialog>
    }
}
