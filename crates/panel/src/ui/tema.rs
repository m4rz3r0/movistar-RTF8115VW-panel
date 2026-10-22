//! Tema de la interfaz ([`TemaUi`]): contexto de la página y bloque «Tema» de
//! «Ajustes del panel» ([`SeccionTema`]).
//!
//! El servidor pone el tema del usuario en `<html data-tema>` y en el
//! contexto; al hidratar, el navegador lo lee de ahí. Elegir otro lo aplica
//! al momento (el contexto es una señal y `<html data-tema>` la sigue, sin
//! recargar) y lo guarda; si no se puede guardar, vuelve al anterior.

use crate::t;
use crate::tema::{TemaUi, guardar_tema_ui};
use crate::ui::panel_ajustes::{mensajes, texto_error};
use leptos::{prelude::*, task::spawn_local};

/// Tema de la página.
#[derive(Clone, Copy)]
struct TemaContexto(RwSignal<TemaUi>);

/// Pone `tema` en el contexto de la aplicación. En el navegador,
/// `<html data-tema>` sigue al tema de la página cuando se cambia.
pub fn provide_tema(tema: TemaUi) {
    let senal = RwSignal::new(tema);
    provide_context(TemaContexto(senal));
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        let codigo = senal.get().codigo();
        if let Some(html) = document().document_element() {
            let _ = html.set_attribute("data-tema", codigo);
        }
    });
}

/// Tema de la página (una señal). Sin contexto, el de por defecto.
pub fn use_tema() -> RwSignal<TemaUi> {
    use_context::<TemaContexto>()
        .map(|c| c.0)
        .unwrap_or_else(|| RwSignal::new(TemaUi::default()))
}

/// Tema con el que arranca la aplicación: el del usuario de la petición en
/// el servidor (sin sesión, automático); el de `<html data-tema>` al
/// hidratar.
pub fn tema_inicial() -> TemaUi {
    if let Some(TemaContexto(t)) = use_context() {
        return t.get_untracked();
    }
    #[cfg(feature = "ssr")]
    if let Some(t) = use_context::<axum::http::request::Parts>().and_then(|p| {
        p.extensions
            .get::<crate::auth::AuthUser>()
            .map(|u| u.0.tema)
    }) {
        return t;
    }
    #[cfg(feature = "hydrate")]
    if let Some(t) = document()
        .document_element()
        .and_then(|e| e.get_attribute("data-tema"))
        .and_then(|v| TemaUi::desde_codigo(&v))
    {
        return t;
    }
    TemaUi::default()
}

/// Nombre del tema.
fn nombre(t: TemaUi) -> String {
    match t {
        TemaUi::Automatico => t!("panel-tema-automatico"),
        TemaUi::Oscuro => t!("panel-tema-oscuro"),
        TemaUi::Claro => t!("panel-tema-claro"),
    }
}

/// Qué hace el tema.
fn ayuda(t: TemaUi) -> String {
    match t {
        TemaUi::Automatico => t!("panel-tema-automatico-ayuda"),
        TemaUi::Oscuro => t!("panel-tema-oscuro-ayuda"),
        TemaUi::Claro => t!("panel-tema-claro-ayuda"),
    }
}

/// Tras no poder guardar `fallido`, vuelve a `antes` solo si la página sigue
/// en `fallido`: si entretanto se eligió otro, se queda en ese.
fn deshacer_si_sigue(tema: RwSignal<TemaUi>, fallido: TemaUi, antes: TemaUi) {
    if tema.get_untracked() == fallido {
        tema.set(antes);
    }
}

/// Bloque «Tema» de «Ajustes del panel»: las tres opciones; elegir una la
/// aplica al momento y la guarda.
#[component]
pub fn SeccionTema() -> impl IntoView {
    let tema = use_tema();
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let elegir = move |nuevo: TemaUi| {
        let antes = tema.get_untracked();
        if nuevo == antes {
            return;
        }
        tema.set(nuevo);
        error.set(None);
        aviso.set(None);
        spawn_local(async move {
            match guardar_tema_ui(nuevo).await {
                Ok(()) => aviso.set(Some(t!("panel-guardado"))),
                Err(e) => {
                    deshacer_si_sigue(tema, nuevo, antes);
                    error.set(Some(texto_error(&e)));
                }
            }
        });
    };
    let opciones = TemaUi::TODOS
        .into_iter()
        .map(|t| {
            let id = format!("panel-tema-{}", t.codigo());
            let ayuda_id = format!("{id}-ayuda");
            view! {
                <div class="opcion">
                    <input
                        id=id.clone()
                        type="radio"
                        name="tema"
                        value=t.codigo()
                        aria-describedby=ayuda_id.clone()
                        checked=tema.get_untracked() == t
                        prop:checked=move || tema.get() == t
                        on:change=move |_| elegir(t)
                    />
                    <span class="opcion-texto">
                        <label class="opcion-nombre" for=id>
                            {nombre(t)}
                        </label>
                        <span class="nota" id=ayuda_id>
                            {ayuda(t)}
                        </span>
                    </span>
                </div>
            }
        })
        .collect_view();
    view! {
        <section class="tarjeta" aria-labelledby="panel-tema">
            <h2 id="panel-tema">{t!("panel-tema")}</h2>
            <p class="nota">{t!("panel-tema-ayuda")}</p>
            <div class="ajustes-formulario">
                <fieldset class="opciones">
                    <legend class="solo-lector">{t!("panel-tema-elegir")}</legend>
                    {opciones}
                </fieldset>
                {mensajes(error, aviso)}
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_fallo_no_pisa_un_tema_elegido_despues() {
        Owner::new().with(|| {
            let tema = RwSignal::new(TemaUi::Claro);
            deshacer_si_sigue(tema, TemaUi::Claro, TemaUi::Automatico);
            assert_eq!(tema.get_untracked(), TemaUi::Automatico);
            let tema = RwSignal::new(TemaUi::Oscuro);
            deshacer_si_sigue(tema, TemaUi::Claro, TemaUi::Automatico);
            assert_eq!(tema.get_untracked(), TemaUi::Oscuro);
        });
    }
}
