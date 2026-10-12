//! Carga, claves de traducción y reglas de `comprobar` sobre un catálogo de
//! funciones y unos ajustes sintéticos.

use ajustes::{
    CatalogoAjustes, Nivel, Tema, TipoCampo, claves_requeridas, comprobar, prefijo_i18n, riesgo,
};
use catalogo::{Catalogo, Riesgo, mask::Mascara};

/// Catálogo de funciones mínimo: una lectura, una escritura de la web
/// avanzada y otra de la básica.
const FUNCIONES: &str = r#"
[[funcion]]
name = "cbGetX"
area = "wifi"
origen = "avanzada"
tipo = "lectura"
descripcion = "Lee X"
riesgo = "bajo"

[[funcion.parametro]]
nombre = "x_id"
etiqueta = "Instancia"
tipo = "int"

[[funcion.parametro]]
nombre = "x_psk"
etiqueta = "Clave"
tipo = "text"

[[funcion]]
name = "cbSetX"
area = "wifi"
origen = "avanzada"
tipo = "escritura"
pagina = "x.asp"
descripcion = "Escribe X"
riesgo = "medio"
lectura_pareada = { funcion = "cbGetX" }

[[funcion.parametro]]
nombre = "x_id"
etiqueta = "Instancia"
tipo = "int"

[[funcion.parametro]]
nombre = "nombre"
etiqueta = "Nombre"
tipo = "text"

[[funcion.parametro]]
nombre = "seguridad"
etiqueta = "Seguridad"
tipo = "enum"
opciones = [["11", "WPA2/WPA3"], ["3", "WPA2"], ["0", "Abierta"]]

[[funcion.parametro]]
nombre = "cifrado"
etiqueta = "Cifrado"
tipo = "enum"
opciones = [["4", "AES"], ["2", "TKIP"], ["0", "Ninguno"]]

[[funcion.parametro]]
nombre = "oculta"
etiqueta = "Oculta"
tipo = "bool"

[[funcion.parametro]]
nombre = "key_phrase"
etiqueta = "Clave"
tipo = "secret"

[[funcion.parametro]]
nombre = "wlPassphrase"
etiqueta = "Clave sin marcar"
tipo = "text"

[[funcion.parametro]]
nombre = "max_clientes"
etiqueta = "Máximo"
tipo = "int"

[[funcion.parametro]]
nombre = "puerta"
etiqueta = "Puerta"
tipo = "ip"

[[funcion.parametro]]
nombre = "mac"
etiqueta = "MAC"
tipo = "mac"

[[funcion]]
name = "te_x.wl"
area = "wifi"
origen = "basica"
tipo = "escritura"
pagina = "te_x.asp"
descripcion = "Escribe X desde la web básica"
riesgo = "alto"
lectura_pareada = { funcion = "cbGetX" }

[[funcion.parametro]]
nombre = "wlWpaPsk"
etiqueta = "Clave"
tipo = "secret"

[[funcion.parametro]]
nombre = "wlSsid"
etiqueta = "Nombre"
tipo = "text"
"#;

const SECRETOS: &str = r#"
fragmentos = ["key", "psk", "passphrase"]
exactos = []
sufijos_numericos = ["_id", "_type"]
"#;

/// Ajustes válidos contra [`FUNCIONES`].
const WIFI: &str = r#"
[[ajuste]]
id = "wifi.red"
tema = "wifi"
nivel = "basico"
efecto = "wifi_reinicia"
leer = { funcion = "cbGetX", params = { x_id = "1" } }
fijos = { x_id = "1" }

[[ajuste.campo]]
id = "nombre"
escribe = { funcion = "cbSetX", parametro = "nombre" }
lee = "nombre"
tipo = "texto"
validar = { min = 1, max = 32 }

[[ajuste.campo]]
id = "seguridad"
escribe = { funcion = "cbSetX", parametro = "seguridad" }
lee = "seguridad"
tipo = "opcion"
opciones = [
  { valor = "11", clave = "wpa2_wpa3", recomendada = true },
  { valor = "3", clave = "wpa2" },
  { valor = "0", clave = "abierta", aviso = "inseguro" },
]

[[ajuste.campo]]
id = "visible"
escribe = { funcion = "cbSetX", parametro = "oculta" }
lee = "oculta"
tipo = "booleano"
invertido = true
recomendado = "true"

[[ajuste.campo]]
id = "clave"
escribe = { funcion = "cbSetX", parametro = "key_phrase" }
lee = "key_phrase"
tipo = "secreto"
validar = { min = 8, max = 63 }

[[ajuste.campo]]
id = "clave_basica"
escribe = { funcion = "te_x.wl", parametro = "wlWpaPsk" }
lee = "key_phrase"
tipo = "secreto"
solo_lectura = "ajustes-solo-lectura-basica"

[[ajuste]]
id = "wifi.avanzado"
tema = "wifi"
nivel = "avanzado"
leer = { funcion = "cbGetX" }

[[ajuste.campo]]
id = "maximo"
escribe = { funcion = "cbSetX", parametro = "max_clientes" }
lee = "max_clientes"
tipo = "numero"
validar = { min = 1, max = 64 }
"#;

const RED_LOCAL: &str = r#"
[[ajuste]]
id = "red_local.direcciones"
tema = "red_local"
nivel = "oculto"
componente = "reservas"
leer = { funcion = "cbGetX" }

[[ajuste.campo]]
id = "puerta"
escribe = { funcion = "cbSetX", parametro = "puerta" }
lee = "puerta"
tipo = "ip"

[[ajuste.campo]]
id = "mac"
escribe = { funcion = "cbSetX", parametro = "mac" }
lee = "mac"
tipo = "mac"
"#;

const EXCLUIDAS: &str = r#"
[[excluida]]
funcion = "te_x.wl"
motivo = "duplicado de la web básica de wifi.red"
"#;

fn funciones() -> Catalogo {
    Catalogo::from_toml(FUNCIONES).expect("catálogo sintético válido")
}

fn mascara() -> Mascara {
    Mascara::from_toml(SECRETOS).expect("secretos sintéticos válidos")
}

fn cargar(ficheros: &[(&str, &str)], excluidas: &str) -> CatalogoAjustes {
    CatalogoAjustes::from_toml(ficheros, excluidas).expect("ajustes sintéticos legibles")
}

/// Fallos de `comprobar` para un único fichero `wifi.toml`.
fn fallos(wifi: &str, excluidas: &str) -> Vec<String> {
    comprobar(
        &cargar(&[("wifi", wifi)], excluidas),
        &funciones(),
        &mascara(),
    )
}

/// `fallos` no está vacío y alguno menciona `id` y `texto`.
fn falla_con(fallos: &[String], id: &str, texto: &str) {
    assert!(
        fallos.iter().any(|f| f.contains(id) && f.contains(texto)),
        "se esperaba un fallo de {id} con «{texto}»: {fallos:#?}"
    );
}

#[test]
fn carga_y_por_tema() {
    let c = cargar(&[("wifi", WIFI), ("red_local", RED_LOCAL)], EXCLUIDAS);
    assert_eq!(c.todos().len(), 3);
    let wifi: Vec<_> = c.por_tema(Tema::Wifi).map(|a| a.id.as_str()).collect();
    assert_eq!(wifi, ["wifi.red", "wifi.avanzado"]);
    assert_eq!(c.por_tema(Tema::Sistema).count(), 0);

    let red = c.ajuste("wifi.red").expect("wifi.red");
    assert_eq!(red.nivel, Nivel::Basico);
    assert_eq!(red.efecto.as_deref(), Some("wifi_reinicia"));
    assert_eq!(red.leer.funcion, "cbGetX");
    assert_eq!(red.leer.params, [("x_id".to_owned(), "1".to_owned())]);
    assert_eq!(red.fijos.get("x_id").map(String::as_str), Some("1"));
    assert!(!red.lista);
    assert_eq!(red.campos.len(), 5);
    let visible = &red.campos[2];
    assert_eq!(visible.tipo, TipoCampo::Booleano);
    assert!(visible.invertido);
    assert_eq!(visible.booleano, None);
    assert_eq!(visible.si_no(), ("1", "0"));
    assert_eq!(red.campos[0].validar.as_ref().and_then(|v| v.max), Some(32));
    assert!(red.campos[1].opciones[0].recomendada);
    assert_eq!(red.campos[1].opciones[2].aviso.as_deref(), Some("inseguro"));

    let lan = c
        .ajuste("red_local.direcciones")
        .expect("red_local.direcciones");
    assert_eq!(lan.nivel, Nivel::Oculto);
    assert_eq!(lan.componente, Some(ajustes::Componente::Reservas));
    assert!(c.ajuste("wifi.nada").is_none());

    assert_eq!(c.excluidas().len(), 1);
    assert_eq!(c.excluidas()[0].funcion, "te_x.wl");

    // Válidos, salvo la exclusión de una función que `wifi.red` usa.
    let sin_excluidas = cargar(&[("wifi", WIFI), ("red_local", RED_LOCAL)], "");
    let f = comprobar(&sin_excluidas, &funciones(), &mascara());
    assert!(f.is_empty(), "{f:#?}");

    // Los temas, en el orden de las pestañas, ida y vuelta por su slug.
    let slugs: Vec<_> = Tema::TODOS.iter().map(|t| t.slug()).collect();
    assert_eq!(
        slugs,
        [
            "internet",
            "wifi",
            "red_local",
            "puertos",
            "seguridad",
            "telefono",
            "sistema"
        ]
    );
    for t in Tema::TODOS {
        assert_eq!(Tema::from_slug(t.slug()), Some(t));
    }
    assert_eq!(Tema::from_slug("nada"), None);
}

#[test]
fn campos_desconocidos_o_toml_roto_no_cargan() {
    let raro = WIFI.replace(
        "nivel = \"avanzado\"",
        "nivel = \"avanzado\"\ncolor = \"rojo\"",
    );
    let e = CatalogoAjustes::from_toml(&[("wifi", &raro)], "").unwrap_err();
    assert!(e.to_string().contains("wifi"), "{e}");
    assert!(CatalogoAjustes::from_toml(&[("wifi", "[[ajuste]")], "").is_err());
    assert!(CatalogoAjustes::from_toml(&[], "[[excluida]]\nfuncion = \"x\"").is_err());
}

#[test]
fn carga_el_catalogo_compilado() {
    let c = CatalogoAjustes::load().expect("ajustes/*.toml legibles");
    let fallos = comprobar(&c, &Catalogo::load().unwrap(), &Mascara::load().unwrap());
    assert!(fallos.is_empty(), "{fallos:#?}");
}

#[test]
fn prefijo_y_claves_requeridas() {
    let toml = r#"
[[ajuste]]
id = "wifi.red_5"
tema = "wifi"
nivel = "basico"
efecto = "wifi_reinicia"
leer = { funcion = "cbGetX" }

[[ajuste.campo]]
id = "nombre"
escribe = { funcion = "cbSetX", parametro = "nombre" }
lee = "nombre"
tipo = "texto"

[[ajuste.campo]]
id = "seguridad"
escribe = { funcion = "cbSetX", parametro = "seguridad" }
lee = "seguridad"
tipo = "opcion"
solo_lectura = "ajustes-solo-lectura-basica"
opciones = [
  { valor = "11", clave = "wpa2_wpa3", recomendada = true },
  { valor = "0", clave = "abierta", aviso = "inseguro" },
]
"#;
    let c = cargar(&[("wifi", toml)], "");
    let a = c.ajuste("wifi.red_5").unwrap();
    assert_eq!(prefijo_i18n(a), "ajuste-wifi-red_5");
    assert_eq!(
        claves_requeridas(a),
        [
            "ajuste-wifi-red_5-titulo",
            "ajuste-wifi-red_5-desc",
            "ajuste-wifi-red_5-ayuda",
            "efecto-wifi_reinicia",
            "ajuste-wifi-red_5-nombre",
            "ajuste-wifi-red_5-nombre-desc",
            "ajuste-wifi-red_5-seguridad",
            "ajuste-wifi-red_5-seguridad-desc",
            "ajustes-solo-lectura-basica",
            "ajuste-wifi-red_5-seguridad-wpa2_wpa3",
            "ajuste-wifi-red_5-seguridad-wpa2_wpa3-desc",
            "ajuste-wifi-red_5-seguridad-abierta",
            "ajuste-wifi-red_5-seguridad-abierta-desc",
            "aviso-inseguro",
            "ajuste-wifi-red_5-seguridad-recomendado",
        ]
    );
}

#[test]
fn ids_con_forma_y_unicos() {
    let otro_tema = WIFI.replace("id = \"wifi.avanzado\"", "id = \"sistema.avanzado\"");
    falla_con(&fallos(&otro_tema, ""), "sistema.avanzado", "tema");
    let sin_nombre = WIFI.replace("id = \"wifi.avanzado\"", "id = \"wifi\"");
    falla_con(&fallos(&sin_nombre, ""), "wifi", "<tema>.<nombre>");
    let repetido = WIFI.replace("id = \"wifi.avanzado\"", "id = \"wifi.red\"");
    falla_con(&fallos(&repetido, ""), "wifi.red", "repetido");
}

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
        "{ valor = \"3\", clave = \"wpa2\", recomendada = true }",
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
