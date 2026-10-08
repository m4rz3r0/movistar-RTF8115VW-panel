//! Página de inicio de sesión.

use crate::auth::{Login, safe_next};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};

/// Formulario de login. Al entrar, navega a `next` (solo rutas locales).
#[component]
pub fn PaginaLogin() -> impl IntoView {
    let action = ServerAction::<Login>::new();
    let query = use_query_map();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if let Some(Ok(())) = action.value().get() {
            let next = query.with(|q| safe_next(q.get_str("next")).to_owned());
            navigate(&next, Default::default());
        }
    });
    let error = move || match action.value().get() {
        Some(Err(ServerFnError::ServerError(msg))) => Some(msg),
        Some(Err(_)) => Some("No se puede contactar con el panel".to_owned()),
        _ => None,
    };
    view! {
        <section class="login" aria-labelledby="login-titulo">
            <h1 id="login-titulo">"Acceso al panel"</h1>
            <ActionForm action>
                <p>
                    <label for="usuario">"Usuario"</label>
                    <input
                        id="usuario"
                        name="usuario"
                        type="text"
                        autocomplete="username"
                        autocapitalize="none"
                        spellcheck="false"
                        required
                        autofocus
                    />
                </p>
                <p>
                    <label for="contrasena">"Contraseña"</label>
                    <input
                        id="contrasena"
                        name="contrasena"
                        type="password"
                        autocomplete="current-password"
                        required
                    />
                </p>
                <p class="error" role="alert" aria-live="assertive">
                    {error}
                </p>
                <button type="submit" disabled=move || action.pending().get()>
                    "Entrar"
                </button>
            </ActionForm>
        </section>
    }
}
