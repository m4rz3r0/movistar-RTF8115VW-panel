//! Lotes de mejoras: carga de `lotes.toml`, sus reglas en `comprobar` y la
//! expansión de cada paso en lo que cambia.

use super::*;
use ajustes::{Destino, PasoExpandido, claves_de_lotes, lotes_del_campo, pasos_del_lote};

/// Una red WiFi principal con los dos ajustes de [`WIFI`] y una acción.
const RED: &str = r#"
[[elemento]]
id = "red-1"
tema = "wifi"
ajustes = ["wifi.red", "wifi.avanzado"]
papel = "principal"

[[accion]]
id = "ocultar_nombre"
tema = "wifi"
campo = "visible"
valor = "false"
"#;

/// Lotes válidos contra [`WIFI`] y [`RED`]: una acción por red, un campo con
/// condición, un campo por red opcional y una elección.
const LOTES: &str = r#"
[[paso]]
respaldo = "criterio"
id = "ocultar"
tema = "wifi"
accion = "ocultar_nombre"
para_cada = "red_wifi"

[[paso]]
respaldo = "criterio"
id = "maximo"
tema = "wifi"
campo = "wifi.avanzado.maximo"
valor = "8"
si = { campo = "wifi.red.visible", valor = "true" }

[[paso]]
respaldo = "criterio"
id = "seguro"
tema = "wifi"
campo = "seguridad"
valor = "wpa2"
para_cada = "red_wifi"
opcional = true

[[paso]]
respaldo = "criterio"
id = "elegido"
tema = "wifi"
ajuste = "wifi.avanzado"
elegir = "tope"

[[eleccion]]
id = "tope"

[[eleccion.opcion]]
id = "poco"
defecto = true
respaldo = "criterio"
valores = { maximo = "4" }

[[eleccion.opcion]]
id = "mucho"
valores = { maximo = "32" }

[[lote]]
id = "prueba"
icono = "escudo"
pasos = ["ocultar", "maximo", "seguro", "elegido"]
avisos = ["uno"]
consejos = ["dos"]
"#;

fn con(lotes: &str) -> CatalogoAjustes {
    cargar(&[("wifi", &format!("{WIFI}\n{RED}"))], "")
        .con_lotes(lotes)
        .expect("lotes sintéticos legibles")
}

fn fallos_de(lotes: &str) -> Vec<String> {
    comprobar(&con(lotes), &funciones(), &mascara())
}

/// `LOTES` con `de` cambiado por `a` (que tiene que estar).
fn cambiado(de: &str, a: &str) -> String {
    assert!(LOTES.contains(de), "{de}");
    LOTES.replacen(de, a, 1)
}

#[test]
fn lotes_validos() {
    assert_eq!(fallos_de(LOTES), Vec::<String>::new());
    let c = con(LOTES);
    let l = c.lote("prueba").unwrap();
    assert_eq!(l.icono, "escudo");
    assert!(c.paso("seguro").unwrap().opcional);
    assert_eq!(c.eleccion("tope").unwrap().opciones.len(), 2);
    assert_eq!(c.eleccion("tope").unwrap().defecto().id, "poco");
    let pasos = pasos_del_lote(&c, l);
    let claves: Vec<String> = pasos.iter().map(PasoExpandido::clave).collect();
    assert_eq!(
        claves,
        ["ocultar@red-1", "maximo", "seguro@red-1", "elegido"]
    );
    let destino = |ajuste: &str, campo: &str, valor: Option<&str>| Destino {
        ajuste: ajuste.into(),
        campo: campo.into(),
        valor: valor.map(Into::into),
    };
    assert_eq!(
        pasos[0].destinos,
        [destino("wifi.red", "visible", Some("false"))]
    );
    assert_eq!(pasos[1].si.as_ref().unwrap().campo, "wifi.red.visible");
    assert_eq!(
        pasos[2].destinos,
        [destino("wifi.red", "seguridad", Some("wpa2"))]
    );
    assert!(pasos[2].opcional);
    assert_eq!(
        pasos[3].destinos,
        [destino("wifi.avanzado", "maximo", None)]
    );
    assert_eq!(pasos[3].elegir.as_deref(), Some("tope"));
    assert_eq!(lotes_del_campo(&c, "wifi.avanzado", "maximo"), ["prueba"]);
    assert!(lotes_del_campo(&c, "wifi.red", "nombre").is_empty());
    assert_eq!(
        claves_de_lotes(&c),
        [
            "mejoras-aviso-uno",
            "mejoras-consejo-dos",
            "mejoras-lote-prueba",
            "mejoras-lote-prueba-desc",
            "mejoras-lote-prueba-titulo",
            "mejoras-paso-elegido",
            "mejoras-paso-elegido-desc",
            "mejoras-paso-maximo",
            "mejoras-paso-maximo-desc",
            "mejoras-paso-ocultar",
            "mejoras-paso-ocultar-desc",
            "mejoras-paso-seguro",
            "mejoras-paso-seguro-desc",
        ]
    );
}

#[test]
fn catalogo_sin_lotes_queda_sin_lotes() {
    let c = cargar(&[("wifi", WIFI)], "");
    assert!(c.lotes().is_empty() && c.paso("ocultar").is_none());
    // Un fichero de lotes con claves desconocidas no carga.
    let roto = cargar(&[("wifi", WIFI)], "").con_lotes("[[lote]]\nid = \"x\"\nraro = 1\n");
    assert!(roto.is_err());
}

#[test]
fn paso_sin_destino_o_con_dos_es_error() {
    let sin = cambiado("accion = \"ocultar_nombre\"\n", "");
    falla_con(&fallos_de(&sin), "paso ocultar", "solo uno de");
    let dos = cambiado(
        "accion = \"ocultar_nombre\"\n",
        "accion = \"ocultar_nombre\"\ncampo = \"visible\"\n",
    );
    falla_con(&fallos_de(&dos), "paso ocultar", "solo uno de");
}

#[test]
fn paso_con_accion_que_no_existe_es_error() {
    let mal = cambiado("accion = \"ocultar_nombre\"", "accion = \"nada\"");
    falla_con(&fallos_de(&mal), "paso ocultar", "la acción nada no existe");
}

#[test]
fn paso_con_valor_no_valido_es_error() {
    let mal = cambiado(
        "campo = \"wifi.avanzado.maximo\"\nvalor = \"8\"",
        "campo = \"wifi.red.visible\"\nvalor = \"quizas\"",
    );
    falla_con(
        &fallos_de(&mal),
        "paso maximo",
        "«quizas» no es un valor válido de wifi.red.visible",
    );
}

#[test]
fn paso_sobre_campo_no_escribible_o_secreto_es_error() {
    let solo_lectura = cambiado(
        "campo = \"wifi.avanzado.maximo\"\nvalor = \"8\"",
        "campo = \"wifi.red.clave_basica\"\nvalor = \"clave-de-prueba-1\"",
    );
    falla_con(
        &fallos_de(&solo_lectura),
        "paso maximo",
        "wifi.red.clave_basica no se puede escribir",
    );
    let secreto = cambiado(
        "campo = \"wifi.avanzado.maximo\"\nvalor = \"8\"",
        "campo = \"wifi.red.clave\"\nvalor = \"clave-de-prueba-1\"",
    );
    falla_con(
        &fallos_de(&secreto),
        "paso maximo",
        "wifi.red.clave es un secreto",
    );
}

#[test]
fn si_con_campo_que_no_existe_es_error() {
    let mal = cambiado("campo = \"wifi.red.visible\"", "campo = \"wifi.red.nada\"");
    falla_con(&fallos_de(&mal), "paso maximo", "wifi.red.nada no existe");
}

#[test]
fn para_cada_fuera_de_wifi_es_error() {
    let mal = cambiado(
        "id = \"seguro\"\ntema = \"wifi\"",
        "id = \"seguro\"\ntema = \"puertos\"",
    );
    falla_con(&fallos_de(&mal), "paso seguro", "para_cada");
}

#[test]
fn eleccion_con_dos_por_defecto_es_error() {
    let mal = cambiado("id = \"mucho\"\n", "id = \"mucho\"\ndefecto = true\n");
    falla_con(
        &fallos_de(&mal),
        "elección tope",
        "más de una opción por defecto",
    );
}

/// Si una opción no diera valor a un campo que dan las otras, al aplicarla
/// ese campo se quedaría sin valor.
#[test]
fn eleccion_con_opciones_de_campos_distintos_es_error() {
    let mal = cambiado(
        "valores = { maximo = \"32\" }",
        "valores = { maximo = \"32\", otro = \"1\" }",
    );
    falla_con(
        &fallos_de(&mal),
        "elección tope",
        "la opción mucho no da valor a los mismos campos que poco",
    );
}

#[test]
fn eleccion_con_valor_no_valido_es_error() {
    let mal = cambiado("maximo = \"32\"", "maximo = \"99\"");
    falla_con(
        &fallos_de(&mal),
        "paso elegido",
        "«99» no es un valor válido de wifi.avanzado.maximo",
    );
}

#[test]
fn lote_con_paso_inexistente_o_repetido_es_error() {
    let inexistente = cambiado("\"elegido\"]", "\"elegido\", \"nada\"]");
    falla_con(
        &fallos_de(&inexistente),
        "lote prueba",
        "el paso nada no existe",
    );
    let repetido = cambiado("\"elegido\"]", "\"elegido\", \"maximo\"]");
    falla_con(
        &fallos_de(&repetido),
        "lote prueba",
        "el paso maximo está repetido",
    );
}

#[test]
fn icono_de_lote_que_no_existe_es_error() {
    let mal = cambiado("icono = \"escudo\"", "icono = \"cohete\"");
    falla_con(&fallos_de(&mal), "lote prueba", "el icono cohete no existe");
}
