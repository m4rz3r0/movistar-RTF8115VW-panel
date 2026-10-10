//! Página de inicio de sesión.

use crate::auth::{Login, safe_next};
use crate::t;
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
        Some(Err(_)) => Some(t!("comun-error-conexion")),
        _ => None,
    };
    view! {
        <main class="acceso">
            <section class="acceso-tarjeta login" aria-labelledby="login-titulo">
                <p class="acceso-marca">"RTF8115VW"</p>
                <h1 id="login-titulo">{t!("login-titulo")}</h1>
                <ActionForm action>
                    <p class="campo">
                        <label for="usuario">{t!("login-usuario")}</label>
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
                    <p class="campo">
                        <label for="contrasena">{t!("login-contrasena")}</label>
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
                    <button class="boton" type="submit" disabled=move || action.pending().get()>
                        {t!("login-entrar")}
                    </button>
                </ActionForm>
            </section>
        </main>
    }
}
