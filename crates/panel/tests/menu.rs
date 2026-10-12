//! Pruebas del menú (HTML del servidor): orden de las secciones, el nombre de
//! usuario como enlace a «Ajustes del panel» y la sección marcada en sus
//! subrutas.

mod common;

use common::{PANEL_USER, body_text, get_con_cookie, panel, send, sesion};

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

/// La navegación principal de la página `ruta`.
async fn pagina(ruta: &str) -> String {
    let p = panel().await;
    let cookie = sesion(&p).await;
    let resp = get_con_cookie(&p.app, ruta, &cookie).await;
    assert_eq!(resp.status(), 200, "{ruta}");
    body_text(resp).await
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
            "/recomendaciones/preferencias",
            "/historial",
            "/explorador",
            "/router/conexion"
        ]
    );
    assert!(!html.contains(r#"href="/prestablecidos""#), "{html}");
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
        ("/router/conexion", "/router/conexion"),
        (
            "/recomendaciones/preferencias",
            "/recomendaciones/preferencias",
        ),
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

#[tokio::test]
async fn recomendaciones_lleva_a_preferencias() {
    let p = panel().await;
    let cookie = sesion(&p).await;
    // Como la pide el navegador.
    let resp = send(
        &p.app,
        axum::http::Request::get("/recomendaciones")
            .header("cookie", &cookie)
            .header("accept", "text/html")
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(resp.status(), 302);
    assert_eq!(resp.headers()["location"], "/recomendaciones/preferencias");
}
