//! Deshacer un alta o una baja de usuarios del router exige reautenticación, desde Ajustes (`deshacer_ajuste`) y desde el Historial (`deshacer`), como crearlos o borrarlos.

use crate::comun::{Panel, PanelPrueba, aplicar_ajuste, esperar_final, llamar, ok, reautenticar};
use serde_json::{Value, json};

/// Cierra la ventana de reautenticación de todas las sesiones.
async fn cerrar_ventana(p: &Panel) {
    sqlx::query("UPDATE sessions SET reautenticado_hasta = NULL")
        .execute(&p.state.db)
        .await
        .unwrap();
}

/// Aplica la operación `op` de la tarjeta de usuarios y devuelve el cambio.
async fn tarjeta(p: &Panel, cookie: &str, op: Value) -> i64 {
    let g = ok(p, "preparar_usuario_router", json!({ "op": op }), cookie).await;
    aplicar_ajuste(p, cookie, &g).await
}

fn esta(p: &Panel, usuario: &str) -> bool {
    p.sim.control.usuarios().iter().any(|(u, _)| u == usuario)
}

#[tokio::test]
async fn deshacer_usuarios_exige_reautenticacion() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    reautenticar(&p, &cookie).await;
    let alta = tarjeta(
        &p,
        &cookie,
        json!({ "Crear": { "usuario": "prueba1", "contrasena": "clave-prueba-1",
            "grupo": "user" } }),
    )
    .await;
    let baja = tarjeta(&p, &cookie, json!({ "Borrar": { "usuario": "invitado1" } })).await;
    cerrar_ventana(&p).await;
    let escritas = p.sim.control.writes().len();
    for id in [alta, baja] {
        for (endpoint, cuerpo) in [
            ("deshacer_ajuste", json!({ "cambio": id })),
            ("deshacer", json!({ "change_id": id })),
        ] {
            let (status, body) = llamar(&p, endpoint, cuerpo, Some(&cookie)).await;
            assert_eq!(status, 401, "{endpoint} {id}: {body}");
            assert!(
                body.contains(panel::auth::ERROR_REAUTENTICACION),
                "{endpoint}: {body}"
            );
        }
    }
    assert_eq!(p.sim.control.writes().len(), escritas);
    // Con la ventana abierta: el alta se deshace desde Ajustes y la baja
    // desde el Historial.
    reautenticar(&p, &cookie).await;
    let d = ok(&p, "deshacer_ajuste", json!({ "cambio": alta }), &cookie).await;
    aplicar_ajuste(&p, &cookie, &d).await;
    assert!(!esta(&p, "prueba1"));
    let prep = ok(&p, "deshacer", json!({ "change_id": baja }), &cookie).await;
    let id = ok(
        &p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    assert!(esta(&p, "invitado1"));
}
