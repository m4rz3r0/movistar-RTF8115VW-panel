//! Cola de cambios: lo que se confirma mientras otro cambio se vigila
//! espera su turno; si el anterior no queda aplicado, no se aplica.

use super::*;
use crate::comun::ConfirmarLigado;
use panel::changes::{ErrorQuitar, MAX_COLA};

const UPNP: &str = "cbSetUpnpCfg";

/// Vigilancia larga (60 s), mirando cada 30 ms; revierte tras 3 fallos.
fn largos() -> Tiempos {
    Tiempos {
        bajo: Duration::from_secs(60),
        medio: Duration::from_secs(60),
        alto: Duration::from_secs(60),
        ..tiempos_rapidos()
    }
}

/// Activar UPnP (no corta el acceso: sin vigilancia).
fn upnp() -> ChangeRequest {
    req(UPNP, None, &[("adm_state", "1")])
}

async fn upnp_activo(p: &Panel) -> bool {
    fn buscar<'a>(x: &'a hgu::xml::XmlNode, tag: &str) -> Option<&'a str> {
        x.text_of(tag)
            .or_else(|| x.children.iter().find_map(|c| buscar(c, tag)))
    }
    let x = p.state.hgu.call("cbGetUpnpCfg", &[]).await.unwrap();
    buscar(&x, "adm_state") == Some("1")
}

/// Cambia la reserva 0 (se vigila: toca el DHCP) y espera a que la
/// vigilancia se pueda terminar.
async fn vigilando(m: &Arc<ChangeManager>) -> i64 {
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m
        .confirmar_ligado(&prep.token, None, "admin")
        .await
        .unwrap();
    esperar_hasta(Duration::from_secs(5), "no se vigila", async || {
        m.terminable(id, "admin").await
    })
    .await;
    id
}

/// Prepara y confirma `r` como `admin`.
async fn confirmar(m: &Arc<ChangeManager>, r: ChangeRequest) -> Result<i64, CambioError> {
    let prep = m.preparar(r).await.unwrap();
    m.confirmar_ligado(&prep.token, None, "admin").await
}

fn texto(c: &panel::changes::Cambio) -> String {
    c.error
        .as_deref()
        .map(|e| Nota::texto(e, Idioma::Es))
        .unwrap_or_default()
}

/// Confirmado durante la vigilancia, espera `en_cola` (el token ya no
/// caduca) y se aplica en cuanto el anterior queda aplicado, aquí al
/// terminar la vigilancia a mano.
#[tokio::test]
async fn espera_y_se_aplica_tras_el_anterior() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let m = gestor(&p, largos()).await;
    let a = vigilando(&m).await;
    assert!(m.admite_cambio().is_ok());
    let b = confirmar(&m, upnp()).await.unwrap();
    let c = m.cambio(b).await.unwrap();
    assert_eq!(c.estado, Estado::EnCola);
    assert!(!c.diff.campos.is_empty(), "{c:?}");
    assert_eq!(m.en_cola(), vec![b]);
    assert_eq!(m.activo(), Some(a));
    assert!(!upnp_activo(&p).await);

    m.terminar(a, "admin").await.unwrap();
    esperar_estado(&m, b, Estado::Aplicado).await;
    esperar_libre_gestor(&m).await;
    assert!(upnp_activo(&p).await);
    assert!(m.en_cola().is_empty());
    // Se puede deshacer como cualquier otro.
    assert!(m.se_puede_deshacer(b).await);
}

/// Si la vigilancia del anterior lo deshace, lo que esperaba no se aplica:
/// `cancelado`, con el motivo.
#[tokio::test]
async fn se_cancela_si_el_anterior_se_deshace() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let m = gestor(&p, largos()).await;
    let a = vigilando(&m).await;
    let b = confirmar(&m, upnp()).await.unwrap();
    p.sim.control.set_wan_up(false);
    esperar_estado(&m, a, Estado::Revertido).await;
    let c = esperar_estado(&m, b, Estado::Cancelado).await;
    assert!(texto(&c).contains("el cambio anterior se deshizo"), "{c:?}");
    assert!(
        c.pasos
            .iter()
            .all(|p| p.estado == panel::changes::EstadoPaso::Cancelado),
        "{c:?}"
    );
    esperar_libre_gestor(&m).await;
    assert!(!upnp_activo(&p).await);
}

/// Como mucho [`MAX_COLA`] esperando: el siguiente, `ColaLlena`, y su token
/// sigue valiendo.
#[tokio::test]
async fn la_cola_tiene_tope() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let m = gestor(&p, largos()).await;
    vigilando(&m).await;
    for _ in 0..MAX_COLA {
        confirmar(&m, upnp()).await.unwrap();
    }
    assert_eq!(m.en_cola().len(), MAX_COLA);
    assert_eq!(m.admite_cambio(), Err(CambioError::ColaLlena));
    let prep = m.preparar(upnp()).await.unwrap();
    let r = m.confirmar_ligado(&prep.token, None, "admin").await;
    assert_eq!(r, Err(CambioError::ColaLlena));
    // Al quitar uno, cabe con el mismo token.
    let primero = m.en_cola()[0];
    m.quitar_de_la_cola(primero, "admin").await.unwrap();
    m.confirmar_ligado(&prep.token, None, "admin")
        .await
        .unwrap();
    m.detener();
}

/// Solo quien lo encoló lo quita; queda `cancelado` con quién lo quitó y
/// la cola sigue con los demás.
#[tokio::test]
async fn quitar_de_la_cola() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let m = gestor(&p, largos()).await;
    let a = vigilando(&m).await;
    let b = confirmar(&m, upnp()).await.unwrap();
    assert_eq!(
        m.quitar_de_la_cola(b, "otra").await,
        Err(ErrorQuitar::OtroUsuario)
    );
    assert_eq!(m.en_cola()[0], b);
    m.quitar_de_la_cola(b, "admin").await.unwrap();
    let c = m.cambio(b).await.unwrap();
    assert_eq!(c.estado, Estado::Cancelado);
    assert!(texto(&c).contains("admin lo quitó de la cola"), "{c:?}");
    assert_eq!(
        m.quitar_de_la_cola(b, "admin").await,
        Err(ErrorQuitar::NoEnCola)
    );
    assert_eq!(
        m.quitar_de_la_cola(a, "admin").await,
        Err(ErrorQuitar::NoEnCola)
    );
    m.terminar(a, "admin").await.unwrap();
    esperar_libre_gestor(&m).await;
    assert!(!upnp_activo(&p).await);
}

/// Justo antes de escribir, lo que esperaba se vuelve a calcular: si el
/// router cambió desde que se revisó, no se escribe (`cancelado`) y la cola
/// se para.
#[tokio::test]
async fn el_router_cambio_desde_que_se_reviso() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let m = gestor(&p, largos()).await;
    let a = vigilando(&m).await;
    // Dos veces lo mismo: al llegarle el turno al segundo, ya está hecho.
    let b = confirmar(&m, upnp()).await.unwrap();
    let c = confirmar(&m, upnp()).await.unwrap();
    let d = confirmar(&m, lease0("192.168.1.82")).await.unwrap();
    m.terminar(a, "admin").await.unwrap();
    esperar_estado(&m, b, Estado::Aplicado).await;
    let c = esperar_estado(&m, c, Estado::Cancelado).await;
    assert!(texto(&c).contains("el router ha cambiado"), "{c:?}");
    let d = esperar_estado(&m, d, Estado::Cancelado).await;
    assert!(
        texto(&d).contains("el cambio anterior no llegó a aplicarse"),
        "{d:?}"
    );
    esperar_libre_gestor(&m).await;
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
}

/// Si el panel se detiene con cambios en la cola, al arrancar quedan
/// `cancelado`: nunca se aplican a ciegas.
#[tokio::test]
async fn al_reiniciar_se_cancela_la_cola() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let a = gestor(&p, largos()).await;
    let primero = vigilando(&a).await;
    let b = confirmar(&a, upnp()).await.unwrap();
    // El proceso muere a mitad de la vigilancia; la fila sigue `en_cola`
    // si no da tiempo a cancelarla.
    a.detener();
    let nuevo = ChangeManager::new(p.state.clone(), largos());
    nuevo.reanudar_pendientes().await;
    let c = esperar_estado(&nuevo, b, Estado::Cancelado).await;
    assert!(texto(&c).contains("se detuvo o se reinició"), "{c:?}");
    assert!(nuevo.en_cola().is_empty());
    // El vigilado se retoma; lo de la cola no se aplica.
    assert_eq!(nuevo.activo(), Some(primero));
    nuevo.detener();
    assert!(!upnp_activo(&p).await);
}
