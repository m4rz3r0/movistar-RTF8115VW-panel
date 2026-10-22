//! Marco de las páginas con sesión: barra lateral (Estado, Ajustes, Mejoras
//! con su contador, Historial, Biblioteca y Configuración del router; debajo,
//! el selector de idioma y la zona de sesión: el nombre de usuario, que abre
//! «Ajustes del panel», y «Salir») que en pantallas estrechas pasa a ser una
//! barra superior con el menú plegable.

use crate::ajustes_router::{CADUCIDAD_LECTURAS, contar_mejoras};
use crate::auth::{Logout, quien_soy};
use crate::t;
use crate::ui::i18n::SelectorIdioma;
use leptos::prelude::*;
use leptos_router::{
    components::{A, Outlet},
    hooks::{use_location, use_navigate},
};

/// ¿`ruta` está en la sección de `prefijo`? Lo está la propia `prefijo` y
/// cualquier ruta que siga con `/` (`/router` y `/router/conexion`, pero no
/// `/routerx`).
pub fn seccion_activa(prefijo: &str, ruta: &str) -> bool {
    ruta.strip_prefix(prefijo)
        .is_some_and(|resto| resto.is_empty() || resto.starts_with('/'))
}

/// Enlace a una sección del menú: marcado (`aria-current="page"`) en
/// cualquier página de la sección, no solo en la de `href` (el `<A>` de
/// leptos_router compara con el `href`). Los atributos y eventos que se le
/// pongan (`attr:class`, `on:click`) van al `<a>`.
#[component]
pub fn EnlaceSeccion(
    href: &'static str,
    prefijo: &'static str,
    children: Children,
) -> impl IntoView {
    let ruta = use_location().pathname;
    let actual = move || ruta.with(|r| seccion_activa(prefijo, r)).then_some("page");
    view! {
        <a href=href aria-current=actual>
            {children()}
        </a>
    }
}

/// Icono de trazo de 20 px; `d` es el trazado.
fn icono(d: &'static str) -> impl IntoView {
    view! {
        <svg
            class="icono"
            viewBox="0 0 20 20"
            width="20"
            height="20"
            fill="none"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"
        >
            <path d=d />
        </svg>
    }
}

/// Pulso (Estado).
const ICONO_ESTADO: &str = "M2 10h3.5l2-5 3.5 10 2-5H18";
/// Reguladores (Ajustes).
const ICONO_AJUSTES: &str = "M3 6h9M16 6h1M3 14h1M8 14h9M14 4v4M6 12v4";
/// Rejilla de bloques (Biblioteca).
const ICONO_BIBLIOTECA: &str =
    "M3 3h5.5v5.5H3zM11.5 3H17v5.5h-5.5zM3 11.5h5.5V17H3zM11.5 11.5H17V17h-5.5z";
/// Flecha hacia arriba en un círculo (Mejoras).
const ICONO_MEJORAS: &str = "M10 2.75a7.25 7.25 0 1 1 0 14.5 7.25 7.25 0 0 1 0-14.5zM10 13.75v-7.5M6.75 9.5 10 6.25l3.25 3.25";
/// Reloj (Historial).
const ICONO_HISTORIAL: &str =
    "M10 2.75a7.25 7.25 0 1 1 0 14.5 7.25 7.25 0 0 1 0-14.5zM10 6v4l2.75 1.75";
/// Router con antenas (Configuración del router).
const ICONO_ROUTER: &str =
    "M3 11.5h14v5H3zM6 14h.01M9 14h.01M5 11.5 4 6M15 11.5l1-5.5M8 8.25a3 3 0 0 1 4 0";
/// Puerta con flecha (Salir).
const ICONO_SALIR: &str =
    "M8 3H4.5A1.5 1.5 0 0 0 3 4.5v11A1.5 1.5 0 0 0 4.5 17H8M12.5 6.5 16 10l-3.5 3.5M16 10H7.5";
/// Persona (usuario).
const ICONO_USUARIO: &str =
    "M10 3a3.25 3.25 0 1 1 0 6.5A3.25 3.25 0 0 1 10 3zM3.75 17c.6-3.1 3.1-5 6.25-5s5.65 1.9 6.25 5";
/// Tres rayas (menú).
const ICONO_MENU: &str = "M3 5.5h14M3 10h14M3 14.5h14";

/// Pide al marco que vuelva a contar lo que muestra la barra lateral (el
/// contador de Mejoras): el seguimiento de los cambios (Ajustes y Mejoras)
/// incrementa la señal al terminar uno, aplicado o deshecho.
#[derive(Clone, Copy)]
pub struct RecargaMarco(pub RwSignal<u32>);

/// La cuenta de pasos pendientes que enseña el marco, ya en el navegador:
/// `None` mientras nadie la ha dado; `Some(None)` si no se pudo leer nada.
/// La escriben el contador, con su consulta, y la página Mejoras, con el
/// total de su lectura.
#[derive(Clone, Copy)]
pub struct CuentaMejoras(pub RwSignal<Option<Option<u32>>>);

/// ¿`ruta` es la página Mejoras, que da la cuenta con su propia lectura?
fn en_mejoras(ruta: &str) -> bool {
    seccion_activa("/mejoras", ruta)
}

/// La píldora del contador: nada sin pasos pendientes o sin lectura.
fn pildora(n: Option<u32>) -> Option<impl IntoView + use<>> {
    n.filter(|n| *n > 0).map(|n| {
        view! {
            <span class="cuenta" role="img" aria-label=t!("comun-nav-mejoras-pendientes", n = n)>
                {n}
            </span>
        }
    })
}

/// El contador de pasos pendientes de Mejoras, en su entrada del menú. Una
/// consulta propia, que no bloquea ni espera al nombre de usuario, al
/// cargar el marco (salvo en `/mejoras`, donde la página da el total) y con
/// [`RecargaMarco`]. Al cambiar de página solo se repite si la cuenta tiene
/// más de [`CADUCIDAD_LECTURAS`] (la caché de lecturas del servidor): así no
/// multiplica las lecturas del router. Sin nada pendiente, o si no se pudo
/// leer nada, no sale.
#[component]
fn ContadorMejoras() -> impl IntoView {
    let ruta = use_location().pathname;
    let recarga = expect_context::<RecargaMarco>().0;
    let valor = expect_context::<CuentaMejoras>().0;
    // La cuenta caducada se vuelve a pedir en el siguiente cambio de página.
    let caducada = StoredValue::new(false);
    let generacion = StoredValue::new(0u32);
    let vuelta = RwSignal::new(0u32);
    Effect::new(move |anterior: Option<String>| {
        let r = ruta.get();
        if anterior.is_some_and(|a| a != r) && !en_mejoras(&r) && caducada.get_value() {
            caducada.set_value(false);
            vuelta.update(|n| *n += 1);
        }
        r
    });
    let cuenta = Resource::new(
        move || (recarga.get(), vuelta.get()),
        move |_| {
            let pedir = !en_mejoras(&ruta.get_untracked());
            async move {
                if pedir {
                    Some(contar_mejoras().await.ok().flatten())
                } else {
                    None
                }
            }
        },
    );
    Effect::new(move |_| {
        if let Some(Some(n)) = cuenta.get() {
            valor.set(Some(n));
        }
    });
    // Cada cuenta nueva (de la consulta o de la página) vale lo que la caché.
    Effect::new(move |_| {
        if valor.get().is_none() {
            return;
        }
        caducada.set_value(false);
        let g = generacion.get_value().wrapping_add(1);
        generacion.set_value(g);
        set_timeout(
            move || {
                if generacion.try_get_value() == Some(g) {
                    caducada.try_set_value(true);
                }
            },
            CADUCIDAD_LECTURAS,
        );
    });
    view! {
        <Transition fallback=|| ()>
            {move || Suspend::new(async move {
                // En el servidor y al hidratar, la de la consulta; después,
                // la última que se dio.
                let inicial = cuenta.await.flatten();
                move || pildora(valor.get().unwrap_or(inicial))
            })}
        </Transition>
    }
}

/// Marco común: navegación y contenido de la ruta hija.
#[component]
pub fn Layout() -> impl IntoView {
    let (abierto, set_abierto) = signal(false);
    // Bloqueante: el nombre (una consulta de la sesión, rápida) ya va en el
    // HTML del servidor, dentro de su enlace, no en un trozo posterior.
    let usuario = OnceResource::new_blocking(quien_soy());
    let salir = ServerAction::<Logout>::new();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if let Some(Ok(())) = salir.value().get() {
            navigate("/login", Default::default());
        }
    });
    let cerrar = move |_| set_abierto.set(false);
    provide_context(RecargaMarco(RwSignal::new(0)));
    provide_context(CuentaMejoras(RwSignal::new(None)));

    view! {
        <a class="saltar" href="#contenido">
            {t!("comun-saltar")}
        </a>
        <div class="marco">
            <header class="barra" class:abierta=abierto>
                <div class="barra-cabeza">
                    <a class="marca" href="/" aria-label=t!("comun-marca-inicio")>
                        <svg
                            class="marca-logo"
                            viewBox="0 0 24 24"
                            width="24"
                            height="24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.6"
                            stroke-linecap="round"
                            aria-hidden="true"
                            focusable="false"
                        >
                            <rect x="2.5" y="12.5" width="19" height="7" rx="1.5" />
                            <path d="M6 16h.01M9 16h.01M6 12.5 4.5 6M18 12.5 19.5 6M9.5 9.5a3.5 3.5 0 0 1 5 0M7.5 7.2a6.4 6.4 0 0 1 9 0" />
                        </svg>
                        <span class="marca-texto">
                            <span class="marca-nombre">"RTF8115VW"</span>
                            <span class="marca-sub">{t!("comun-marca-sub")}</span>
                        </span>
                    </a>
                    <button
                        class="menu-boton"
                        type="button"
                        aria-controls="barra-cuerpo"
                        aria-expanded=move || abierto.get().to_string()
                        on:click=move |_| set_abierto.update(|a| *a = !*a)
                    >
                        {icono(ICONO_MENU)}
                        <span class="solo-lector">{t!("comun-menu")}</span>
                    </button>
                </div>
                <div class="barra-cuerpo" id="barra-cuerpo">
                    <nav class="nav" aria-label=t!("comun-nav-principal") on:click=cerrar>
                        <A href="/" exact=true>
                            {icono(ICONO_ESTADO)}
                            <span>{t!("comun-nav-estado")}</span>
                        </A>
                        <EnlaceSeccion href=crate::ui::ajustes_router::RUTA_AJUSTES prefijo="/ajustes">
                            {icono(ICONO_AJUSTES)}
                            <span>{t!("comun-nav-ajustes")}</span>
                        </EnlaceSeccion>
                        <EnlaceSeccion href="/mejoras" prefijo="/mejoras">
                            {icono(ICONO_MEJORAS)}
                            <span>{t!("comun-nav-mejoras")}</span>
                            <ContadorMejoras />
                        </EnlaceSeccion>
                        <EnlaceSeccion href="/historial" prefijo="/historial">
                            {icono(ICONO_HISTORIAL)}
                            <span>{t!("comun-nav-historial")}</span>
                        </EnlaceSeccion>
                        <EnlaceSeccion href="/explorador" prefijo="/explorador">
                            {icono(ICONO_BIBLIOTECA)}
                            <span>{t!("comun-nav-biblioteca")}</span>
                        </EnlaceSeccion>
                        <EnlaceSeccion href=crate::ui::router::RUTA_CONEXION prefijo="/router">
                            {icono(ICONO_ROUTER)}
                            <span>{t!("comun-nav-router")}</span>
                        </EnlaceSeccion>
                    </nav>
                    <div class="sesion">
                        <SelectorIdioma />
                        <EnlaceSeccion
                            href="/panel"
                            prefijo="/panel"
                            attr:class="sesion-usuario"
                            on:click=cerrar
                        >
                            {icono(ICONO_USUARIO)}
                            <span class="sesion-nombre">
                                <Suspense fallback=|| view! { "…" }>
                                    {move || Suspend::new(async move {
                                        usuario.await.unwrap_or_else(|_| "—".to_owned())
                                    })}
                                </Suspense>
                            </span>
                            <span class="solo-lector">{t!("comun-usuario-ajustes")}</span>
                        </EnlaceSeccion>
                        <button
                            class="b-sec salir"
                            type="button"
                            disabled=move || salir.pending().get()
                            on:click=move |_| {
                                salir.dispatch(Logout {});
                            }
                        >
                            {icono(ICONO_SALIR)}
                            <span>{t!("comun-salir")}</span>
                        </button>
                    </div>
                </div>
            </header>
            <main class="contenido" id="contenido" tabindex="-1">
                <Outlet />
            </main>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::seccion_activa;

    #[test]
    fn seccion_activa_por_prefijo() {
        assert!(seccion_activa("/router", "/router/conexion"));
        assert!(seccion_activa("/router", "/router"));
        assert!(!seccion_activa("/router", "/routerx"));
        assert!(!seccion_activa("/recomendaciones", "/"));
    }
}
