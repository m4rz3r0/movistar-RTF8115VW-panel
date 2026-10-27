//! Pruebas del menú (HTML del servidor): orden de las secciones, el nombre de
//! usuario como enlace a «Ajustes del panel» y la sección marcada en sus
//! subrutas.

use crate::comun::{PANEL_USER, PanelPrueba, send, sesion};

/// El elemento `<etiqueta …>` que contiene `marca` en su apertura, hasta su
/// cierre (sin anidar la misma etiqueta).
fn elemento<'a>(html: &'a str, etiqueta: &str, marca: &str) -> &'a str {
    let abre = format!("<{etiqueta}");
    let cierra = format!("</{etiqueta}>");
    let mut desde = 0;
    while let Some(i) = html[desde..].find(&abre) {
        let inicio = desde + i;
        let fin_apertura = inicio + html[inicio..].find('>').expect("etiqueta sin cerrar");
        if html[inicio..fin_apertura].contains(marca) {
            let fin = fin_apertura + html[fin_apertura..].find(&cierra).expect("sin cierre");
            return &html[inicio..fin + cierra.len()];
        }
        desde = fin_apertura;
    }
    panic!("no hay <{etiqueta}> con {marca}: {html}");
}

/// Aperturas de los `<a …>` de `trozo`.
fn enlaces(trozo: &str) -> Vec<&str> {
    trozo
        .match_indices("<a ")
        .map(|(i, _)| &trozo[i..i + trozo[i..].find('>').unwrap()])
        .collect()
}

/// Valor del atributo `nombre` de una apertura de etiqueta.
fn atributo<'a>(apertura: &'a str, nombre: &str) -> Option<&'a str> {
    let marca = format!(" {nombre}=\"");
    let i = apertura.find(&marca)? + marca.len();
    Some(&apertura[i..i + apertura[i..].find('"')?])
}

/// El HTML de la página `ruta`. Con el gestor de cambios: el Historial lo
/// necesita.
async fn pagina(ruta: &str) -> String {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    crate::comun::pagina(&p, &cookie, ruta).await
}

fn nav_principal(html: &str) -> &str {
    elemento(html, "nav", r#"aria-label="Principal""#)
}

/// El enlace de la zona de sesión (el nombre de usuario).
fn enlace_usuario(html: &str) -> &str {
    let a = elemento(html, "a", r#"class="sesion-usuario""#);
    assert_eq!(atributo(a, "href"), Some("/panel"), "{a}");
    a
}

#[tokio::test]
async fn menu_en_orden() {
    let html = pagina("/").await;
    let hrefs: Vec<&str> = enlaces(nav_principal(&html))
        .into_iter()
        .filter_map(|a| atributo(a, "href"))
        .collect();
    assert_eq!(
        hrefs,
        [
            "/",
            "/ajustes/internet",
            "/mejoras",
            "/historial",
            "/explorador",
            "/router/conexion"
        ]
    );
    assert!(!html.contains(r#"href="/prestablecidos""#), "{html}");
    // Recomendaciones volverá con sus tarjetas en una versión futura.
    assert!(!html.contains(r#"href="/recomendaciones"#), "{html}");
}

#[tokio::test]
async fn nombre_de_usuario_abre_ajustes_del_panel() {
    let html = pagina("/").await;
    let enlace = enlace_usuario(&html);
    assert!(enlace.contains(PANEL_USER), "{enlace}");
    assert!(
        enlace.contains("Ajustes del panel"),
        "texto para lectores: {enlace}"
    );
    assert!(
        !nav_principal(&html).contains(r#"href="/panel""#),
        "la navegación principal no enlaza «Ajustes del panel»"
    );
    assert!(!html.contains(r#"aria-label="Usuario""#), "{html}");
}

#[tokio::test]
async fn seccion_marcada_en_subrutas() {
    let actual = |a: &str| atributo(a, "aria-current") == Some("page");
    for (ruta, href) in [
        ("/", "/"),
        ("/ajustes/wifi", "/ajustes/internet"),
        ("/mejoras", "/mejoras"),
        ("/historial", "/historial"),
        ("/explorador/lan_dhcp", "/explorador"),
        ("/router/conexion", "/router/conexion"),
    ] {
        let html = pagina(ruta).await;
        let marcados: Vec<&str> = enlaces(nav_principal(&html))
            .into_iter()
            .filter(|a| actual(a))
            .filter_map(|a| atributo(a, "href"))
            .collect();
        assert_eq!(marcados, [href], "{ruta}");
        assert!(!actual(enlace_usuario(&html)), "{ruta}");
    }
    let html = pagina("/panel").await;
    assert!(actual(enlace_usuario(&html)), "{html}");
    assert!(
        !enlaces(nav_principal(&html)).into_iter().any(actual),
        "en /panel no se marca ninguna sección del menú"
    );
}

/// Las rutas de Recomendaciones (todavía sin contenido) llevan al
/// Estado: los enlaces guardados no se rompen.
#[tokio::test]
async fn recomendaciones_lleva_al_estado() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    for ruta in ["/recomendaciones", "/recomendaciones/preferencias"] {
        // Como la pide el navegador.
        let resp = send(
            &p.app,
            axum::http::Request::get(ruta)
                .header("cookie", &cookie)
                .header("accept", "text/html")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), 302, "{ruta}");
        assert_eq!(resp.headers()["location"], "/", "{ruta}");
    }
}
