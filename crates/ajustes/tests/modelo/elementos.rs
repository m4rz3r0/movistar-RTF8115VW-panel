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
    let desconocido = ELEMENTOS.replace("valor = \"false\"", "valor = \"false\"\nquitar = true");
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

/// El glifo de un elemento tiene que ser uno de los que dibuja el panel.
#[test]
fn icono_de_un_elemento() {
    let con = |icono: &str| {
        ELEMENTOS.replace(
            "papel = \"principal\"",
            &format!("papel = \"principal\"\nicono = \"{icono}\""),
        )
    };
    assert_eq!(fallos_con(&con("reloj")), Vec::<String>::new());
    falla_con(&fallos_con(&con("cohete")), "red-1", "icono");
}

/// Funciones con `cbSetX` como lista (su instancia, `max_clientes`), para
/// que `wifi.avanzado` pueda ser una lista.
fn funciones_con_lista() -> Catalogo {
    Catalogo::from_toml(&FUNCIONES.replace(
        "riesgo = \"medio\"",
        "riesgo = \"medio\"\nlista = { id_param = \"max_clientes\" }",
    ))
    .unwrap()
}

/// [`WIFI`] con `wifi.avanzado` como lista con `columnas`, y `extra`
/// (acciones) al final: sus fallos.
fn fallos_de_lista(columnas: &str, extra: &str) -> Vec<String> {
    let lista = WIFI.replace(
        "nivel = \"avanzado\"",
        &format!("nivel = \"avanzado\"\nlista = true\ncolumnas = {columnas}"),
    );
    comprobar(
        &cargar(&[("wifi", &format!("{lista}\n{extra}"))], ""),
        &funciones_con_lista(),
        &mascara(),
    )
}

/// Las columnas de una lista son campos suyos (o un rango `<x>_desde` y
/// `<x>_hasta`) y piden su cabecera en los dos idiomas; un ajuste que no es
/// lista no las lleva.
#[test]
fn columnas_de_una_lista() {
    assert_eq!(fallos_de_lista("[\"maximo\"]", ""), Vec::<String>::new());
    falla_con(
        &fallos_de_lista("[\"maximo\", \"nada\"]", ""),
        "wifi.avanzado",
        "la columna nada no existe",
    );
    falla_con(
        &fallos_de_lista("[\"maximo\", \"maximo\"]", ""),
        "wifi.avanzado",
        "columna maximo repetida",
    );
    let suelta = WIFI.replace(
        "nivel = \"basico\"",
        "nivel = \"basico\"\ncolumnas = [\"nombre\"]",
    );
    falla_con(&fallos(&suelta, ""), "wifi.red", "solo una lista");
    let c = cargar(
        &[(
            "wifi",
            &WIFI.replace(
                "nivel = \"avanzado\"",
                "nivel = \"avanzado\"\nlista = true\ncolumnas = [\"maximo\"]",
            ),
        )],
        "",
    );
    let a = c.ajuste("wifi.avanzado").unwrap();
    assert_eq!(a.columnas, ["maximo"]);
    assert!(claves_requeridas(a).contains(&"ajuste-wifi-avanzado-col-maximo".to_owned()));
}

/// Un rango: `externo` vale si están `externo_desde` y `externo_hasta`.
#[test]
fn columna_de_un_rango() {
    let rango = WIFI
        .replace(
            "nivel = \"avanzado\"",
            "nivel = \"avanzado\"\nlista = true\ncolumnas = [\"puerta\"]",
        )
        .replace("id = \"maximo\"", "id = \"puerta_desde\"")
        + "\n[[ajuste.campo]]\nid = \"puerta_hasta\"\nescribe = { funcion = \"cbSetX\", \
           parametro = \"puerta\" }\nlee = \"puerta\"\ntipo = \"ip\"\n";
    let f = comprobar(
        &cargar(&[("wifi", &rango)], ""),
        &funciones_con_lista(),
        &mascara(),
    );
    assert!(
        !f.iter().any(|f| f.contains("columna")),
        "un rango es una columna: {f:#?}"
    );
}

/// Acciones de una lista: activar y desactivar escriben un campo de cada
/// instancia; borrar no lleva campo ni valor, y siempre con el botón de
/// peligro.
#[test]
fn acciones_de_una_lista() {
    let borrar = "[[accion]]\nid = \"borrar_x\"\ntema = \"wifi\"\nlista = \"wifi.avanzado\"\n\
                  borrar = true\npeligro = true\n";
    let poner = "[[accion]]\nid = \"poner_x\"\ntema = \"wifi\"\nlista = \"wifi.avanzado\"\n\
                 campo = \"maximo\"\nvalor = \"8\"\n";
    let cols = "[\"maximo\"]";
    assert_eq!(
        fallos_de_lista(cols, &format!("{borrar}\n{poner}")),
        Vec::<String>::new()
    );
    let sin_peligro = borrar.replace("peligro = true\n", "");
    falla_con(
        &fallos_de_lista(cols, &sin_peligro),
        "borrar_x",
        "botón de peligro",
    );
    let con_campo = format!("{borrar}campo = \"maximo\"\n");
    falla_con(
        &fallos_de_lista(cols, &con_campo),
        "borrar_x",
        "sin `campo` ni `valor`",
    );
    let sin_lista = borrar.replace("lista = \"wifi.avanzado\"\n", "");
    falla_con(&fallos_de_lista(cols, &sin_lista), "borrar_x", "`lista`");
    let no_es_lista = borrar.replace("wifi.avanzado", "wifi.red");
    falla_con(
        &fallos_de_lista(cols, &no_es_lista),
        "borrar_x",
        "wifi.red no es una lista de su tema",
    );
    let campo_ajeno = poner.replace("campo = \"maximo\"", "campo = \"nombre\"");
    falla_con(
        &fallos_de_lista(cols, &campo_ajeno),
        "poner_x",
        "campo nombre",
    );
}
