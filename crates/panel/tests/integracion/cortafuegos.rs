//! Cortafuegos: la tarjeta de Seguridad con las reglas, «No responder al ping desde internet» y su deshacer, que borra solo su regla.

use crate::comun::{Panel, ajustes_de};
use crate::comun::{PanelPrueba, aplicar_ajuste, esperar_final, leer, llamar, ok};
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
            .is_some_and(|m| m.contains("Regla del panel puesta")),
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

/// Con la regla del panel puesta, la del operador que deja pasar el ping
/// sigue delante (como en el router real, E5, cambio 11): la tarjeta no
/// dice que el router no responde (sigue en ámbar) y el motivo y lo
/// pendiente explican que la regla está puesta pero no basta.
#[tokio::test]
async fn sin_ping_puesta_no_afirma_que_no_responda() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    aplicar_sin_ping(&p, &cookie).await;
    let e = elemento(&p, &cookie, "cortafuegos").await;
    assert_eq!(e["estado"], "atencion", "{e}");
    assert_eq!(e["texto_estado"], "Responde al ping", "{e}");
    let sin_efecto =
        "Regla del panel puesta; el router sigue respondiendo al ping por otra regla del operador.";
    assert_eq!(e["accion_propia"]["motivo"], sin_efecto, "{e}");
    let problema = e["recomendacion"]["problema"].as_str().unwrap_or_default();
    assert!(problema.contains(sin_efecto), "{e}");
    assert!(problema.contains("por IPv6"), "{e}");
}

/// Sin la regla y con los dos abiertos, «Responde al ping» a secas.
#[tokio::test]
async fn sin_regla_responde_al_ping_por_las_dos() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let e = elemento(&p, &cookie, "cortafuegos").await;
    assert_eq!(e["texto_estado"], "Responde al ping", "{e}");
}

/// El dato «Abierto solo desde redes concretas» del elemento (`None` si no
/// sale).
fn abierto_redes(e: &Value) -> Option<String> {
    e["datos"]
        .as_array()?
        .iter()
        .find(|d| d["clave"] == "ajustes-dato-abierto_redes")
        .and_then(|d| d["valor"]["Texto"].as_str())
        .map(str::to_owned)
}

/// La web, el SSH y la gestión remota del operador abiertos solo
/// desde redes concretas (el estado de serie) son un dato de la tarjeta, no
/// algo pendiente; si una regla los abre a cualquiera, sí lo son.
#[tokio::test]
async fn recomendacion_con_la_web_y_el_ssh_abiertos() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let e = elemento(&p, &cookie, "cortafuegos").await;
    let problema = e["recomendacion"]["problema"].as_str().unwrap_or_default();
    assert!(!problema.contains("redes concretas"), "{e}");
    assert!(!problema.contains("Abierto desde cualquier sitio"), "{e}");
    assert_eq!(
        abierto_redes(&e).as_deref(),
        Some("La web del router, el SSH y la gestión remota del operador"),
        "{e}"
    );
    // El SSH de la regla 0, abierto a cualquiera.
    let mut ssh = p.sim.control.cortafuegos()[0].reglas[0].clone();
    assert_eq!(ssh.rid, "0");
    ssh.orig_addr = "0.0.0.0".into();
    p.sim.control.cortafuegos_sustituir("0", "0", ssh);
    p.state.lecturas.invalidar();
    let e = elemento(&p, &cookie, "cortafuegos").await;
    let problema = e["recomendacion"]["problema"].as_str().unwrap_or_default();
    assert!(
        problema.contains("Abierto desde cualquier sitio de internet: el SSH."),
        "{e}"
    );
    assert_eq!(
        abierto_redes(&e).as_deref(),
        Some("La web del router y la gestión remota del operador"),
        "{e}"
    );
    let texto = e["recomendacion"]["texto"].as_str().unwrap_or_default();
    assert!(texto.contains("pedir al operador"), "{e}");
}

/// Con todo abierto solo desde redes concretas (también el ping, por las
/// dos versiones), no hay recomendación pendiente: solo el dato.
#[tokio::test]
async fn solo_desde_redes_concretas_no_es_pendiente() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let mut v4 = p.sim.control.cortafuegos()[0].reglas[8].clone();
    v4.orig_addr = "192.0.2.1".into();
    p.sim.control.cortafuegos_sustituir("0", "8", v4);
    let mut v6 = p.sim.control.cortafuegos()[1]
        .reglas
        .iter()
        .find(|r| r.icmp_type == "55")
        .cloned()
        .unwrap();
    v6.orig_addr = "2001:db8::1".into();
    let rid = v6.rid.clone();
    p.sim.control.cortafuegos_sustituir("1", &rid, v6);
    p.state.lecturas.invalidar();
    let e = elemento(&p, &cookie, "cortafuegos").await;
    assert!(e["recomendacion"].is_null(), "{e}");
    assert_eq!(
        abierto_redes(&e).as_deref(),
        Some("El ping, la web del router, el SSH y la gestión remota del operador"),
        "{e}"
    );
}

/// La regla del panel en el router pero desactivada: motivo propio y sin
/// ofrecer ponerla.
#[tokio::test]
async fn regla_del_panel_desactivada() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    aplicar_sin_ping(&p, &cookie).await;
    let mut r = regla_16(&p).unwrap();
    r.enable = "0".into();
    p.sim.control.cortafuegos_sustituir("0", "16", r);
    p.state.lecturas.invalidar();
    let e = elemento(&p, &cookie, "cortafuegos").await;
    let motivo = e["accion_propia"]["motivo"].as_str().unwrap_or_default();
    assert!(motivo.contains("desactivada"), "{e}");
    let texto = e["recomendacion"]["texto"].as_str().unwrap_or_default();
    assert!(!texto.contains("pase más desapercibido"), "{e}");
}

/// El ping de IPv6 solo desde redes concretas no se pierde porque el de
/// IPv4 esté abierto a cualquiera: el de IPv4 es lo pendiente y el de IPv6,
/// el dato.
#[tokio::test]
async fn ping_de_ipv6_restringido_con_el_de_ipv4_abierto() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let mut v6 = p.sim.control.cortafuegos()[1]
        .reglas
        .iter()
        .find(|r| r.icmp_type == "55")
        .cloned()
        .unwrap();
    v6.orig_addr = "2001:db8::1".into();
    let rid = v6.rid.clone();
    p.sim.control.cortafuegos_sustituir("1", &rid, v6);
    p.state.lecturas.invalidar();
    let e = elemento(&p, &cookie, "cortafuegos").await;
    let problema = e["recomendacion"]["problema"].as_str().unwrap_or_default();
    assert!(problema.contains("por IPv4"), "{e}");
    assert!(
        abierto_redes(&e).is_some_and(|d| d.starts_with("El ping por IPv6, la web")),
        "{e}"
    );
}

/// La baja de la regla «sin ping» solo se da por buena si su filtro sigue y
/// le falta solo esa regla: si el router borra el filtro entero u otra
/// regla, el deshacer no queda aplicado y dice por qué.
#[tokio::test]
async fn deshacer_sin_ping_que_borra_de_mas_no_se_da_por_bueno() {
    for (borra_filtro, motivo) in [
        (true, "ya no está el filtro"),
        (false, "faltan otras reglas"),
    ] {
        let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
        let id = aplicar_sin_ping(&p, &cookie).await;
        if borra_filtro {
            p.sim.control.cortafuegos_proxima_baja_borra_filtro();
        } else {
            p.sim.control.cortafuegos_proxima_baja_borra_tambien("0");
        }
        let d = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
        // Puede fallar ya al confirmar (502) o al acabar: lo que cuenta es
        // cómo queda el cambio.
        let (status, body) = llamar(
            &p,
            "confirmar_ajuste",
            json!({ "token": d["token"], "desmarcados": [] }),
            Some(&cookie),
        )
        .await;
        if status == 200 {
            esperar_final(&p, &cookie, body.trim().parse().unwrap()).await;
        }
        let (estado, error): (String, String) = sqlx::query_as(
            "SELECT estado, COALESCE(error, '') FROM changes ORDER BY id DESC LIMIT 1",
        )
        .fetch_one(&p.state.db)
        .await
        .unwrap();
        assert_ne!(estado, "aplicado", "{status} {body}: {error}");
        assert!(error.contains(motivo), "{motivo}: {error}");
    }
}

/// El deshacer de «sin ping» anota la baja que envía (no el alta), y
/// deshacerlo a su vez vuelve a poner la regla.
#[tokio::test]
async fn baja_de_sin_ping_anota_la_funcion_que_se_envia() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let id = aplicar_sin_ping(&p, &cookie).await;
    let d = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    let baja = aplicar_ajuste(&p, &cookie, &d).await;
    let (cambio, paso): (String, String) = sqlx::query_as(
        "SELECT c.funcion, p.funcion FROM changes c JOIN cambio_pasos p ON p.cambio_id = c.id \
         WHERE c.id = ?",
    )
    .bind(baja)
    .fetch_one(&p.state.db)
    .await
    .unwrap();
    let remove = hgu::cortafuegos::BAJA;
    assert_eq!((cambio.as_str(), paso.as_str()), (remove, remove));
    let d = ok(&p, "deshacer_ajuste", json!({ "cambio": baja }), &cookie).await;
    aplicar_ajuste(&p, &cookie, &d).await;
    assert!(
        p.sim.control.cortafuegos()[0]
            .reglas
            .iter()
            .any(es_sin_ping)
    );
}
