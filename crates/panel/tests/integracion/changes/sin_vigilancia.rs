//! Vigilancia según lo que toca el cambio: lo que no puede cortar el acceso
//! del panel queda aplicado al momento; lo que puede, se vigila; la WiFi,
//! según por dónde llegue el panel al router.

use super::*;
use crate::comun::{ConfirmarLigado, PanelPrueba, esperar_libre, fijar, ok, valores_de, variante};
use panel::changes::Enlace;
use serde_json::{Value, json};

/// Vigilancia larga: lo que se vigile sigue `Vigilando` durante la prueba.
fn tiempos_largos() -> Tiempos {
    Tiempos {
        bajo: Duration::from_secs(60),
        medio: Duration::from_secs(60),
        alto: Duration::from_secs(60),
        ..tiempos_rapidos()
    }
}

async fn estado_de(p: &Panel, cookie: &str, id: i64) -> Value {
    ok(p, "estado_cambio", json!({ "id": id }), cookie).await
}

/// Prepara el cambio del nombre de la red WiFi 1 por la Biblioteca.
async fn preparar_ssid(p: &Panel, cookie: &str) -> Value {
    let f = ok(
        p,
        "formulario",
        json!({ "funcion": "cbSetWifiAccPt", "instancia": null,
                "claves": [["dpm_cfg_wifi_acc_pt_id", "1"]] }),
        cookie,
    )
    .await;
    let mut valores = valores_de(&f);
    fijar(&mut valores, "ssid", "Otra red");
    let peticion = json!({ "funcion": "cbSetWifiAccPt", "instancia": null,
                           "valores": valores, "borrar": false });
    ok(
        p,
        "preparar_cambio",
        json!({ "peticion": peticion }),
        cookie,
    )
    .await
}

async fn confirmar(p: &Panel, cookie: &str, prep: &Value) -> i64 {
    ok(
        p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap()
}

/// Activar UPnP no puede cortar el acceso: el diálogo no anuncia
/// vigilancia y, al confirmarlo, el cambio ya está aplicado (con la nota
/// que lo explica), el gestor libre y se puede deshacer.
#[tokio::test]
async fn lo_que_no_corta_queda_aplicado_al_momento() {
    let (p, cookie) = PanelPrueba::nuevo()
        .tiempos(tiempos_largos())
        .arrancar_con_gestor()
        .await;
    let peticion = json!({ "funcion": "cbSetUpnpCfg", "instancia": null,
                           "valores": [["adm_state", "1"]], "borrar": false });
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": peticion }),
        &cookie,
    )
    .await;
    assert_eq!(prep["vigilancia_s"], 0, "{prep}");
    let id = confirmar(&p, &cookie, &prep).await;
    let c = estado_de(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
    assert!(
        c["nota"]
            .as_str()
            .unwrap_or_default()
            .contains("sin vigilancia"),
        "{c}"
    );
    esperar_libre(&p).await;
    assert!(p.state.cambios().unwrap().se_puede_deshacer(id).await);
}

/// Una reserva de IP fija toca el DHCP: se vigila.
#[tokio::test]
async fn lo_que_puede_cortar_se_vigila() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let m = gestor(&p, tiempos_largos()).await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    assert!(prep.vigila);
    let id = m
        .confirmar_ligado(&prep.token, None, "admin")
        .await
        .unwrap();
    assert_eq!(m.cambio(id).await.unwrap().estado, Estado::Vigilando);
    assert_eq!(m.activo(), Some(id));
    m.detener();
}

/// La WiFi: con el panel por cable no se vigila; por WiFi, o si el panel
/// no está en la lista del router (el simulador, en 127.0.0.1, de serie),
/// sí.
#[tokio::test]
async fn la_wifi_se_vigila_salvo_por_cable() {
    for (capa, enlace) in [
        (None, Enlace::Desconocido),
        (Some("panel-por-wifi"), Enlace::Wifi),
        (Some("panel-por-cable"), Enlace::Cable),
    ] {
        let mut pp = PanelPrueba::nuevo().tiempos(tiempos_largos());
        let dir = capa.map(variante);
        if let Some(d) = &dir {
            pp = pp.capa(d);
        }
        let (p, cookie) = pp.arrancar_con_gestor().await;
        let m = p.state.cambios().unwrap();
        assert_eq!(m.enlace().await, enlace, "{capa:?}");
        let prep = preparar_ssid(&p, &cookie).await;
        let vigila = enlace != Enlace::Cable;
        assert_eq!(prep["vigilancia_s"] != 0, vigila, "{capa:?}: {prep}");
        let id = confirmar(&p, &cookie, &prep).await;
        let estado = estado_de(&p, &cookie, id).await["estado"].clone();
        let esperado = if vigila { "Vigilando" } else { "Aplicado" };
        assert_eq!(estado, esperado, "{capa:?}");
        m.detener();
    }
}
