//! Internet, Seguridad, Teléfono y Sistema en rejilla (Tarea 4 del
//! rediseño): sus elementos, su estado, su recomendación principal y su
//! franja de resumen.

use crate::comun::{Panel, PanelPrueba, leer, sesion};
use serde_json::{Value, json};

/// Elemento `id` de la página.
fn elemento<'a>(pagina: &'a Value, id: &str) -> &'a Value {
    pagina["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .unwrap_or_else(|| panic!("falta el elemento {id}: {pagina}"))
}

/// Ids de los elementos de la página, en orden.
fn ids(pagina: &Value) -> Vec<&str> {
    pagina["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect()
}

/// Escribe en el simulador como lo haría el router (otro que no es el
/// panel) y olvida la caché de lecturas.
async fn escribir(p: &Panel, pagina: &str, funcion: &str, pares: &[(&str, &str)]) {
    p.state.hgu.write(pagina, funcion, pares).await.unwrap();
    p.state.lecturas.invalidar();
}

#[tokio::test]
async fn pagina_seguridad_gestion_remota() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    escribir(
        &p,
        "cwmp.asp",
        "cbSetTr69Config",
        &[("enableCWMP", "0"), ("periodicInformEnable", "0")],
    )
    .await;
    let pagina = leer(&p, &cookie, "seguridad").await;
    assert_eq!(
        ids(&pagina),
        ["gestion-remota", "contrasena-router", "usuarios"]
    );
    let e = elemento(&pagina, "gestion-remota");
    assert_eq!(e["estado"], "apagada", "{e}");
    assert_eq!(e["texto_estado"], "Apagada", "{e}");
    assert!(e["recomendacion"].is_null(), "{e}");
    // Sus tres ajustes, en el detalle: lo esencial, el aviso y el servidor.
    assert_eq!(e["ajustes"].as_array().unwrap().len(), 3, "{e}");

    escribir(&p, "cwmp.asp", "cbSetTr69Config", &[("enableCWMP", "1")]).await;
    let pagina = leer(&p, &cookie, "seguridad").await;
    let e = elemento(&pagina, "gestion-remota");
    assert_eq!(e["estado"], "atencion", "{e}");
    assert_eq!(e["texto_estado"], "Encendida", "{e}");
    let r = &e["recomendacion"];
    assert_eq!(r["riesgo"], true, "{r}");
    assert_eq!(
        (&r["ajuste"], &r["campo"], &r["valor"]),
        (
            &json!("seguridad.gestion_remota"),
            &json!("activa"),
            &json!("apagada")
        ),
        "{r}"
    );
    assert_eq!(r["boton"], "Apagar la gestión remota", "{r}");
    assert!(
        r["problema"]
            .as_str()
            .unwrap()
            .starts_with("La gestión remota está encendida"),
        "{r}"
    );
    assert!(
        r["texto"].as_str().unwrap().starts_with("Recomendado si"),
        "{r}"
    );
    assert_eq!(e["acciones"], json!(["apagar_gestion_remota"]), "{e}");
    // La acción, en la barra de la página.
    assert_eq!(pagina["acciones"][0]["id"], "apagar_gestion_remota");
}

#[tokio::test]
async fn pagina_sistema_hora() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    // Las fixtures traen el servidor de hora del operador.
    let pagina = leer(&p, &cookie, "sistema").await;
    assert_eq!(ids(&pagina), ["hora", "diagnostico"]);
    let hora = elemento(&pagina, "hora");
    assert_eq!(hora["estado"], "emite", "{hora}");
    let r = &hora["recomendacion"];
    assert_eq!(
        (&r["ajuste"], &r["campo"], &r["valor"]),
        (
            &json!("sistema.hora"),
            &json!("servidor_1"),
            &json!("hora.roa.es")
        ),
        "{r}"
    );
    assert_eq!(r["riesgo"], false, "{r}");
    assert_eq!(r["boton"], "Usar un servidor de hora neutro", "{r}");
    let texto = r["texto"].as_str().unwrap();
    assert!(texto.starts_with("Recomendado si"), "{texto}");
    assert!(
        texto.contains("A favor:") && texto.contains("En contra:"),
        "{texto}"
    );
    assert!(!r["problema"].as_str().unwrap().is_empty(), "{r}");
    assert_eq!(hora["acciones"], json!(["usar_hora_neutra"]), "{hora}");
    // El espejo apagado (lo recomendado): gris y sin recomendación.
    let diag = elemento(&pagina, "diagnostico");
    assert_eq!(diag["estado"], "apagada", "{diag}");
    assert!(diag["recomendacion"].is_null(), "{diag}");

    escribir(
        &p,
        "ntp.asp",
        "cbSetTimeNtp",
        &[("ntp_svr_1", "hora.roa.es")],
    )
    .await;
    let pagina = leer(&p, &cookie, "sistema").await;
    let hora = elemento(&pagina, "hora");
    assert!(hora["recomendacion"].is_null(), "{hora}");
}

/// La línea de teléfono se guarda reenviando una clave que el router no
/// deja leer: ni la tarjeta ni la barra preparan un cambio sin ella (se
/// cambia desde su detalle, que la pide).
#[tokio::test]
async fn telefono_sin_atajos_que_borren_la_clave() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    escribir(
        &p,
        "voiceadvance.asp",
        "cbSetVoiceLine",
        &[("interwork_id", "0"), ("adm_state", "0")],
    )
    .await;
    let pagina = leer(&p, &cookie, "telefono").await;
    assert_eq!(
        ids(&pagina),
        ["linea", "llamadas", "servidores", "fax-y-modem"]
    );
    let linea = elemento(&pagina, "linea");
    assert_eq!(linea["estado"], "apagada", "{linea}");
    assert!(linea["interruptor"].is_null(), "{linea}");
    assert!(linea["recomendacion"].is_null(), "{linea}");
    assert_eq!(linea["acciones"], json!([]), "{linea}");
    assert_eq!(pagina["acciones"], json!([]));
}

/// Los cuatro temas tienen su franja de resumen, de cuatro datos, en los
/// dos idiomas; ninguna cifra ni explicación se queda en su clave.
#[tokio::test]
async fn cada_tema_tiene_resumen() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let en = format!("{cookie}; rp_lang=en");
    let mut por_idioma = Vec::new();
    for (idioma, c) in [("es", &cookie), ("en", &en)] {
        let mut detalles = Vec::new();
        for tema in ["internet", "seguridad", "telefono", "sistema"] {
            let pagina = leer(&p, c, tema).await;
            let resumen = pagina["resumen"].as_array().unwrap();
            assert_eq!(resumen.len(), 4, "{idioma} {tema}: {resumen:?}");
            for d in resumen {
                for k in ["cifra", "detalle"] {
                    let v = d[k].as_str().unwrap();
                    assert!(
                        !v.is_empty() && !v.starts_with("ajuste"),
                        "{idioma} {tema} {k}: {d}"
                    );
                }
                detalles.push(d["detalle"].as_str().unwrap().to_owned());
            }
            assert_eq!(
                resumen[3]["clave"], "ajustes-resumen-recomendaciones",
                "{idioma} {tema}"
            );
        }
        por_idioma.push(detalles);
    }
    // Cada explicación, en su idioma: ninguna se repite tal cual.
    for (es, en) in por_idioma[0].iter().zip(&por_idioma[1]) {
        assert_ne!(es, en);
    }
    // Las cifras de las fixtures, en español.
    let cifra = |pagina: &Value, clave: &str| {
        pagina["resumen"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["clave"] == clave)
            .unwrap_or_else(|| panic!("falta {clave}: {pagina}"))["cifra"]
            .clone()
    };
    let internet = leer(&p, &cookie, "internet").await;
    assert_eq!(
        cifra(&internet, "ajustes-resumen-internet-conexion"),
        "Conectada"
    );
    let seguridad = leer(&p, &cookie, "seguridad").await;
    let remota = &seguridad["resumen"][0];
    assert_eq!(remota["cifra"], "Encendida", "{remota}");
    assert_eq!(remota["atencion"], true, "{remota}");
    assert_eq!(cifra(&seguridad, "ajustes-resumen-seguridad-cuentas"), "3");
    let sistema = leer(&p, &cookie, "sistema").await;
    assert_eq!(cifra(&sistema, "ajustes-resumen-recomendaciones"), "1");
}
