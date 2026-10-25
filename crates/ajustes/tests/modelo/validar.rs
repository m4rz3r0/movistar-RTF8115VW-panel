//! Reglas de `comprobar`: funciones, parámetros, tipos, secretos, opciones y riesgo.

use super::*;

#[test]
fn rechaza_funcion_inexistente_bloqueada_o_lectura() {
    let inexistente = WIFI.replace(
        "{ funcion = \"cbSetX\", parametro = \"max_clientes\" }",
        "{ funcion = \"cbSetNada\", parametro = \"max_clientes\" }",
    );
    falla_con(&fallos(&inexistente, ""), "wifi.avanzado", "cbSetNada");

    let lectura = WIFI.replace(
        "{ funcion = \"cbSetX\", parametro = \"max_clientes\" }",
        "{ funcion = \"cbGetX\", parametro = \"max_clientes\" }",
    );
    falla_con(
        &fallos(&lectura, ""),
        "wifi.avanzado",
        "no es una escritura",
    );

    let leer_escritura = WIFI.replace(
        "leer = { funcion = \"cbGetX\" }",
        "leer = { funcion = \"cbSetX\" }",
    );
    falla_con(
        &fallos(&leer_escritura, ""),
        "wifi.avanzado",
        "no es una lectura",
    );
    let leer_nada = WIFI.replace(
        "leer = { funcion = \"cbGetX\" }",
        "leer = { funcion = \"cbGetNada\" }",
    );
    falla_con(&fallos(&leer_nada, ""), "wifi.avanzado", "cbGetNada");

    let bloqueada = Catalogo::from_toml(&FUNCIONES.replace(
        "descripcion = \"Escribe X\"",
        "descripcion = \"Escribe X\"\nbloqueada = \"motivo de prueba suficiente\"",
    ))
    .unwrap();
    let f = comprobar(&cargar(&[("wifi", WIFI)], ""), &bloqueada, &mascara());
    falla_con(&f, "wifi.avanzado", "bloqueada");
}

#[test]
fn rechaza_parametro_inexistente() {
    let malo = WIFI.replace("parametro = \"max_clientes\"", "parametro = \"minimo\"");
    falla_con(&fallos(&malo, ""), "wifi.avanzado", "minimo");
}

#[test]
fn rechaza_tipo_incompatible() {
    // Un entero declarado como IP.
    let ip = WIFI.replace(
        "lee = \"max_clientes\"\ntipo = \"numero\"",
        "lee = \"max_clientes\"\ntipo = \"ip\"",
    );
    falla_con(&fallos(&ip, ""), "wifi.avanzado.maximo", "incompatible");
    // Un texto declarado como booleano.
    let booleano = WIFI.replace(
        "lee = \"nombre\"\ntipo = \"texto\"",
        "lee = \"nombre\"\ntipo = \"booleano\"",
    );
    falla_con(&fallos(&booleano, ""), "wifi.red.nombre", "incompatible");
    // Un enum de tres opciones no es un booleano.
    let tres = WIFI.replace(
        "lee = \"seguridad\"\ntipo = \"opcion\"",
        "lee = \"seguridad\"\ntipo = \"booleano\"",
    );
    falla_con(&fallos(&tres, ""), "wifi.red.seguridad", "incompatible");
}

#[test]
fn rechaza_secreto_mal_declarado() {
    // Parámetro secreto en un campo de texto.
    let visible = WIFI.replace(
        "lee = \"key_phrase\"\ntipo = \"secreto\"\nvalidar",
        "lee = \"key_phrase\"\ntipo = \"texto\"\nvalidar",
    );
    falla_con(
        &fallos(&visible, ""),
        "wifi.red.clave",
        "el parámetro key_phrase es secreto y el campo no es `secreto`",
    );
    // Parámetro `text` que la máscara reconoce como secreto.
    let por_mascara = WIFI.replace("parametro = \"nombre\"", "parametro = \"wlPassphrase\"");
    falla_con(
        &fallos(&por_mascara, ""),
        "wifi.red.nombre",
        "el parámetro wlPassphrase es secreto y el campo no es `secreto`",
    );
    // Un parámetro corriente que se lee de un elemento secreto: el valor
    // leído llegaría a la página.
    let lee_secreto = WIFI.replace(
        "lee = \"nombre\"\ntipo = \"texto\"",
        "lee = \"key_phrase\"\ntipo = \"texto\"",
    );
    falla_con(
        &fallos(&lee_secreto, ""),
        "wifi.red.nombre",
        "el elemento leído key_phrase es secreto y el campo no es `secreto`",
    );
    // Campo secreto sobre un parámetro que no lo es.
    let falso = WIFI.replace(
        "lee = \"nombre\"\ntipo = \"texto\"",
        "lee = \"nombre\"\ntipo = \"secreto\"",
    );
    falla_con(
        &fallos(&falso, ""),
        "wifi.red.nombre",
        "campo `secreto` sobre nombre, que no es un parámetro secreto",
    );
    // `wlPassphrase` como secreto sí vale (lo dice la máscara).
    let bien = WIFI
        .replace("parametro = \"nombre\"", "parametro = \"wlPassphrase\"")
        .replace(
            "lee = \"nombre\"\ntipo = \"texto\"",
            "lee = \"nombre\"\ntipo = \"secreto\"",
        );
    assert!(fallos(&bien, "").is_empty(), "{:#?}", fallos(&bien, ""));
}

#[test]
fn rechaza_opcion_fuera_del_enum() {
    let malo = WIFI.replace(
        "{ valor = \"3\", clave = \"wpa2\" }",
        "{ valor = \"7\", clave = \"wpa2\" }",
    );
    falla_con(&fallos(&malo, ""), "wifi.red", "7");
}

#[test]
fn rechaza_lista_sin_lista() {
    let malo = WIFI.replace("nivel = \"avanzado\"", "nivel = \"avanzado\"\nlista = true");
    falla_con(&fallos(&malo, ""), "wifi.avanzado", "lista");
    // Con `lista` en la función, vale.
    let con_lista = Catalogo::from_toml(&FUNCIONES.replace(
        "riesgo = \"medio\"",
        "riesgo = \"medio\"\nlista = { id_param = \"max_clientes\" }",
    ))
    .unwrap();
    let f = comprobar(&cargar(&[("wifi", &malo)], ""), &con_lista, &mascara());
    assert!(f.is_empty(), "{f:#?}");
}

/// El id de una instancia sale tal cual hacia la interfaz: el parámetro de
/// instancia de una lista (de su escritura o de su lectura) no puede ser
/// secreto.
#[test]
fn rechaza_id_de_lista_secreto() {
    let lista = WIFI.replace("nivel = \"avanzado\"", "nivel = \"avanzado\"\nlista = true");
    // De la escritura.
    let escritura = Catalogo::from_toml(&FUNCIONES.replace(
        "riesgo = \"medio\"",
        "riesgo = \"medio\"\nlista = { id_param = \"key_phrase\" }",
    ))
    .unwrap();
    let f = comprobar(&cargar(&[("wifi", &lista)], ""), &escritura, &mascara());
    falla_con(&f, "wifi.avanzado", "key_phrase");
    // De la lectura (con la de la escritura válida).
    let lectura = Catalogo::from_toml(
        &FUNCIONES
            .replace(
                "riesgo = \"medio\"",
                "riesgo = \"medio\"\nlista = { id_param = \"max_clientes\" }",
            )
            .replacen(
                "riesgo = \"bajo\"",
                "riesgo = \"bajo\"\nlista = { id_param = \"x_psk\" }",
                1,
            ),
    )
    .unwrap();
    let f = comprobar(&cargar(&[("wifi", &lista)], ""), &lectura, &mascara());
    falla_con(&f, "wifi.avanzado", "x_psk");
    // Sin `lista = true` en el ajuste, la regla no aplica.
    let f = comprobar(&cargar(&[("wifi", WIFI)], ""), &lectura, &mascara());
    assert!(f.is_empty(), "{f:#?}");
}

#[test]
fn secreto_de_web_basica_exige_solo_lectura() {
    let malo = WIFI.replace("solo_lectura = \"ajustes-solo-lectura-basica\"\n", "");
    falla_con(&fallos(&malo, ""), "wifi.red", "solo_lectura");
    // Un campo no secreto de la web básica sí puede ser editable.
    let texto_basico = format!(
        "{WIFI}\n[[ajuste.campo]]\nid = \"nombre_basico\"\n\
         escribe = {{ funcion = \"te_x.wl\", parametro = \"wlSsid\" }}\nlee = \"nombre\"\ntipo = \"texto\"\n"
    );
    assert!(
        fallos(&texto_basico, "").is_empty(),
        "{:#?}",
        fallos(&texto_basico, "")
    );
}

#[test]
fn excluida_y_cubierta_a_la_vez() {
    let cubierta = "[[excluida]]\nfuncion = \"cbSetX\"\nmotivo = \"duplicada\"\n";
    falla_con(&fallos(WIFI, cubierta), "cbSetX", "excluida");
    let inexistente = "[[excluida]]\nfuncion = \"cbSetNada\"\nmotivo = \"no existe\"\n";
    falla_con(&fallos(WIFI, inexistente), "cbSetNada", "no existe");
    let sin_motivo = "[[excluida]]\nfuncion = \"te_x.wl\"\nmotivo = \" \"\n";
    falla_con(&fallos(WIFI, sin_motivo), "te_x.wl", "motivo");
    // `te_x.wl` también es la función de `wifi.red.clave_basica`.
    let f = fallos(WIFI, EXCLUIDAS);
    falla_con(&f, "te_x.wl", "excluida");
    // Si `clave_basica` escribe con `cbSetX` (y deja de ser de solo lectura,
    // porque ya no es de la web básica), la exclusión vale.
    let sin_basica = WIFI
        .replace(
            "escribe = { funcion = \"te_x.wl\", parametro = \"wlWpaPsk\" }",
            "escribe = { funcion = \"cbSetX\", parametro = \"key_phrase\" }",
        )
        .replace("solo_lectura = \"ajustes-solo-lectura-basica\"\n", "");
    assert!(
        fallos(&sin_basica, EXCLUIDAS).is_empty(),
        "{:#?}",
        fallos(&sin_basica, EXCLUIDAS)
    );
}

#[test]
fn opciones_y_recomendaciones_coherentes() {
    let dos_recomendadas = WIFI.replace(
        "{ valor = \"3\", clave = \"wpa2\" }",
        "{ valor = \"3\", clave = \"wpa2\", recomendada = true, respaldo = \"criterio\" }",
    );
    falla_con(&fallos(&dos_recomendadas, ""), "wifi.red", "recomendada");
    let recomendado_invalido = WIFI.replace("recomendado = \"true\"", "recomendado = \"quizá\"");
    falla_con(
        &fallos(&recomendado_invalido, ""),
        "wifi.red",
        "recomendado",
    );
    let sin_opciones = WIFI.replace(
        "lee = \"max_clientes\"\ntipo = \"numero\"",
        "lee = \"max_clientes\"\ntipo = \"opcion\"",
    );
    falla_con(&fallos(&sin_opciones, ""), "wifi.avanzado", "opciones");
}

#[test]
fn riesgo_es_el_mayor() {
    let c = cargar(&[("wifi", WIFI), ("red_local", RED_LOCAL)], "");
    let f = funciones();
    // `wifi.red` solo escribe con `cbSetX` (medio): `clave_basica`, de
    // `te_x.wl` (alto), es de solo lectura.
    assert_eq!(riesgo(c.ajuste("wifi.red").unwrap(), &f), Riesgo::Medio);
    // Con un campo editable de `te_x.wl`, alto.
    let con_basica = WIFI.replace(
        "fijos = { x_id = \"1\" }\n",
        "\n[[ajuste.campo]]\nid = \"nombre_basico\"\n\
         escribe = { funcion = \"te_x.wl\", parametro = \"wlSsid\" }\nlee = \"nombre\"\ntipo = \"texto\"\n",
    );
    let c2 = cargar(&[("wifi", &con_basica)], "");
    assert_eq!(riesgo(c2.ajuste("wifi.red").unwrap(), &f), Riesgo::Alto);
    assert_eq!(
        riesgo(c.ajuste("wifi.avanzado").unwrap(), &f),
        Riesgo::Medio
    );
    // Sin campos (solo lectura), bajo.
    let solo = cargar(
        &[(
            "red_local",
            "[[ajuste]]\nid = \"red_local.dispositivos\"\ntema = \"red_local\"\n\
           nivel = \"basico\"\ncomponente = \"dispositivos\"\nleer = { funcion = \"cbGetX\" }\n",
        )],
        "",
    );
    assert_eq!(riesgo(&solo.todos()[0], &f), Riesgo::Bajo);
}

#[test]
fn rechaza_fijo_o_param_de_lectura_inexistente() {
    // Parámetro de lectura que `cbGetX` no tiene.
    let lectura = WIFI.replace(
        "leer = { funcion = \"cbGetX\", params = { x_id = \"1\" } }",
        "leer = { funcion = \"cbGetX\", params = { y_id = \"1\" } }",
    );
    falla_con(
        &fallos(&lectura, ""),
        "wifi.red",
        "la lectura cbGetX no tiene el parámetro y_id",
    );
    // Parámetro de lectura secreto.
    let lectura_secreta = WIFI.replace(
        "params = { x_id = \"1\" }",
        "params = { x_id = \"1\", x_psk = \"1\" }",
    );
    falla_con(
        &fallos(&lectura_secreta, ""),
        "wifi.red",
        "el parámetro de lectura x_psk es secreto",
    );
    // Fijo que `cbSetX` no tiene.
    let fijo = WIFI.replace("fijos = { x_id = \"1\" }", "fijos = { y_id = \"1\" }");
    falla_con(
        &fallos(&fijo, ""),
        "wifi.red",
        "cbSetX no tiene el fijo y_id",
    );
    // Fijo secreto.
    let fijo_secreto = WIFI.replace(
        "fijos = { x_id = \"1\" }",
        "fijos = { x_id = \"1\", key_phrase = \"x\" }",
    );
    falla_con(
        &fallos(&fijo_secreto, ""),
        "wifi.red",
        "el fijo key_phrase es secreto",
    );
    // El fijo debe estar en cada función de escritura del ajuste: `te_x.wl`
    // no tiene `x_id`.
    let otra_funcion = WIFI.replace(
        "fijos = { x_id = \"1\" }\n",
        "fijos = { x_id = \"1\" }\n\n[[ajuste.campo]]\nid = \"nombre_basico\"\n\
         escribe = { funcion = \"te_x.wl\", parametro = \"wlSsid\" }\nlee = \"nombre\"\ntipo = \"texto\"\n",
    );
    falla_con(
        &fallos(&otra_funcion, ""),
        "wifi.red",
        "te_x.wl no tiene el fijo x_id",
    );
    // Un fijo que también escribe un campo.
    let pisado = WIFI.replace("fijos = { x_id = \"1\" }", "fijos = { nombre = \"1\" }");
    falla_con(
        &fallos(&pisado, ""),
        "wifi.red",
        "el fijo nombre también lo escribe un campo",
    );
    // Fijos en un ajuste que no escribe nada.
    let sin_escritura = "[[ajuste]]\nid = \"wifi.estado\"\ntema = \"wifi\"\nnivel = \"basico\"\n\
        leer = { funcion = \"cbGetX\" }\nfijos = { x_id = \"1\" }\n";
    falla_con(
        &fallos(sin_escritura, ""),
        "wifi.estado",
        "fijos en un ajuste que no escribe",
    );
}

/// Un secreto no puede ser `opcional`: al enviarlo vacío se vaciaría la
/// clave del router.
#[test]
fn secreto_no_puede_ser_opcional() {
    let malo = WIFI.replace(
        "lee = \"key_phrase\"\ntipo = \"secreto\"\nvalidar = { min = 8, max = 63 }",
        "lee = \"key_phrase\"\ntipo = \"secreto\"\nopcional = true\nvalidar = { min = 8, max = 63 }",
    );
    assert_ne!(malo, WIFI, "el reemplazo tiene que casar");
    falla_con(&fallos(&malo, ""), "wifi.red.clave", "`opcional`");
}
