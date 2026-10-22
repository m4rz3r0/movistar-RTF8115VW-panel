//! Un secreto en pantalla ([`Secreto`]): un campo de contraseña ya relleno
//! con su valor, censurado, y un botón con un ojo que lo muestra u oculta al
//! instante. Por decisión del usuario (2026-10-18), el valor viaja con la
//! página (que va con `no-store`), no se oculta solo y, si se puede
//! escribir, se edita en el mismo campo.
//!
//! Lo usan Ajustes (los campos de un ajuste, editables o no), la Biblioteca
//! (una celda de una lectura) y el Historial (los valores de un cambio). Al
//! salir de la página, el componente se libera y su valor con él.

use crate::t;
use leptos::prelude::*;

/// El ojo dice lo que hace el botón, como es costumbre: abierto si la clave
/// está censurada («mostrar»), tachado si está a la vista («ocultar»).
/// Decorativo: el nombre lo da el botón.
#[component]
fn IconoOjo(visible: Signal<bool>) -> impl IntoView {
    view! {
        <svg
            class="icono icono-ojo"
            viewBox="0 0 16 16"
            width="16"
            height="16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"
        >
            <path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" />
            <circle cx="8" cy="8" r="1.9" />
            {move || visible.get().then(|| view! { <path d="M2.5 13.5l11-11" /> })}
        </svg>
    }
}

/// Campo de contraseña con su valor (`valor`) y el ojo. Con `editable`, se
/// escribe en `valor` (el formulario lo envía si cambió); si no, es de solo
/// lectura. `id` va en el `<input>` (para su `<label for>`); sin etiqueta
/// visible, `nombre` es su nombre accesible. `vacio` es lo que se lee si no
/// hay valor (el router no deja leerlo).
#[component]
pub fn Secreto(
    valor: RwSignal<String>,
    #[prop(into)] nombre: String,
    #[prop(optional)] editable: bool,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] vacio: Option<String>,
    #[prop(optional, into)] descrito: Option<String>,
    #[prop(optional, into)] invalido: Signal<bool>,
    #[prop(optional, into)] bloqueado: Signal<bool>,
    #[prop(optional)] error: Option<RwSignal<Option<String>>>,
) -> impl IntoView {
    let visible = RwSignal::new(false);
    // Sin `<label>` (una celda, una fila de la vista resumida), el nombre.
    let etiqueta = id.is_none().then(|| nombre.clone());
    let control = id.clone();
    // `new-password`: el gestor de contraseñas del navegador no la rellena
    // ni propone guardarla (es la del router, no la de este sitio).
    let autocompletar = if editable { "new-password" } else { "off" };
    view! {
        <span class="secreto-campo">
            <input
                id=id
                class="secreto-valor"
                type=move || if visible.get() { "text" } else { "password" }
                autocomplete=autocompletar
                autocapitalize="none"
                spellcheck="false"
                readonly=!editable
                aria-label=etiqueta
                aria-describedby=descrito
                aria-invalid=move || invalido.get().then_some("true")
                placeholder=vacio
                // Que los gestores de contraseñas no la rellenen ni la
                // guarden: es una clave del router, no de este sitio.
                data-1p-ignore=""
                data-lpignore="true"
                data-bwignore=""
                data-form-type="other"
                disabled=move || editable && bloqueado.get()
                // El atributo, para el HTML del servidor; luego manda la
                // propiedad (lo que se escribe).
                value=valor.get_untracked()
                prop:value=move || valor.get()
                on:input=move |ev| {
                    if editable {
                        valor.set(event_target_value(&ev));
                        if let Some(e) = error {
                            e.set(None);
                        }
                    }
                }
            />
            <button
                class="b-txt boton-ojo"
                type="button"
                aria-controls=control
                aria-pressed=move || visible.get().to_string()
                aria-label={
                    // Con el nombre del campo: en una tabla hay varios ojos.
                    let nombre = nombre.clone();
                    move || {
                        if visible.get() {
                            t!("secreto-ocultar", campo = nombre.clone())
                        } else {
                            t!("secreto-ver", campo = nombre.clone())
                        }
                    }
                }
                on:click=move |_| visible.update(|v| *v = !*v)
            >
                <IconoOjo visible=visible.into() />
            </button>
        </span>
    }
}

/// [`Secreto`] de solo lectura con el valor `valor` (`None`: el router no
/// deja leerlo, y se dice).
#[component]
pub fn SecretoLeido(valor: Option<String>, #[prop(into)] nombre: String) -> impl IntoView {
    let vacio = valor.is_none().then(|| t!("secreto-sin-leer"));
    let valor = RwSignal::new(valor.unwrap_or_default());
    match vacio {
        Some(vacio) => view! { <Secreto valor nombre vacio /> }.into_any(),
        None => view! { <Secreto valor nombre /> }.into_any(),
    }
}
