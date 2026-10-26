//! La cola de cambios en la interfaz: quitar un cambio que espera su turno
//! (solo quien lo encoló). Donde se ve un cambio en cola (los diálogos, el
//! aviso de la página y el Historial) sale el botón «Quitar de la cola».

use crate::t;
use crate::ui::explorador::{CambioVista, texto_error};
use leptos::{prelude::*, server_fn::codec::Json, task::spawn_local};

/// Quita de la cola el cambio `id`, que encoló quien llama: queda
/// cancelado y el Historial anota quién lo quitó. Devuelve su estado nuevo.
/// 409 si ya no espera (se está aplicando o ya terminó); 403 si lo encoló
/// otro usuario.
#[server(endpoint = "quitar_de_la_cola", input = Json)]
pub async fn quitar_de_la_cola(id: i64) -> Result<CambioVista, ServerFnError> {
    use crate::changes::ErrorQuitar;
    use crate::ui::explorador::servidor::{app_state, con_estado, estado_cambio, gestor};
    use axum::http::StatusCode;
    let user = crate::auth::require_user().await?;
    let state = app_state()?;
    gestor(&state)?
        .quitar_de_la_cola(id, &user.username)
        .await
        .map_err(|e| {
            let status = match e {
                ErrorQuitar::OtroUsuario => StatusCode::FORBIDDEN,
                ErrorQuitar::NoEnCola => StatusCode::CONFLICT,
            };
            con_estado(status, crate::ui::i18n::traducir(e.clave(), None))
        })?;
    estado_cambio(&state, id, &user.username).await
}

/// Botón secundario «Quitar de la cola» del cambio `id`. Al quitarlo,
/// llama a `al_quitar` con el estado nuevo; si no se puede, pone el motivo
/// en `error`.
#[component]
pub fn QuitarDeLaCola(
    id: i64,
    #[prop(into)] al_quitar: Callback<CambioVista>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let enviando = RwSignal::new(false);
    let quitar = move |_| {
        if enviando.get_untracked() {
            return;
        }
        enviando.set(true);
        spawn_local(async move {
            match quitar_de_la_cola(id).await {
                Ok(c) => {
                    error.try_set(None);
                    al_quitar.run(c);
                }
                Err(e) => {
                    error.try_set(Some(texto_error(&e)));
                }
            }
            enviando.try_set(false);
        });
    };
    view! {
        <button
            type="button"
            class="b-sec b-terminar"
            aria-disabled=move || enviando.get().then_some("true")
            on:click=quitar
        >
            {move || if enviando.get() { t!("cola-quitando") } else { t!("cola-quitar") }}
        </button>
    }
}
