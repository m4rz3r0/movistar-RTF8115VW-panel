//! La barra de los temas: las pestañas y, a la derecha, la lectura con el
//! icono de volver a leer (sin franja propia ni selector aparte).

use crate::comun::{PanelPrueba, pagina, partir, sesion};
use regex::Regex;

#[tokio::test]
async fn pestanas_y_lectura_en_una_barra() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let html = pagina(&p, &cookie, "/ajustes/wifi").await;
    let (visible, _) = partir(&html, "<script", "</script>");
    let barra = Regex::new(r#"(?s)<div class="temas-barra">.*?</nav>(.*?)</button>"#).unwrap();
    let c = barra.captures(&visible).expect("la barra de los temas");
    assert!(c[0].contains(r#"aria-current="page""#), "{}", &c[0]);
    // La lectura va en la barra; la pinta el navegador al hidratar (hasta
    // entonces, oculta).
    let lectura = &c[1];
    assert!(lectura.contains(r#"class="tema-lectura""#), "{lectura}");
    let boton = Regex::new(r#"<button[^>]*class="recargar"[^>]*>"#).unwrap();
    let b = boton
        .find(lectura)
        .expect("el icono de volver a leer")
        .as_str();
    assert!(b.contains(r#"aria-label="Volver a leer""#), "{b}");
    assert!(b.contains(r#"title="Volver a leer""#), "{b}");
    assert!(b.contains(r#"type="button""#), "{b}");
    // Ni el selector de temas de antes ni una segunda lectura fuera.
    assert!(!visible.contains(r#"id="ajustes-tema""#), "{visible}");
    assert_eq!(visible.matches("tema-lectura").count(), 1, "{visible}");
}
