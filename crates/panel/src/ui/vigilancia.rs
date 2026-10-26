//! «Todo bien, terminar»: dar por buena la vigilancia de un cambio antes de
//! que acabe su plazo. El botón sale donde se ve un cambio vigilándose (el
//! diálogo de Ajustes, el de la Biblioteca, el aviso de la página y el
//! Historial) cuando quien mira lo puede terminar.

use crate::t;
use crate::ui::explorador::{CambioVista, texto_error};
use leptos::{prelude::*, server_fn::codec::Json, task::spawn_local};

/// Termina ya la vigilancia del cambio `id` (solo quien lo hizo, mientras el
/// gestor lo vigila y la última comprobación salió bien): queda aplicado y
/// el Historial lo anota. Devuelve su estado nuevo. 409 si ya no se vigila
/// o el router no respondió en la última comprobación; 403 si lo hizo otro
/// usuario.
#[server(endpoint = "terminar_vigilancia", input = Json)]
pub async fn terminar_vigilancia(id: i64) -> Result<CambioVista, ServerFnError> {
    use crate::changes::ErrorTerminar;
    use crate::ui::explorador::servidor::{app_state, con_estado, estado_cambio, gestor};
    use axum::http::StatusCode;
    let user = crate::auth::require_user().await?;
    let state = app_state()?;
    gestor(&state)?
        .terminar(id, &user.username)
        .await
        .map_err(|e| {
            let status = match e {
                ErrorTerminar::OtroUsuario => StatusCode::FORBIDDEN,
                ErrorTerminar::NoSeVigila | ErrorTerminar::Fallando => StatusCode::CONFLICT,
            };
            con_estado(status, crate::ui::i18n::traducir(e.clave(), None))
        })?;
    estado_cambio(&state, id, &user.username).await
}

/// Botón secundario «Todo bien, terminar» del cambio `id`. Al terminar,
/// llama a `al_terminar` con el estado nuevo; si no se puede, pone el
/// motivo en `error`.
#[component]
pub fn TerminarVigilancia(
    id: i64,
    #[prop(into)] al_terminar: Callback<CambioVista>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let enviando = RwSignal::new(false);
    let terminar = move |_| {
        if enviando.get_untracked() {
            return;
        }
        enviando.set(true);
        spawn_local(async move {
            match terminar_vigilancia(id).await {
                Ok(c) => {
                    error.try_set(None);
                    al_terminar.run(c);
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
            on:click=terminar
        >
            {move || {
                if enviando.get() { t!("vigilancia-terminando") } else { t!("vigilancia-terminar") }
            }}
        </button>
    }
}
