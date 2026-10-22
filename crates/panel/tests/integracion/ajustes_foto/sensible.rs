//! Volver a un dato sensible (el usuario del ACS) que la foto guardó tapado
//! (`****`): no se escribe la máscara como usuario.

use super::*;

async fn escribir_tr69(p: &Panel, usuario: &str) {
    p.state
        .hgu
        .write("tr69.asp", "cbSetTr69Config", &[("acs_username", usuario)])
        .await
        .unwrap();
    p.state.lecturas.invalidar();
}

#[tokio::test]
async fn volver_a_un_sensible_tapado_no_lo_escribe() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    escribir_tr69(&p, "****").await;
    tomar_foto(&p).await;
    escribir_tr69(&p, "usuario-nuevo-7310").await;
    let antes = p.sim.control.writes().len();
    let (status, body) = volver(
        &p,
        Some(&cookie),
        "seguridad.gestion_remota_oculto",
        Some("usuario"),
        None,
    )
    .await;
    // El gestor lo bloquea como a una clave tapada.
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("reenviarla podría borrarla"), "{body}");
    assert_eq!(p.sim.control.writes().len(), antes, "no se escribe nada");
    let n = p.state.hgu.call("cbGetTr69Config", &[]).await.unwrap();
    assert_eq!(
        elemento(&n, "acs_username").as_deref(),
        Some("usuario-nuevo-7310")
    );
}
