//! Tests del diálogo de grupo: resumen, pasos con sus casillas y pestañas, y valores legibles.

use super::pasos::por_pestana;
use super::*;
use crate::ajustes_router::PasoVista;
use crate::ui::explorador::MASCARA;

fn cambio(campo: &str, antes: &str, despues: &str, tipo: TipoCampo) -> CambioLegible {
    CambioLegible {
        campo: campo.into(),
        antes: antes.into(),
        despues: despues.into(),
        secreto: tipo == TipoCampo::Secreto,
        tipo,
    }
}

fn grupo(pasos: Vec<PasoVista>) -> GrupoVista {
    GrupoVista {
        token: "t".into(),
        titulo: "ajuste-wifi-red_24-titulo".into(),
        pasos,
        riesgo: RiesgoCambio::Medio,
        efecto: Some("efecto-wifi_red".into()),
        vigilancia_s: 120,
        detalle: None,
        aviso: None,
        borra: false,
        proveedor_dns: None,
    }
}

fn html(g: GrupoVista) -> String {
    Owner::new().with(|| view! { <DialogoGrupo grupo=g on_cerrar=|_| {} /> }.to_html())
}

/// Una MAC tal cual lleva un `<wbr>` tras cada «:» (se parte solo por
/// ahí); un sí/no traducido, ninguno.
#[test]
fn valor_tal_cual_se_parte_por_sus_separadores() {
    let paso = PasoVista {
        campos: vec![
            cambio(
                "ajuste-red_local-reservas-mac",
                "",
                "02:00:00:00:00:03",
                TipoCampo::Texto,
            ),
            cambio(
                "ajuste-red_local-reservas-activa",
                "false",
                "true",
                TipoCampo::Booleano,
            ),
        ],
        ..PasoVista::default()
    };
    let h = html(grupo(vec![paso]));
    assert!(h.contains("02:<wbr>"), "{h}");
    assert!(h.contains("03<wbr>"), "{h}");
    assert_eq!(h.matches("<wbr>").count(), 6, "solo los de la MAC: {h}");
}

/// El resumen nombra a qué se refiere el cambio si el título no basta
/// (el dispositivo de una IP fija).
#[test]
fn el_resumen_lleva_el_detalle() {
    let mut g = grupo(Vec::new());
    g.titulo = "ajuste-red_local-reservas-titulo".into();
    g.detalle = Some("host-2".into());
    let h = html(g);
    let titulo = traducir("ajuste-red_local-reservas-titulo", None);
    assert!(h.contains(&format!("{titulo}: host-2")), "{h}");
}

/// Al crear o borrar una instancia, un sí/no o una opción sin valor se
/// ve vacío, no como «valor no reconocido».
#[test]
fn sin_valor_es_vacio_en_cualquier_tipo() {
    for tipo in [TipoCampo::Booleano, TipoCampo::Opcion, TipoCampo::Ip] {
        let c = cambio("ajuste-red_local-reservas-activa", "", "true", tipo);
        assert_eq!(texto_valor(&c, ""), t!("ajustes-grupo-vacio"), "{tipo:?}");
    }
}

#[test]
fn resumen_con_etiquetas_y_valores_legibles() {
    let g = grupo(vec![PasoVista {
        indice: 0,
        sin_cambios: false,
        campos: vec![
            cambio(
                "ajuste-wifi-red_24-nombre",
                "casa",
                "casa-nueva",
                TipoCampo::Texto,
            ),
            cambio(
                "ajuste-wifi-red_24-seguridad",
                "99",
                "abierta",
                TipoCampo::Opcion,
            ),
            cambio(
                "ajuste-wifi-red_24-visible",
                "true",
                "false",
                TipoCampo::Booleano,
            ),
            cambio(
                "ajuste-wifi-red_24-contrasena",
                MASCARA,
                MASCARA,
                TipoCampo::Secreto,
            ),
        ],
        titulo: None,
        nota: None,
        ..Default::default()
    }]);
    let h = html(g);
    assert!(h.contains("Revisa el cambio"), "{h}");
    assert!(h.contains("tabindex=\"-1\""), "título enfocable: {h}");
    // Etiquetas del ajuste, no parámetros.
    assert!(
        h.contains(&traducir("ajuste-wifi-red_24-nombre", None)),
        "{h}"
    );
    assert!(
        h.contains(&traducir("ajuste-wifi-red_24-seguridad-abierta", None)),
        "{h}"
    );
    assert!(
        h.contains(&t!("ajustes-valor-desconocido", valor = "99")),
        "{h}"
    );
    assert!(
        h.contains(&t!("comun-si")) && h.contains(&t!("comun-no")),
        "{h}"
    );
    assert!(h.contains(MASCARA), "{h}");
    assert!(h.contains(&traducir("efecto-wifi_red", None)), "{h}");
    assert!(h.contains("2 min"), "plazo de vigilancia: {h}");
    assert!(h.contains(&t!("ajustes-riesgo-medio")), "{h}");
    // Un solo paso: sin casillas.
    assert!(!h.contains("checkbox"), "{h}");
    for api in ["ssid", "authentication", "key_phrase", "cbSet"] {
        assert!(!h.contains(api), "{api}: {h}");
    }
    assert!(!h.contains("style="), "sin estilos en línea: {h}");
}

#[test]
fn pasos_con_casillas_y_ya_esta_asi_deshabilitado() {
    let g = grupo(vec![
        PasoVista {
            indice: 0,
            sin_cambios: false,
            campos: vec![cambio(
                "ajuste-wifi-red_24-nombre",
                "a",
                "b",
                TipoCampo::Texto,
            )],
            titulo: None,
            nota: None,
            ..Default::default()
        },
        PasoVista {
            indice: 1,
            sin_cambios: true,
            campos: vec![],
            titulo: None,
            nota: None,
            ..Default::default()
        },
    ]);
    let h = html(g);
    assert_eq!(h.matches("type=\"checkbox\"").count(), 2, "{h}");
    assert!(h.contains(&t!("ajustes-grupo-ya-esta")), "{h}");
    assert_eq!(
        h.matches(" disabled").count(),
        1,
        "solo el paso sin cambios: {h}"
    );
    assert!(h.contains(&t!("ajustes-grupo-paso", n = 2)), "{h}");
}

/// Un paso con título («TCP», «UDP») lo enseña en vez de «Paso N», con
/// su nota debajo; sin título, sigue siendo «Paso N».
#[test]
fn pasos_con_titulo_y_nota() {
    let paso = |indice, titulo: Option<&str>, nota: Option<&str>| PasoVista {
        indice,
        sin_cambios: false,
        campos: vec![cambio(
            "ajuste-puertos-reglas-protocolo",
            "",
            "udp",
            TipoCampo::Opcion,
        )],
        titulo: titulo.map(str::to_owned),
        nota: nota.map(str::to_owned),
        ..Default::default()
    };
    let h = html(grupo(vec![
        paso(0, Some("ajuste-puertos-reglas-protocolo-tcp"), None),
        paso(
            1,
            Some("ajuste-puertos-reglas-protocolo-udp"),
            Some("ajuste-puertos-reglas-paso-igual"),
        ),
    ]));
    assert!(h.contains(">TCP</label>"), "{h}");
    assert!(h.contains(">UDP</label>"), "{h}");
    assert!(!h.contains(&t!("ajustes-grupo-paso", n = 1)), "{h}");
    assert!(
        h.contains(&traducir("ajuste-puertos-reglas-paso-igual", None)),
        "{h}"
    );
    let h = html(grupo(vec![paso(0, None, None), paso(1, None, None)]));
    assert!(h.contains(&t!("ajustes-grupo-paso", n = 2)), "{h}");
}

/// Encender o apagar una red dice «Apagada → Encendida», no «No → Sí»:
/// los textos de valor del catálogo; un sí/no sin ellos, «Sí» y «No».
#[test]
fn un_si_no_con_sus_textos_de_valor() {
    let c = cambio(
        "ajuste-wifi-red_24-activa",
        "false",
        "true",
        TipoCampo::Booleano,
    );
    assert_eq!(texto_valor(&c, "false"), "Apagada");
    assert_eq!(texto_valor(&c, "true"), "Encendida");
    let c = cambio(
        "ajuste-wifi-red_24-wps",
        "true",
        "false",
        TipoCampo::Booleano,
    );
    assert_eq!(texto_valor(&c, "true"), t!("comun-si"));
    assert_eq!(texto_valor(&c, "false"), t!("comun-no"));
}

#[test]
fn deshacer_un_deshacer_vuelve_a_ser_un_cambio_normal() {
    assert_eq!(siguiente_deshace(None, 5), Some(5));
    assert_eq!(siguiente_deshace(Some(5), 6), None);
    assert_eq!(siguiente_deshace(None, 7), Some(7));
}

#[test]
fn desmarcados_solo_los_pasos_con_cambios() {
    let paso = |indice, sin_cambios| PasoVista {
        indice,
        sin_cambios,
        campos: vec![],
        titulo: None,
        nota: None,
        ..Default::default()
    };
    let pasos = vec![paso(0, false), paso(1, true), paso(2, false)];
    assert_eq!(desmarcados(&pasos, &[true, false, false]), vec![2]);
    assert_eq!(
        desmarcados(&pasos, &[true, false, true]),
        Vec::<usize>::new()
    );
    assert_eq!(marcados_de(&grupo(pasos)), vec![true, false, true]);
}

#[test]
fn marcados_iniciales_respetan_desmarcado() {
    let paso = |indice, sin_cambios, desmarcado| PasoVista {
        indice,
        sin_cambios,
        desmarcado,
        ..Default::default()
    };
    let g = grupo(vec![
        paso(0, false, false),
        paso(1, false, true),
        paso(2, true, false),
    ]);
    assert_eq!(marcados_de(&g), vec![true, false, false]);
}

/// Al cambiar de proveedor, el grupo nuevo conserva lo que se había
/// tocado, paso a paso por su clave, aunque cambie de posición.
#[test]
fn marcas_se_conservan_por_clave() {
    let paso = |indice, clave: &str, desmarcado| PasoVista {
        indice,
        clave: Some(clave.into()),
        desmarcado,
        ..Default::default()
    };
    let viejo = grupo(vec![
        paso(0, "dns_casa", false),
        paso(1, "aviso_operador", false),
        paso(2, "hora_neutra", true),
    ]);
    // El aviso, desmarcado a mano; la hora, marcada a mano.
    let marcas = [true, false, true];
    let mut nuevo = grupo(vec![
        paso(0, "aviso_operador", false),
        paso(1, "gestion_remota", false),
        paso(2, "dns_casa", false),
        paso(3, "hora_neutra", true),
    ]);
    assert_eq!(
        marcas_conservadas(&viejo, &marcas, &nuevo),
        vec![false, true, true, true]
    );
    // Lo que ya no cambia nada, desmarcado.
    nuevo.pasos[2].sin_cambios = true;
    assert_eq!(
        marcas_conservadas(&viejo, &marcas, &nuevo),
        vec![false, true, false, true]
    );
}

#[test]
fn por_pestana_agrupa_seguidos() {
    let paso = |indice, pestana: Option<&str>| PasoVista {
        indice,
        pestana: pestana.map(str::to_owned),
        ..Default::default()
    };
    let red = Some("ajustes-tema-red_local");
    let seg = Some("ajustes-tema-seguridad");
    let pasos = [
        paso(0, red),
        paso(1, seg),
        paso(2, seg),
        paso(3, None),
        paso(4, None),
        paso(5, red),
    ];
    let s = |x: Option<&str>| x.map(str::to_owned);
    assert_eq!(
        por_pestana(&pasos),
        vec![
            (s(red), vec![0]),
            (s(seg), vec![1, 2]),
            (None, vec![3, 4]),
            (s(red), vec![5]),
        ]
    );
}

/// Los pasos de un lote van bajo el nombre de su pestaña; los de un
/// ajuste (sin pestaña), sin encabezado.
#[test]
fn encabezado_por_pestana() {
    let paso = |indice, pestana: Option<&str>| PasoVista {
        indice,
        pestana: pestana.map(str::to_owned),
        ..Default::default()
    };
    let h = html(grupo(vec![
        paso(0, Some("ajustes-tema-red_local")),
        paso(1, Some("ajustes-tema-seguridad")),
        paso(2, Some("ajustes-tema-seguridad")),
    ]));
    assert_eq!(h.matches("<h3").count(), 2, "{h}");
    let red = h.find(&t!("ajustes-tema-red_local")).expect("Red local");
    let seg = h.find(&t!("ajustes-tema-seguridad")).expect("Seguridad");
    assert!(red < seg, "{h}");
    let h = html(grupo(vec![paso(0, None), paso(1, None)]));
    assert!(!h.contains("<h3"), "{h}");
}

/// En un lote, la casilla de cada paso se nombra con lo que hace y su
/// elemento: dos pasos de la misma red no se llaman igual. Fuera de un
/// lote (sin pestaña), con el título, como siempre.
#[test]
fn casillas_de_un_lote_con_nombre_propio() {
    let paso = |indice, nota: &str, pestana: Option<&str>| PasoVista {
        indice,
        titulo: Some("ajuste-wifi-red_5-titulo".into()),
        nota: Some(nota.into()),
        pestana: pestana.map(str::to_owned),
        campos: vec![cambio("c", "a", "b", TipoCampo::Texto)],
        ..Default::default()
    };
    let wifi = Some("ajustes-tema-wifi");
    let h = html(grupo(vec![
        paso(0, "mejoras-paso-wps", wifi),
        paso(1, "mejoras-paso-wpa2", wifi),
    ]));
    let red = t!("ajuste-wifi-red_5-titulo");
    for nota in ["mejoras-paso-wps", "mejoras-paso-wpa2"] {
        let nombre = format!(r#"aria-label="{} · {red}""#, traducir(nota, None));
        assert_eq!(h.matches(&nombre).count(), 1, "{nombre}: {h}");
    }
    let h = html(grupo(vec![
        paso(0, "mejoras-paso-wps", None),
        paso(1, "mejoras-paso-wpa2", None),
    ]));
    assert!(!h.contains(&format!(" · {red}")), "{h}");
}
