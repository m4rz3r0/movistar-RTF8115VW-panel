//! Guardas de los usuarios del router y de «Sin ping» en el gestor: el interruptor de la validación real, los motivos con texto y la reversión automática de un borrado.

use crate::comun::{Panel, pagina, registro_global};
use crate::comun::{PanelPrueba, USER, aplicar_ajuste, esperar_final, esperar_hasta, llamar, ok};
use panel::validacion_real::{Funcion3, motivo};
use serde_json::{Value, json};
use std::time::Duration;

const INVITADO: &str = "invitado1";
const CLAVE: &str = "clave-prueba-1";

/// Una petición de la Biblioteca (`preparar_cambio`).
async fn biblioteca(
    p: &Panel,
    cookie: &str,
    funcion: &str,
    instancia: Option<&str>,
    valores: Value,
    borrar: bool,
) -> (u16, String) {
    llamar(
        p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": funcion, "instancia": instancia,
            "valores": valores, "borrar": borrar } }),
        Some(cookie),
    )
    .await
}

fn alta(nombre: &str, grupo: &str) -> Value {
    json!([["rqUser", nombre], ["rqPwd", CLAVE], ["rqGroup", grupo]])
}

/// Con `Usuarios` y `SinPing` desactivadas, el gestor rechaza el alta y la
/// baja de usuarios (tarjeta, Biblioteca) y el alta de «Sin ping» con 409 y
/// su motivo, sin escribir; el deshacer de lo de antes sigue funcionando.
#[tokio::test]
async fn interruptor_desactivado_vale_en_el_gestor() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = ok(
        &p,
        "preparar_usuario_router",
        json!({ "op": { "Crear": { "usuario": "prueba1", "contrasena": CLAVE,
            "grupo": "user" } } }),
        &cookie,
    )
    .await;
    let alta_previa = aplicar_ajuste(&p, &cookie, &g).await;
    let s = ok(&p, "preparar_sin_ping", json!({}), &cookie).await;
    let sin_ping_previo = aplicar_ajuste(&p, &cookie, &s).await;
    p.state
        .cambios()
        .unwrap()
        .forzar_interruptores(|f| match f {
            Funcion3::Usuarios | Funcion3::SinPing => Some(motivo(f)),
            Funcion3::ContrasenaOtro => None,
        });
    let escritas = p.sim.control.writes().len();
    let pendiente = "aún no se ha comprobado con el router de verdad";
    for (status, body) in [
        biblioteca(
            &p,
            &cookie,
            "cbAddUser",
            None,
            alta("prueba2", "user"),
            false,
        )
        .await,
        biblioteca(
            &p,
            &cookie,
            "cbDeleteUser",
            None,
            json!([["rqDelList", "prueba1:"]]),
            false,
        )
        .await,
        biblioteca(&p, &cookie, "cbAddUser", Some("prueba1"), json!([]), true).await,
    ] {
        assert_eq!(status, 409, "{body}");
        assert!(body.contains(pendiente), "{body}");
    }
    // La de «Sin ping», con su regla quitada para que no salga «ya puesta».
    let d = ok(
        &p,
        "deshacer_ajuste",
        json!({ "cambio": sin_ping_previo }),
        &cookie,
    )
    .await;
    aplicar_ajuste(&p, &cookie, &d).await;
    let escritas_tras_deshacer = p.sim.control.writes().len();
    assert_eq!(escritas_tras_deshacer, escritas + 1);
    let (status, body) = llamar(&p, "preparar_sin_ping", json!({}), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains(pendiente), "{body}");
    assert_eq!(p.sim.control.writes().len(), escritas_tras_deshacer);
    // El deshacer del alta de antes sigue.
    let d = ok(
        &p,
        "deshacer_ajuste",
        json!({ "cambio": alta_previa }),
        &cookie,
    )
    .await;
    aplicar_ajuste(&p, &cookie, &d).await;
    assert!(!p.sim.control.usuarios().iter().any(|(u, _)| u == "prueba1"));
}

/// Los motivos concretos de los usuarios llegan con su texto, no como una
/// petición no válida genérica.
#[tokio::test]
async fn motivos_de_usuarios_con_su_texto() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let casos = [
        (
            biblioteca(
                &p,
                &cookie,
                "cbDeleteUser",
                None,
                json!([["rqDelList", "invitado1:otro01:"]]),
                false,
            )
            .await,
            400,
            "de uno en uno",
        ),
        (
            biblioteca(
                &p,
                &cookie,
                "cbDeleteUser",
                None,
                json!([["rqDelList", "nadie01:"]]),
                false,
            )
            .await,
            409,
            "ya no está en el router",
        ),
        (
            biblioteca(
                &p,
                &cookie,
                "cbAddUser",
                None,
                alta("prueba1", "system"),
                false,
            )
            .await,
            400,
            "es el del operador",
        ),
        (
            biblioteca(
                &p,
                &cookie,
                "cbAddUser",
                None,
                alta("prueba1", "root"),
                false,
            )
            .await,
            400,
            "Elige un tipo de cuenta",
        ),
    ];
    for ((status, body), esperado, texto) in casos {
        assert_eq!(status, esperado, "{body}");
        assert!(body.contains(texto), "{texto}: {body}");
    }
    assert!(p.sim.control.writes().is_empty());
}

/// Borrar el usuario del panel con la forma de la tarjeta (`cbAddUser` con
/// `borrar`) desde la Biblioteca: 403 sin escribir.
#[tokio::test]
async fn biblioteca_no_borra_un_protegido_como_alta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = biblioteca(&p, &cookie, "cbAddUser", Some(USER), json!([]), true).await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("protegido"), "{body}");
    assert!(p.sim.control.writes().is_empty());
}

/// Si la vigilancia revierte el borrado de un usuario, vuelve con una
/// contraseña que el panel no guarda: el cambio lleva una nota que lo dice
/// y la contraseña no sale en el registro ni en el Historial.
#[tokio::test]
async fn reversion_de_un_borrado_avisa_de_la_contrasena() {
    let registro = registro_global();
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = ok(
        &p,
        "preparar_usuario_router",
        json!({ "op": { "Borrar": { "usuario": INVITADO } } }),
        &cookie,
    )
    .await;
    let id = ok(
        &p,
        "confirmar_ajuste",
        json!({ "token": g["token"], "desmarcados": [] }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    esperar_hasta(Duration::from_secs(5), "no llega a vigilarse", async || {
        ok(&p, "estado_cambio", json!({ "id": id }), &cookie).await["estado"] == "Vigilando"
    })
    .await;
    p.sim.control.set_wan_up(false);
    let c = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Revertido", "{c}");
    assert!(
        p.sim
            .control
            .usuarios()
            .contains(&(INVITADO.to_owned(), "user".to_owned()))
    );
    let nota: String = sqlx::query_scalar("SELECT COALESCE(nota, '') FROM changes WHERE id = ?")
        .bind(id)
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert!(nota.contains("usuarios-router-nota-revertido"), "{nota}");
    let html = pagina(&p, &cookie, "/historial").await;
    assert!(html.contains("contraseña nueva que el panel no guarda"));
    let x = p.sim.control.contrasena_de(INVITADO).unwrap();
    assert_eq!(x.len(), 16);
    assert!(!registro.texto().contains(&x), "en el registro");
    assert!(!html.contains(&x), "en el HTML de /historial");
}

/// Si la reversión de un borrado llega a escribir el alta pero no se
/// verifica, el usuario pudo volver con una contraseña que el panel no
/// guarda: el cambio, fallido, también lleva la nota que lo dice.
#[tokio::test]
async fn reversion_fallida_tras_escribir_tambien_avisa() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = ok(
        &p,
        "preparar_usuario_router",
        json!({ "op": { "Borrar": { "usuario": INVITADO } } }),
        &cookie,
    )
    .await;
    let id = ok(
        &p,
        "confirmar_ajuste",
        json!({ "token": g["token"], "desmarcados": [] }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    esperar_hasta(Duration::from_secs(5), "no llega a vigilarse", async || {
        ok(&p, "estado_cambio", json!({ "id": id }), &cookie).await["estado"] == "Vigilando"
    })
    .await;
    let escritas = p.sim.control.writes().len();
    // El alta de la reversión responde bien pero no se aplica.
    p.sim.control.ignore_next_write();
    p.sim.control.set_wan_up(false);
    let c = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Fallido", "{c}");
    assert!(
        p.sim.control.writes().len() > escritas,
        "no llegó a escribir"
    );
    let (nota, error): (String, String) =
        sqlx::query_as("SELECT COALESCE(nota, ''), COALESCE(error, '') FROM changes WHERE id = ?")
            .bind(id)
            .fetch_one(&p.state.db)
            .await
            .unwrap();
    assert!(nota.contains("usuarios-router-nota-revertido"), "{nota}");
    assert!(!error.is_empty(), "sin el detalle de la reversión");
}
