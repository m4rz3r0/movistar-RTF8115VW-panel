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
    // `try_`: quien lo pinta puede desecharse un instante antes que el
    // botón (una relectura de la página que llega mientras otra se pinta);
    // entonces no se toca nada.
    view! {
        <button
            type="button"
            class="int"
            role="switch"
            aria-checked=move || encendido.try_get().map(|e| e.to_string())
            aria-label=move || etiqueta.try_get()
            aria-disabled=move || desactivado.try_get().unwrap_or(true).then_some("true")
            aria-describedby=descrito
            on:click=move |_| {
                if let (Some(e), Some(d)) =
                    (encendido.try_get_untracked(), desactivado.try_get_untracked())
                    && let Some(nuevo) = al_pulsar(e, d)
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
