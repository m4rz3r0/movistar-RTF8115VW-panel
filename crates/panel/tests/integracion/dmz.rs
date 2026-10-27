//! DMZ de solo lectura y nota sobre el control parental en Puertos (Tarea 6 de la fase 3).
//! En el router real la DMZ es una instancia más de las redirecciones, con
//! `desc=DMZ` (la crea natdmz.asp): `puertos.dmz` se queda con ella y
//! `puertos.reglas` la deja fuera.

use crate::comun::{Panel, PanelPrueba, esperar_final, leer, ok, pagina, variante};
use serde_json::{Value, json};

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

/// Activa, pero la instancia no trae el equipo: el texto habla de «un
/// dispositivo de tu casa», sin IP.
#[tokio::test]
async fn dmz_activa_sin_ip() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("dmz-sin-ip"))
        .arrancar_con_gestor()
        .await;
    let e = dmz(&p, &cookie).await;
    assert_eq!(e["texto_estado"], "Activa", "{e}");
    let problema = e["recomendacion"]["problema"].as_str().unwrap();
    assert!(
        problema.starts_with("Todo lo que llega de internet va a un dispositivo de tu casa"),
        "{problema}"
    );
}

/// Ruling M′: marcada encendida pero con el equipo 0.0.0.0 (ninguno) no
/// manda nada a ningún dispositivo: «Desactivada» y sin recomendación, para
/// no dar una falsa alarma.
#[tokio::test]
async fn dmz_con_ip_cero_no_cuenta_como_activa() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("dmz-ip-cero"))
        .arrancar_con_gestor()
        .await;
    let e = dmz(&p, &cookie).await;
    assert_eq!(instancias(&e), 1, "{e}");
    assert_eq!(e["texto_estado"], "Desactivada", "{e}");
    assert_eq!(e["estado"], "apagada", "{e}");
    assert!(e["recomendacion"].is_null(), "{e}");
}

/// Con la lectura de la DMZ fallida, «Sin leer»: nunca «Desactivada» ni
/// una recomendación.
#[tokio::test]
async fn dmz_sin_leer_no_se_da_por_desactivada() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    p.sim.control.fail_reads("cbGetNatPortMapping", true);
    let e = dmz(&p, &cookie).await;
    assert_eq!(e["texto_estado"], "Sin leer", "{e}");
    assert_ne!(e["estado"], "apagada", "{e}");
    assert!(e["recomendacion"].is_null(), "{e}");
}

/// La instancia de la DMZ no es una regla: la lista de reglas no la enseña.
#[tokio::test]
async fn la_dmz_no_sale_entre_las_reglas() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("dmz-activa"))
        .arrancar_con_gestor()
        .await;
    let tema = leer(&p, &cookie, "puertos").await;
    let reglas: Vec<&Value> = tema["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|e| e["ajustes"].as_array().unwrap())
        .filter(|a| a["id"] == "puertos.reglas")
        .flat_map(|a| a["instancias"].as_array().unwrap())
        .collect();
    assert_eq!(reglas.len(), 1, "{tema}");
    assert!(!tema.to_string().contains(r#""DMZ""#), "{tema}");
}

/// Aplica `prep` (de la Biblioteca o un deshacer) y devuelve el id.
async fn confirmar(p: &Panel, cookie: &str, prep: &Value) -> i64 {
    let id = ok(
        p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap();
    assert_eq!(esperar_final(p, cookie, id).await["estado"], "Aplicado");
    id
}

/// La DMZ se crea como natdmz.asp: la CGI de las redirecciones con sus
/// cinco campos y `desc=DMZ`; sale en la DMZ, no entre las reglas, y su
/// deshacer la borra.
#[tokio::test]
async fn crear_la_dmz_como_la_web_y_deshacerla() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetNatPortMapping.dmz", "instancia": null,
                              "valores": [["adm_state", "1"], ["int_ip_addr", "192.168.1.75"],
                                          ["ip_intf_id", "2"]],
                              "borrar": false } }),
        &cookie,
    )
    .await;
    let id = confirmar(&p, &cookie, &prep).await;
    let (cgi, ps) = p.sim.control.writes().pop().unwrap();
    assert_eq!(cgi, "cbSetNatPortMapping");
    let mut claves: Vec<&str> = ps.iter().map(|(k, _)| k.as_str()).collect();
    claves.sort_unstable();
    assert_eq!(
        claves,
        [
            "adm_state",
            "desc",
            "int_ip_addr",
            "ip_intf_id",
            "nat_port_mapping_id"
        ],
        "{ps:?}"
    );
    assert!(
        ps.contains(&("desc".to_owned(), "DMZ".to_owned())),
        "{ps:?}"
    );
    p.state.lecturas.invalidar();
    let e = dmz(&p, &cookie).await;
    assert_eq!(instancias(&e), 1, "{e}");
    assert_eq!(e["texto_estado"], "Activa", "{e}");

    let deshacer = ok(&p, "deshacer", json!({ "change_id": id }), &cookie).await;
    confirmar(&p, &cookie, &deshacer).await;
    p.state.lecturas.invalidar();
    let e = dmz(&p, &cookie).await;
    assert_eq!(instancias(&e), 0, "{e}");
}
