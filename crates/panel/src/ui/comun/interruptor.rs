//! Interruptor de encendido (`role="switch"`), verde cuando está encendido.

use leptos::prelude::*;

/// Interruptor de encendido. `etiqueta` es su nombre accesible y dice el
/// elemento y su estado («Red WiFi 5 GHz encendida»). Al pulsarlo llama a
/// `al_cambiar` con el valor nuevo; no cambia por sí solo: lo cambia quien
/// lo pinta, al aplicar el cambio.
#[component]
pub fn Interruptor(
    #[prop(into)] encendido: Signal<bool>,
    #[prop(into)] etiqueta: Signal<String>,
    #[prop(into, optional)] desactivado: Signal<bool>,
    al_cambiar: Callback<bool>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class="int"
            role="switch"
            aria-checked=move || encendido.get().to_string()
            aria-label=move || etiqueta.get()
            disabled=move || desactivado.get()
            on:click=move |_| al_cambiar.run(!encendido.get_untracked())
        ></button>
    }
}
