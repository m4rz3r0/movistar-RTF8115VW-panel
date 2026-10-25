//! Fuentes de las recomendaciones: formato de `fuentes.toml`, referencias y
//! que cada recomendación diga de dónde sale (o que es criterio del panel).

use super::*;
use ajustes::ClaseFuente;

/// Dos fuentes válidas: una guía y un foro.
const FUENTES: &str = r#"
[[fuente]]
id = "guia-1"
titulo = "Guía de prueba"
organismo = "Organismo de prueba"
url = "https://guia.example/wifi"
consultada = "2026-10-22"
clase = "guia"
verificada_con = ["foro-1"]

[[fuente]]
id = "foro-1"
titulo = "Hilo de prueba"
organismo = "Foro de prueba"
url = "https://foro.example/hilo"
consultada = "2026-10-22"
clase = "foro"
"#;

/// Un ajuste con un booleano recomendado y una opción recomendada; `extra`
/// va en el booleano (sus fuentes o su respaldo) y `opcion` en la opción.
fn wifi(extra: &str, opcion: &str) -> String {
    format!(
        r#"
[[ajuste]]
id = "wifi.red"
tema = "wifi"
nivel = "basico"
leer = {{ funcion = "cbGetX", params = {{ x_id = "1" }} }}
fijos = {{ x_id = "1" }}

[[ajuste.campo]]
id = "visible"
escribe = {{ funcion = "cbSetX", parametro = "oculta" }}
lee = "oculta"
tipo = "booleano"
invertido = true
recomendado = "true"
{extra}

[[ajuste.campo]]
id = "seguridad"
escribe = {{ funcion = "cbSetX", parametro = "seguridad" }}
lee = "seguridad"
tipo = "opcion"
opciones = [
  {{ valor = "3", clave = "wpa2", recomendada = true{opcion} }},
  {{ valor = "0", clave = "abierta" }},
]
"#
    )
}

/// Fallos de `comprobar` con `wifi.toml` y las fuentes dadas.
fn fallos_con(wifi: &str, fuentes: &str) -> Vec<String> {
    let c = cargar(&[("wifi", wifi)], "")
        .con_fuentes(fuentes)
        .expect("fuentes legibles");
    comprobar(&c, &funciones(), &mascara())
}

/// Lo que basta para que la opción recomendada esté bien.
const OPCION_BIEN: &str = r#", fuentes = ["guia-1"], respaldo = "consenso""#;

#[test]
fn fuentes_validas() {
    let w = wifi(r#"fuentes = ["guia-1", "foro-1"]"#, OPCION_BIEN);
    assert_eq!(fallos_con(&w, FUENTES), Vec::<String>::new());
    let c = cargar(&[("wifi", &w)], "").con_fuentes(FUENTES).unwrap();
    assert_eq!(c.fuentes().len(), 2);
    let g = c.fuente("guia-1").expect("guia-1");
    assert_eq!(g.clase, ClaseFuente::Guia);
    assert_eq!(g.verificada_con, ["foro-1"]);
    assert_eq!(c.fuente("foro-1").unwrap().clase, ClaseFuente::Foro);
    assert!(c.fuente("otra").is_none());
    // Criterio del panel, sin fuentes, también vale.
    let w = wifi(r#"respaldo = "criterio""#, r#", respaldo = "criterio""#);
    assert_eq!(fallos_con(&w, FUENTES), Vec::<String>::new());
}

#[test]
fn fuente_con_url_http_es_error() {
    let w = wifi(r#"fuentes = ["guia-1"]"#, OPCION_BIEN);
    let f = fallos_con(&w, &FUENTES.replace("https://guia", "http://guia"));
    falla_con(&f, "guia-1", "https://");
    let f = fallos_con(
        &w,
        &FUENTES.replace(
            "2026-10-22\"\nclase = \"guia",
            "22/10/2026\"\nclase = \"guia",
        ),
    );
    falla_con(&f, "guia-1", "AAAA-MM-DD");
}

#[test]
fn fuente_repetida_es_error() {
    let w = wifi(r#"fuentes = ["guia-1"]"#, OPCION_BIEN);
    let f = fallos_con(&w, &FUENTES.replace("id = \"foro-1\"", "id = \"guia-1\""));
    falla_con(&f, "guia-1", "repetid");
}

#[test]
fn referencia_a_fuente_inexistente_es_error() {
    let w = wifi(r#"fuentes = ["no-existe"]"#, OPCION_BIEN);
    falla_con(&fallos_con(&w, FUENTES), "no-existe", "no existe");
    let w = wifi(r#"fuentes = ["guia-1"]"#, r#", fuentes = ["otra"]"#);
    falla_con(&fallos_con(&w, FUENTES), "otra", "no existe");
    // `verificada_con` también se comprueba.
    let w = wifi(r#"fuentes = ["guia-1"]"#, OPCION_BIEN);
    let f = fallos_con(&w, &FUENTES.replace("[\"foro-1\"]", "[\"foro-9\"]"));
    falla_con(&f, "foro-9", "no existe");
}

#[test]
fn recomendado_sin_fuente_ni_criterio_es_error() {
    // El booleano recomendado, sin nada.
    let f = fallos_con(&wifi("", OPCION_BIEN), FUENTES);
    falla_con(&f, "wifi.red.visible", "fuentes");
    // Consenso sin fuentes no basta.
    let f = fallos_con(&wifi(r#"respaldo = "consenso""#, OPCION_BIEN), FUENTES);
    falla_con(&f, "wifi.red.visible", "fuentes");
    // La opción recomendada, sin nada.
    let f = fallos_con(&wifi(r#"fuentes = ["guia-1"]"#, ""), FUENTES);
    falla_con(&f, "wifi.red.seguridad", "fuentes");
}

#[test]
fn respaldo_foro_sin_fuente_de_foro_es_error() {
    let w = wifi(
        r#"fuentes = ["guia-1"]
respaldo = "foro""#,
        OPCION_BIEN,
    );
    falla_con(&fallos_con(&w, FUENTES), "wifi.red.visible", "foro");
    let w = wifi(
        r#"fuentes = ["foro-1"]
respaldo = "foro""#,
        OPCION_BIEN,
    );
    assert_eq!(fallos_con(&w, FUENTES), Vec::<String>::new());
}
