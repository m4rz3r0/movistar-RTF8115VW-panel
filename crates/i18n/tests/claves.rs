//! Coherencia de los mensajes: los dos idiomas tienen las mismas claves, las
//! que usa el panel existen y las vistas no llevan texto suelto.

use fluent_syntax::{ast, parser};
use regex::Regex;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Claves de un `.ftl` y las que no tienen valor. Falla si no se puede leer
/// o tiene errores de sintaxis.
fn claves_de(idioma: &str, fichero: &str) -> (BTreeSet<String>, Vec<String>) {
    let ruta = raiz().join(format!("locales/{idioma}/{fichero}.ftl"));
    let fuente = fs::read_to_string(&ruta).unwrap_or_else(|e| panic!("{}: {e}", ruta.display()));
    let recurso = parser::parse(fuente.as_str())
        .unwrap_or_else(|(_, errores)| panic!("{}: {errores:?}", ruta.display()));
    let mut claves = BTreeSet::new();
    let mut sin_valor = Vec::new();
    for entrada in recurso.body {
        if let ast::Entry::Message(m) = entrada {
            if m.value.is_none() {
                sin_valor.push(m.id.name.to_owned());
            }
            assert!(
                claves.insert(m.id.name.to_owned()),
                "{}: clave repetida {}",
                ruta.display(),
                m.id.name
            );
        }
    }
    (claves, sin_valor)
}

/// Todas las claves en español.
fn claves_es() -> BTreeSet<String> {
    let mut todas = BTreeSet::new();
    for f in i18n::FICHEROS {
        for k in claves_de("es", f).0 {
            assert!(todas.insert(k.clone()), "clave {k} en dos ficheros");
        }
    }
    todas
}

/// Ficheros `.rs` bajo `dir`, recursivamente.
fn fuentes_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            fuentes_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Fuentes del panel sin las líneas de comentario (los ejemplos de la
/// documentación no son usos).
fn fuentes_panel() -> Vec<(PathBuf, String)> {
    let mut rutas = Vec::new();
    fuentes_rs(&raiz().join("../panel/src"), &mut rutas);
    assert!(!rutas.is_empty());
    rutas
        .into_iter()
        .map(|p| {
            let s = fs::read_to_string(&p)
                .unwrap()
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            (p, s)
        })
        .collect()
}

#[test]
fn claves_iguales_en_es_y_en() {
    // Los ficheros de cada idioma son los registrados.
    for idioma in ["es", "en"] {
        let mut en_disco: Vec<String> = fs::read_dir(raiz().join("locales").join(idioma))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "ftl"))
            .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
            .collect();
        en_disco.sort();
        let mut registrados: Vec<String> = i18n::FICHEROS.iter().map(|s| s.to_string()).collect();
        registrados.sort();
        assert_eq!(en_disco, registrados, "ficheros de locales/{idioma}");
    }
    for f in i18n::FICHEROS {
        let (es, es_sin) = claves_de("es", f);
        let (en, en_sin) = claves_de("en", f);
        let solo_es: Vec<_> = es.difference(&en).collect();
        let solo_en: Vec<_> = en.difference(&es).collect();
        assert!(
            solo_es.is_empty() && solo_en.is_empty(),
            "{f}.ftl: solo en es {solo_es:?}; solo en en {solo_en:?}"
        );
        assert!(
            es_sin.is_empty() && en_sin.is_empty(),
            "{f}.ftl: mensajes sin valor {es_sin:?} {en_sin:?}"
        );
    }
}

#[test]
fn claves_usadas_existen() {
    let claves = claves_es();
    let t = Regex::new(r#"\bt!\(\s*"([^"]+)""#).unwrap();
    let tr = Regex::new(r#"\btr\([^,()]*,\s*"([^"]+)""#).unwrap();
    let mut usadas = 0;
    let mut faltan = Vec::new();
    for (ruta, fuente) in fuentes_panel() {
        for c in t.captures_iter(&fuente).chain(tr.captures_iter(&fuente)) {
            usadas += 1;
            if !claves.contains(&c[1]) {
                faltan.push(format!("{}: {}", ruta.display(), &c[1]));
            }
        }
    }
    assert!(usadas > 0, "el panel no usa ninguna clave");
    assert!(faltan.is_empty(), "claves que no existen: {faltan:#?}");
}

/// `fuente` sin los elementos marcados `#[cfg(test)]` (de la marca a la
/// llave que cierra el elemento): los datos de prueba no son la interfaz.
fn sin_pruebas(fuente: &str) -> String {
    let mut out = String::new();
    let mut resto = fuente;
    while let Some(i) = resto.find("#[cfg(test)]") {
        out.push_str(&resto[..i]);
        let tras = &resto[i..];
        let Some(abre) = tras.find('{') else {
            return out;
        };
        let (mut nivel, mut en_cadena, mut escape, mut fin) = (0usize, false, false, tras.len());
        for (j, c) in tras[abre..].char_indices() {
            match c {
                _ if escape => escape = false,
                '\\' if en_cadena => escape = true,
                '"' => en_cadena = !en_cadena,
                '{' if !en_cadena => nivel += 1,
                '}' if !en_cadena => {
                    nivel -= 1;
                    if nivel == 0 {
                        fin = abre + j + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        resto = &tras[fin..];
    }
    out.push_str(resto);
    out
}

#[test]
fn las_pruebas_no_cuentan_como_vistas() {
    let fuente = "fn a() { view! { <p>{t!(\"x\")}</p> } }\n#[cfg(test)]\nmod tests {\n    fn b() { view! { <p>\"Suelto\"</p> } }\n}\nfn c() {}\n";
    let limpia = sin_pruebas(fuente);
    assert!(!limpia.contains("Suelto"), "{limpia}");
    assert!(
        limpia.contains("fn a()") && limpia.contains("fn c()"),
        "{limpia}"
    );
}

/// Cuerpos de los `view! { … }` de `fuente`.
fn vistas(fuente: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut resto = fuente;
    while let Some(i) = resto.find("view! {") {
        let cuerpo = &resto[i + "view! {".len()..];
        let (mut nivel, mut en_cadena, mut escape, mut fin) = (1usize, false, false, cuerpo.len());
        for (j, c) in cuerpo.char_indices() {
            match c {
                _ if escape => escape = false,
                '\\' if en_cadena => escape = true,
                '"' => en_cadena = !en_cadena,
                '{' if !en_cadena => nivel += 1,
                '}' if !en_cadena => {
                    nivel -= 1;
                    if nivel == 0 {
                        fin = j;
                        break;
                    }
                }
                _ => {}
            }
        }
        out.push(cuerpo[..fin].to_owned());
        resto = &cuerpo[fin..];
    }
    out
}

/// Textos que pueden ir sueltos en una vista: símbolos, unidades, siglas y
/// nombres propios (uno por línea en `tests/permitidos.txt`, `#` comenta).
fn permitidos() -> BTreeSet<String> {
    fs::read_to_string(raiz().join("tests/permitidos.txt"))
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

#[test]
fn sin_texto_literal_en_vistas() {
    // Heurística: un literal es texto de la vista si va justo tras el cierre
    // de una etiqueta (`>"…"`) o justo antes de una (`"…"<`), o si es el
    // valor de un atributo o prop que se lee en pantalla. Con letras y fuera
    // de la lista de permitidos, es un texto sin traducir.
    let texto = Regex::new(r#"(?:>\s*"([^"]*)")|(?:"([^"]*)"\s*<)"#).unwrap();
    let atributo = Regex::new(
        r#"\b(?:aria-label|placeholder|title|alt|titulo|nota|vacia|texto|sub|valor)="([^"]*)""#,
    )
    .unwrap();
    let letras = Regex::new(r"[A-Za-zÁÉÍÓÚÜÑáéíóúüñ]").unwrap();
    let permitidos = permitidos();
    let mut sueltos = Vec::new();
    for (ruta, fuente) in fuentes_panel() {
        for v in vistas(&sin_pruebas(&fuente)) {
            let literales = texto
                .captures_iter(&v)
                .filter_map(|c| c.get(1).or(c.get(2)))
                .chain(atributo.captures_iter(&v).filter_map(|c| c.get(1)));
            for l in literales {
                let l = l.as_str().trim();
                if letras.is_match(l) && !permitidos.contains(l) {
                    sueltos.push(format!("{}: «{l}»", ruta.display()));
                }
            }
        }
    }
    assert!(
        sueltos.is_empty(),
        "texto sin traducir en vistas: {sueltos:#?}"
    );
}

/// Sin sugerencias de DNS públicos: los proveedores se elegirán en la fase
/// 3/4 con fuentes citadas y comprobando que admiten DNS normal (algunos solo
/// funcionan cifrados, y el router reparte DNS normal por DHCP).
#[test]
fn sin_proveedores_de_dns_en_textos_ni_vistas() {
    let proveedores = Regex::new(r"(?i)quad9|mullvad|cloudflare").unwrap();
    let mut hallados = Vec::new();
    for idioma in ["es", "en"] {
        for f in i18n::FICHEROS {
            let ruta = raiz().join("locales").join(idioma).join(format!("{f}.ftl"));
            for (n, l) in fs::read_to_string(&ruta).unwrap().lines().enumerate() {
                if proveedores.is_match(l) {
                    hallados.push(format!("{}:{}", ruta.display(), n + 1));
                }
            }
        }
    }
    for (ruta, fuente) in fuentes_panel() {
        if proveedores.is_match(&fuente) {
            hallados.push(ruta.display().to_string());
        }
    }
    assert!(hallados.is_empty(), "proveedores de DNS: {hallados:#?}");
}
