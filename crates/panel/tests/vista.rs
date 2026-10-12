//! Modos de visualización (texto, gráfica, mixto): preferencia por usuario,
//! `<html data-vista>` y lo que pinta la página de estado en cada modo.

mod common;

use common::{
    Panel, body_text, get_con_cookie, llamar, login_body, ok, panel, post_form, send, sesion,
    set_cookie,
};
use panel::poller::{Poller, PollerConfig, PollerHandle};
use serde_json::json;
use std::time::{Duration, Instant};

/// Arranca el sondeo con intervalos cortos contra el simulador.
fn arrancar_sondeo(p: &mut Panel) -> PollerHandle {
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, common::USER, common::PASS);
    cfg.timeout = Duration::from_millis(200);
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(50),
            slow: Duration::from_millis(200),
            backoff_max: Duration::from_millis(200),
            ..PollerConfig::default()
        },
    )
}

async fn esperar(que: &str, mut cond: impl FnMut() -> bool) {
    let start = Instant::now();
    while !cond() {
        assert!(start.elapsed() < Duration::from_secs(5), "no llega: {que}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Panel con sesión y con datos del router (la página de estado pinta los
/// indicadores solo cuando hay un dato).
async fn panel_con_datos() -> (Panel, String, PollerHandle) {
    let mut p = panel().await;
    let cookie = sesion(&p).await;
    let h = arrancar_sondeo(&mut p);
    esperar("primer dato", || h.healthy()).await;
    (p, cookie, h)
}

async fn pagina(p: &Panel, ruta: &str, cookie: &str) -> String {
    body_text(get_con_cookie(&p.app, ruta, cookie).await).await
}

async fn guardar(p: &Panel, cookie: &str, modo: &str) {
    ok(p, "guardar_vista", json!({ "modo": modo }), cookie).await;
}

/// Los `<svg` del contenido de la página: lo que va en `<main>` y los trozos
/// que el servidor manda después de `</body>` (el contenido de los
/// `Suspense`/`Transition`, que van dentro de `<main>`), sin el menú ni el
/// logo. Exige que el contenido de Estado esté ahí (si no, no probaría nada).
fn svgs_del_contenido(html: &str) -> Vec<String> {
    let inicio = html.find("<main").expect("<main>");
    let fin = inicio + html[inicio..].find("</main>").expect("</main>");
    let tras_body = html.find("</body>").expect("</body>");
    let contenido = format!("{}{}", &html[inicio..fin], &html[tras_body..]);
    assert!(
        contenido.contains("historico-texto"),
        "sin el contenido de Estado: {html}"
    );
    contenido
        .match_indices("<svg")
        .map(|(i, _)| {
            let resto = &contenido[i..];
            resto[..resto.find('>').unwrap_or(resto.len())].to_owned()
        })
        .collect()
}

#[tokio::test]
async fn vista_por_defecto_mixta() {
    let (p, cookie, _h) = panel_con_datos().await;
    let v = ok(&p, "leer_vista", json!({}), &cookie).await;
    assert_eq!(v, json!("mixto"));
    let html = pagina(&p, "/", &cookie).await;
    assert!(html.contains(r#"data-vista="mixto""#), "{html}");
    assert!(html.contains(r#"class="minigrafica""#), "{html}");
    assert!(html.contains(r#"class="grafica""#), "{html}");
    // Lo de hoy: indicadores antes que el histórico.
    let indicadores = html.find("rejilla-indicadores").expect("indicadores");
    let graficas = html.find("graficas-titulo").expect("gráficas");
    assert!(indicadores < graficas, "{html}");
}

#[tokio::test]
async fn vista_por_usuario() {
    let (p, cookie, _h) = panel_con_datos().await;
    panel::auth::create_user(&p.state.db, "otra", "otra contraseña larga")
        .await
        .unwrap();
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body("otra", "otra contraseña larga"),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 200);
    let otra = set_cookie(&resp)
        .split(';')
        .next()
        .unwrap()
        .trim()
        .to_owned();

    guardar(&p, &cookie, "texto").await;
    guardar(&p, &otra, "grafica").await;
    assert_eq!(
        ok(&p, "leer_vista", json!({}), &cookie).await,
        json!("texto")
    );
    assert_eq!(
        ok(&p, "leer_vista", json!({}), &otra).await,
        json!("grafica")
    );
    let html = pagina(&p, "/", &cookie).await;
    assert!(html.contains(r#"data-vista="texto""#), "{html}");
    let html = pagina(&p, "/", &otra).await;
    assert!(html.contains(r#"data-vista="grafica""#), "{html}");
    // Se conserva en otras páginas y en una sesión nueva.
    let html = pagina(&p, "/panel", &cookie).await;
    assert!(html.contains(r#"data-vista="texto""#), "{html}");
    let nueva = common::sesion_de(&p).await;
    assert_eq!(
        ok(&p, "leer_vista", json!({}), &nueva).await,
        json!("texto")
    );
    // Volver a «mixto» lo deja como al principio.
    guardar(&p, &cookie, "mixto").await;
    let html = pagina(&p, "/", &cookie).await;
    assert!(html.contains(r#"data-vista="mixto""#), "{html}");
}

#[tokio::test]
async fn vista_texto_sin_svg() {
    let (p, cookie, _h) = panel_con_datos().await;
    guardar(&p, &cookie, "texto").await;
    let html = pagina(&p, "/", &cookie).await;
    assert!(html.contains(r#"data-vista="texto""#), "{html}");
    let sueltos = svgs_del_contenido(&html);
    assert!(
        sueltos.is_empty(),
        "SVG en la página de estado: {sueltos:#?}"
    );
    // El histórico, en una tabla con último, mínimo, media y máximo.
    let tabla = &html[html
        .find("historico-texto")
        .unwrap_or_else(|| panic!("tabla del histórico: {html}"))..];
    for texto in [
        "Último",
        "Mínimo",
        "Media",
        "Máximo",
        "Bajada",
        "Subida",
        "RAM usada",
    ] {
        assert!(tabla.contains(texto), "{texto}: {html}");
    }
    // Cada fila, con su encabezado (el nombre de la serie).
    assert!(tabla.contains(r#"<th scope="row""#), "{html}");
    assert!(tabla.contains(">Bajada</th>"), "{html}");
    // Los indicadores, como pares nombre/valor.
    let lista = &html[html
        .find(r#"class="indicadores-texto""#)
        .unwrap_or_else(|| panic!("indicadores: {html}"))..];
    assert!(lista.contains("<dt>Internet"), "{html}");
    // En inglés, también.
    let html = body_text(get_con_cookie(&p.app, "/", &format!("{cookie}; rp_lang=en")).await).await;
    assert!(svgs_del_contenido(&html).is_empty(), "{html}");
    let tabla = &html[html
        .find("historico-texto")
        .unwrap_or_else(|| panic!("tabla del histórico: {html}"))..];
    for texto in ["Latest", "Minimum", "Average", "Maximum", "Download"] {
        assert!(tabla.contains(texto), "{texto}: {html}");
    }
}

#[tokio::test]
async fn vista_grafica_sin_minigraficas() {
    let (p, cookie, _h) = panel_con_datos().await;
    guardar(&p, &cookie, "grafica").await;
    let html = pagina(&p, "/", &cookie).await;
    assert!(html.contains(r#"data-vista="grafica""#), "{html}");
    assert!(!html.contains("minigrafica"), "{html}");
    assert!(html.contains(r#"class="grafica""#), "{html}");
    // Las gráficas primero y a todo el ancho; luego los indicadores,
    // compactos.
    let graficas = html.find("graficas-titulo").expect("gráficas");
    let indicadores = html.find("rejilla-indicadores").expect("indicadores");
    assert!(graficas < indicadores, "{html}");
    assert!(html.contains("rejilla-estado una-columna"), "{html}");
    assert!(html.contains("rejilla-indicadores compactos"), "{html}");
    // Compactos, pero con su subtítulo (la IP pública, el máximo de 1 h…).
    assert!(html.contains("indicador-sub"), "{html}");
}

#[tokio::test]
async fn guardar_vista_exige_sesion() {
    let p = panel().await;
    let (status, body) = llamar(&p, "guardar_vista", json!({ "modo": "texto" }), None).await;
    assert_eq!(status, 401, "{body}");
    let (status, body) = llamar(&p, "leer_vista", json!({}), None).await;
    assert_eq!(status, 401, "{body}");
    // Un modo que no existe no se guarda.
    let cookie = sesion(&p).await;
    let (status, body) = llamar(
        &p,
        "guardar_vista",
        json!({ "modo": "otro" }),
        Some(&cookie),
    )
    .await;
    assert_ne!(status, 200, "{body}");
    assert_eq!(
        ok(&p, "leer_vista", json!({}), &cookie).await,
        json!("mixto")
    );
}
