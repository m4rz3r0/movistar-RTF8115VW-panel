//! La página `/ajustes/foto-de-partida` en el HTML del servidor.

use super::*;

/// Texto de la página sin foto.
const SIN_FOTO: &str = "No hay con qué comparar";
const SIN_DIFERENCIAS: &str = "Todo está como al instalar el panel.";
const ENLACE: &str = "href=\"/ajustes/foto-de-partida\"";

#[tokio::test]
async fn foto_de_partida_no_es_un_tema() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    // Sin foto: la página lo explica y los temas no la enlazan.
    let sin = html(&p, &cookie, RUTA).await;
    assert!(sin.contains("Cómo estaba al instalar"), "su título");
    assert!(!sin.contains("Página no encontrada"), "no es un tema");
    assert!(sin.contains(SIN_FOTO), "explica que no hay foto");
    assert!(!sin.contains(SIN_DIFERENCIAS));
    assert!(!html(&p, &cookie, "/ajustes/puertos").await.contains(ENLACE));

    tomar_foto(&p).await;
    let igual = html(&p, &cookie, RUTA).await;
    assert!(igual.contains(SIN_DIFERENCIAS), "sin diferencias");
    assert!(!igual.contains(SIN_FOTO));
    assert!(!igual.contains("<table"), "sin tabla vacía");
    assert!(html(&p, &cookie, "/ajustes/puertos").await.contains(ENLACE));

    cambiar_upnp(&p, &cookie, true).await;
    let cambiada = html(&p, &cookie, RUTA).await;
    assert!(!cambiada.contains(SIN_DIFERENCIAS));
    for texto in [
        "Puertos",
        "Apertura automática de puertos",
        "Al instalar",
        "Ahora",
        "Volver a este valor",
    ] {
        assert!(cambiada.contains(texto), "{texto}");
    }
    // Ningún nombre de la API a la vista.
    let api = regex::Regex::new(r"\bcb[A-Z][A-Za-z]+|adm_state|\w+\.asp\b").unwrap();
    let (fuera, _) = partir(&cambiada, "<script", "</script>");
    assert!(!api.is_match(&fuera), "{:?}", api.find(&fuera));
    // Sin sesión, al login.
    let resp = crate::comun::get(&p.app, RUTA).await;
    assert!(resp.status().is_redirection(), "{}", resp.status());
}

/// En inglés, con sus textos.
#[tokio::test]
async fn foto_de_partida_en_ingles() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let resp = crate::comun::send(
        &p.app,
        axum::http::Request::get(RUTA)
            .header("cookie", format!("{cookie}; rp_lang=en"))
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await;
    let en = crate::comun::body_text(resp).await;
    assert!(en.contains("How it was at installation"), "{en}");
    assert!(en.contains("There is nothing to compare with"));
}
