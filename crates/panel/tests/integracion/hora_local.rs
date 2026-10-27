//! Hora del router a mano (`cbSetLocalTime`), como js/ntp.js: se escribe la
//! fecha y la hora del router y se manda `SysTime` en segundos desde 1970,
//! con la zona del router (`SysTzOffset` de `cbGetSummary`, en minutos,
//! UTC menos la hora local).

use crate::comun::{Panel, PanelPrueba, esperar_final, llamar, ok};
use serde_json::json;

const FUNCION: &str = "cbSetLocalTime";

async fn fijar_hora(p: &Panel, cookie: &str, hora: &str) -> (u16, String) {
    llamar(
        p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": FUNCION, "instancia": null,
                              "valores": [["SysTime", hora]], "borrar": false } }),
        Some(cookie),
    )
    .await
}

async fn enviada(p: &Panel, cookie: &str, hora: &str) -> String {
    let (status, body) = fijar_hora(p, cookie, hora).await;
    assert_eq!(status, 200, "{body}");
    let prep: serde_json::Value = serde_json::from_str(&body).unwrap();
    let id = ok(
        p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": "CONFIRMAR" }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap();
    assert_eq!(esperar_final(p, cookie, id).await["estado"], "Aplicado");
    let w = p.sim.control.writes();
    let (_, ps) = w.last().unwrap();
    ps.iter().find(|(k, _)| k == "SysTime").unwrap().1.clone()
}

#[tokio::test]
async fn la_fecha_y_hora_del_router_van_en_segundos() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    // 2026-01-01 00:00 en el router, que va 120 minutos por delante de UTC
    // (SysTzOffset = -120 en la fixture): 2025-12-31 22:00 UTC.
    assert_eq!(enviada(&p, &cookie, "2026-01-01 00:00").await, "1767218400");
    assert_eq!(enviada(&p, &cookie, "2026-01-01T00:00").await, "1767218400");
    // En segundos, tal cual.
    assert_eq!(enviada(&p, &cookie, "1767218400").await, "1767218400");
}

#[tokio::test]
async fn una_fecha_que_no_existe_no_vale() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for mala in ["2026-02-30 10:00", "2026-01-01 24:00", "ayer", ""] {
        let (status, body) = fijar_hora(&p, &cookie, mala).await;
        assert_eq!(status, 400, "{mala}: {body}");
    }
}
