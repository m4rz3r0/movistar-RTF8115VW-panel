//! Interruptor de encendido (`role="switch"`), verde cuando está encendido.

use leptos::prelude::*;

/// Valor nuevo al pulsar el interruptor: el contrario, o ninguno si está
/// desactivado.
fn al_pulsar(encendido: bool, desactivado: bool) -> Option<bool> {
    (!desactivado).then_some(!encendido)
}

/// Interruptor de encendido. `etiqueta` es su nombre accesible y dice el
/// elemento y su estado («Red WiFi 5 GHz encendida»). Al pulsarlo llama a
/// `al_cambiar` con el valor nuevo; no cambia por sí solo: lo cambia quien
/// lo pinta, al aplicar el cambio. Desactivado lleva `aria-disabled` (no
/// `disabled`), como el resto del panel: sigue recibiendo el foco, para que
/// se pueda explicar por qué (`descrito`: el id de esa explicación), y el
/// clic no hace nada.
#[component]
pub fn Interruptor(
    #[prop(into)] encendido: Signal<bool>,
    #[prop(into)] etiqueta: Signal<String>,
    #[prop(into, optional)] desactivado: Signal<bool>,
    #[prop(into, optional_no_strip)] descrito: Option<String>,
    al_cambiar: Callback<bool>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class="int"
            role="switch"
            aria-checked=move || encendido.get().to_string()
            aria-label=move || etiqueta.get()
            aria-disabled=move || desactivado.get().then_some("true")
            aria-describedby=descrito
            on:click=move |_| {
                if let Some(nuevo) =
                    al_pulsar(encendido.get_untracked(), desactivado.get_untracked())
                {
                    al_cambiar.run(nuevo);
                }
            }
        ></button>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desactivado_no_cambia() {
        assert_eq!(al_pulsar(true, false), Some(false));
        assert_eq!(al_pulsar(false, false), Some(true));
        assert_eq!(al_pulsar(true, true), None);
        assert_eq!(al_pulsar(false, true), None);
    }
}
