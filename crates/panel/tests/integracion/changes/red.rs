//! El router rehace su red justo después de una escritura (como con
//! `cbSetQueue` en el router real): corta las conexiones y deja colgadas las
//! sesiones abiertas. El panel se recupera solo en cuanto vuelve a
//! responder: el cambio termina aplicado y las lecturas siguen.

use super::*;
use crate::comun::ConfirmarLigado;
use panel::lecturas::Clave;

#[tokio::test]
async fn tras_rehacer_la_red_el_panel_sigue_hablando_con_el_router() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    p.sim.control.rehacer_red_tras_proxima_escritura();
    let id = m
        .confirmar_ligado(&prep.token, None, "admin")
        .await
        .unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
    // Las páginas (la caché de lecturas, en una tanda) también.
    let k = Clave::new::<&str, &str>("cbGetUpnpCfg", &[]);
    let r = p
        .state
        .lecturas
        .leer(&p.state.hgu, std::slice::from_ref(&k))
        .await;
    assert!(r[&k].is_ok(), "{:?}", r[&k]);
    assert_eq!(
        p.state.hgu.contacto().map(|c| c.estado),
        Some(hgu::client::EstadoContacto::Responde)
    );
}
