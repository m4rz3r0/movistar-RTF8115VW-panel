//! «Copia de la configuración» (Ajustes → Sistema): como backup_restore.asp,
//! el panel guarda la configuración (`cbSaveConfig save=1`), genera la copia
//! (`backup=1`) y la descarga (`cbDownloadConfig`). El fichero lleva todas
//! las contraseñas del router: no se anota ni se guarda en el panel.

use crate::comun::{PanelPrueba, llamar, ok, pagina, registro_global, sesion};
use base64::{Engine, engine::general_purpose::STANDARD};
use panel::instancia::{self, claves};
use serde_json::json;

const DESCARGAR: &str = "descargar_copia_router";
const ESTADO: &str = "estado_copia_router";

/// Contenido inventado de la copia, fácil de buscar en el registro.
const CONTENIDO: &[u8] = b"ASP_CM\0copia de prueba con clave-inventada-7731\n";

fn guardar_y_generar() -> Vec<(String, Vec<(String, String)>)> {
    vec![
        (
            "cbSaveConfig".to_owned(),
            vec![("save".to_owned(), "1".to_owned())],
        ),
        (
            "cbSaveConfig".to_owned(),
            vec![("backup".to_owned(), "1".to_owned())],
        ),
    ]
}

#[tokio::test]
async fn exige_sesion() {
    let p = PanelPrueba::nuevo().arrancar().await;
    for f in [DESCARGAR, ESTADO] {
        let (status, _) = llamar(&p, f, json!({}), None).await;
        assert_eq!(status, 401, "{f}");
    }
    assert!(p.sim.control.writes().is_empty());
    assert!(p.sim.control.copias_descargadas().is_empty());
}

/// Guarda, genera y descarga, en ese orden, y devuelve el fichero tal cual
/// con el nombre que da el router.
#[tokio::test]
async fn guarda_genera_y_devuelve_el_fichero_del_router() {
    let registro = registro_global();
    let p = PanelPrueba::nuevo().arrancar().await;
    p.sim.control.fijar_copia(CONTENIDO.to_vec());
    let cookie = sesion(&p).await;
    let e = ok(&p, ESTADO, json!({}), &cookie).await;
    assert_eq!(e, json!(null), "{e}");
    let r = ok(&p, DESCARGAR, json!({}), &cookie).await;
    let nombre = r["nombre"].as_str().unwrap();
    assert!(
        nombre.starts_with("config_") && nombre.ends_with(".tar"),
        "{nombre}"
    );
    let datos = STANDARD.decode(r["datos"].as_str().unwrap()).unwrap();
    assert_eq!(datos, CONTENIDO);
    // Guardar y generar, con su sessionKey (el simulador no anota las
    // escrituras sin él), y luego la descarga del fichero que dio `backup=1`.
    assert_eq!(p.sim.control.writes(), guardar_y_generar());
    assert_eq!(p.sim.control.copias_descargadas(), vec![nombre.to_owned()]);
    // Nada del fichero en el registro.
    let texto = registro.texto();
    assert!(!texto.contains("clave-inventada-7731"), "{texto}");
    assert!(!texto.contains(r["datos"].as_str().unwrap()), "{texto}");
}

/// En la versión normal, sin comprobar en el router real: ni se ofrece ni
/// se hace, y no se manda nada al router.
#[tokio::test]
async fn sin_comprobar_no_se_hace() {
    for funcion in ["cbDownloadConfig", "cbSaveConfig"] {
        let p = PanelPrueba::nuevo().sin_comprobar(funcion).arrancar().await;
        let cookie = sesion(&p).await;
        let e = ok(&p, ESTADO, json!({}), &cookie).await;
        assert_eq!(e, json!("funcion-sin-comprobar"), "{funcion}");
        let (status, cuerpo) = llamar(&p, DESCARGAR, json!({}), Some(&cookie)).await;
        assert_eq!(status, 409, "{funcion}: {cuerpo}");
        assert!(cuerpo.contains("no se ha comprobado"), "{cuerpo}");
        assert!(p.sim.control.writes().is_empty(), "{funcion}");
        assert!(p.sim.control.copias_descargadas().is_empty(), "{funcion}");
        // La página la enseña desactivada, con el motivo.
        let html = pagina(&p, &cookie, "/ajustes/sistema").await;
        assert!(html.contains("Copia de la configuración"), "{html}");
        assert!(
            html.contains("no se ha comprobado en este router"),
            "{html}"
        );
    }
}

/// En solo lectura (firmware sin probar), tampoco.
#[tokio::test]
async fn en_solo_lectura_no_se_hace() {
    let p = PanelPrueba::nuevo().arrancar().await;
    instancia::escribir(&p.state.db, claves::SOLO_LECTURA, &json!(true))
        .await
        .unwrap();
    let cookie = sesion(&p).await;
    let e = ok(&p, ESTADO, json!({}), &cookie).await;
    assert_eq!(e, json!("panel-solo-lectura"));
    let (status, cuerpo) = llamar(&p, DESCARGAR, json!({}), Some(&cookie)).await;
    assert_eq!(status, 403, "{cuerpo}");
    assert!(cuerpo.contains("solo lectura"), "{cuerpo}");
    assert!(p.sim.control.writes().is_empty());
    assert!(p.sim.control.copias_descargadas().is_empty());
}

/// Con un cambio en curso, no: guardar a medias dejaría en la flash un
/// estado intermedio.
#[tokio::test]
async fn no_con_un_cambio_en_curso() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let _reserva = p.state.cambios().unwrap().excluir_cambios().unwrap();
    let (status, cuerpo) = llamar(&p, DESCARGAR, json!({}), Some(&cookie)).await;
    assert_eq!(status, 409, "{cuerpo}");
    assert!(cuerpo.contains("cambio en curso"), "{cuerpo}");
    assert!(p.sim.control.writes().is_empty());
}

#[tokio::test]
async fn sin_router_configurado() {
    let p = PanelPrueba::nuevo().sin_router().arrancar().await;
    let cookie = sesion(&p).await;
    let (status, cuerpo) = llamar(&p, DESCARGAR, json!({}), Some(&cookie)).await;
    assert_eq!(status, 503, "{cuerpo}");
    assert!(cuerpo.contains("router"), "{cuerpo}");
}

/// Si el router falla, un error genérico, sin nada del router.
#[tokio::test]
async fn un_fallo_del_router_es_502() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    p.sim.control.fail_next_write("9");
    let (status, cuerpo) = llamar(&p, DESCARGAR, json!({}), Some(&cookie)).await;
    assert_eq!(status, 502, "{cuerpo}");
    assert!(p.sim.control.copias_descargadas().is_empty());
}

/// La sección está en Sistema y solo en Sistema, y avisa de las contraseñas.
#[tokio::test]
async fn la_seccion_esta_en_sistema() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let html = pagina(&p, &cookie, "/ajustes/sistema").await;
    assert!(html.contains("Copia de la configuración"), "{html}");
    assert!(html.contains("todas las contraseñas del router"), "{html}");
    let html = pagina(&p, &cookie, "/ajustes/wifi").await;
    assert!(!html.contains("Copia de la configuración"));
}
