//! La página `/mejoras` en el HTML del servidor: el título, las dos
//! tarjetas con su insignia, «Volver a leer» cuando algo no se pudo leer,
//! el contador de la barra lateral y, en solo lectura, sin «Revisar y
//! aplicar».

use super::{dns_del_pool, escribir};
use crate::comun::{Panel, PanelPrueba};
use panel::instancia::{self, claves};
use serde_json::json;

/// La tarjeta del lote `id` (de `<article` a `</article>`).
fn tarjeta<'a>(html: &'a str, id: &str) -> &'a str {
    let marca = format!(r#"aria-labelledby="lote-{id}-titulo""#);
    let i = html
        .find(&marca)
        .unwrap_or_else(|| panic!("falta {id}: {html}"));
    let inicio = html[..i].rfind("<article").unwrap();
    let fin = i + html[i..].find("</article>").unwrap();
    &html[inicio..fin]
}

async fn mejoras(p: &Panel, cookie: &str) -> String {
    crate::comun::pagina(p, cookie, "/mejoras").await
}

#[tokio::test]
async fn pagina_mejoras_ssr() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let html = mejoras(&p, &cookie).await;
    assert!(
        html.contains("Mejoras recomendadas para tu router"),
        "{html}"
    );
    assert_eq!(html.matches(r#"class="tarjeta-lote"#).count(), 2, "{html}");
    assert!(html.contains("5 de 11 pendientes"), "{html}");
    assert!(html.contains("3 de 5 pendientes"), "{html}");
    assert!(html.contains("Revisar y aplicar"), "{html}");
    assert!(html.contains("Más consejos"), "{html}");
    // Todo se leyó: nada que volver a leer.
    assert!(!html.contains("Volver a leer"), "{html}");

    instancia::escribir(&p.state.db, claves::SOLO_LECTURA, &json!(true))
        .await
        .unwrap();
    let html = mejoras(&p, &cookie).await;
    assert_eq!(html.matches(r#"class="tarjeta-lote"#).count(), 2, "{html}");
    assert!(!html.contains("Revisar y aplicar"), "{html}");
}

/// El contador de la barra lateral sale en el HTML de las demás páginas
/// (seis pasos sin repetir). En `/mejoras` no hace su propia lectura: toma
/// el total de la página, ya en el navegador.
#[tokio::test]
async fn contador_en_la_barra_salvo_en_mejoras() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let html = crate::comun::pagina(&p, &cookie, "/historial").await;
    assert!(
        html.contains(r#"aria-label="6 pasos pendientes""#),
        "{html}"
    );
    let html = mejoras(&p, &cookie).await;
    assert!(!html.contains(r#"class="cuenta""#), "{html}");
}

/// Con pendientes y una pestaña sin leer (la red de invitados), la tarjeta
/// sigue en «x de y» pero ofrece «Volver a leer»; la otra, no.
#[tokio::test]
async fn volver_a_leer_con_pendientes_y_pasos_sin_leer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.sim
        .control
        .fail_reads_con("cbGetWifiAccPt", "dpm_cfg_wifi_acc_pt_id", "2", true);
    let html = mejoras(&p, &cookie).await;
    let seguridad = tarjeta(&html, "seguridad");
    assert!(seguridad.contains("pendientes"), "{seguridad}");
    assert!(seguridad.contains(">Sin leer<"), "{seguridad}");
    assert!(seguridad.contains("Volver a leer"), "{seguridad}");
    let privacidad = tarjeta(&html, "privacidad");
    assert!(!privacidad.contains("Volver a leer"), "{privacidad}");
}

/// Un lote «sin leer» (lo leído ya está y dos pasos no se pudieron leer):
/// insignia neutra, el motivo y «Volver a leer»; «Revisar y aplicar»,
/// desactivado.
#[tokio::test]
async fn tarjeta_de_lote_sin_leer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    dns_del_pool(&p, "9.9.9.9", "149.112.112.112").await;
    escribir(
        &p,
        "ntp.asp",
        "cbSetTimeNtp",
        &[("ntp_svr_1", "hora.roa.es")],
    )
    .await;
    p.sim.control.fail_reads("cbGetTr69Config", true);
    let html = mejoras(&p, &cookie).await;
    let privacidad = tarjeta(&html, "privacidad");
    assert!(
        privacidad.contains(r#"<span class="ins ins-off">Sin leer</span>"#),
        "{privacidad}"
    );
    assert!(
        privacidad.contains("2 pasos no se pudieron leer; lo demás ya está."),
        "{privacidad}"
    );
    assert!(privacidad.contains("Volver a leer"), "{privacidad}");
    let boton = regex::Regex::new(r#"<button[^>]*class="b-pri"[^>]*>"#).unwrap();
    let b = boton.find(privacidad).expect("el botón").as_str();
    assert!(b.contains("disabled"), "{b}");
}
