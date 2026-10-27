//! El registro del sistema: `cbSetSyslog` con `export=EXPORT` lo genera, sin
//! cambiar ningún ajuste, y `/syslog.log` lo sirve después, como hace
//! syslog.asp en la web avanzada.

use super::*;

/// Línea del registro de serie del simulador.
const LINEA_DE_SERIE: &str = "syslogd started";

#[tokio::test]
async fn sin_exportar_no_hay_registro() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let e = c.basic_page("/syslog.log").await.unwrap_err();
    assert!(
        matches!(&e, Error::Unexpected(m) if m.contains("404")),
        "{e:?}"
    );
}

#[tokio::test]
async fn exportar_no_cambia_el_ajuste_y_sirve_el_registro() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let antes = c.call("cbGetSyslog", &[]).await.unwrap();
    c.write("syslog.asp", "cbSetSyslog", &[("export", "EXPORT")])
        .await
        .unwrap();
    assert_eq!(c.call("cbGetSyslog", &[]).await.unwrap(), antes);
    let log = c.basic_page("/syslog.log").await.unwrap();
    assert!(log.contains(LINEA_DE_SERIE), "{log}");
}

/// El cliente exporta (solo `export=EXPORT` con el `sessionKey` de
/// syslog.asp) y lee `/syslog.log`.
#[tokio::test]
async fn el_cliente_exporta_y_lee() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let antes = c.call("cbGetSyslog", &[]).await.unwrap();
    let r = c.registro_del_sistema().await.unwrap();
    assert!(r.texto.contains(LINEA_DE_SERIE), "{}", r.texto);
    assert!(!r.recortado);
    assert_eq!(
        sim.control.writes(),
        vec![(
            "cbSetSyslog".to_owned(),
            vec![("export".to_owned(), "EXPORT".to_owned())]
        )]
    );
    assert_eq!(c.call("cbGetSyslog", &[]).await.unwrap(), antes);
}

/// Un registro enorme llega recortado: solo las últimas líneas, enteras,
/// y nunca más de [`hgu::client::TOPE_REGISTRO`] bytes.
#[tokio::test]
async fn un_registro_enorme_llega_recortado() {
    let sim = start().await;
    let tope = hgu::client::TOPE_REGISTRO;
    let lineas: Vec<String> = (0..tope / 20)
        .map(|i| format!("linea {i:08} del registro"))
        .collect();
    let texto = lineas.join("\n") + "\n";
    assert!(texto.len() > tope);
    sim.control.fijar_registro(&texto);
    let c = client(&sim, Duration::from_secs(5));
    let r = c.registro_del_sistema().await.unwrap();
    assert!(r.recortado);
    assert!(r.texto.len() <= tope, "{}", r.texto.len());
    assert!(r.texto.ends_with(&format!("{}\n", lineas.last().unwrap())));
    let primera = r.texto.lines().next().unwrap();
    assert!(
        lineas.iter().any(|l| l == primera),
        "línea cortada: {primera}"
    );
}
