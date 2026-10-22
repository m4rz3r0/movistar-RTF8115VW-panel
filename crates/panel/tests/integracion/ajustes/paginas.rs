//! Páginas de los temas: sin nombres de la API a la vista, secretos y textos.

use super::*;

// ------------------------------------------------- páginas /ajustes/<tema>

/// La página `ruta` como la pide el navegador: estado y HTML.
async fn pagina(p: &Panel, cookie: &str, ruta: &str) -> (u16, String, String) {
    let resp = send(
        &p.app,
        axum::http::Request::get(ruta)
            .header("cookie", cookie)
            .header("accept", "text/html")
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await;
    let status = resp.status().as_u16();
    let location = resp
        .headers()
        .get(header::LOCATION)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    (status, location, body_text(resp).await)
}

/// Texto que se lee en pantalla (o en voz alta) de un trozo de HTML: el de
/// fuera de las etiquetas y los atributos que se leen.
fn texto_legible(html: &str) -> String {
    let atributos =
        regex::Regex::new(r#"\b(?:aria-label|title|placeholder|alt)="([^"]*)""#).unwrap();
    let mut out: String = atributos
        .captures_iter(html)
        .map(|c| format!("{} ", &c[1]))
        .collect();
    let etiquetas = regex::Regex::new(r"<[^>]*>").unwrap();
    out.push_str(&etiquetas.replace_all(html, " "));
    out
}

/// Nombres de parámetro del catálogo de ajustes (lecturas, fijos, campos y
/// sus `ademas`).
fn parametros_del_catalogo() -> Vec<String> {
    let c = ajustes::CatalogoAjustes::load().unwrap();
    let mut nombres = std::collections::BTreeSet::new();
    for a in c.todos() {
        nombres.extend(a.leer.params.iter().map(|(k, _)| k.clone()));
        nombres.extend(a.fijos.keys().cloned());
        for x in &a.campos {
            nombres.insert(x.lee.clone());
            nombres.extend(x.escribe.iter().map(|e| e.parametro.clone()));
            for ad in &x.ademas {
                nombres.insert(ad.parametro.clone());
                nombres.insert(ad.lee.clone());
            }
        }
    }
    nombres.into_iter().collect()
}

/// Palabras corrientes que también son parámetros (las mismas que deja
/// pasar `ajustes::catalogo_real`).
const FORMAS_CORRIENTES: [&str; 9] = [
    "account",
    "Account",
    "all",
    "All",
    "beamforming",
    "isolation",
    "password",
    "Password",
    "WMM",
];

#[tokio::test]
async fn pagina_tema_sin_nombres_de_api_fuera_de_detalles() {
    // La regla de puertos de las fixtures se llama «desc-1» (el seudónimo
    // del simulador), que se lee como el parámetro `desc`: aquí lleva otro
    // nombre que el simulador admite («Default»), que no es de la API.
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("puertos-nombre"))
        .arrancar_con_gestor()
        .await;
    let funcion = regex::Regex::new(r"\bcb[A-Z]\w+").unwrap();
    let pagina_api = regex::Regex::new(r"\bte_\w+\.(asp|cmd|cgi|wl)\b").unwrap();
    let parametros: Vec<regex::Regex> = parametros_del_catalogo()
        .iter()
        .map(|n| regex::Regex::new(&format!(r"(?i)\b{}\b", regex::escape(n))).unwrap())
        .collect();
    // En los temas en rejilla, los detalles técnicos van en el detalle
    // abierto de un elemento (y, cerrado, ni eso).
    let en_rejilla = ["wifi", "internet", "seguridad", "telefono", "sistema"];
    for tema in [
        "wifi",
        "wifi/red-24",
        "puertos",
        "red_local",
        "internet",
        "internet/ddns",
        "seguridad/gestion-remota",
        "seguridad/usuarios",
        "telefono/linea",
        "sistema/hora",
    ] {
        let ruta = format!("/ajustes/{tema}");
        let (status, _, html) = pagina(&p, &cookie, &ruta).await;
        assert_eq!(status, 200, "{ruta}: {html}");
        let (sin_detalles, detalles) =
            partir(&html, r#"<details class="detalles-tecnicos""#, "</details>");
        assert!(
            en_rejilla.contains(&tema) || !detalles.is_empty(),
            "{ruta}: sin detalles técnicos"
        );
        // Los datos de la hidratación (`<script>`) no se pintan: llevan los
        // detalles técnicos para hidratar sus bloques plegados.
        let (visible, _) = partir(&sin_detalles, "<script", "</script>");
        let api: Vec<&str> = funcion
            .find_iter(&visible)
            .chain(pagina_api.find_iter(&visible))
            .map(|m| m.as_str())
            .collect();
        assert!(
            api.is_empty(),
            "{ruta}: {api:?} fuera de los detalles técnicos"
        );
        let texto = texto_legible(&visible);
        for p in &parametros {
            for m in p.find_iter(&texto) {
                assert!(
                    FORMAS_CORRIENTES.contains(&m.as_str()),
                    "{ruta}: parámetro «{}» en el texto",
                    m.as_str()
                );
            }
        }
        if tema == "wifi/red-24" {
            assert!(
                detalles.iter().any(|d| d.contains("cbSetWifiAccPt")),
                "{ruta}: {detalles:#?}"
            );
        }
    }
}

/// Desde la Tarea 11 (decisión del usuario), la página lleva el valor de sus
/// secretos para el campo censurado con el ojo: solo en el `value` de ese
/// campo (y en los datos de la página), nunca como texto a la vista.
#[tokio::test]
async fn pagina_tema_con_el_secreto_solo_en_su_campo() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let clave = "clave-conocida-del-sim-7";
    escribir_red_24(&p, &[("key_phrase", clave)]).await;
    let (status, _, html) = pagina(&p, &cookie, "/ajustes/wifi").await;
    assert_eq!(status, 200, "{html}");
    assert!(html.contains("Red WiFi 2,4 GHz"), "{html}");
    let (fuera, _) = partir(&html, "<script", "</script>");
    let en_value = format!("value=\"{clave}\"");
    assert!(fuera.contains(&en_value), "en su campo");
    assert_eq!(
        fuera.matches(clave).count(),
        fuera.matches(&en_value).count(),
        "solo en el value de su campo"
    );
}

#[tokio::test]
async fn ajustes_lleva_a_internet() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for ruta in ["/ajustes", "/ajustes/"] {
        let (status, location, body) = pagina(&p, &cookie, ruta).await;
        assert_eq!(status, 302, "{ruta}: {body}");
        assert_eq!(location, "/ajustes/internet", "{ruta}");
    }
}

#[tokio::test]
async fn tema_desconocido_no_encontrada() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, _, html) = pagina(&p, &cookie, "/ajustes/no-existe").await;
    assert_eq!(status, 404, "{html}");
    assert!(html.contains("Página no encontrada"), "{html}");
    assert!(p.sim.control.writes().is_empty());
}

/// Con el espejo apagado, la toma se ve como «Ninguna» (no como un valor
/// no reconocido) y no cuenta como elegida al encenderlo.
#[tokio::test]
async fn espejo_apagado_es_ninguna() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let tema = leer(&p, &cookie, "sistema").await;
    let puerto = campo(ajuste(&tema, "sistema.diagnostico"), "puerto");
    assert_eq!(puerto["valor"], json!({ "Opcion": "ninguna" }), "{puerto}");
    assert_eq!(puerto["sin_eleccion"], true, "{puerto}");
    let (status, body) = preparar_ajuste(
        &p,
        &cookie,
        "sistema.diagnostico",
        json!({ "espejo": "true", "puerto": "ninguna" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("Hace falta elegirlo"), "{body}");
    let (_, _, html) = pagina(&p, &cookie, "/ajustes/sistema").await;
    assert!(!html.contains("Valor no reconocido (0)"), "{html}");
}

/// Los temas en rejilla no esconden nada bajo «Más opciones»: cada
/// elemento tiene su tarjeta (también Sistema, sin ajustes básicos, y lo
/// oculto de Teléfono). Puertos, que aún no va en rejilla, sí lo pliega.
#[tokio::test]
async fn rejilla_sin_mas_opciones() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for (ruta, tarjeta) in [
        ("/ajustes/wifi", "Red WiFi 2,4 GHz"),
        ("/ajustes/internet", "Relé DNS"),
        ("/ajustes/seguridad", "Gestión remota del operador"),
        ("/ajustes/telefono", "Servidores de teléfono del operador"),
        ("/ajustes/sistema", "Hora del router"),
    ] {
        let (_, _, html) = pagina(&p, &cookie, ruta).await;
        assert!(html.contains("rejilla-elementos"), "{ruta}: {html}");
        assert!(html.contains(tarjeta), "{ruta}: {html}");
        assert!(
            !html.contains(r#"<details class="mas-opciones""#),
            "{ruta}: {html}"
        );
    }
    let (_, _, html) = pagina(&p, &cookie, "/ajustes/puertos").await;
    assert!(html.contains(r#"<details class="mas-opciones""#), "{html}");
}

/// La nota de «Oculto» va una sola vez por página en los temas que no van
/// en rejilla (Puertos); en el detalle de uno en rejilla, bajo el título de
/// la columna de lo oculto.
#[tokio::test]
async fn nota_de_oculto_una_vez() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (_, _, html) = pagina(&p, &cookie, "/ajustes/telefono/servidores").await;
    let (visible, _) = partir(&html, "<script", "</script>");
    assert_eq!(
        visible.matches("No aparece en la web del operador").count(),
        1,
        "{visible}"
    );
    assert!(
        visible.contains(r#"class="detalle-nota-oculto""#),
        "{visible}"
    );
    let (_, _, html) = pagina(&p, &cookie, "/ajustes/puertos").await;
    let (visible, _) = partir(&html, "<script", "</script>");
    assert!(visible.matches("chip-oculto").count() >= 1, "{visible}");
    // Cada etiqueta «Oculto» es un enlace a la nota, que la describe (un
    // lector de pantalla no siempre anuncia la descripción de un `<span>`).
    let chip = "<a href=\"#ajustes-nota-oculto\" aria-describedby=\"ajustes-nota-oculto\" \
                class=\"chip chip-nivel chip-oculto\"";
    assert_eq!(
        visible.matches(chip).count(),
        visible.matches("chip-oculto").count(),
        "{visible}"
    );
    assert!(visible.contains("id=\"ajustes-nota-oculto\""), "{visible}");
    assert_eq!(
        visible.matches("No aparece en la web del operador").count(),
        1,
        "{visible}"
    );
}

/// La tarjeta deja ver su recomendación principal, para quién es, sin
/// abrir el detalle; en el detalle, la opción del selector no repite la
/// marca.
#[tokio::test]
async fn recomendacion_visible_en_la_tarjeta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (_, _, html) = pagina(&p, &cookie, "/ajustes/wifi").await;
    let (visible, _) = partir(&html, "<script", "</script>");
    let (_, recos) = partir(&visible, r#"<div class="tarjeta-rec"#, "</div>");
    assert!(
        recos.iter().any(|r| r
            .contains("Recomendado si conectas tus dispositivos escribiendo la contraseña.")
            && r.contains("Quitar WPS")),
        "{recos:#?}"
    );
    // Solo para quién es: a favor y en contra, en el detalle.
    assert!(recos.iter().all(|r| !r.contains("A favor:")), "{recos:#?}");
    let (_, _, html) = pagina(&p, &cookie, "/ajustes/wifi/red-24").await;
    let (visible, _) = partir(&html, "<script", "</script>");
    assert!(visible.contains("A favor:"), "en el detalle");
    assert!(
        !visible.contains("(recomendada)"),
        "marca repetida en el selector"
    );
}

/// La descripción de la opción elegida forma parte de la descripción del
/// `<select>` (`aria-describedby`).
#[tokio::test]
async fn descripcion_de_la_opcion_en_el_selector() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (_, _, html) = pagina(&p, &cookie, "/ajustes/wifi/red-5").await;
    let (visible, _) = partir(&html, "<script", "</script>");
    let describe = regex::Regex::new(r#"<select[^>]*aria-describedby="([^"]*)""#).unwrap();
    let mut vistos = 0;
    for c in describe.captures_iter(&visible) {
        let opcion = c[1]
            .split(' ')
            .find(|id| id.ends_with("-opcion"))
            .unwrap_or_else(|| panic!("sin la descripción de la opción: {}", &c[1]));
        assert!(visible.contains(&format!(r#"id="{opcion}""#)), "{opcion}");
        vistos += 1;
    }
    assert!(vistos > 0, "sin <select> en WiFi");
}

/// La tarjeta del elemento `id` (`red-24`) de la página, sin los datos de
/// la hidratación.
fn tarjeta<'h>(html: &'h str, id: &str) -> &'h str {
    let marca = format!("aria-labelledby=\"elemento-{id}-titulo\"");
    let desde = &html[html.find(&marca).unwrap_or_else(|| panic!("{id}"))..];
    &desde[..desde.find("</article>").unwrap()]
}

/// Cada red dice en su cabecera si está encendida o apagada, y la radio
/// encendida sin ninguna red encendida lo avisa (en es y en en). En las
/// fixtures, como en el router real, todas las redes están apagadas y las
/// radios, encendidas: aquí se enciende la de 5 GHz.
#[tokio::test]
async fn cada_red_dice_si_esta_encendida() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.state
        .hgu
        .write(
            "wifi.asp",
            "cbSetWifiAccPt",
            &[("dpm_cfg_wifi_acc_pt_id", "1"), ("adm_state", "1")],
        )
        .await
        .unwrap();
    let casos = [
        ("es", "Apagada", "Encendida", "ninguna de sus redes emite"),
        ("en", "Off", "On", "none of its networks is broadcasting"),
    ];
    for (idioma, apagada, encendida, nota) in casos {
        let cookie = format!("{cookie}; rp_lang={idioma}");
        let (status, _, html) = pagina(&p, &cookie, "/ajustes/wifi").await;
        assert_eq!(status, 200, "{html}");
        // Gris si está apagada; verde si emite.
        let insignia = |clase: &str, texto: &str| format!(r#"class="ins {clase}">{texto}</span>"#);
        let red_24 = tarjeta(&html, "red-24");
        assert!(
            red_24.contains(&insignia("ins-off", apagada)),
            "{idioma}: {red_24}"
        );
        let red_5 = tarjeta(&html, "red-5");
        assert!(
            red_5.contains(&insignia("ins-ok", encendida)),
            "{idioma}: {red_5}"
        );
        assert!(tarjeta(&html, "radio-24").contains(nota), "{idioma}");
        assert!(!tarjeta(&html, "radio-5").contains(nota), "{idioma}");
        assert!(!tarjeta(&html, "red-24").contains(nota), "{idioma}");
    }
}

/// Un punto de acceso que el panel no muestra (el 4, en la radio de
/// 2,4 GHz) encendido: la radio no dice que no emite ninguna WiFi, sino
/// que emite una red que no se ve aquí, con el enlace a la Biblioteca.
#[tokio::test]
async fn red_oculta_encendida_se_avisa() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.state
        .hgu
        .write(
            "wifi.asp",
            "cbSetWifiAccPt",
            &[("dpm_cfg_wifi_acc_pt_id", "4"), ("adm_state", "1")],
        )
        .await
        .unwrap();
    let casos = [
        (
            "es",
            "La radio emite una red que el panel no muestra.",
            "ninguna de sus redes",
        ),
        (
            "en",
            "The radio broadcasts one network that the panel does not show.",
            "none of its networks",
        ),
    ];
    for (idioma, ocultas, ninguna) in casos {
        let cookie = format!("{cookie}; rp_lang={idioma}");
        let (status, _, html) = pagina(&p, &cookie, "/ajustes/wifi").await;
        assert_eq!(status, 200, "{html}");
        let radio = tarjeta(&html, "radio-24");
        assert!(radio.contains(ocultas), "{idioma}: {radio}");
        assert!(
            radio.contains("href=\"/explorador/wifi\""),
            "{idioma}: {radio}"
        );
        assert!(!radio.contains(ninguna), "{idioma}: {radio}");
        assert!(tarjeta(&html, "radio-5").contains(ninguna), "{idioma}");
    }
}

/// Un elemento que no existe en la dirección: en un tema en rejilla, se
/// dice; en uno que no va en rejilla (no tiene elementos en la dirección),
/// «página no encontrada».
#[tokio::test]
async fn elemento_desconocido_en_la_direccion() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, _, html) = pagina(&p, &cookie, "/ajustes/wifi/nada").await;
    assert_eq!(status, 200, "{html}");
    assert!(
        html.contains("No hay nada que se llame «nada» en esta pestaña."),
        "{html}"
    );
    assert!(!html.contains(r#"class="detalle""#), "{html}");
    let (status, _, html) = pagina(&p, &cookie, "/ajustes/puertos/x").await;
    assert_eq!(status, 404, "{html}");
    assert!(html.contains("Página no encontrada"), "{html}");
}

/// En el detalle, lo que solo se consulta: la contraseña del router lleva
/// el enlace a donde se cambia y las cuentas van en una tabla; ninguno de
/// los dos tiene «Guardar cambios». Lo que se edita (la hora), sí.
#[tokio::test]
async fn detalle_de_lo_que_solo_se_consulta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let guardar = "Guardar cambios";
    let (status, _, html) = pagina(&p, &cookie, "/ajustes/seguridad/contrasena-router").await;
    assert_eq!(status, 200, "{html}");
    let (visible, _) = partir(&html, "<script", "</script>");
    assert!(visible.contains(r#"class="detalle""#), "{visible}");
    assert!(visible.contains("#router-cambiar-contrasena"), "{visible}");
    assert!(!visible.contains(guardar), "{visible}");
    let (_, _, html) = pagina(&p, &cookie, "/ajustes/seguridad/usuarios").await;
    let (visible, _) = partir(&html, "<script", "</script>");
    assert!(visible.contains(r#"<table class="tabla">"#), "{visible}");
    assert!(visible.contains("Administrador"), "{visible}");
    assert!(!visible.contains(guardar), "{visible}");
    let (_, _, html) = pagina(&p, &cookie, "/ajustes/sistema/hora").await;
    let (visible, _) = partir(&html, "<script", "</script>");
    assert!(visible.contains(guardar), "{visible}");
}
