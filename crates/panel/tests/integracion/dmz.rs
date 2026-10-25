//! DMZ de solo lectura y nota sobre el control parental en Puertos (Tarea 6 de la fase 3).

use crate::comun::{Panel, PanelPrueba, leer, pagina, variante};
use serde_json::Value;

/// El elemento `dmz` de la página de Puertos.
async fn dmz(p: &Panel, cookie: &str) -> Value {
    let tema = leer(p, cookie, "puertos").await;
    tema["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "dmz")
        .cloned()
        .unwrap_or_else(|| panic!("sin dmz: {tema}"))
}

/// Instancias del ajuste de la DMZ en su elemento.
fn instancias(e: &Value) -> usize {
    e["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "puertos.dmz")
        .and_then(|a| a["instancias"].as_array())
        .map_or_else(|| panic!("sin puertos.dmz: {e}"), Vec::len)
}

#[tokio::test]
async fn dmz_desactivada_con_las_fixtures() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let e = dmz(&p, &cookie).await;
    assert_eq!(instancias(&e), 0, "{e}");
    assert_eq!(e["texto_estado"], "Desactivada", "{e}");
    assert_eq!(e["estado"], "apagada", "{e}");
    assert!(e["recomendacion"].is_null(), "{e}");
}

#[tokio::test]
async fn dmz_activa_avisa() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("dmz-activa"))
        .arrancar_con_gestor()
        .await;
    let e = dmz(&p, &cookie).await;
    assert_eq!(instancias(&e), 1, "{e}");
    assert_eq!(e["estado"], "atencion", "{e}");
    assert_eq!(e["texto_estado"], "Activa", "{e}");
    let r = &e["recomendacion"];
    assert_eq!(r["informativa"], true, "{e}");
    let problema = r["problema"].as_str().unwrap();
    assert!(problema.contains("192.168.1.75"), "{problema}");
    assert!(
        problema.starts_with("Todo lo que llega de internet va a"),
        "{problema}"
    );
    assert!(
        r["texto"]
            .as_str()
            .is_some_and(|t| t.starts_with("Recomendado si")
                && t.contains("A favor:")
                && t.contains("En contra:")),
        "{r}"
    );
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn pagina_puertos_remite_al_control_parental() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let html = pagina(&p, &cookie, "/ajustes/puertos").await;
    // El menú ya enlaza a Mejoras: se mira dentro de la nota.
    let nota = html
        .split_once(r#"class="tema-nota""#)
        .and_then(|(_, resto)| resto.split_once("</section>"))
        .map(|(nota, _)| nota)
        .unwrap_or_else(|| panic!("sin la nota: {html}"));
    assert!(nota.contains(r#"href="/mejoras""#), "{nota}");
    assert!(nota.contains("b-txt"), "{nota}");
    assert!(nota.contains("control parental"), "{nota}");
    // Con nombre accesible: la sección se nombra con su encabezado.
    let (antes, despues) = html.split_once(r#"class="tema-nota""#).unwrap();
    let etiqueta = format!(
        "{}{}",
        &antes[antes.rfind('<').unwrap()..],
        &despues[..despues.find('>').unwrap()]
    );
    assert!(etiqueta.starts_with("<section "), "{etiqueta}");
    assert!(
        etiqueta.contains(r#"aria-labelledby="tema-nota-titulo""#),
        "{etiqueta}"
    );
    let titulo = nota
        .split_once(r#"id="tema-nota-titulo""#)
        .and_then(|(antes, resto)| antes.ends_with("<h2 ").then_some(resto))
        .unwrap_or_else(|| panic!("sin el h2: {nota}"));
    assert!(titulo.contains("Control parental</h2>"), "{nota}");
    // Remite a la opción de DNS familiar de Mejoras.
    assert!(nota.contains("Elegir el DNS familiar"), "{nota}");
    // En las demás páginas, no.
    let html = pagina(&p, &cookie, "/ajustes/wifi").await;
    assert!(!html.contains(r#"class="tema-nota""#));
}

/// Una DMZ apagada pero guardada (la instancia con `activa` en falso) no
/// cuenta como activa: «Desactivada» y sin recomendación.
#[tokio::test]
async fn dmz_apagada_guardada_desactivada() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("dmz-apagada"))
        .arrancar_con_gestor()
        .await;
    let e = dmz(&p, &cookie).await;
    assert_eq!(instancias(&e), 1, "{e}");
    assert_eq!(e["estado"], "apagada", "{e}");
    assert_eq!(e["texto_estado"], "Desactivada", "{e}");
    assert!(e["recomendacion"].is_null(), "{e}");
}

/// Activa, pero sin equipo (la instancia no lo trae o trae 0.0.0.0): el
/// texto habla de «un dispositivo de tu casa», sin IP.
#[tokio::test]
async fn dmz_activa_sin_ip() {
    for capa in ["dmz-sin-ip", "dmz-ip-cero"] {
        let (p, cookie) = PanelPrueba::nuevo()
            .capa(&variante(capa))
            .arrancar_con_gestor()
            .await;
        let e = dmz(&p, &cookie).await;
        assert_eq!(e["texto_estado"], "Activa", "{capa}: {e}");
        let problema = e["recomendacion"]["problema"].as_str().unwrap();
        assert!(
            problema.starts_with("Todo lo que llega de internet va a un dispositivo de tu casa"),
            "{capa}: {problema}"
        );
        assert!(!problema.contains("0.0.0.0"), "{capa}: {problema}");
    }
}

/// Con la lectura de la DMZ fallida, «Sin leer»: nunca «Desactivada» ni
/// una recomendación.
#[tokio::test]
async fn dmz_sin_leer_no_se_da_por_desactivada() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.sim
        .control
        .fail_reads_con("cbGetNatPortMapping", "dmz", "1", true);
    let e = dmz(&p, &cookie).await;
    assert_eq!(e["texto_estado"], "Sin leer", "{e}");
    assert_ne!(e["estado"], "apagada", "{e}");
    assert!(e["recomendacion"].is_null(), "{e}");
}
