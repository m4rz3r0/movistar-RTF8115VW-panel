//! La página `/mejoras` en el HTML del servidor: el título, las dos
//! tarjetas con su insignia, el contador de la barra lateral y, en solo
//! lectura, sin «Revisar y aplicar».

use crate::comun::PanelPrueba;
use panel::instancia::{self, claves};
use serde_json::json;

#[tokio::test]
async fn pagina_mejoras_ssr() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let html = crate::comun::pagina(&p, &cookie, "/mejoras").await;
    assert!(
        html.contains("Mejoras recomendadas para tu router"),
        "{html}"
    );
    assert_eq!(html.matches(r#"class="tarjeta-lote"#).count(), 2, "{html}");
    assert!(html.contains("5 de 11 pendientes"), "{html}");
    assert!(html.contains("3 de 5 pendientes"), "{html}");
    assert!(html.contains("Revisar y aplicar"), "{html}");
    assert!(html.contains("Más consejos"), "{html}");
    // El contador de la barra lateral: seis pasos sin repetir.
    assert!(
        html.contains(r#"aria-label="6 pasos pendientes""#),
        "{html}"
    );

    instancia::escribir(&p.state.db, claves::SOLO_LECTURA, &json!(true))
        .await
        .unwrap();
    let html = crate::comun::pagina(&p, &cookie, "/mejoras").await;
    assert_eq!(html.matches(r#"class="tarjeta-lote"#).count(), 2, "{html}");
    assert!(!html.contains("Revisar y aplicar"), "{html}");
}
