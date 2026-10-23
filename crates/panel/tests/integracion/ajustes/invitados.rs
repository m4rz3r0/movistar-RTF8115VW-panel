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
