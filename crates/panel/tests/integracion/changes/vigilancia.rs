//! Vigilancia tras escribir: escrituras sin respuesta, WAN caída y reversión.

use super::*;

/// I1: la escritura llega y el router la aplica, pero no contesta (reinicia
/// un servicio, va cargado…): se relee, se ve aplicada y se vigila, con
/// deshacer disponible.
#[tokio::test]
async fn escritura_sin_respuesta_aplicada_se_vigila() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    p.sim.control.no_answer_next_write(true);
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    let c = esperar_estado(&m, id, Estado::Aplicado).await;
    // La nota se guarda como claves y se pinta en el idioma de quien mira.
    let nota = c.nota.unwrap_or_default();
    let es = Nota::texto(&nota, Idioma::Es);
    assert!(
        es.contains("no confirmó") && es.contains("releído: aplicada"),
        "{es}"
    );
    let en = Nota::texto(&nota, Idioma::En);
    assert!(
        en.contains("didn't confirm") && en.contains("read again: applied"),
        "{en}"
    );
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
    esperar_libre_gestor(&m).await;
    assert!(m.se_puede_deshacer(id).await);
}

/// I1: igual, pero la WAN cae durante la vigilancia: se revierte.
#[tokio::test]
async fn escritura_sin_respuesta_aplicada_se_revierte_si_cae_la_wan() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    p.sim.control.no_answer_next_write(true);
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_wan_up(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

/// I1: la escritura se corta sin aplicarse: el router sigue con lo de
/// antes ⇒ fallido («no se aplicó»), sin vigilar ni revertir.
#[tokio::test]
async fn escritura_sin_respuesta_no_aplicada_queda_fallida() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    p.sim.control.no_answer_next_write(false);
    let r = m.confirmar(&prep.token, None, "admin").await;
    assert!(
        matches!(&r, Err(CambioError::Router(e)) if e.contains("no se aplicó")),
        "{r:?}"
    );
    let ultimo = m.historial(1).await.remove(0);
    assert_eq!(ultimo.estado, Estado::Fallido);
    assert_eq!(p.sim.control.writes().len(), 1, "sin reversión");
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
    assert_eq!(m.activo(), None);
}

#[tokio::test]
async fn revierte_si_se_pierde_la_wan() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_wan_up(false);
    let c = esperar_estado(&m, id, Estado::Revertido).await;
    assert!(c.error.unwrap().contains("WAN"));
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
    let w = p.sim.control.writes();
    assert_eq!(w.len(), 2, "cambio y reversión");
    let revert: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(revert["ipv4_addr"], "192.168.1.68");
    assert_eq!(revert["rtm_dhcpv4_svr_pool_static_id"], "0");
    esperar_libre_gestor(&m).await;
}

#[tokio::test]
async fn wan_ya_caida_no_provoca_reversion() {
    let (p, m) = preparado().await;
    p.sim.control.set_wan_up(false);
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    let c = esperar_estado(&m, id, Estado::Aplicado).await;
    let nota = c.nota.unwrap();
    assert!(
        Nota::texto(&nota, Idioma::Es).contains("la WAN ya estaba caída"),
        "{nota}"
    );
    assert!(
        Nota::texto(&nota, Idioma::En).contains("the WAN was already down"),
        "{nota}"
    );
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.80"));
}

#[tokio::test]
async fn revierte_si_el_router_deja_de_responder() {
    let p = panel_rapido(Duration::from_millis(100)).await;
    let m = gestor(&p, tiempos_rapidos()).await;
    let prep = m.preparar(lease0("192.168.1.80")).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_unresponsive(true);
    // Tres fallos seguidos (100 ms de espera cada uno) y, cuando el router
    // vuelve, se revierte.
    tokio::time::sleep(Duration::from_millis(600)).await;
    p.sim.control.set_unresponsive(false);
    let c = esperar_estado(&m, id, Estado::Revertido).await;
    assert!(c.error.unwrap().contains("no responde"));
    assert_eq!(ip_de_lease(&p, "0").await.as_deref(), Some("192.168.1.68"));
}

#[tokio::test]
async fn revierte_una_creacion() {
    let (p, m) = preparado().await;
    let prep = m.preparar(nueva_reserva()).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_wan_up(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    assert_eq!(ip_de_lease(&p, "2").await, None);
    let w = p.sim.control.writes();
    let borrado: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(borrado["del"], "DEL");
    assert_eq!(borrado["rtm_dhcpv4_svr_pool_static_id"], "2");
}

/// D1: una reserva nueva idéntica a una que ya existía (como cuando el
/// router «añade» sobrescribiendo): la instancia del cambio es la que
/// aparece nueva, no la primera con esos valores; al revertir se borra ésa
/// y la de antes sigue.
#[tokio::test]
async fn crear_un_duplicado_revierte_solo_la_nueva() {
    let (p, m) = preparado().await;
    let antes = read::static_leases(&p.state.hgu).await.unwrap();
    let l0 = antes
        .iter()
        .find(|l| l.id == "0")
        .expect("reserva 0")
        .clone();
    let dup = req(
        LEASES,
        None,
        &[
            (POOL, "0"),
            ("adm_state", "1"),
            ("mac_addr", &l0.mac),
            ("ipv4_addr", &l0.ip),
        ],
    );
    let prep = m.preparar(dup).await.unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    let c = esperar_estado(&m, id, Estado::Vigilando).await;
    assert_eq!(c.instancia.as_deref(), Some("2"), "la nueva, no la 0");
    p.sim.control.set_wan_up(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    let despues = read::static_leases(&p.state.hgu).await.unwrap();
    assert!(
        despues.iter().any(|l| l.id == "0"),
        "la reserva de antes sigue"
    );
    assert!(!despues.iter().any(|l| l.id == "2"));
    let w = p.sim.control.writes();
    let borrado: BTreeMap<_, _> = w[1].1.iter().cloned().collect();
    assert_eq!(borrado["rtm_dhcpv4_svr_pool_static_id"], "2");
}

/// D1: si aparecen dos instancias nuevas no se sabe cuál es la del cambio:
/// fallido con un motivo claro y no se borra nada.
#[tokio::test]
async fn crear_con_dos_instancias_nuevas_no_borra_nada() {
    let (p, m) = preparado().await;
    let prep = m.preparar(nueva_reserva()).await.unwrap();
    p.sim.control.create_twice_next_write();
    let r = m.confirmar(&prep.token, None, "admin").await;
    let Err(CambioError::Verificacion(v)) = r else {
        panic!("{r:?}");
    };
    assert!(v.iter().any(|m| m.contains("2 instancias nuevas")), "{v:?}");
    let ultimo = m.historial(1).await.remove(0);
    assert_eq!(ultimo.estado, Estado::Fallido);
    assert_eq!(p.sim.control.writes().len(), 1, "no se borra nada");
    let leases = read::static_leases(&p.state.hgu).await.unwrap();
    assert_eq!(leases.iter().filter(|l| l.ip == "192.168.1.99").count(), 2);
}

#[tokio::test]
async fn revierte_un_secreto_que_estaba_vacio() {
    let (p, m) = preparado().await;
    let ap = [("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "")];
    p.state
        .hgu
        .write("wifi.asp", "cbSetWifiAccPt", &ap)
        .await
        .unwrap();
    let prep = m
        .preparar(req(
            "cbSetWifiAccPt",
            None,
            &[
                ("dpm_cfg_wifi_acc_pt_id", "1"),
                ("key_phrase", "clave-nueva-123"),
            ],
        ))
        .await
        .unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_wan_up(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    let w = p.sim.control.writes();
    assert_eq!(w.len(), 3, "preparación, cambio y reversión");
    let revert: BTreeMap<_, _> = w[2].1.iter().cloned().collect();
    assert_eq!(revert["key_phrase"], "");
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "1")])
        .await
        .unwrap();
    let key = n
        .get("dpm_cfg_wifi_acc_pt")
        .unwrap()
        .text_of("key_phrase")
        .map(str::to_owned);
    assert_eq!(key.as_deref(), Some(""));
}

#[tokio::test]
async fn revierte_un_valor_fuera_de_las_opciones() {
    let (p, m) = preparado().await;
    // El router tiene un valor que el catálogo no conoce.
    p.state
        .hgu
        .write(
            "dhcp.asp",
            LEASES,
            &[
                (POOL, "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("adm_state", "2"),
            ],
        )
        .await
        .unwrap();
    let prep = m
        .preparar(req(LEASES, Some("0"), &[(POOL, "0"), ("adm_state", "1")]))
        .await
        .unwrap();
    let id = m.confirmar(&prep.token, None, "admin").await.unwrap();
    p.sim.control.set_wan_up(false);
    esperar_estado(&m, id, Estado::Revertido).await;
    let n = p
        .state
        .hgu
        .call(
            "cbDhcpV4SvrPoolGetStatic",
            &[(POOL, "0"), ("rtm_dhcpv4_svr_pool_static_id", "0")],
        )
        .await
        .unwrap();
    let adm = n
        .get("rtm_dhcpv4_svr_pool_static")
        .unwrap()
        .text_of("adm_state")
        .map(str::to_owned);
    assert_eq!(adm.as_deref(), Some("2"));
}
