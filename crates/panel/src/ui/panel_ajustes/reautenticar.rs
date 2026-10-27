//! El diálogo que pide la contraseña del panel antes de lo delicado
//! ([`DialogoReautenticar`]).

use super::{Reautenticacion, texto_error};
use crate::panel_ajustes::reautenticar;
use crate::t;
use leptos::{ev::SubmitEvent, html, prelude::*, task::spawn_local};

/// Pide la contraseña del panel y, si es buena, repite la operación
/// pendiente.
#[component]
pub(crate) fn DialogoReautenticar() -> impl IntoView {
    let reauth = expect_context::<Reautenticacion>();
    let dlg = NodeRef::<html::Dialog>::new();
    crate::ui::explorador::abrir_modal(dlg);
    let contrasena = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let pidiendo = RwSignal::new(false);
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        if pidiendo.get_untracked() {
            return;
        }
        error.set(None);
        pidiendo.set(true);
        let clave = contrasena.get_untracked();
        spawn_local(async move {
            let r = reautenticar(clave).await;
            contrasena.set(String::new());
            pidiendo.set(false);
            match r {
                Ok(()) => {
                    let pendiente = reauth.pendiente.get_value();
                    reauth.pendiente.set_value(None);
                    if let Some(d) = dlg.get_untracked() {
                        d.close();
                    }
                    if let Some(p) = pendiente {
                        p.run(());
                    }
                }
                Err(e) => error.set(Some(texto_error(&e))),
            }
        });
    };
    let cerrar = move |_| {
        if let Some(d) = dlg.get_untracked() {
            d.close();
        }
    };
    view! {
        <dialog
            class="dialogo dialogo-estrecho"
            node_ref=dlg
            aria-labelledby="reautenticar-titulo"
            aria-modal="true"
            on:close=move |_| {
                reauth.abierto.set(false);
                reauth.pendiente.set_value(None);
            }
        >
            <form class="dialogo-cuerpo" on:submit=enviar>
                <header class="dialogo-cabecera">
                    <h2 id="reautenticar-titulo">{t!("panel-reautenticar")}</h2>
                </header>
                <p>{t!("panel-reautenticar-texto")}</p>
                <p class="campo">
                    <label for="reautenticar-contrasena">
                        {t!("panel-reautenticar-contrasena")}
                    </label>
                    <input
                        id="reautenticar-contrasena"
                        type="password"
                        autocomplete="current-password"
                        required
                        autofocus
                        prop:value=contrasena
                        on:input=move |ev| contrasena.set(event_target_value(&ev))
                    />
                </p>
                <p class="error-formulario" role="alert">{move || error.get()}</p>
                <div class="dialogo-pie">
                    <button class="b-sec" type="button" on:click=cerrar>
                        {t!("panel-cancelar")}
                    </button>
                    <button class="b-pri" type="submit" disabled=move || pidiendo.get()>
                        {t!("panel-reautenticar-seguir")}
                    </button>
                </div>
            </form>
        </dialog>
    }
}
