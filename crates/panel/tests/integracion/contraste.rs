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

/// Las reglas de todos los `.css` de `style/`: (fichero, selector,
/// declaraciones), sin comentarios. Solo los bloques sin otros dentro (las
/// reglas de un `@media` salen sueltas).
fn reglas_css() -> Vec<(String, String, String)> {
    fn recorrer(dir: &std::path::Path, ficheros: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                recorrer(&p, ficheros);
            } else if p.extension().is_some_and(|x| x == "css") {
                ficheros.push(p);
            }
        }
    }
    let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("style");
    let mut ficheros = Vec::new();
    recorrer(&raiz, &mut ficheros);
    let mut reglas = Vec::new();
    for f in ficheros {
        let mut css = std::fs::read_to_string(&f).unwrap();
        while let Some(i) = css.find("/*") {
            let fin = i + css[i..].find("*/").expect("comentario cerrado") + 2;
            css.replace_range(i..fin, "");
        }
        let mut desde = 0;
        let mut selector = None;
        for (i, c) in css.char_indices() {
            match c {
                '{' => {
                    selector = Some(css[desde..i].trim().to_owned());
                    desde = i + 1;
                }
                '}' => {
                    if let Some(s) = selector.take() {
                        let nombre = f.strip_prefix(&raiz).unwrap().display().to_string();
                        reglas.push((nombre, s, css[desde..i].to_owned()));
                    }
                    desde = i + 1;
                }
                _ => {}
            }
        }
    }
    reglas
}

#[test]
fn sobre_acento_solo_sobre_el_acento() {
    // `--sobre-acento` (casi negro) solo se lee sobre el ámbar relleno: la
    // misma regla tiene que pintar el fondo con `--acento`. Sobre un velo o
    // un fondo transparente va `--acento-texto`.
    let reglas = reglas_css();
    assert!(reglas.len() > 100, "{}", reglas.len());
    let malas: Vec<_> = reglas
        .iter()
        .filter(|(_, _, d)| d.contains("var(--sobre-acento)"))
        .filter(|(_, _, d)| {
            !d.split(';').any(|decl| {
                decl.split_once(':').is_some_and(|(n, v)| {
                    matches!(n.trim(), "background" | "background-color")
                        && v.trim() == "var(--acento)"
                })
            })
        })
        .map(|(f, s, _)| format!("{f}: {s}"))
        .collect();
    assert!(
        malas.is_empty(),
        "--sobre-acento sin fondo --acento: {malas:#?}"
    );
}

/// Declaraciones de la regla (de cualquier `.css` de `style/`) que incluye
/// exactamente el selector `selector` en su lista.
fn regla(reglas: &[(String, String, String)], selector: &str) -> Option<String> {
    reglas
        .iter()
        .find(|(_, s, _)| s.split(',').any(|x| x.trim() == selector))
        .map(|(_, _, d)| d.clone())
}

/// Valor de la propiedad `nombre` en unas declaraciones.
fn propiedad(declaraciones: &str, nombre: &str) -> Option<String> {
    declaraciones.split(';').find_map(|d| {
        let (n, v) = d.split_once(':')?;
        (n.trim() == nombre).then(|| v.trim().to_owned())
    })
}

#[test]
fn interruptor_apagado_se_distingue() {
    // WCAG 1.4.11: el carril de un interruptor apagado (el de la tarjeta,
    // `.int`, y el del detalle, `.interruptor`) se distingue de la
    // superficie en la que está, de 3:1 en adelante, en los dos temas.
    let reglas = reglas_css();
    let mut fallos = Vec::new();
    for selector in [".int", ".interruptor"] {
        let Some(fondo) = regla(&reglas, selector).and_then(|d| propiedad(&d, "background")) else {
            fallos.push(format!("{selector}: sin regla con «background»"));
            continue;
        };
        let Some(token) = fondo
            .strip_prefix("var(--")
            .and_then(|t| t.strip_suffix(')'))
        else {
            fallos.push(format!("{selector}: «{fondo}» no es un token"));
            continue;
        };
        for (tema, bloque) in [("oscuro", OSCURO), ("claro", CLARO)] {
            let c = colores(bloque);
            for sup in ["sup", "sup2"] {
                let r = relacion_wcag(&c[token], &c[sup]);
                if r < 3.0 {
                    fallos.push(format!(
                        "{tema}: {selector} (--{token}) sobre --{sup} da {r:.2}:1"
                    ));
                }
            }
        }
    }
    assert!(fallos.is_empty(), "{fallos:#?}");
}

#[test]
fn interruptor_del_detalle_como_el_de_la_tarjeta() {
    // En solo lectura, el interruptor del detalle (`disabled`) se ve
    // desactivado como el de la tarjeta, y en el móvil tiene la misma zona
    // táctil de 40 px.
    let reglas = reglas_css();
    for (selector, propiedades) in [
        (".interruptor:disabled", &["opacity", "cursor"][..]),
        (".interruptor::before", &["inset"][..]),
    ] {
        let d = regla(&reglas, selector).unwrap_or_else(|| panic!("falta {selector}"));
        for p in propiedades {
            assert!(propiedad(&d, p).is_some(), "{selector} sin {p}: {d}");
        }
    }
    assert_eq!(
        regla(&reglas, ".interruptor:disabled").and_then(|d| propiedad(&d, "cursor")),
        Some("not-allowed".to_owned())
    );
    // Y el estilo de los campos de texto no le pisa el carril.
    for selector in [".campo input", ".campo-ajuste input"] {
        let fondo = regla(&reglas, selector).and_then(|d| propiedad(&d, "background"));
        assert_eq!(fondo, None, "{selector} pinta también el interruptor");
    }
}
