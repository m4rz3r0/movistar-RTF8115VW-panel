//! Carga del catálogo de ajustes: temas, TOML roto, prefijos e identificadores.

use super::*;

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
