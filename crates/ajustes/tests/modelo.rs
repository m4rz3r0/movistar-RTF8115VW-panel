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
nombre = "nombre"
etiqueta = "Nombre"
tipo = "text"

[[funcion.parametro]]
nombre = "seguridad"
etiqueta = "Seguridad"
tipo = "enum"
opciones = [["11", "WPA2/WPA3"], ["3", "WPA2"], ["0", "Abierta"]]

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
    assert_eq!(
        (visible.booleano.si.as_str(), visible.booleano.no.as_str()),
        ("1", "0")
    );
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
    falla_con(&fallos(&ip, ""), "wifi.avanzado", "maximo");
    // Un texto declarado como booleano.
    let booleano = WIFI.replace(
        "lee = \"nombre\"\ntipo = \"texto\"",
        "lee = \"nombre\"\ntipo = \"booleano\"",
    );
    falla_con(&fallos(&booleano, ""), "wifi.red", "nombre");
    // Un enum de tres opciones no es un booleano.
    let tres = WIFI.replace(
        "lee = \"seguridad\"\ntipo = \"opcion\"",
        "lee = \"seguridad\"\ntipo = \"booleano\"",
    );
    falla_con(&fallos(&tres, ""), "wifi.red", "seguridad");
}

#[test]
fn rechaza_secreto_mal_declarado() {
    // Parámetro secreto en un campo de texto.
    let visible = WIFI.replace(
        "lee = \"key_phrase\"\ntipo = \"secreto\"\nvalidar",
        "lee = \"key_phrase\"\ntipo = \"texto\"\nvalidar",
    );
    falla_con(&fallos(&visible, ""), "wifi.red", "secreto");
    // Parámetro `text` que la máscara reconoce como secreto.
    let por_mascara = WIFI.replace("parametro = \"nombre\"", "parametro = \"wlPassphrase\"");
    falla_con(&fallos(&por_mascara, ""), "wifi.red", "secreto");
    // Campo secreto sobre un parámetro que no lo es.
    let falso = WIFI.replace(
        "lee = \"nombre\"\ntipo = \"texto\"",
        "lee = \"nombre\"\ntipo = \"secreto\"",
    );
    falla_con(&fallos(&falso, ""), "wifi.red", "secreto");
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
    let sin_basica = WIFI.replace(
        "escribe = { funcion = \"te_x.wl\", parametro = \"wlWpaPsk\" }",
        "escribe = { funcion = \"cbSetX\", parametro = \"key_phrase\" }",
    );
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
    // `wifi.red` escribe con `cbSetX` (medio) y `te_x.wl` (alto).
    assert_eq!(riesgo(c.ajuste("wifi.red").unwrap(), &f), Riesgo::Alto);
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
