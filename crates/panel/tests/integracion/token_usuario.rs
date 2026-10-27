//! El token de preparar y confirmar va ligado al usuario del panel que lo preparó: otro no lo confirma ni lo gasta.

use crate::comun::{Panel, PanelPrueba, esperar_final, llamar, ok, reautenticar};
use crate::comun::{login_body, post_form, send, set_cookie};
use serde_json::{Value, json};

/// Crea otro usuario del panel e inicia sesión con él.
async fn sesion_de_otro(p: &Panel) -> String {
    panel::auth::create_user(&p.state.db, "otro01", "otra contraseña larga")
        .await
        .unwrap();
    let resp = send(
        &p.app,
        post_form(
            "/api/login",
            &login_body("otro01", "otra contraseña larga"),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 200);
    set_cookie(&resp)
        .split(';')
        .next()
        .unwrap()
        .trim()
        .to_owned()
}

/// Confirma `token` por `endpoint` con la cookie `cookie`.
async fn confirmar(p: &Panel, cookie: &str, endpoint: &str, token: &Value) -> (u16, String) {
    let cuerpo = match endpoint {
        "confirmar_ajuste" => json!({ "token": token, "desmarcados": [] }),
        _ => json!({ "token": token, "palabra": null }),
    };
    llamar(p, endpoint, cuerpo, Some(cookie)).await
}

#[tokio::test]
async fn otro_usuario_no_confirma_ni_gasta_el_token() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let otro = sesion_de_otro(&p).await;
    reautenticar(&p, &cookie).await;
    // Un ajuste (la regla «sin ping») y una escritura de la Biblioteca.
    let ajuste = ok(&p, "preparar_sin_ping", json!({}), &cookie).await;
    let alta = json!({ "funcion": "cbAddUser", "instancia": null,
        "valores": [["rqUser", "prueba1"], ["rqPwd", "clave-prueba-1"], ["rqGroup", "user"]],
        "borrar": false });
    let biblioteca = ok(&p, "preparar_cambio", json!({ "peticion": alta }), &cookie).await;
    for (endpoint, prep) in [
        ("confirmar_ajuste", &ajuste),
        ("confirmar_cambio", &biblioteca),
    ] {
        let escritas = p.sim.control.writes().len();
        let (status, body) = confirmar(&p, &otro, endpoint, &prep["token"]).await;
        assert_eq!(status, 400, "{endpoint}: {body}");
        assert_eq!(p.sim.control.writes().len(), escritas, "{endpoint}");
        // Quien lo preparó aún puede confirmarlo.
        let (status, body) = confirmar(&p, &cookie, endpoint, &prep["token"]).await;
        assert_eq!(status, 200, "{endpoint}: {body}");
        let id: i64 = body.trim().parse().unwrap();
        assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
        crate::comun::esperar_libre(&p).await;
    }
}

/// Un token que no está ligado a ningún usuario (preparado con el gestor
/// directamente) no se confirma, ni por el gestor ni por HTTP, y no escribe
/// nada; ligado, sí.
#[tokio::test]
async fn token_sin_usuario_se_rechaza() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let m = p.state.cambios().unwrap();
    let lease = crate::comun::req(
        "cbDhcpV4SvrPoolSetStatic",
        Some("0"),
        &[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("ipv4_addr", "192.168.1.80"),
        ],
    );
    let prep = m.preparar(lease).await.unwrap();
    let r = m.confirmar(&prep.token, None, "admin").await;
    assert!(
        matches!(r, Err(panel::changes::CambioError::TokenInvalido)),
        "{r:?}"
    );
    let (status, body) = confirmar(&p, &cookie, "confirmar_cambio", &json!(prep.token)).await;
    assert_eq!(status, 400, "{body}");
    assert!(p.sim.control.writes().is_empty());
    // Ligado a quien confirma, sí vale (el token no se gastó).
    m.ligar(&prep.token, "admin");
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    assert!(id > 0);
}
