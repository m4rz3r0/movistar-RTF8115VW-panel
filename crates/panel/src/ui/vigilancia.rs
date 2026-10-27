//! «Todo bien, terminar»: dar por buena la vigilancia de un cambio antes de
//! que acabe su plazo. El botón sale donde se ve un cambio vigilándose (el
//! diálogo de Ajustes, el de la Biblioteca, el aviso de la página y el
//! Historial) cuando quien mira lo puede terminar; junto a él, el de quitar
//! de la cola ([`crate::ui::cola`]) cuando el cambio espera su turno.

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
                    // Si el aviso ya se ha vuelto a pintar, quien lo
                    // pidió no está: el sondeo trae el estado nuevo.
                    al_terminar.try_run(c);
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

/// Los botones de un cambio en curso que quien mira puede usar: «Todo bien,
/// terminar» mientras se vigila y «Quitar de la cola» mientras espera. Al
/// usarlos, `al_cambiar` recibe el estado nuevo; si no se puede, el motivo
/// va a `error`.
#[component]
pub fn BotonesEnCurso(
    id: i64,
    terminable: bool,
    quitable: bool,
    #[prop(into)] al_cambiar: Callback<CambioVista>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    use crate::ui::cola::QuitarDeLaCola;
    view! {
        {terminable.then(|| view! { <TerminarVigilancia id al_terminar=al_cambiar error /> })}
        {quitable.then(|| view! { <QuitarDeLaCola id al_quitar=al_cambiar error /> })}
    }
}

/// [`BotonesEnCurso`] del cambio que se sigue en `cambio` (los diálogos):
/// se pintan de nuevo solo si cambia lo que se puede hacer, no en cada
/// sondeo.
#[component]
pub fn BotonesDelCambio(
    cambio: RwSignal<Option<CambioVista>>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let que =
        Memo::new(move |_| cambio.with(|c| c.as_ref().map(|c| (c.id, c.terminable, c.quitable))));
    move || {
        que.get().map(|(id, terminable, quitable)| {
            view! {
                <BotonesEnCurso
                    id
                    terminable
                    quitable
                    al_cambiar=move |c: CambioVista| cambio.set(Some(c))
                    error
                />
            }
        })
    }
}
