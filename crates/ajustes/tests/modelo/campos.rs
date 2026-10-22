//! Campos: opciones de varios parámetros, solo lectura, formas y dependencias.

use super::*;

/// `wifi.seguridad`: cada opción escribe `seguridad` y `cifrado`.
const SEGURIDAD: &str = r#"
[[ajuste]]
id = "wifi.seguridad"
tema = "wifi"
nivel = "basico"
leer = { funcion = "cbGetX" }

[[ajuste.campo]]
id = "seguridad"
escribe = { funcion = "cbSetX", parametro = "seguridad" }
lee = "seguridad"
tipo = "opcion"
ademas = [{ parametro = "cifrado", lee = "cifrado" }]
opciones = [
  { valor = "11", clave = "wpa2_wpa3", recomendada = true, ademas = { cifrado = "4" } },
  { valor = "3", clave = "wpa2", ademas = { cifrado = "4" } },
  { valor = "3", clave = "wpa2_tkip", aviso = "inseguro", ademas = { cifrado = "2" } },
  { valor = "0", clave = "abierta", aviso = "inseguro", ademas = { cifrado = "0" } },
]
"#;

#[test]
fn opcion_que_escribe_varios_parametros() {
    let f = fallos(SEGURIDAD, "");
    assert!(f.is_empty(), "{f:#?}");
    let a = cargar(&[("wifi", SEGURIDAD)], "");
    let campo = &a.ajuste("wifi.seguridad").unwrap().campos[0];
    assert_eq!(campo.ademas[0].parametro, "cifrado");
    assert_eq!(
        campo.opciones[2].ademas.get("cifrado").map(String::as_str),
        Some("2")
    );

    // Regla 3: el parámetro extra existe en la función.
    let inexistente = SEGURIDAD.replace(
        "parametro = \"cifrado\", lee",
        "parametro = \"encriptado\", lee",
    );
    falla_con(
        &fallos(&inexistente, ""),
        "wifi.seguridad.seguridad",
        "cbSetX no tiene el parámetro encriptado",
    );
    // Regla 5: ni el parámetro extra ni su lectura son secretos.
    let secreto = SEGURIDAD.replace(
        "parametro = \"cifrado\", lee = \"cifrado\"",
        "parametro = \"key_phrase\", lee = \"cifrado\"",
    );
    falla_con(
        &fallos(&secreto, ""),
        "wifi.seguridad.seguridad",
        "el parámetro key_phrase es secreto",
    );
    let lee_secreto = SEGURIDAD.replace("lee = \"cifrado\"", "lee = \"key_phrase\"");
    falla_con(
        &fallos(&lee_secreto, ""),
        "wifi.seguridad.seguridad",
        "el elemento leído key_phrase es secreto",
    );
    // Regla 6: el valor extra está en el enum.
    let fuera = SEGURIDAD.replace(
        "ademas = { cifrado = \"2\" }",
        "ademas = { cifrado = \"9\" }",
    );
    falla_con(
        &fallos(&fuera, ""),
        "wifi.seguridad.seguridad",
        "el valor 9 no está entre los del parámetro cifrado",
    );
    // Cada opción da valor a todos los parámetros extra, y solo a ellos.
    let falta = SEGURIDAD.replace(
        "aviso = \"inseguro\", ademas = { cifrado = \"0\" }",
        "aviso = \"inseguro\"",
    );
    falla_con(
        &fallos(&falta, ""),
        "wifi.seguridad.seguridad",
        "la opción abierta no da valor a cifrado",
    );
    let sobra = SEGURIDAD.replace(
        "ademas = { cifrado = \"0\" }",
        "ademas = { cifrado = \"0\", nombre = \"x\" }",
    );
    falla_con(
        &fallos(&sobra, ""),
        "wifi.seguridad.seguridad",
        "la opción abierta da valor a nombre",
    );
    // Dos opciones que escriben lo mismo.
    let repetida = SEGURIDAD.replace(
        "clave = \"wpa2_tkip\", aviso = \"inseguro\", ademas = { cifrado = \"2\" }",
        "clave = \"wpa2_tkip\", aviso = \"inseguro\", ademas = { cifrado = \"4\" }",
    );
    falla_con(
        &fallos(&repetida, ""),
        "wifi.seguridad.seguridad",
        "opción con el valor 3 repetido",
    );
    // El parámetro extra no repite el del campo.
    let mismo = SEGURIDAD.replace(
        "parametro = \"cifrado\", lee",
        "parametro = \"seguridad\", lee",
    );
    falla_con(
        &fallos(&mismo, ""),
        "wifi.seguridad.seguridad",
        "`ademas` repite el parámetro seguridad",
    );
    // `ademas` solo en un campo `opcion`.
    let en_texto = WIFI.replace(
        "lee = \"nombre\"\ntipo = \"texto\"",
        "lee = \"nombre\"\ntipo = \"texto\"\nademas = [{ parametro = \"cifrado\", lee = \"cifrado\" }]",
    );
    falla_con(
        &fallos(&en_texto, ""),
        "wifi.red.nombre",
        "`ademas` solo vale en un campo `opcion`",
    );
}

#[test]
fn campos_de_solo_lectura() {
    // Sin `escribe`, con su motivo: vale y no cuenta para el riesgo.
    let solo = "[[ajuste]]\nid = \"wifi.estado\"\ntema = \"wifi\"\nnivel = \"basico\"\n\
        leer = { funcion = \"cbGetX\" }\n\n[[ajuste.campo]]\nid = \"nombre\"\nlee = \"nombre\"\n\
        tipo = \"texto\"\nsolo_lectura = \"ajustes-solo-lectura-estado\"\n";
    let f = fallos(solo, "");
    assert!(f.is_empty(), "{f:#?}");
    let c = cargar(&[("wifi", solo)], "");
    let a = c.ajuste("wifi.estado").unwrap();
    assert!(a.campos[0].escribe.is_none());
    assert!(a.campos[0].escribible().is_none());
    assert_eq!(riesgo(a, &funciones()), Riesgo::Bajo);
    assert!(claves_requeridas(a).contains(&"ajustes-solo-lectura-estado".to_owned()));

    // Sin `escribe` ni `solo_lectura`.
    let sin_nada = solo.replace("solo_lectura = \"ajustes-solo-lectura-estado\"\n", "");
    falla_con(
        &fallos(&sin_nada, ""),
        "wifi.estado.nombre",
        "sin `escribe`, el campo necesita `solo_lectura`",
    );
    // `solo_lectura` en un campo que se puede escribir.
    let con_escribe = WIFI.replace(
        "lee = \"nombre\"\ntipo = \"texto\"",
        "lee = \"nombre\"\ntipo = \"texto\"\nsolo_lectura = \"ajustes-solo-lectura-estado\"",
    );
    falla_con(
        &fallos(&con_escribe, ""),
        "wifi.red.nombre",
        "`solo_lectura` con `escribe`",
    );
}

#[test]
fn forma_de_booleanos_enteros_opcionales_y_claves() {
    // Opciones sobre un parámetro `int`: enteras.
    let entero = WIFI.replace(
        "lee = \"max_clientes\"\ntipo = \"numero\"\nvalidar = { min = 1, max = 64 }",
        "lee = \"max_clientes\"\ntipo = \"opcion\"\n\
         opciones = [{ valor = \"8\", clave = \"pocos\" }, { valor = \"muchos\", clave = \"muchos\" }]",
    );
    falla_con(
        &fallos(&entero, ""),
        "wifi.avanzado.maximo",
        "el valor muchos del parámetro max_clientes no es un entero",
    );
    // `booleano` con el mismo valor para sí y no.
    let igual = WIFI.replace(
        "tipo = \"booleano\"\ninvertido = true",
        "tipo = \"booleano\"\nbooleano = { si = \"1\", no = \"1\" }\ninvertido = true",
    );
    falla_con(
        &fallos(&igual, ""),
        "wifi.red.visible",
        "`booleano` con el mismo valor para sí y no",
    );
    // `booleano` en un campo que no lo es.
    let en_texto = WIFI.replace(
        "lee = \"nombre\"\ntipo = \"texto\"",
        "lee = \"nombre\"\ntipo = \"texto\"\nbooleano = { si = \"1\", no = \"0\" }",
    );
    falla_con(
        &fallos(&en_texto, ""),
        "wifi.red.nombre",
        "`booleano` solo vale en un campo `booleano`",
    );
    // `textos_valor` en un campo que no es booleano.
    let textos_en_texto = WIFI.replace(
        "lee = \"nombre\"\ntipo = \"texto\"",
        "lee = \"nombre\"\ntipo = \"texto\"\ntextos_valor = true",
    );
    falla_con(
        &fallos(&textos_en_texto, ""),
        "wifi.red.nombre",
        "`textos_valor` solo vale en un campo `booleano`",
    );
    // `opcional` no vale en opciones ni booleanos; en texto, sí.
    let opcional_opcion = WIFI.replace(
        "lee = \"seguridad\"\ntipo = \"opcion\"",
        "lee = \"seguridad\"\ntipo = \"opcion\"\nopcional = true",
    );
    falla_con(
        &fallos(&opcional_opcion, ""),
        "wifi.red.seguridad",
        "`opcional` no vale",
    );
    let opcional_texto = WIFI.replace(
        "lee = \"nombre\"\ntipo = \"texto\"",
        "lee = \"nombre\"\ntipo = \"texto\"\nopcional = true",
    );
    let f = fallos(&opcional_texto, "");
    assert!(f.is_empty(), "{f:#?}");
    // Ids de campo y claves de opción que chocan con claves reservadas.
    for reservado in ["titulo", "desc", "ayuda"] {
        let malo = WIFI.replace("id = \"nombre\"", &format!("id = \"{reservado}\""));
        falla_con(
            &fallos(&malo, ""),
            &format!("wifi.red.{reservado}"),
            "reservado",
        );
    }
    for reservada in ["desc", "recomendado"] {
        let malo = WIFI.replace("clave = \"wpa2\"", &format!("clave = \"{reservada}\""));
        falla_con(&fallos(&malo, ""), "wifi.red.seguridad", "reservad");
    }
    // `efecto` con forma de nombre.
    let efecto = WIFI.replace("efecto = \"wifi_reinicia\"", "efecto = \"Wifi-Reinicia\"");
    falla_con(&fallos(&efecto, ""), "wifi.red", "efecto");
    // `ascii` solo en texto o secreto.
    let ascii_numero = WIFI.replace(
        "tipo = \"numero\"\nvalidar = { min = 1, max = 64 }",
        "tipo = \"numero\"\nvalidar = { min = 1, max = 64, ascii = true }",
    );
    falla_con(
        &fallos(&ascii_numero, ""),
        "wifi.avanzado.maximo",
        "`ascii` solo vale en texto o secreto",
    );
}

/// Regla 9: solo se excluye una escritura no bloqueada (una lectura, una
/// acción o una función bloqueada no necesitan ajuste ni motivo).
#[test]
fn excluida_solo_de_escrituras_no_bloqueadas() {
    let solo = |ex: &str, f: &Catalogo| comprobar(&cargar(&[], ex), f, &mascara());
    let lectura = "[[excluida]]\nfuncion = \"cbGetX\"\nmotivo = \"es una lectura\"\n";
    falla_con(
        &solo(lectura, &funciones()),
        "cbGetX",
        "no es una escritura",
    );
    let set_x = "[[excluida]]\nfuncion = \"cbSetX\"\nmotivo = \"sin ajuste\"\n";
    assert!(
        solo(set_x, &funciones()).is_empty(),
        "{:#?}",
        solo(set_x, &funciones())
    );
    let bloqueada = Catalogo::from_toml(&FUNCIONES.replace(
        "descripcion = \"Escribe X\"",
        "descripcion = \"Escribe X\"\nbloqueada = \"motivo de prueba suficiente\"",
    ))
    .unwrap();
    falla_con(&solo(set_x, &bloqueada), "cbSetX", "bloqueada");
}

/// `requiere`: al poner un campo en un valor, otro del mismo ajuste tiene que
/// venir también (la toma del espejo al encenderlo). `comprobar` exige que el
/// otro campo exista, no sea el mismo y que el valor sea válido.
#[test]
fn requiere_otro_campo_del_mismo_ajuste() {
    let con = |requiere: &str| {
        WIFI.replace(
            "invertido = true\nrecomendado = \"true\"",
            &format!("invertido = true\nrecomendado = \"true\"\nrequiere = {requiere}"),
        )
    };
    let bien = con(r#"{ campo = "seguridad", cuando = "true" }"#);
    let f = fallos(&bien, "");
    assert!(f.is_empty(), "{f:#?}");
    let c = cargar(&[("wifi", &bien)], "");
    let r = c.ajuste("wifi.red").unwrap().campo("visible").unwrap();
    let r = r.requiere.as_ref().expect("requiere cargado");
    assert_eq!((r.campo.as_str(), r.cuando.as_str()), ("seguridad", "true"));

    falla_con(
        &fallos(&con(r#"{ campo = "no_existe", cuando = "true" }"#), ""),
        "wifi.red.visible",
        "requiere",
    );
    falla_con(
        &fallos(&con(r#"{ campo = "visible", cuando = "true" }"#), ""),
        "wifi.red.visible",
        "requiere",
    );
    falla_con(
        &fallos(&con(r#"{ campo = "seguridad", cuando = "quizá" }"#), ""),
        "wifi.red.visible",
        "requiere",
    );
    // El campo exigido tiene que poder escribirse.
    falla_con(
        &fallos(&con(r#"{ campo = "clave_basica", cuando = "true" }"#), ""),
        "wifi.red.visible",
        "requiere",
    );
}

/// `requiere` en un campo que no se puede escribir nunca se cumpliría:
/// `comprobar` lo rechaza.
#[test]
fn requiere_desde_un_campo_de_solo_lectura() {
    let en_solo_lectura = WIFI.replace(
        "solo_lectura = \"ajustes-solo-lectura-basica\"",
        "solo_lectura = \"ajustes-solo-lectura-basica\"\n\
         requiere = { campo = \"seguridad\", cuando = \"clave-larga\" }",
    );
    falla_con(
        &fallos(&en_solo_lectura, ""),
        "wifi.red.clave_basica",
        "requiere",
    );
}

/// `sin_eleccion`: el valor del router que significa «ninguna» (la toma del
/// espejo apagado) solo vale en un campo `opcion` y no puede ser el de una
/// opción.
#[test]
fn sin_eleccion_solo_en_opciones_y_distinto() {
    let con = |campo: &str, valor: &str| {
        WIFI.replace(
            &format!("lee = \"{campo}\"\n"),
            &format!("lee = \"{campo}\"\nsin_eleccion = \"{valor}\"\n"),
        )
    };
    let bien = con("seguridad", "7");
    let f = fallos(&bien, "");
    assert!(f.is_empty(), "{f:#?}");
    let c = cargar(&[("wifi", &bien)], "");
    let a = c.ajuste("wifi.red").unwrap();
    assert!(
        claves_requeridas(a).contains(&"ajuste-wifi-red-seguridad-ninguna".to_owned()),
        "{:?}",
        claves_requeridas(a)
    );
    falla_con(
        &fallos(&con("seguridad", "3"), ""),
        "wifi.red.seguridad",
        "sin_eleccion",
    );
    falla_con(
        &fallos(&con("nombre", "0"), ""),
        "wifi.red.nombre",
        "sin_eleccion",
    );
}

/// Límites de la excepción de la regla 8: solo un secreto de la web básica
/// con `solo_lectura` conserva su `escribe`. Un texto de la web básica o un
/// secreto de la web avanzada con `solo_lectura` y `escribe` siguen siendo
/// un error del catálogo.
#[test]
fn excepcion_de_solo_lectura_con_escribe_solo_para_secretos_basicos() {
    let f = fallos(WIFI, "");
    assert!(f.is_empty(), "el secreto básico de WIFI vale: {f:#?}");
    let texto_basico = format!(
        "{WIFI}\n[[ajuste.campo]]\nid = \"nombre_basico\"\n\
         escribe = {{ funcion = \"te_x.wl\", parametro = \"wlSsid\" }}\nlee = \"nombre\"\n\
         tipo = \"texto\"\nsolo_lectura = \"ajustes-solo-lectura-basica\"\n"
    );
    falla_con(
        &fallos(&texto_basico, ""),
        "wifi.avanzado.nombre_basico",
        "`solo_lectura` con `escribe`",
    );
    let secreto_avanzado = WIFI.replace(
        "lee = \"key_phrase\"\ntipo = \"secreto\"\nvalidar = { min = 8, max = 63 }",
        "lee = \"key_phrase\"\ntipo = \"secreto\"\nvalidar = { min = 8, max = 63 }\n\
         solo_lectura = \"ajustes-solo-lectura-basica\"",
    );
    assert_ne!(secreto_avanzado, WIFI, "el reemplazo tiene que casar");
    falla_con(
        &fallos(&secreto_avanzado, ""),
        "wifi.red.clave",
        "`solo_lectura` con `escribe`",
    );
}

/// Un secreto que el router da tapado al leer tiene que poder pedirse: un
/// ajuste que escribe su función necesita un campo `secreto` editable para
/// él.
#[test]
fn secreto_tapado_necesita_su_campo() {
    let marcadas = FUNCIONES.replace(
        "nombre = \"key_phrase\"\netiqueta = \"Clave\"\ntipo = \"secret\"",
        "nombre = \"key_phrase\"\netiqueta = \"Clave\"\ntipo = \"secret\"\ntapado = true",
    );
    assert_ne!(marcadas, FUNCIONES, "el reemplazo tiene que casar");
    let f = Catalogo::from_toml(&marcadas).unwrap();
    let con = comprobar(&cargar(&[("wifi", WIFI)], ""), &f, &mascara());
    // wifi.red tiene su campo; wifi.avanzado escribe cbSetX sin él.
    assert!(!con.iter().any(|x| x.starts_with("wifi.red:")), "{con:#?}");
    falla_con(&con, "wifi.avanzado", "tapado");
    // Sin el campo, también wifi.red.
    let sin = WIFI.replace(
        "[[ajuste.campo]]\nid = \"clave\"\nescribe = { funcion = \"cbSetX\", parametro = \"key_phrase\" }\n\
         lee = \"key_phrase\"\ntipo = \"secreto\"\nvalidar = { min = 8, max = 63 }\n",
        "",
    );
    assert_ne!(sin, WIFI, "el reemplazo tiene que casar");
    let fallos = comprobar(&cargar(&[("wifi", &sin)], ""), &f, &mascara());
    falla_con(&fallos, "wifi.red:", "tapado");
}

/// `aceptados`: otros valores que valen como lo recomendado, solo en un
/// texto con `recomendado`; `*.dominio` acepta cualquier nombre de él.
#[test]
fn aceptados_en_un_texto_con_recomendado() {
    let con = |extra: &str| {
        WIFI.replacen(
            "tipo = \"texto\"\nvalidar = { min = 1, max = 32 }",
            &format!("tipo = \"texto\"\nvalidar = {{ min = 1, max = 32 }}\n{extra}"),
            1,
        )
    };
    let bien =
        con("recomendado = \"hora.example\"\naceptados = [\"*.pool.example\", \"otra.example\"]");
    let f = fallos(&bien, "");
    assert!(f.is_empty(), "{f:#?}");
    falla_con(
        &fallos(&con("aceptados = [\"otra.example\"]"), ""),
        "wifi.red.nombre",
        "aceptados",
    );
    falla_con(
        &fallos(
            &con("recomendado = \"a.example\"\naceptados = [\"*.\"]"),
            "",
        ),
        "wifi.red.nombre",
        "aceptados",
    );
    let a = ["*.pool.example".to_owned(), "otra.example".to_owned()];
    assert!(ajustes::valor_aceptado(&a, "es.pool.example"));
    assert!(ajustes::valor_aceptado(&a, " OTRA.example "));
    assert!(!ajustes::valor_aceptado(&a, "pool.example"));
    assert!(!ajustes::valor_aceptado(&a, "malpool.example"));
    assert!(!ajustes::valor_aceptado(&a, "hora.operador.example"));
}
