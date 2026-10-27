//! Aplicación Leptos: documento HTML y rutas de la interfaz.

use crate::t;
use crate::ui::{
    ajustes_router::{InicioAjustes, PaginaFoto, PaginaTema, SeccionAjustes, SinVista},
    charts::{provide_zona, zona_inicial},
    estado::PaginaEstado,
    explorador::{Explorador, InicioExplorador, PaginaArea, PaginaFuncion},
    historial::PaginaHistorial,
    i18n::{idioma_inicial, provide_idioma},
    instalar::PaginaInstalar,
    layout::Layout,
    login::PaginaLogin,
    mejoras::PaginaMejoras,
    panel_ajustes::PaginaPanel,
    recomendaciones::RecomendacionesAlEstado,
    router::{InicioRouter, PaginaConexion, SeccionRouter},
    tema::{provide_tema, tema_inicial},
    vista::{provide_vista, vista_inicial},
};
use leptos::prelude::*;
use leptos_meta::{HashedStylesheet, Link, MetaTags, Title, provide_meta_context};
use leptos_router::{
    ParamSegment, StaticSegment,
    components::{Outlet, ParentRoute, Route, Router, Routes},
};

/// Documento HTML que envuelve la aplicación en el servidor.
///
/// En el servidor fija la CSP de la página con el nonce que Leptos pone en sus
/// scripts en línea (carga de la hidratación y datos serializados), el
/// idioma de `<html lang>`, la zona horaria de `<html data-zona>`, el modo
/// de visualización del usuario de `<html data-vista>` y su tema de
/// `<html data-tema>`, que el navegador lee al hidratar. Con el tema en el
/// HTML, la página se pinta ya con sus colores, sin destello.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    #[cfg(feature = "ssr")]
    csp_con_nonce();
    let idioma = idioma_inicial();
    // La misma zona para el atributo y para toda la página.
    let zona = zona_inicial();
    provide_zona(zona);
    let vista = vista_inicial();
    let tema = tema_inicial();
    view! {
        <!DOCTYPE html>
        <html
            lang=idioma.codigo()
            data-zona=zona.codigo()
            data-vista=vista.codigo()
            data-tema=tema.codigo()
        >
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="color-scheme" content="dark light" />
                <AutoReload options=options.clone() />
                <HashedStylesheet options=options.clone() id="leptos" />
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
    provide_vista(vista_inicial());
    provide_tema(tema_inicial());
    // `<Redirect>` en el servidor: 302 con `Location` (`/router`,
    // `/ajustes`, `/recomendaciones/*`).
    #[cfg(feature = "ssr")]
    leptos_router::components::provide_server_redirect(leptos_axum::redirect);
    view! {
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
                    <ParentRoute path=StaticSegment("ajustes") view=SeccionAjustes>
                        <Route path=StaticSegment("") view=InicioAjustes />
                        // Antes que los temas: no es uno de ellos.
                        <Route path=StaticSegment("foto-de-partida") view=PaginaFoto />
                        // El tema y, en la rejilla, el elemento abierto
                        // (`/ajustes/wifi/red-5`): la misma página, que no se
                        // vuelve a montar al abrir o cerrar un detalle.
                        <ParentRoute path=ParamSegment("tema") view=PaginaTema>
                            <Route path=StaticSegment("") view=SinVista />
                            <Route path=ParamSegment("elemento") view=SinVista />
                        </ParentRoute>
                    </ParentRoute>
                    <Route path=StaticSegment("mejoras") view=PaginaMejoras />
                    <Route path=StaticSegment("historial") view=PaginaHistorial />
                    <Route path=StaticSegment("panel") view=PaginaPanel />
                    // Volverá con sus tarjetas en una versión futura.
                    <ParentRoute path=StaticSegment("recomendaciones") view=Outlet>
                        <Route path=StaticSegment("") view=RecomendacionesAlEstado />
                        <Route path=StaticSegment("preferencias") view=RecomendacionesAlEstado />
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
