//! Marco de las páginas con sesión: barra lateral (Estado, Explorador,
//! Historial, usuario y «Salir») que en pantallas estrechas pasa a ser una
//! barra superior con el menú plegable.

use crate::auth::{Logout, quien_soy};
use leptos::prelude::*;
use leptos_router::{components::A, components::Outlet, hooks::use_navigate};

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
/// Rejilla de bloques (Explorador).
const ICONO_EXPLORADOR: &str =
    "M3 3h5.5v5.5H3zM11.5 3H17v5.5h-5.5zM3 11.5h5.5V17H3zM11.5 11.5H17V17h-5.5z";
/// Reloj (Historial).
const ICONO_HISTORIAL: &str =
    "M10 2.75a7.25 7.25 0 1 1 0 14.5 7.25 7.25 0 0 1 0-14.5zM10 6v4l2.75 1.75";
/// Puerta con flecha (Salir).
const ICONO_SALIR: &str =
    "M8 3H4.5A1.5 1.5 0 0 0 3 4.5v11A1.5 1.5 0 0 0 4.5 17H8M12.5 6.5 16 10l-3.5 3.5M16 10H7.5";
/// Persona (usuario).
const ICONO_USUARIO: &str =
    "M10 3a3.25 3.25 0 1 1 0 6.5A3.25 3.25 0 0 1 10 3zM3.75 17c.6-3.1 3.1-5 6.25-5s5.65 1.9 6.25 5";
/// Tres rayas (menú).
const ICONO_MENU: &str = "M3 5.5h14M3 10h14M3 14.5h14";

/// Marco común: navegación y contenido de la ruta hija.
#[component]
pub fn Layout() -> impl IntoView {
    let (abierto, set_abierto) = signal(false);
    let usuario = OnceResource::new(quien_soy());
    let salir = ServerAction::<Logout>::new();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if let Some(Ok(())) = salir.value().get() {
            navigate("/login", Default::default());
        }
    });
    let cerrar = move |_| set_abierto.set(false);

    view! {
        <a class="saltar" href="#contenido">
            "Saltar al contenido"
        </a>
        <div class="marco">
            <header class="barra" class:abierta=abierto>
                <div class="barra-cabeza">
                    <a class="marca" href="/" aria-label="RTF8115VW, inicio">
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
                            <span class="marca-sub">"Panel del router"</span>
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
                        <span class="solo-lector">"Menú"</span>
                    </button>
                </div>
                <div class="barra-cuerpo" id="barra-cuerpo">
                    <nav class="nav" aria-label="Principal" on:click=cerrar>
                        <A href="/" exact=true>
                            {icono(ICONO_ESTADO)}
                            <span>"Estado"</span>
                        </A>
                        <A href="/explorador">
                            {icono(ICONO_EXPLORADOR)}
                            <span>"Explorador"</span>
                        </A>
                        <A href="/historial">
                            {icono(ICONO_HISTORIAL)}
                            <span>"Historial"</span>
                        </A>
                    </nav>
                    <div class="sesion">
                        <span class="sesion-usuario">
                            {icono(ICONO_USUARIO)}
                            <Suspense fallback=|| view! { <span>"…"</span> }>
                                {move || Suspend::new(async move {
                                    usuario.await.unwrap_or_else(|_| "—".to_owned())
                                })}
                            </Suspense>
                        </span>
                        <button
                            class="boton-sutil salir"
                            type="button"
                            disabled=move || salir.pending().get()
                            on:click=move |_| {
                                salir.dispatch(Logout {});
                            }
                        >
                            {icono(ICONO_SALIR)}
                            <span>"Salir"</span>
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
