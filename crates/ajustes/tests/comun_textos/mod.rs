//! Ayudas comunes de las pruebas de textos: el formato de las recomendaciones.

use i18n::Idioma;

/// Marcas que exige cada `…-recomendado`, por idioma y en este orden: para
/// quién es, lo que se gana y lo que se pierde (decisión del usuario,
/// 2026-10-13).
pub fn marcas_de_recomendacion(idioma: Idioma) -> [&'static str; 3] {
    match idioma {
        Idioma::Es => ["Recomendado si", "A favor:", "En contra:"],
        Idioma::En => ["Recommended if", "Pros:", "Cons:"],
    }
}

/// `texto` empieza por la primera marca, trae las tres en orden y, tras
/// cada una, algo más que espacios y signos de puntuación.
pub fn recomendacion_bien_formada(texto: &str, marcas: [&str; 3]) -> bool {
    let Some(mut resto) = texto.strip_prefix(marcas[0]) else {
        return false;
    };
    let con_texto = |s: &str| s.chars().any(char::is_alphanumeric);
    for marca in &marcas[1..] {
        let Some((antes, despues)) = resto.split_once(marca) else {
            return false;
        };
        if !con_texto(antes) {
            return false;
        }
        resto = despues;
    }
    con_texto(resto)
}
