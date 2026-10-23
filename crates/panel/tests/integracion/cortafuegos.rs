//! Cortafuegos (Tarea 3 de la fase 3): la tarjeta de Seguridad con las reglas, «No responder al ping desde internet» y su deshacer, que borra solo su regla.

use crate::comun::{Panel, ajustes_de};
use crate::comun::{PanelPrueba, aplicar_ajuste, leer, llamar, ok};
use hgu::cortafuegos::{Filtro, Regla, es_sin_ping};
use serde_json::{Value, json};

/// La regla «sin ping» (RID 16) en el filtro 0 del simulador.
fn regla_16(p: &Panel) -> Option<Regla> {
    p.sim.control.cortafuegos()[0]
        .reglas
        .iter()
        .find(|r| r.rid == "16")
        .cloned()
}

fn par(k: &str, v: &str) -> (String, String) {
    (k.to_owned(), v.to_owned())
}

/// Las escrituras del cortafuegos recibidas por el simulador.
fn escrituras(p: &Panel) -> Vec<Vec<(String, String)>> {
    p.sim
        .control
        .writes()
        .into_iter()
        .filter(|(f, _)| f == "scbidirectionalflt.cmd")
        .map(|(_, ps)| ps)
        .collect()
}

/// El elemento `id` de la página de Seguridad.
async fn elemento(p: &Panel, cookie: &str, id: &str) -> Value {
    let tema = leer(p, cookie, "seguridad").await;
    tema["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .cloned()
        .unwrap_or_else(|| panic!("sin {id}: {tema}"))
}

async fn preparar_sin_ping(p: &Panel, cookie: &str) -> (u16, String) {
    llamar(p, "preparar_sin_ping", json!({}), Some(cookie)).await
}

/// Aplica «No responder al ping» y devuelve el id del cambio.
async fn aplicar_sin_ping(p: &Panel, cookie: &str) -> i64 {
    let g = ok(p, "preparar_sin_ping", json!({}), cookie).await;
    aplicar_ajuste(p, cookie, &g).await
}

#[tokio::test]
async fn cortafuegos_en_seguridad() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let tema = leer(&p, &cookie, "seguridad").await;
    let a = ajustes_de(&tema)
        .into_iter()
        .find(|a| a["id"] == "seguridad.cortafuegos")
        .unwrap();
    assert_eq!(a["componente"], "cortafuegos", "{a}");
    assert_eq!(a["instancias"].as_array().unwrap().len(), 16, "{a}");
    let e = elemento(&p, &cookie, "cortafuegos").await;
    assert_eq!(e["estado"], "atencion", "{e}");
    let r = &e["recomendacion"];
    assert_eq!(r["informativa"], true, "{e}");
    assert!(
        r["problema"]
            .as_str()
            .is_some_and(|t| t.contains("responde al ping")),
        "{e}"
    );
    assert_eq!(
        e["accion_propia"],
        json!({ "id": "sin_ping", "motivo": null }),
        "{e}"
    );
}

#[tokio::test]
async fn sin_ping_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = ok(&p, "preparar_sin_ping", json!({}), &cookie).await;
    assert_eq!(g["pasos"].as_array().unwrap().len(), 1, "{g}");
    assert_eq!(g["riesgo"], "Alto", "{g}");
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    let r = regla_16(&p).expect("la regla 16");
    assert!(es_sin_ping(&r), "{r:?}");
    let w = escrituras(&p);
    assert_eq!(w.len(), 1, "{w:?}");
    let mut alta = w[0].clone();
    alta.sort();
    assert_eq!(
        alta,
        [
            par("action", "add"),
            par("belong", "0"),
            par("enabled", "1"),
            par("icmpType", "25"),
            par("protocol", "1"),
            par("specificAction", "0"),
            par("target", "rule"),
            par("version", "0"),
        ]
    );
    let e = elemento(&p, &cookie, "cortafuegos").await;
    assert!(
        e["accion_propia"]["motivo"]
            .as_str()
            .is_some_and(|m| m.contains("ya puesta")),
        "{e}"
    );
    // Deshacer la quita: solo su regla.
    let d = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    aplicar_ajuste(&p, &cookie, &d).await;
    let w = escrituras(&p);
    assert_eq!(w.len(), 2, "{w:?}");
    let mut baja = w[1].clone();
    baja.sort();
    assert_eq!(
        baja,
        [
            par("action", "remove"),
            par("escape", "true"),
            par("item", "0|16")
        ]
    );
    let f: Vec<Filtro> = p.sim.control.cortafuegos();
    assert_eq!(f[0].reglas.len(), 10, "{f:?}");
    assert_eq!(f.len(), 2);
}

#[tokio::test]
async fn deshacer_sin_ping_no_borra_otra_regla() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let id = aplicar_sin_ping(&p, &cookie).await;
    // Alguien cambia la regla 16: ya no es la del panel.
    let mut otra = regla_16(&p).unwrap();
    otra.icmp_type = "1".into();
    p.sim.control.cortafuegos_sustituir("0", "16", otra);
    let (status, body) = llamar(
        &p,
        "deshacer_ajuste",
        json!({ "cambio": id }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("ya no es igual en el router"), "{body}");
    let w = escrituras(&p);
    assert_eq!(w.len(), 1, "solo el alta: {w:?}");
    for ps in &w {
        if ps.contains(&par("action", "remove")) {
            let item = ps.iter().find(|(k, _)| k == "item").unwrap();
            assert!(item.1.contains('|'), "{ps:?}");
        }
    }
    assert_eq!(p.sim.control.cortafuegos()[0].reglas.len(), 11);
}

#[tokio::test]
async fn cortafuegos_ilegible_sin_leer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.sim.control.cortafuegos_ilegible(true);
    let e = elemento(&p, &cookie, "cortafuegos").await;
    let motivo = e["lectura"]["SinLeer"]
        .as_str()
        .unwrap_or_else(|| panic!("leído: {e}"))
        .to_owned();
    assert!(!motivo.is_empty());
    assert!(!motivo.contains("scbidirectional"), "{motivo}");
    let api = regex::Regex::new(r"cb[A-Z]").unwrap();
    assert!(!api.is_match(&motivo), "{motivo}");
    // «No responder al ping», desactivada con su motivo.
    assert!(e["accion_propia"]["motivo"].as_str().is_some(), "{e}");
    let remota = elemento(&p, &cookie, "gestion-remota").await;
    assert_eq!(remota["lectura"], "Leido", "{remota}");
    let (status, body) = preparar_sin_ping(&p, &cookie).await;
    assert_eq!(status, 409, "{body}");
    assert!(
        p.sim.control.writes().is_empty(),
        "{:?}",
        p.sim.control.writes()
    );
}

#[tokio::test]
async fn biblioteca_no_escribe_el_cortafuegos() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let carga = json!([
        ["belong", "0"],
        ["version", "0"],
        ["enabled", "1"],
        ["protocol", "1"],
        ["specificAction", "0"],
        ["icmpType", "25"],
        ["target", "rule"]
    ]);
    for funcion in [
        "scbidirectionalflt.cmd?action=add",
        "scbidirectionalflt.cmd?action=modify",
    ] {
        let (status, body) = llamar(
            &p,
            "preparar_cambio",
            json!({ "peticion": { "funcion": funcion, "instancia": null,
                "valores": carga, "borrar": false } }),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, 403, "{funcion}: {body}");
        assert!(body.contains("bloqueada"), "{funcion}: {body}");
    }
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn sin_ping_ya_puesta_409() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    aplicar_sin_ping(&p, &cookie).await;
    let (status, body) = preparar_sin_ping(&p, &cookie).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("ya puesta"), "{body}");
    assert_eq!(escrituras(&p).len(), 1);
}
