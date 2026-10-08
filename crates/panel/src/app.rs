//! Aplicación Leptos: documento HTML y rutas de la interfaz.

use crate::ui::login::PaginaLogin;
use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
};

/// Documento HTML que envuelve la aplicación en el servidor.
///
/// En el servidor fija la CSP de la página con el nonce que Leptos pone en sus
/// scripts en línea (carga de la hidratación y datos serializados).
pub fn shell(options: LeptosOptions) -> impl IntoView {
    #[cfg(feature = "ssr")]
    csp_con_nonce();
    view! {
        <!DOCTYPE html>
        <html lang="es">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[cfg(feature = "ssr")]
fn csp_con_nonce() {
    use axum::http::{HeaderValue, header};
    use leptos::nonce::use_nonce;
    use leptos_axum::ResponseOptions;

    let (Some(nonce), Some(resp)) = (use_nonce(), use_context::<ResponseOptions>()) else {
        return;
    };
    if let Ok(value) = HeaderValue::from_str(&crate::security::csp(Some(&nonce))) {
        resp.insert_header(header::CONTENT_SECURITY_POLICY, value);
    }
}

/// Raíz de la interfaz.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/panel.css" />
        <Title text="Router · RTF8115VW" />
        <Router>
            <main>
                <Routes fallback=|| "Página no encontrada".into_view()>
                    <Route path=StaticSegment("") view=Inicio />
                    <Route path=StaticSegment("login") view=PaginaLogin />
                </Routes>
            </main>
        </Router>
    }
}

/// Página de inicio provisional.
#[component]
fn Inicio() -> impl IntoView {
    view! { <h1>"Panel en construcción"</h1> }
}
