//! La recomendación de una tarjeta de Ajustes dice de qué lotes de Mejoras
//! forma parte su ajuste (y no lo dice si ya está como lo deja el lote).

use crate::comun::{PanelPrueba, leer, sesion};
use serde_json::{Value, json};

/// La recomendación del elemento `id` de la página.
fn recomendacion<'a>(pagina: &'a Value, id: &str) -> &'a Value {
    let e = pagina["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .unwrap_or_else(|| panic!("falta el elemento {id}: {pagina}"));
    &e["recomendacion"]
}

/// Con las fixtures: el WPS de la red de 2,4 GHz es de Seguridad
/// recomendada; la gestión remota, de los dos lotes; el DNS de la red de
/// casa, de Privacidad máxima.
#[tokio::test]
async fn recomendacion_dice_sus_lotes() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let wifi = leer(&p, &cookie, "wifi").await;
    let r = recomendacion(&wifi, "red-24");
    assert_eq!((&r["campo"], &r["valor"]), (&json!("wps"), &json!("false")));
    assert_eq!(r["lotes"], json!(["mejoras-lote-seguridad"]), "{r}");
    let seguridad = leer(&p, &cookie, "seguridad").await;
    let r = recomendacion(&seguridad, "gestion-remota");
    assert_eq!(
        r["lotes"],
        json!(["mejoras-lote-seguridad", "mejoras-lote-privacidad"]),
        "{r}"
    );
    let red_local = leer(&p, &cookie, "red_local").await;
    let r = recomendacion(&red_local, "dns-casa");
    assert_eq!(r["lotes"], json!(["mejoras-lote-privacidad"]), "{r}");
}

/// Con el WPS ya quitado (como lo deja el lote), la tarjeta no lo
/// recomienda ni nombra ningún lote: lo que queda (encender la red) no es
/// de ninguno.
#[tokio::test]
async fn ajuste_como_el_lote_sin_aviso() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    p.state
        .hgu
        .write(
            "wifi.asp",
            "cbSetWifiAccPt",
            &[("dpm_cfg_wifi_acc_pt_id", "0"), ("wps_adm_state", "0")],
        )
        .await
        .unwrap();
    let wifi = leer(&p, &cookie, "wifi").await;
    let r = recomendacion(&wifi, "red-24");
    assert_ne!(r["campo"], "wps", "{r}");
    assert!(
        r.is_null() || r["lotes"] == json!([]),
        "sin lotes que avisar: {r}"
    );
}
