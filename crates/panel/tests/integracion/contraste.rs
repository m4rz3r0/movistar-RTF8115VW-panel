//! Contraste WCAG AA de los tokens de color (`style/base/tokens.css`) en los
//! dos temas: cada texto sobre su fondo, de 4,5:1 en adelante.

use std::collections::BTreeMap;

const TOKENS: &str = include_str!("../../style/base/tokens.css");

/// Selectores de los bloques de cada tema (uno dentro del otro, en orden).
const OSCURO: &[&str] = &[r#":root[data-tema="oscuro"]"#];
const CLARO: &[&str] = &[r#":root[data-tema="claro"]"#];
const AUTOMATICO_CLARO: &[&str] = &[
    "@media (prefers-color-scheme: light)",
    r#":root[data-tema="automatico"]"#,
];

/// Pares (texto, fondo) que tienen que leerse bien.
const PARES: &[(&str, &str)] = &[
    ("texto", "sup"),
    ("texto", "sup2"),
    ("suave", "sup"),
    ("suave", "sup2"),
    ("tenue", "sup"),
    ("tenue", "sup2"),
    ("acento-texto", "acento-velo"),
    ("ok", "ok-velo"),
    ("apag", "apag-velo"),
    ("aviso", "aviso-velo"),
    ("peligro", "peligro-velo"),
    ("sobre-acento", "acento"),
];

/// `tokens.css` sin comentarios.
fn sin_comentarios() -> String {
    let mut s = TOKENS.to_owned();
    while let Some(i) = s.find("/*") {
        let fin = i + s[i..].find("*/").expect("comentario cerrado") + 2;
        s.replace_range(i..fin, "");
    }
    s
}

/// Los `--nombre: #hex` del primer bloque `{…}` que sigue al último de
/// `selectores` (cada uno se busca a partir del anterior).
fn colores(selectores: &[&str]) -> BTreeMap<String, String> {
    let css = sin_comentarios();
    let mut inicio = 0;
    for selector in selectores {
        inicio += css[inicio..]
            .find(selector)
            .unwrap_or_else(|| panic!("sin el bloque {selector} en tokens.css"));
    }
    let resto = &css[inicio..];
    let abre = resto.find('{').expect("«{» del bloque");
    let mut nivel = 0;
    let mut cierra = resto.len();
    for (i, c) in resto[abre..].char_indices() {
        match c {
            '{' => nivel += 1,
            '}' => {
                nivel -= 1;
                if nivel == 0 {
                    cierra = abre + i;
                    break;
                }
            }
            _ => {}
        }
    }
    resto[abre + 1..cierra]
        .split(';')
        .filter_map(|d| {
            let (nombre, valor) = d.trim().split_once(':')?;
            let nombre = nombre.trim().strip_prefix("--")?;
            let valor = valor.trim();
            valor
                .starts_with('#')
                .then(|| (nombre.to_owned(), valor.to_owned()))
        })
        .collect()
}

/// Componentes sRGB (0–1) de `#rgb` o `#rrggbb`.
fn rgb(hex: &str) -> [f64; 3] {
    let h = hex.trim_start_matches('#');
    let largo: String = match h.len() {
        3 => h.chars().flat_map(|c| [c, c]).collect(),
        6 => h.to_owned(),
        _ => panic!("color no válido: {hex}"),
    };
    let c = |i: usize| f64::from(u8::from_str_radix(&largo[i..i + 2], 16).unwrap()) / 255.0;
    [c(0), c(2), c(4)]
}

/// Luminancia relativa (WCAG 2.x).
fn luminancia(hex: &str) -> f64 {
    let lineal = |c: f64| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let [r, g, b] = rgb(hex).map(lineal);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn relacion_wcag(a: &str, b: &str) -> f64 {
    let (la, lb) = (luminancia(a), luminancia(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[test]
fn relacion_wcag_conocida() {
    assert!((relacion_wcag("#000", "#fff") - 21.0).abs() < 1e-9);
    assert!((relacion_wcag("#777777", "#777777") - 1.0).abs() < 1e-9);
}

#[test]
fn contraste_de_los_tokens() {
    let mut fallos = Vec::new();
    for (tema, selector) in [("oscuro", OSCURO), ("claro", CLARO)] {
        let c = colores(selector);
        for (texto, fondo) in PARES {
            let (Some(t), Some(f)) = (c.get(*texto), c.get(*fondo)) else {
                fallos.push(format!("{tema}: falta --{texto} o --{fondo}"));
                continue;
            };
            let r = relacion_wcag(t, f);
            if r < 4.5 {
                fallos.push(format!("{tema}: --{texto} sobre --{fondo} da {r:.2}:1"));
            }
        }
    }
    assert!(fallos.is_empty(), "{fallos:#?}");
}

#[test]
fn los_dos_temas_tienen_los_mismos_tokens() {
    let oscuro = colores(OSCURO);
    let claro = colores(CLARO);
    assert!(oscuro.len() >= 20, "{oscuro:#?}");
    assert_eq!(
        oscuro.keys().collect::<Vec<_>>(),
        claro.keys().collect::<Vec<_>>()
    );
}

#[test]
fn automatico_en_claro_es_el_tema_claro() {
    // En «automático» con el sistema en claro, los mismos colores que el
    // tema claro (el bloque está repetido: el CSS no depende de JavaScript).
    assert_eq!(colores(AUTOMATICO_CLARO), colores(CLARO));
}
