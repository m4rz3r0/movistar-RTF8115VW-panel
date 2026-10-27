//! «Registro del router» (Ajustes → Sistema): exporta el registro del
//! sistema como syslog.asp (`cbSetSyslog` con solo `export=EXPORT`) y lo
//! lee de `/syslog.log`, sin pasar por el gestor ni cambiar ningún ajuste.

use crate::comun::{PanelPrueba, llamar, ok, pagina, sesion};
use serde_json::json;

const LEER: &str = "leer_registro_router";

#[tokio::test]
async fn exige_sesion() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let (status, _) = llamar(&p, LEER, json!({}), None).await;
    assert_eq!(status, 401);
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn exporta_y_devuelve_el_registro_sin_cambiar_nada() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let antes = p.state.hgu.call("cbGetSyslog", &[]).await.unwrap();
    let r = ok(&p, LEER, json!({}), &cookie).await;
    assert!(
        r["texto"].as_str().unwrap().contains("syslogd started"),
        "{r}"
    );
    assert_eq!(r["recortado"], false);
    // Solo la exportación, con el sessionKey válido (el simulador no anota
    // las escrituras sin él) y nada más.
    assert_eq!(
        p.sim.control.writes(),
        vec![(
            "cbSetSyslog".to_owned(),
            vec![("export".to_owned(), "EXPORT".to_owned())]
        )]
    );
    assert_eq!(p.state.hgu.call("cbGetSyslog", &[]).await.unwrap(), antes);
}

/// Con el registro apagado en el router (`syslog_enable=0`) no hay nada que
/// leer: se dice, para que no parezca un registro vacío sin más.
#[tokio::test]
async fn con_el_registro_apagado_lo_dice() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let r = ok(&p, LEER, json!({}), &cookie).await;
    assert_eq!(r["apagado"], false, "{r}");
    p.state
        .hgu
        .write("syslog.asp", "cbSetSyslog", &[("syslog_enable", "0")])
        .await
        .unwrap();
    p.sim.control.fijar_registro("");
    let r = ok(&p, LEER, json!({}), &cookie).await;
    assert_eq!(r["apagado"], true, "{r}");
    assert_eq!(r["texto"], "", "{r}");
}

#[tokio::test]
async fn un_registro_enorme_llega_recortado() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let tope = hgu::client::TOPE_REGISTRO;
    let linea = "una línea del registro bastante larga\n";
    p.sim
        .control
        .fijar_registro(&linea.repeat(tope / linea.len() + 100));
    let r = ok(&p, LEER, json!({}), &cookie).await;
    assert_eq!(r["recortado"], true);
    let texto = r["texto"].as_str().unwrap();
    assert!(texto.len() <= tope, "{}", texto.len());
    assert!(texto.starts_with("una línea"));
}

#[tokio::test]
async fn sin_router_configurado() {
    let p = PanelPrueba::nuevo().sin_router().arrancar().await;
    let cookie = sesion(&p).await;
    let (status, cuerpo) = llamar(&p, LEER, json!({}), Some(&cookie)).await;
    assert_eq!(status, 503, "{cuerpo}");
    assert!(cuerpo.contains("router"), "{cuerpo}");
}

/// La sección está en Sistema y solo en Sistema.
#[tokio::test]
async fn la_seccion_esta_en_sistema() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let html = pagina(&p, &cookie, "/ajustes/sistema").await;
    assert!(html.contains("Registro del router"), "{html}");
    let html = pagina(&p, &cookie, "/ajustes/wifi").await;
    assert!(!html.contains("Registro del router"));
}
