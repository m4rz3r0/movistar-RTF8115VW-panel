//! Elementos y acciones de una pestaña: carga y reglas de `comprobar`.

use super::*;

/// Un elemento con los dos ajustes de [`WIFI`] y una acción válidos.
const ELEMENTOS: &str = r#"
[[elemento]]
id = "red-1"
tema = "wifi"
ajustes = ["wifi.red", "wifi.avanzado"]
papel = "principal"
interruptor = "wifi.red.visible"
datos = ["wifi.red.seguridad", "wifi.red.clave", "derivado:conectados", "wifi.avanzado.maximo"]
estado = "derivado:estado_red"

[[accion]]
id = "ocultar_nombre"
tema = "wifi"
campo = "visible"
valor = "false"
"#;

/// Fallos de [`WIFI`] con `extra` (elementos y acciones) al final.
fn fallos_con(extra: &str) -> Vec<String> {
    fallos(&format!("{WIFI}\n{extra}"), "")
}

#[test]
fn elementos_y_acciones_validos() {
    assert_eq!(fallos_con(ELEMENTOS), Vec::<String>::new());
    let c = cargar(&[("wifi", &format!("{WIFI}\n{ELEMENTOS}"))], "");
    let e = &c.elementos()[0];
    assert_eq!(e.ajustes, ["wifi.red", "wifi.avanzado"]);
    assert_eq!(e.interruptor.as_deref(), Some("wifi.red.visible"));
    assert_eq!(c.elementos_de(Tema::Wifi).count(), 1);
    assert_eq!(c.elementos_de(Tema::Puertos).count(), 0);
    let a = &c.acciones()[0];
    assert_eq!(
        (a.campo.as_deref(), a.valor.as_deref(), a.peligro),
        (Some("visible"), Some("false"), false)
    );
    assert_eq!(
        claves_de_elementos(&c),
        [
            "ajuste-accion-ocultar_nombre-titulo",
            "ajustes-accion-ocultar_nombre",
            "ajustes-papel-principal"
        ]
    );
}

#[test]
fn elemento_con_ajuste_repetido_es_error() {
    let otro = r#"
[[elemento]]
id = "red-2"
tema = "wifi"
ajustes = ["wifi.red"]
"#;
    let f = fallos_con(&format!("{ELEMENTOS}\n{otro}"));
    falla_con(&f, "red-2", "wifi.red ya está en el elemento red-1");
    // Ni dos veces en el mismo.
    let doble = ELEMENTOS.replace(
        "ajustes = [\"wifi.red\", \"wifi.avanzado\"]",
        "ajustes = [\"wifi.red\", \"wifi.red\", \"wifi.avanzado\"]",
    );
    falla_con(&fallos_con(&doble), "red-1", "ya está en el elemento red-1");
}

#[test]
fn interruptor_debe_ser_booleano_escribible() {
    let texto = ELEMENTOS.replace("wifi.red.visible\"", "wifi.red.nombre\"");
    falla_con(
        &fallos_con(&texto),
        "red-1",
        "booleano que se pueda escribir",
    );
    // Un sí/no que no se puede escribir, tampoco.
    let solo_lectura = WIFI.replace(
        "escribe = { funcion = \"cbSetX\", parametro = \"oculta\" }\n",
        "solo_lectura = \"ajustes-solo-lectura-basica\"\n",
    );
    let f = fallos(&format!("{solo_lectura}\n{ELEMENTOS}"), "");
    falla_con(&f, "red-1", "booleano que se pueda escribir");
}

#[test]
fn como_mucho_seis_datos() {
    let siete = ELEMENTOS.replace(
        "\"wifi.avanzado.maximo\"]",
        "\"wifi.avanzado.maximo\", \"wifi.red.nombre\", \"wifi.red.visible\", \
         \"derivado:canal_y_ancho\"]",
    );
    falla_con(&fallos_con(&siete), "red-1", "como mucho 6");
}

#[test]
fn referencias_que_no_existen_son_error() {
    let casos = [
        (
            "\"wifi.avanzado\"]",
            "\"wifi.avanzado\", \"wifi.nada\"]",
            "wifi.nada no existe",
        ),
        (
            "wifi.red.visible\"",
            "wifi.red.nada\"",
            "el campo nada no existe",
        ),
        (
            "\"wifi.red.clave\"",
            "\"wifi.red.nada\"",
            "el campo nada no existe",
        ),
        (
            "derivado:conectados",
            "derivado:nada",
            "el dato derivado nada no existe",
        ),
        (
            "derivado:estado_red",
            "derivado:nada",
            "la regla de estado nada no existe",
        ),
        (
            "derivado:estado_red",
            "wifi.red.visible",
            "derivado:<nombre>",
        ),
    ];
    for (de, a, texto) in casos {
        falla_con(&fallos_con(&ELEMENTOS.replace(de, a)), "red-1", texto);
    }
    // Un campo de un ajuste que no es del elemento.
    let ajeno = ELEMENTOS.replace(
        "ajustes = [\"wifi.red\", \"wifi.avanzado\"]",
        "ajustes = [\"wifi.red\"]",
    );
    falla_con(
        &fallos_con(&ajeno),
        "red-1",
        "wifi.avanzado no es del elemento",
    );
    // Un ajuste de otro tema.
    let otro_tema = ELEMENTOS.replace("tema = \"wifi\"\najustes", "tema = \"puertos\"\najustes");
    falla_con(&fallos_con(&otro_tema), "red-1", "de otro tema");
    // Una acción sobre un campo que no tiene ningún elemento.
    let accion = ELEMENTOS.replace("campo = \"visible\"", "campo = \"nada\"");
    falla_con(&fallos_con(&accion), "ocultar_nombre", "campo nada");
}

#[test]
fn accion_con_campo_valor_validos() {
    let sin_valor = ELEMENTOS.replace("valor = \"false\"\n", "");
    falla_con(
        &fallos_con(&sin_valor),
        "ocultar_nombre",
        "`campo` y `valor`",
    );
    let mal_valor = ELEMENTOS.replace("valor = \"false\"", "valor = \"quizá\"");
    falla_con(
        &fallos_con(&mal_valor),
        "ocultar_nombre",
        "no es un valor válido",
    );
    let secreto = ELEMENTOS.replace("campo = \"visible\"", "campo = \"clave\"");
    falla_con(&fallos_con(&secreto), "ocultar_nombre", "es un secreto");
    let repetida = format!(
        "{ELEMENTOS}\n[[accion]]\nid = \"ocultar_nombre\"\ntema = \"wifi\"\n\
         campo = \"visible\"\nvalor = \"true\"\n"
    );
    falla_con(&fallos_con(&repetida), "ocultar_nombre", "id repetido");
    // Un campo desconocido en una acción no se carga.
    let desconocido = ELEMENTOS.replace("valor = \"false\"", "valor = \"false\"\nborrar = true");
    assert!(
        CatalogoAjustes::from_toml(&[("wifi", &format!("{WIFI}\n{desconocido}"))], "").is_err()
    );
}

#[test]
fn id_de_elemento_para_la_url() {
    let mal = ELEMENTOS.replace("id = \"red-1\"", "id = \"Red 1\"");
    falla_con(&fallos_con(&mal), "Red 1", "minúsculas, cifras y -");
}

#[test]
fn seguridad_solo_con_recomendacion() {
    let sin = WIFI.replace(
        "lee = \"nombre\"\ntipo = \"texto\"",
        "lee = \"nombre\"\ntipo = \"texto\"\nseguridad = true",
    );
    falla_con(
        &fallos(&sin, ""),
        "wifi.red.nombre",
        "`seguridad` sin recomendación",
    );
    let con = WIFI.replace(
        "invertido = true\nrecomendado = \"true\"",
        "invertido = true\nrecomendado = \"true\"\nseguridad = true",
    );
    assert_eq!(fallos(&con, ""), Vec::<String>::new());
}
