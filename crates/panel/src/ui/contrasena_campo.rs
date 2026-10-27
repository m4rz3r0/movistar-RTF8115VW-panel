//! Campo de una contraseña de este panel ([`CampoContrasena`]): el de entrar,
//! los del asistente y los de los usuarios del panel. Lleva el ojo de los
//! secretos ([`super::secreto`]), que la muestra u oculta al instante y no la
//! oculta solo, pero, al revés que ellos, deja actuar al gestor de
//! contraseñas del navegador: es la contraseña de este sitio.

use crate::t;
use crate::ui::secreto::IconoOjo;
use leptos::prelude::*;

/// Campo de contraseña con su etiqueta, el ojo y, si la tiene, su ayuda.
/// `valor` sigue lo escrito; con `nombre`, el formulario también lo envía
/// sin JavaScript. `autocompletar`: `current-password` o `new-password`.
#[component]
pub fn CampoContrasena(
    #[prop(into)] id: String,
    #[prop(into)] etiqueta: String,
    autocompletar: &'static str,
    valor: RwSignal<String>,
    #[prop(optional, into)] nombre: Option<String>,
    #[prop(optional, into)] ayuda: Option<String>,
    #[prop(optional)] requerido: bool,
    #[prop(optional, into)] invalido: Signal<bool>,
) -> impl IntoView {
    let visible = RwSignal::new(false);
    let ayuda_id = format!("{id}-ayuda");
    let descrito = ayuda.is_some().then(|| ayuda_id.clone());
    let quien = minuscula_inicial(&etiqueta);
    let control = id.clone();
    view! {
        <p class="campo campo-secreto">
            <label for=id.clone()>{etiqueta}</label>
            <span class="secreto-campo">
                <input
                    id=id
                    name=nombre
                    class="secreto-valor"
                    type=move || if visible.get() { "text" } else { "password" }
                    autocomplete=autocompletar
                    autocapitalize="none"
                    spellcheck="false"
                    required=requerido
                    aria-describedby=descrito
                    aria-invalid=move || invalido.get().then_some("true")
                    prop:value=valor
                    on:input=move |ev| valor.set(event_target_value(&ev))
                />
                <button
                    class="b-txt boton-ojo"
                    type="button"
                    aria-controls=control
                    aria-pressed=move || visible.get().to_string()
                    aria-label=move || {
                        if visible.get() {
                            t!("secreto-ocultar", campo = quien.clone())
                        } else {
                            t!("secreto-ver", campo = quien.clone())
                        }
                    }
                    on:click=move |_| visible.update(|v| *v = !*v)
                >
                    <IconoOjo visible=visible.into() />
                </button>
            </span>
            {ayuda.map(|a| view! { <span class="nota" id=ayuda_id>{a}</span> })}
        </p>
    }
}

/// «Contraseña nueva» → «contraseña nueva», para «Mostrar contraseña nueva».
fn minuscula_inicial(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|p| p.to_lowercase().chain(c).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn minuscula_inicial() {
        assert_eq!(
            super::minuscula_inicial("Contraseña nueva"),
            "contraseña nueva"
        );
        assert_eq!(super::minuscula_inicial(""), "");
    }
}
