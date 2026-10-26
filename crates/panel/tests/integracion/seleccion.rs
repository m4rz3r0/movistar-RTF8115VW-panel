//! Selección de tarjetas: una pantalla solo deja marcar si hay alguna acción
//! que aplicar a varios elementos a la vez, y solo en los elementos a los
//! que vale. Se comprueba en el HTML que pinta el servidor.

use crate::comun::{Panel, PanelPrueba, pagina, partir, sesion};

/// Lo visible de `ruta` (sin los scripts de la hidratación).
async fn visible(p: &Panel, cookie: &str, ruta: &str) -> String {
    let html = pagina(p, cookie, ruta).await;
    partir(&html, "<script", "</script>").0
}

#[tokio::test]
async fn sin_acciones_para_varios_no_hay_seleccion() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    // Seguridad tiene una acción, pero solo para la gestión remota (y su
    // tarjeta ya la ofrece): ni marca, ni barra. Igual en los demás temas
    // cuyas acciones valen cada una para un solo elemento.
    for ruta in [
        "/ajustes/seguridad",
        "/ajustes/sistema",
        "/ajustes/internet",
        "/ajustes/telefono",
    ] {
        let html = visible(&p, &cookie, ruta).await;
        assert!(html.contains("tarjeta-el"), "{ruta} sin tarjetas: {html}");
        assert!(
            !html.contains(r#"class="marcar""#),
            "{ruta} deja marcar: {html}"
        );
        assert!(!html.contains("barra-acciones"), "{ruta} con barra: {html}");
    }
}

#[tokio::test]
async fn con_acciones_se_marca_solo_lo_que_las_tiene() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let html = visible(&p, &cookie, "/ajustes/wifi").await;
    // Las redes comparten encender, apagar, quitar el WPS...: se marcan.
    for red in ["Red WiFi 2,4 GHz", "Red WiFi 5 GHz"] {
        assert!(
            html.contains(&format!(r#"aria-label="Marcar {red}""#)),
            "{red}: {html}"
        );
    }
    // Las radios no tienen ninguna acción: sin marca.
    assert!(!html.contains(r#"aria-label="Marcar Radio"#), "{html}");
    assert!(html.contains("barra-acciones"), "{html}");
}
