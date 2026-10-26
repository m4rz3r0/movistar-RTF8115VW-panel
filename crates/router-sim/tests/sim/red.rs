//! El cliente se recupera solo cuando el router rehace su red o su servidor
//! web (en el router real, tras escribir `cbSetQueue`): conexiones que dejan
//! de responder sin cerrarse y sesiones que se quedan colgadas.

use super::*;
use hgu::client::EstadoContacto;
use std::time::Instant;

/// Con las conexiones de antes cortadas, la petición siguiente ya va bien:
/// el cliente no reutiliza conexiones (antes reutilizaba la cortada y
/// agotaba el tiempo de espera).
#[tokio::test]
async fn conexiones_cortadas_no_se_reutilizan() {
    let sim = start().await;
    let c = client(&sim, Duration::from_millis(500));
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
    sim.control.cortar_conexiones();
    let t = Instant::now();
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
    assert!(
        t.elapsed() < Duration::from_millis(400),
        "{:?}",
        t.elapsed()
    );
}

/// Una sesión colgada falla una vez (tiempo agotado) y la siguiente
/// petición entra de nuevo: el error de red descarta la sesión. Antes se
/// seguía mandando la misma para siempre.
#[tokio::test]
async fn sesion_colgada_se_abandona_tras_un_error() {
    let sim = start().await;
    let c = client(&sim, Duration::from_millis(300));
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
    sim.control.colgar_sesiones();
    let r = c.call("cbGetUpnpCfg", &[]).await;
    assert!(matches!(&r, Err(Error::Network(_))), "{r:?}");
    assert_eq!(
        c.contacto().map(|k| k.estado),
        Some(EstadoContacto::SinRespuesta)
    );
    c.call("cbGetUpnpCfg", &[]).await.unwrap();
    assert_eq!(
        c.contacto().map(|k| k.estado),
        Some(EstadoContacto::Responde)
    );
}

/// Como en el router real: la escritura responde y después el router rehace
/// su red. Una sola lectura falla y las siguientes van bien, también por la
/// página de la web básica.
#[tokio::test]
async fn tras_rehacer_la_red_se_recupera() {
    let sim = start().await;
    let c = client(&sim, Duration::from_millis(300));
    sim.control.rehacer_red_tras_proxima_escritura();
    c.write("upnp.asp", "cbSetUpnpCfg", &[("adm_state", "1")])
        .await
        .unwrap();
    let r = c.call("cbGetUpnpCfg", &[]).await;
    assert!(matches!(&r, Err(Error::Network(_))), "{r:?}");
    for _ in 0..3 {
        c.call("cbGetUpnpCfg", &[]).await.unwrap();
    }
    c.basic_page("te_wifi.asp").await.unwrap();
}

/// El error de red lleva la cadena de causas (no solo «error sending
/// request») y nunca la consulta de la URL.
#[tokio::test]
async fn el_error_de_red_lleva_su_causa() {
    let sim = start().await;
    let c = client(&sim, Duration::from_millis(200));
    sim.control.set_unresponsive(true);
    let r = c
        .basic_write("te_wifi.asp", "te_wifi.wl", &[("wlSsid", "x")])
        .await;
    sim.control.set_unresponsive(false);
    let Err(Error::Network(e)) = r else {
        panic!("{r:?}");
    };
    assert!(e.contains(": "), "{e}");
    assert!(!e.contains("sessionKey") && !e.contains("wlSsid"), "{e}");
}
