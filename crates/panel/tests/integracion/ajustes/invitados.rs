//! La red de invitados: separada de la red de casa (solo se lee) y aislada
//! entre sí (se escribe).

use super::*;

/// «Separada de la red de casa» sale en la red de invitados con su valor y
/// como solo de lectura, con su motivo; el aislamiento sigue editable.
#[tokio::test]
async fn invitados_separada_solo_se_lee() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let tema = leer(&p, &cookie, "wifi").await;
    let a = ajuste(&tema, "wifi.invitados_avanzado");
    let separada = campo(a, "separada");
    assert_eq!(separada["valor"], json!({ "Booleano": true }), "{separada}");
    assert_eq!(
        separada["solo_lectura"], "ajuste-wifi-invitados_avanzado-separada-motivo",
        "{separada}"
    );
    assert!(campo(a, "aislamiento")["solo_lectura"].is_null(), "{a}");
}

/// El motivo dice que es el panel quien aún no sabe escribirlo, no que el
/// router no deje (no está comprobado).
#[test]
fn motivo_de_separada_no_culpa_al_router() {
    let ruta = concat!(env!("CARGO_MANIFEST_DIR"), "/../i18n/locales");
    for (idioma, esperado) in [
        ("es", "El panel todavía no sabe"),
        ("en", "The panel does not yet know"),
    ] {
        let ftl = std::fs::read_to_string(format!("{ruta}/{idioma}/ajustes-wifi.ftl")).unwrap();
        let linea = ftl
            .lines()
            .find(|l| l.starts_with("ajuste-wifi-invitados_avanzado-separada-motivo ="))
            .unwrap();
        assert!(linea.contains(esperado), "{idioma}: {linea}");
    }
}
