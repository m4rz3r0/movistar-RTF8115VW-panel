//! La copia de la configuración: `cbSaveConfig` con `save=1` la guarda y con `backup=1` da el nombre del fichero; `cbDownloadConfig` (POST vacío con `fileName` y `sessionKey` en la URL) lo descarga.

use super::*;

/// Lo que da `cbSaveConfig` con `backup=1`.
async fn generar(c: &Client) -> String {
    let n = c
        .write("backup_restore.asp", "cbSaveConfig", &[("backup", "1")])
        .await
        .unwrap();
    n.text_of("FileName").unwrap().to_owned()
}

#[tokio::test]
async fn backup_da_el_nombre_del_fichero() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let nombre = generar(&c).await;
    assert!(
        nombre.starts_with("config_") && nombre.ends_with(".tar"),
        "{nombre}"
    );
}

/// POST sin cuerpo con los parámetros en la URL, en la sesión avanzada:
/// el fichero, como `application/octet-stream` y con su nombre.
#[tokio::test]
async fn la_descarga_va_por_la_url_con_el_cuerpo_vacio() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.write("save.asp", "cbSaveConfig", &[("save", "1")])
        .await
        .unwrap();
    let nombre = generar(&c).await;
    let (cookies, clave) = sim.control.sesion_avanzada_y_clave().unwrap();
    let http = reqwest::Client::new();
    let url = format!(
        "{}/cgi-bin/cbDownloadConfig.xml?fileName={nombre}&sessionKey={clave}",
        sim.base_url
    );
    let r = http
        .post(&url)
        .header("Cookie", &cookies)
        .header("Content-Length", "0")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 200);
    assert_eq!(
        r.headers()["content-type"].to_str().unwrap(),
        "application/octet-stream"
    );
    assert_eq!(
        r.headers()["content-disposition"].to_str().unwrap(),
        format!("attachment; filename=\"{nombre}\"")
    );
    let datos = r.bytes().await.unwrap();
    assert!(datos.starts_with(b"ASP_CM"), "{datos:?}");
    // La copia es de lo último guardado.
    assert!(
        String::from_utf8_lossy(&datos).contains("guardada 1 vez"),
        "{datos:?}"
    );
    // Con cuerpo, o con otro fichero, no.
    let (cookies, clave) = sim.control.sesion_avanzada_y_clave().unwrap();
    let url = format!(
        "{}/cgi-bin/cbDownloadConfig.xml?fileName={nombre}&sessionKey={clave}",
        sim.base_url
    );
    let r = http
        .post(&url)
        .header("Cookie", &cookies)
        .body(format!("fileName={nombre}"))
        .send()
        .await
        .unwrap();
    assert_ne!(r.status().as_u16(), 200);
    let url = format!(
        "{}/cgi-bin/cbDownloadConfig.xml?fileName=otro.tar&sessionKey={clave}",
        sim.base_url
    );
    let r = http
        .post(&url)
        .header("Cookie", &cookies)
        .header("Content-Length", "0")
        .send()
        .await
        .unwrap();
    assert_ne!(r.status().as_u16(), 200);
}

/// El cliente hace lo de la web, en orden: guardar, generar y descargar.
#[tokio::test]
async fn el_cliente_guarda_genera_y_descarga() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let copia = c.copia_de_configuracion().await.unwrap();
    assert!(copia.nombre.starts_with("config_"), "{}", copia.nombre);
    assert!(copia.datos.starts_with(b"ASP_CM"));
    let w = sim.control.writes();
    assert_eq!(
        w,
        vec![
            (
                "cbSaveConfig".to_owned(),
                vec![("save".to_owned(), "1".to_owned())]
            ),
            (
                "cbSaveConfig".to_owned(),
                vec![("backup".to_owned(), "1".to_owned())]
            ),
        ]
    );
    // El fichero pedido es el que dio `backup=1`.
    assert_eq!(sim.control.copias_descargadas(), vec![copia.nombre.clone()]);
    // La depuración no enseña el contenido.
    assert!(!format!("{copia:?}").contains("ASP_CM"));
}

/// Un fichero mayor que el tope no se descarga a medias: error.
#[tokio::test]
async fn una_copia_enorme_es_error() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let mut grande = b"ASP_CM".to_vec();
    grande.resize(hgu::client::TOPE_COPIA + 1, b'x');
    sim.control.fijar_copia(grande);
    let e = c.copia_de_configuracion().await.unwrap_err();
    assert!(e.to_string().contains("demasiado"), "{e}");
}
