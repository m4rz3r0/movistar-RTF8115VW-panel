//! Aplicación Leptos: documento HTML y rutas de la interfaz.

use crate::t;
use crate::ui::{
    charts::{provide_zona, zona_inicial},
    estado::PaginaEstado,
    explorador::{Explorador, InicioExplorador, PaginaArea, PaginaFuncion},
    historial::PaginaHistorial,
    i18n::{idioma_inicial, provide_idioma},
    instalar::PaginaInstalar,
    layout::Layout,
    login::PaginaLogin,
    panel_ajustes::PaginaPanel,
    recomendaciones::{InicioRecomendaciones, PaginaPreferencias, SeccionRecomendaciones},
    router::{InicioRouter, PaginaConexion, SeccionRouter},
};
use leptos::prelude::*;
use leptos_meta::{Link, MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    ParamSegment, StaticSegment,
    components::{ParentRoute, Route, Router, Routes},
};

/// Documento HTML que envuelve la aplicación en el servidor.
///
/// En el servidor fija la CSP de la página con el nonce que Leptos pone en sus
/// scripts en línea (carga de la hidratación y datos serializados), el
/// idioma de `<html lang>` y la zona horaria de `<html data-zona>`, que el
/// navegador lee al hidratar.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    #[cfg(feature = "ssr")]
    csp_con_nonce();
    let idioma = idioma_inicial();
    // La misma zona para el atributo y para toda la página.
    let zona = zona_inicial();
    provide_zona(zona);
    view! {
        <!DOCTYPE html>
        <html lang=idioma.codigo() data-zona=zona.codigo()>
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
    provide_idioma(idioma_inicial());
    provide_zona(zona_inicial());
    // `<Redirect>` en el servidor: 302 con `Location` (`/router`,
    // `/recomendaciones`).
    #[cfg(feature = "ssr")]
    leptos_router::components::provide_server_redirect(leptos_axum::redirect);
    view! {
        <Stylesheet id="leptos" href="/pkg/panel.css" />
        // Las fuentes de títulos y texto, antes de que las pida la hoja.
        <Link rel="preload" href="/fonts/inter-var.woff2" as_="font" type_="font/woff2" crossorigin="anonymous" />
        <Link rel="preload" href="/fonts/chakra-petch-600.woff2" as_="font" type_="font/woff2" crossorigin="anonymous" />
        <Title text="Router · RTF8115VW" />
        <Router>
            <Routes fallback=NoEncontrada>
                <Route path=StaticSegment("login") view=PaginaLogin />
                <Route path=StaticSegment("instalar") view=PaginaInstalar />
                <ParentRoute path=StaticSegment("") view=Layout>
                    <Route path=StaticSegment("") view=PaginaEstado />
                    <Route path=StaticSegment("historial") view=PaginaHistorial />
                    <Route path=StaticSegment("panel") view=PaginaPanel />
                    <ParentRoute path=StaticSegment("recomendaciones") view=SeccionRecomendaciones>
                        <Route path=StaticSegment("") view=InicioRecomendaciones />
                        <Route path=StaticSegment("preferencias") view=PaginaPreferencias />
                    </ParentRoute>
                    <ParentRoute path=StaticSegment("router") view=SeccionRouter>
                        <Route path=StaticSegment("") view=InicioRouter />
                        <Route path=StaticSegment("conexion") view=PaginaConexion />
                    </ParentRoute>
                    <ParentRoute path=StaticSegment("explorador") view=Explorador>
                        <Route path=StaticSegment("") view=InicioExplorador />
                        <Route path=ParamSegment("area") view=PaginaArea />
                        <Route
                            path=(ParamSegment("area"), ParamSegment("funcion"))
                            view=PaginaFuncion
                        />
                    </ParentRoute>
                </ParentRoute>
            </Routes>
        </Router>
    }
}

/// Página para rutas que no existen.
#[component]
fn NoEncontrada() -> impl IntoView {
    view! {
        <main class="acceso">
            <section class="acceso-tarjeta">
                <h1>{t!("comun-no-encontrada")}</h1>
                <p>
                    <a href="/">{t!("comun-volver-al-estado")}</a>
                </p>
            </section>
        </main>
    }
}
