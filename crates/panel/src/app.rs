//! Aplicación Leptos: documento HTML y rutas de la interfaz.

use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
};

/// Documento HTML que envuelve la aplicación en el servidor.
pub fn shell(options: LeptosOptions) -> impl IntoView {
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
