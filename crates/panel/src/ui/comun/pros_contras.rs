//! A favor y en contra de una recomendación ([`ProsContras`]), en dos
//! bloques con su encabezado en vez de un párrafo corrido: los contras se
//! ven tanto como los pros.

use crate::t;
use leptos::prelude::*;

/// Parte «A favor: X En contra: Y» en `(X, Some(Y))` con las marcas `favor`
/// y `contra` del idioma. Sin «En contra:», todo va a favor; sin «A favor:»
/// delante, también (el texto se enseña tal cual).
pub(crate) fn partir_pros(texto: &str, favor: &str, contra: &str) -> (String, Option<String>) {
    let texto = texto.trim();
    let sin_favor = texto.strip_prefix(favor).unwrap_or(texto).trim();
    match sin_favor.find(contra) {
        Some(i) => (
            sin_favor[..i].trim().to_owned(),
            Some(sin_favor[i + contra.len()..].trim().to_owned()),
        ),
        None => (sin_favor.to_owned(), None),
    }
}

/// Los pros y los contras de `texto` («A favor: … En contra: …»).
#[component]
pub fn ProsContras(#[prop(into)] texto: String) -> impl IntoView {
    let (favor, contra) = partir_pros(
        &texto,
        &t!("ajustes-recomendado-marca"),
        &t!("ajustes-recomendado-contra-marca"),
    );
    view! {
        <dl class="pros-contras">
            <div class="pros-contras-favor">
                <dt>{t!("ajustes-recomendado-favor")}</dt>
                <dd>{crate::ui::mayuscula_inicial(&favor)}</dd>
            </div>
            {contra.map(|c| view! {
                <div class="pros-contras-contra">
                    <dt>{t!("ajustes-recomendado-contra")}</dt>
                    <dd>{crate::ui::mayuscula_inicial(&c)}</dd>
                </div>
            })}
        </dl>
    }
}

#[cfg(test)]
mod tests {
    use super::partir_pros;

    #[test]
    fn parte_a_favor_y_en_contra() {
        let (f, c) = partir_pros(
            "A favor: cierra una vía. En contra: sin actualizaciones.",
            "A favor:",
            "En contra:",
        );
        assert_eq!(f, "cierra una vía.");
        assert_eq!(c.as_deref(), Some("sin actualizaciones."));
        let (f, c) = partir_pros("Pros: faster. Cons: louder.", "Pros:", "Cons:");
        assert_eq!((f.as_str(), c.as_deref()), ("faster.", Some("louder.")));
    }

    /// Sin marcas, el texto entero va a favor, tal cual.
    #[test]
    fn sin_marcas_va_entero() {
        let (f, c) = partir_pros("Solo una frase.", "A favor:", "En contra:");
        assert_eq!((f.as_str(), c), ("Solo una frase.", None));
    }
}
