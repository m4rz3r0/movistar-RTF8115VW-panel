//! Modo de visualización en la interfaz ([`ModoVista`]): contexto de la
//! página, selector rápido de la cabecera de Estado ([`SelectorVista`]) y
//! bloque «Visualización» de «Ajustes del panel» ([`SeccionVista`]).
//!
//! El servidor pone el modo del usuario en `<html data-vista>` y en el
//! contexto; al hidratar, el navegador lo lee de `<html data-vista>`: los dos
//! pintan lo mismo. Cambiarlo no recarga la página: el contexto es una señal,
//! lo que depende del modo se repinta y `<html data-vista>` se actualiza.

use crate::t;
use crate::ui::panel_ajustes::{mensajes, texto_error};
use crate::vista::{ModoVista, guardar_vista};
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};

/// Modo de visualización de la página.
#[derive(Clone, Copy)]
struct VistaContexto(RwSignal<ModoVista>);

/// Pone `modo` en el contexto de la aplicación. En el navegador,
/// `<html data-vista>` sigue al modo de la página cuando se cambia.
pub fn provide_vista(modo: ModoVista) {
    let senal = RwSignal::new(modo);
    provide_context(VistaContexto(senal));
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        let codigo = senal.get().codigo();
        if let Some(html) = document().document_element() {
            let _ = html.set_attribute("data-vista", codigo);
        }
    });
}

/// Modo de visualización de la página (una señal: se puede seguir y
/// cambiar). Sin contexto, el de por defecto.
pub fn use_vista() -> RwSignal<ModoVista> {
    use_context::<VistaContexto>()
        .map(|c| c.0)
        .unwrap_or_else(|| RwSignal::new(ModoVista::default()))
}

/// Modo con el que arranca la aplicación: el del usuario de la petición en
/// el servidor; el de `<html data-vista>` (que puso el servidor) al
/// hidratar.
pub fn vista_inicial() -> ModoVista {
    if let Some(VistaContexto(m)) = use_context() {
        return m.get_untracked();
    }
    #[cfg(feature = "ssr")]
    if let Some(m) = use_context::<axum::http::request::Parts>().and_then(|p| {
        p.extensions
            .get::<crate::auth::AuthUser>()
            .map(|u| u.0.modo_vista)
    }) {
        return m;
    }
    #[cfg(feature = "hydrate")]
    if let Some(m) = document()
        .document_element()
        .and_then(|e| e.get_attribute("data-vista"))
        .and_then(|v| ModoVista::desde_codigo(&v))
    {
        return m;
    }
    ModoVista::default()
}

/// Nombre del modo.
fn nombre(m: ModoVista) -> String {
    match m {
        ModoVista::Texto => t!("panel-vista-texto"),
        ModoVista::Grafica => t!("panel-vista-grafica"),
        ModoVista::Mixto => t!("panel-vista-mixto"),
    }
}

/// Qué hace el modo.
fn ayuda(m: ModoVista) -> String {
    match m {
        ModoVista::Texto => t!("panel-vista-texto-ayuda"),
        ModoVista::Grafica => t!("panel-vista-grafica-ayuda"),
        ModoVista::Mixto => t!("panel-vista-mixto-ayuda"),
    }
}

/// Selector rápido de la cabecera de Estado: cambia el modo de la página al
/// momento y lo guarda; si no se puede guardar, vuelve al anterior y lo dice.
#[component]
pub fn SelectorVista() -> impl IntoView {
    let vista = use_vista();
    let error = RwSignal::new(None::<String>);
    let elegir = move |m: ModoVista| {
        let antes = vista.get_untracked();
        if m == antes {
            return;
        }
        vista.set(m);
        error.set(None);
        spawn_local(async move {
            if let Err(e) = guardar_vista(m).await {
                vista.set(antes);
                error.set(Some(t!("estado-vista-error", error = texto_error(&e))));
            }
        });
    };
    let botones = ModoVista::TODOS
        .into_iter()
        .map(|m| {
            view! {
                <button
                    type="button"
                    class="selector-boton"
                    value=m.codigo()
                    title=ayuda(m)
                    aria-pressed=move || (vista.get() == m).to_string()
                    on:click=move |_| elegir(m)
                >
                    {nombre(m)}
                </button>
            }
        })
        .collect_view();
    view! {
        <div class="selector-vista">
            <div class="selector" role="group" aria-label=t!("estado-vista")>
                {botones}
            </div>
            <p class="error-formulario selector-vista-error" role="alert">{move || error.get()}</p>
        </div>
    }
}

/// Bloque «Visualización» de «Ajustes del panel»: los tres modos explicados
/// y el botón de guardar.
#[component]
pub fn SeccionVista() -> impl IntoView {
    let vista = use_vista();
    let elegido = RwSignal::new(vista.get_untracked());
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let m = elegido.get_untracked();
        spawn_local(async move {
            match guardar_vista(m).await {
                Ok(()) => {
                    vista.set(m);
                    aviso.set(Some(t!("panel-guardado")));
                }
                Err(e) => error.set(Some(texto_error(&e))),
            }
            pendiente.set(false);
        });
    };
    let opciones = ModoVista::TODOS
        .into_iter()
        .map(|m| {
            let id = format!("panel-vista-{}", m.codigo());
            let ayuda_id = format!("{id}-ayuda");
            view! {
                <div class="opcion">
                    <input
                        id=id.clone()
                        type="radio"
                        name="vista"
                        value=m.codigo()
                        aria-describedby=ayuda_id.clone()
                        checked=vista.get_untracked() == m
                        prop:checked=move || elegido.get() == m
                        on:change=move |_| elegido.set(m)
                    />
                    <span class="opcion-texto">
                        <label class="opcion-nombre" for=id>
                            {nombre(m)}
                        </label>
                        <span class="nota" id=ayuda_id>
                            {ayuda(m)}
                        </span>
                    </span>
                </div>
            }
        })
        .collect_view();
    view! {
        <section class="tarjeta" aria-labelledby="panel-vista">
            <h2 id="panel-vista">{t!("panel-vista")}</h2>
            <p class="nota">{t!("panel-vista-ayuda")}</p>
            <form class="ajustes-formulario" on:submit=enviar>
                <fieldset class="opciones">
                    <legend class="solo-lector">{t!("panel-vista-modo")}</legend>
                    {opciones}
                </fieldset>
                {mensajes(error, aviso)}
                <div class="botones">
                    <button class="boton" type="submit" disabled=move || pendiente.get()>
                        {t!("panel-guardar")}
                    </button>
                </div>
            </form>
        </section>
    }
}
