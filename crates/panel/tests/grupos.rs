//! Pruebas del gestor de cambios por grupos de pasos contra `router-sim`,
//! con tiempos de vigilancia cortos.

mod common;

use catalogo::Riesgo;
use common::{Panel, get, panel};
use hgu::read;
use panel::{
    changes::{
        Cambio, CambioError, ChangeManager, ChangeRequest, Estado, EstadoPaso, GrupoRequest, Paso,
        Tiempos,
    },
    poller::{Poller, PollerConfig},
};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

const LEASES: &str = "cbDhcpV4SvrPoolSetStatic";
const POOL: &str = "rtm_cfg_dhcpv4_svr_pool_id";
const UPNP: &str = "cbSetUpnpCfg";
const WIFI: &str = "cbSetWifiAccPt";

fn tiempos() -> Tiempos {
    Tiempos {
        bajo: Duration::from_millis(300),
        medio: Duration::from_millis(300),
        alto: Duration::from_millis(600),
        intervalo: Duration::from_millis(30),
        fallos_para_revertir: 3,
        pausa_max: Duration::from_secs(2),
    }
}

async fn panel_rapido(timeout: Duration) -> Panel {
    let mut p = panel().await;
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, common::USER, common::PASS);
    cfg.timeout = timeout;
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    p
}

async fn gestor(p: &Panel, t: Tiempos) -> Arc<ChangeManager> {
    let m = ChangeManager::new(p.state.clone(), t);
    m.reanudar_pendientes().await;
    m
}

async fn preparado() -> (Panel, Arc<ChangeManager>) {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let m = gestor(&p, tiempos()).await;
    (p, m)
}

fn req(funcion: &str, instancia: Option<&str>, valores: &[(&str, &str)]) -> ChangeRequest {
    ChangeRequest {
        funcion: funcion.into(),
        instancia: instancia.map(str::to_owned),
        valores: valores
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect::<BTreeMap<_, _>>(),
        borrar: false,
    }
}

fn paso(r: ChangeRequest, etiqueta: &str) -> Paso {
    Paso {
        req: r,
        etiqueta: Some(etiqueta.into()),
    }
}

/// UPnP (`adm_state` 0 en las fixtures).
fn upnp(v: &str) -> Paso {
    paso(req(UPNP, None, &[("adm_state", v)]), "ajuste-upnp")
}

/// IP de la reserva 0 (`192.168.1.68` en las fixtures).
fn lease0(ip: &str) -> Paso {
    paso(
        req(LEASES, Some("0"), &[(POOL, "0"), ("ipv4_addr", ip)]),
        "ajuste-reserva",
    )
}

/// SSID del punto de acceso 1 (`RED-1` en las fixtures). Riesgo medio.
fn ssid(v: &str) -> Paso {
    paso(
        req(WIFI, None, &[("dpm_cfg_wifi_acc_pt_id", "1"), ("ssid", v)]),
        "ajuste-ssid",
    )
}

fn grupo(pasos: Vec<Paso>) -> GrupoRequest {
    GrupoRequest {
        pasos,
        titulo: Some("ajuste-prueba".into()),
    }
}

async fn esperar_estado(m: &ChangeManager, id: i64, estado: Estado) -> Cambio {
    let start = Instant::now();
    loop {
        let c = m.cambio(id).await.expect("el cambio existe");
        if c.estado == estado {
            return c;
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "el cambio {id} sigue en {:?} (esperado {estado:?}): {:?}",
            c.estado,
            c.error
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Espera a que el cambio llegue a un estado final.
async fn esperar_final(m: &ChangeManager, id: i64) -> Cambio {
    let start = Instant::now();
    loop {
        let c = m.cambio(id).await.expect("el cambio existe");
        if !matches!(c.estado, Estado::Aplicando | Estado::Vigilando) {
            return c;
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "el cambio {id} sigue en {:?}",
            c.estado
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn esperar_libre(m: &ChangeManager) {
    let start = Instant::now();
    while m.activo().is_some() {
        assert!(start.elapsed() < Duration::from_secs(10), "sigue activo");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn ip_de_lease0(p: &Panel) -> String {
    read::static_leases(&p.state.hgu)
        .await
        .unwrap()
        .into_iter()
        .find(|l| l.id == "0")
        .map(|l| l.ip)
        .unwrap()
}

async fn upnp_actual(p: &Panel) -> String {
    let n = p.state.hgu.call("cbGetUpnpCfg", &[]).await.unwrap();
    n.get("upnp_cfg")
        .and_then(|u| u.text_of("adm_state"))
        .unwrap()
        .to_owned()
}

async fn ssid_actual(p: &Panel) -> String {
    let n = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "1")])
        .await
        .unwrap();
    n.get("dpm_cfg_wifi_acc_pt")
        .and_then(|a| a.text_of("ssid"))
        .unwrap()
        .to_owned()
}

/// Funciones de las escrituras recibidas, en orden.
fn escritas(p: &Panel) -> Vec<String> {
    p.sim.control.writes().into_iter().map(|(f, _)| f).collect()
}

fn estados_pasos(c: &Cambio) -> Vec<EstadoPaso> {
    c.pasos.iter().map(|p| p.estado).collect()
}

#[tokio::test]
async fn grupo_aplica_en_orden_y_verifica() {
    let (p, m) = preparado().await;
    let prep = m
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.84")]))
        .await
        .unwrap();
    assert_eq!(prep.riesgo, Riesgo::Bajo);
    assert_eq!(prep.pasos.len(), 2);
    assert_eq!(prep.pasos[0].indice, 0);
    assert_eq!(prep.pasos[0].funcion, UPNP);
    assert_eq!(prep.pasos[0].etiqueta.as_deref(), Some("ajuste-upnp"));
    assert!(prep.pasos.iter().all(|p| !p.sin_cambios));
    assert_eq!(prep.pasos[1].diff.campos[0].despues, "192.168.1.84");

    let id = m.confirmar_grupo(&prep.token, &[], "admin").await.unwrap();
    let c = esperar_estado(&m, id, Estado::Aplicado).await;
    assert_eq!(escritas(&p), [UPNP, LEASES]);
    assert_eq!(upnp_actual(&p).await, "1");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.84");
    assert_eq!(
        estados_pasos(&c),
        [EstadoPaso::Aplicado, EstadoPaso::Aplicado]
    );
    assert_eq!(c.pasos[0].orden, 0);
    assert_eq!(c.pasos[1].funcion, LEASES);
    assert_eq!(c.pasos[1].instancia.as_deref(), Some("0"));
    assert_eq!(c.pasos[1].etiqueta.as_deref(), Some("ajuste-reserva"));
    assert_eq!(c.titulo.as_deref(), Some("ajuste-prueba"));
    // El diff del grupo junta los de los pasos.
    assert_eq!(c.diff.campos.len(), 2);
    esperar_libre(&m).await;
}

#[tokio::test]
async fn fallo_en_paso_k_revierte_anteriores() {
    let (p, m) = preparado().await;
    let prep = m
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.84"), ssid("OTRA")]))
        .await
        .unwrap();
    assert_eq!(prep.riesgo, Riesgo::Medio, "el mayor de los pasos");
    p.sim.control.fail_write_n(3);
    let r = m.confirmar_grupo(&prep.token, &[], "admin").await;
    assert!(r.is_err(), "{r:?}");
    let c = m.historial(1).await.remove(0);
    assert_eq!(c.estado, Estado::Fallido, "{:?}", c.error);
    // Los pasos 2 y 1, en orden inverso.
    assert_eq!(escritas(&p), [UPNP, LEASES, WIFI, LEASES, UPNP]);
    assert_eq!(upnp_actual(&p).await, "0");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    assert_eq!(ssid_actual(&p).await, "RED-1");
    assert_eq!(
        estados_pasos(&c),
        [
            EstadoPaso::Revertido,
            EstadoPaso::Revertido,
            EstadoPaso::Fallido
        ]
    );
    assert_eq!(m.activo(), None);
}

#[tokio::test]
async fn fallo_al_revertir_es_revertido_parcial() {
    let (p, m) = preparado().await;
    let prep = m
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.84"), ssid("OTRA")]))
        .await
        .unwrap();
    // Falla el paso 3 y, al revertir, la reversión del paso 1 (escritura 5).
    p.sim.control.fail_write_n(3);
    p.sim.control.fail_write_n(5);
    let r = m.confirmar_grupo(&prep.token, &[], "admin").await;
    let Err(CambioError::Router(e)) = &r else {
        panic!("{r:?}");
    };
    assert!(e.contains("paso 1"), "{e}");
    let c = m.historial(1).await.remove(0);
    assert_eq!(c.estado, Estado::RevertidoParcial);
    let detalle = c.error.clone().unwrap();
    assert!(detalle.contains("paso 1"), "{detalle}");
    assert!(!detalle.contains("paso 2:"), "{detalle}");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    assert_eq!(upnp_actual(&p).await, "1", "el paso 1 sigue aplicado");
    assert_eq!(
        estados_pasos(&c),
        [
            EstadoPaso::Aplicado,
            EstadoPaso::Revertido,
            EstadoPaso::Fallido
        ]
    );
    assert_eq!(m.activo(), None);
}

#[tokio::test]
async fn vigilancia_revierte_grupo_entero() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    // El riesgo medio vigila más que el bajo: la vigilancia del grupo es la
    // del paso de mayor riesgo.
    let t = Tiempos {
        bajo: Duration::from_millis(50),
        medio: Duration::from_secs(5),
        ..tiempos()
    };
    let m = gestor(&p, t).await;
    let prep = m
        .preparar_grupo(grupo(vec![lease0("192.168.1.84"), ssid("OTRA")]))
        .await
        .unwrap();
    assert_eq!(prep.riesgo, Riesgo::Medio);
    let id = m.confirmar_grupo(&prep.token, &[], "admin").await.unwrap();
    assert_eq!(m.cambio(id).await.unwrap().riesgo, Riesgo::Medio);
    // Pasado el plazo del riesgo bajo, la vigilancia sigue.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(m.cambio(id).await.unwrap().estado, Estado::Vigilando);
    p.sim.control.set_wan_up(false);
    let c = esperar_estado(&m, id, Estado::Revertido).await;
    assert!(c.error.as_deref().unwrap().contains("WAN"));
    assert_eq!(escritas(&p), [LEASES, WIFI, WIFI, LEASES]);
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    assert_eq!(ssid_actual(&p).await, "RED-1");
    assert_eq!(
        estados_pasos(&c),
        [EstadoPaso::Revertido, EstadoPaso::Revertido]
    );
    esperar_libre(&m).await;
}

#[tokio::test]
async fn deshacer_grupo() {
    let (p, m) = preparado().await;
    let prep = m
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.84")]))
        .await
        .unwrap();
    let id = m.confirmar_grupo(&prep.token, &[], "admin").await.unwrap();
    esperar_estado(&m, id, Estado::Aplicado).await;
    esperar_libre(&m).await;
    assert!(m.se_puede_deshacer(id).await);

    let d = m.deshacer(id).await.unwrap();
    // Los dos pasos, en orden inverso.
    assert_eq!(d.pasos.len(), 2);
    assert_eq!(d.pasos[0].funcion, LEASES);
    assert_eq!(d.pasos[1].funcion, UPNP);
    assert_eq!(d.diff.campos.len(), 2);
    let id2 = m.confirmar(&d.token, None, "admin").await.unwrap();
    let c2 = esperar_estado(&m, id2, Estado::Aplicado).await;
    assert_eq!(c2.pasos.len(), 2);
    assert_eq!(escritas(&p), [UPNP, LEASES, LEASES, UPNP]);
    assert_eq!(upnp_actual(&p).await, "0");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    esperar_libre(&m).await;
    assert!(!m.se_puede_deshacer(id).await, "ya deshecho");
}

#[tokio::test]
async fn reanuda_grupo_a_medias() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        ..tiempos()
    };
    let a = gestor(&p, lento).await;
    let prep = a
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.84")]))
        .await
        .unwrap();
    // El router se cuelga tras la escritura del paso 2 y el panel muere
    // antes de terminar.
    p.sim.control.hang_after_write_n(2);
    let a2 = a.clone();
    let token = prep.token.clone();
    let tarea = tokio::spawn(async move { a2.confirmar_grupo(&token, &[], "admin").await });
    let start = Instant::now();
    while p.sim.control.writes().len() < 2 {
        assert!(start.elapsed() < Duration::from_secs(5), "sin paso 2");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    a.detener();
    tarea.abort();
    let id = a.historial(1).await.remove(0).id;
    p.sim.control.set_unresponsive(false);

    let b = ChangeManager::new(p.state.clone(), tiempos());
    b.reanudar_pendientes().await;
    let c = esperar_final(&b, id).await;
    let (upnp, ip) = (upnp_actual(&p).await, ip_de_lease0(&p).await);
    match c.estado {
        Estado::Aplicado => assert_eq!((upnp.as_str(), ip.as_str()), ("1", "192.168.1.84")),
        Estado::Fallido | Estado::Revertido => {
            assert_eq!((upnp.as_str(), ip.as_str()), ("0", "192.168.1.68"))
        }
        e => panic!("estado {e:?}: {:?}", c.error),
    }
    esperar_libre(&b).await;
}

/// Interrumpido antes de escribir el paso 2: al retomar se revierte el 1.
#[tokio::test]
async fn reanuda_grupo_sin_terminar_revierte_lo_escrito() {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let a = gestor(&p, tiempos()).await;
    let prep = a
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.84")]))
        .await
        .unwrap();
    // El router se cuelga tras el paso 1: su verificación no llega a leer.
    p.sim.control.hang_after_write_n(1);
    let a2 = a.clone();
    let token = prep.token.clone();
    let tarea = tokio::spawn(async move { a2.confirmar_grupo(&token, &[], "admin").await });
    let start = Instant::now();
    while p.sim.control.writes().is_empty() {
        assert!(start.elapsed() < Duration::from_secs(5), "sin paso 1");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    a.detener();
    tarea.abort();
    let id = a.historial(1).await.remove(0).id;
    let antes = a.cambio(id).await.unwrap();
    assert_eq!(antes.pasos[1].estado, EstadoPaso::Pendiente);
    p.sim.control.set_unresponsive(false);

    let b = ChangeManager::new(p.state.clone(), tiempos());
    b.reanudar_pendientes().await;
    let c = esperar_final(&b, id).await;
    assert_eq!(c.estado, Estado::Fallido, "{:?}", c.error);
    assert_eq!(upnp_actual(&p).await, "0");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    assert_eq!(escritas(&p), [UPNP, UPNP]);
    assert_eq!(c.pasos[0].estado, EstadoPaso::Revertido);
    esperar_libre(&b).await;
}

#[tokio::test]
async fn pasos_sin_cambio_y_desmarcados_se_omiten() {
    let (p, m) = preparado().await;
    // Ya está así.
    let r = m
        .preparar_grupo(grupo(vec![upnp("0"), lease0("192.168.1.68")]))
        .await;
    assert!(matches!(r, Err(CambioError::SinCambios)), "{r:?}");

    let prep = m
        .preparar_grupo(grupo(vec![upnp("0"), lease0("192.168.1.84"), ssid("OTRA")]))
        .await
        .unwrap();
    assert_eq!(
        prep.pasos.iter().map(|p| p.sin_cambios).collect::<Vec<_>>(),
        [true, false, false]
    );
    assert!(prep.pasos[0].diff.campos.is_empty());
    // Sin el paso de riesgo medio (desmarcado) el riesgo no cambia aquí: es
    // el de lo preparado.
    let id = m.confirmar_grupo(&prep.token, &[2], "admin").await.unwrap();
    let c = esperar_estado(&m, id, Estado::Aplicado).await;
    assert_eq!(escritas(&p), [LEASES]);
    assert_eq!(ssid_actual(&p).await, "RED-1");
    assert_eq!(
        estados_pasos(&c),
        [
            EstadoPaso::Omitido,
            EstadoPaso::Aplicado,
            EstadoPaso::Omitido
        ]
    );
    esperar_libre(&m).await;

    // Todo desmarcado: nada que hacer, nada guardado.
    let prep = m
        .preparar_grupo(grupo(vec![upnp("1"), ssid("OTRA")]))
        .await
        .unwrap();
    let r = m.confirmar_grupo(&prep.token, &[0, 1], "admin").await;
    assert!(matches!(r, Err(CambioError::SinCambios)), "{r:?}");
    assert_eq!(m.historial(10).await.len(), 1);
    assert_eq!(escritas(&p).len(), 1);
}

/// Un grupo no admite acciones: no se podrían revertir si falla otro paso.
#[tokio::test]
async fn acciones_solo_en_grupos_de_un_paso() {
    let (_p, m) = preparado().await;
    let r = m
        .preparar_grupo(grupo(vec![
            upnp("1"),
            paso(req("cbReboot", None, &[]), "ajuste-reiniciar"),
        ]))
        .await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    // Y un grupo con una acción necesita la palabra (va por `confirmar`).
    let prep = m
        .preparar_grupo(grupo(vec![paso(
            req("cbReboot", None, &[]),
            "ajuste-reiniciar",
        )]))
        .await
        .unwrap();
    let r = m.confirmar_grupo(&prep.token, &[], "admin").await;
    assert!(matches!(r, Err(CambioError::PalabraIncorrecta)), "{r:?}");
}

/// Escritura sin respuesta tras la que el router no muestra ni lo de antes
/// ni lo nuevo: no se da por aplicada; se revierte.
#[tokio::test]
async fn sin_respuesta_con_otros_valores_se_revierte() {
    let (p, m) = preparado().await;
    let prep = m.preparar(lease0("192.168.1.84").req).await.unwrap();
    p.sim.control.no_answer_next_write(true);
    let m2 = m.clone();
    let token = prep.token.clone();
    let tarea = tokio::spawn(async move { m2.confirmar(&token, None, "admin").await });
    let start = Instant::now();
    while p.sim.control.writes().is_empty() {
        assert!(start.elapsed() < Duration::from_secs(5), "sin escritura");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    // Mientras el panel espera, otro cambia la reserva a un tercer valor.
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, common::USER, common::PASS);
    cfg.min_gap = Duration::from_millis(1);
    let otro = hgu::Client::new(cfg);
    otro.write(
        "dhcp.asp",
        LEASES,
        &[
            (POOL, "0"),
            ("rtm_dhcpv4_svr_pool_static_id", "0"),
            ("ipv4_addr", "192.168.1.77"),
        ],
    )
    .await
    .unwrap();
    let r = tarea.await.unwrap();
    assert!(
        matches!(&r, Err(CambioError::Router(e)) if e.contains("otros valores")),
        "{r:?}"
    );
    let c = m.historial(1).await.remove(0);
    assert_eq!(c.estado, Estado::Fallido, "{:?}", c.error);
    assert!(c.error.unwrap().contains("revertido"));
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.68");
    assert_eq!(m.activo(), None);
}

/// Tras una escritura sin respuesta, si la relectura falla, el sondeo deja
/// de estar en pausa: `/salud` refleja el router (aquí, caído) en vez de
/// quedarse congelado hasta el fin de la relectura.
#[tokio::test]
async fn pausa_se_suelta_en_relectura() {
    let p = panel_rapido(Duration::from_millis(300)).await;
    let sondeo = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(50),
            ..PollerConfig::default()
        },
    );
    let start = Instant::now();
    while !sondeo.healthy() {
        assert!(start.elapsed() < Duration::from_secs(5), "sondeo sin datos");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    // La relectura se reintentaría durante un minuto.
    let lento = Tiempos {
        bajo: Duration::from_secs(60),
        intervalo: Duration::from_millis(50),
        ..tiempos()
    };
    let m = gestor(&p, lento).await;
    let prep = m.preparar(lease0("192.168.1.84").req).await.unwrap();
    p.sim.control.no_answer_next_write(true);
    p.sim.control.hang_after_next_write();
    let m2 = m.clone();
    let token = prep.token.clone();
    tokio::spawn(async move { m2.confirmar(&token, None, "admin").await });
    let start = Instant::now();
    while p.sim.control.writes().is_empty() {
        assert!(start.elapsed() < Duration::from_secs(5), "sin escritura");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    // Escritura (300 ms) + primera relectura (300 ms) + dos ciclos del
    // sondeo (cada uno agota su espera de 300 ms).
    let limite = Duration::from_millis(2500);
    let start = Instant::now();
    loop {
        let body = common::body_text(get(&p.app, "/salud").await).await;
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        if v["router"] == "error" {
            break;
        }
        assert!(
            start.elapsed() < limite,
            "/salud sigue congelado en {:?}",
            v["router"]
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    p.sim.control.set_unresponsive(false);
    m.detener();
}

/// Las filas de antes de los grupos pasan a ser grupos de un paso.
#[tokio::test]
async fn migracion_pasa_cambios_antiguos_a_un_paso() {
    use sqlx::{
        Row,
        sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    };
    let dir = tempfile::tempdir().unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(dir.path().join("p.db"))
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let todas = sqlx::migrate!("./migrations");
    let mut previas = sqlx::migrate!("./migrations");
    previas.migrations = std::borrow::Cow::Owned(
        todas
            .migrations
            .iter()
            .filter(|m| m.version < 10)
            .cloned()
            .collect(),
    );
    previas.run(&pool).await.unwrap();
    for (id, estado) in [(1, "aplicado"), (2, "vigilando")] {
        sqlx::query(
            "INSERT INTO changes (id, ts, updated, funcion, instancia, riesgo, estado, diff, \
             anteriores, anteriores_cifrados, operacion, solicitud, nuevos, ids_antes) \
             VALUES (?, 1, 1, 'cbDhcpV4SvrPoolSetStatic', '0', 'bajo', ?, \
             '[{\"nombre\":\"ipv4_addr\",\"antes\":\"a\",\"despues\":\"b\",\"secreto\":false}]', \
             '{\"ipv4_addr\":\"a\"}', X'0102', 'editar', '{\"x\":\"1\"}', \
             '{\"ipv4_addr\":\"b\"}', '[\"3\"]')",
        )
        .bind(id)
        .bind(estado)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE changes SET deshecho_por = 2 WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    todas.run(&pool).await.unwrap();
    let deshecho: Option<i64> = sqlx::query_scalar("SELECT deshecho_por FROM changes WHERE id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(deshecho, Some(2), "se conserva el enlace de deshacer");
    let filas = sqlx::query("SELECT * FROM cambio_pasos ORDER BY cambio_id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(filas.len(), 2);
    let f = &filas[0];
    assert_eq!(f.get::<i64, _>("orden"), 0);
    assert_eq!(f.get::<String, _>("funcion"), LEASES);
    assert_eq!(
        f.get::<Option<String>, _>("instancia").as_deref(),
        Some("0")
    );
    assert_eq!(f.get::<String, _>("estado"), "aplicado");
    assert_eq!(
        f.get::<Vec<u8>, _>("valores_anteriores"),
        br#"{"ipv4_addr":"a"}"#
    );
    assert_eq!(f.get::<Vec<u8>, _>("valores_anteriores_cifrados"), [1, 2]);
    assert_eq!(
        f.get::<Vec<u8>, _>("valores_nuevos"),
        br#"{"ipv4_addr":"b"}"#
    );
    assert_eq!(
        f.get::<Option<String>, _>("ids_antes").as_deref(),
        Some(r#"["3"]"#)
    );
    // Un cambio en vigilancia ya escribió y verificó su paso.
    assert_eq!(filas[1].get::<String, _>("estado"), "aplicado");
    // El nuevo estado del grupo es válido.
    sqlx::query("UPDATE changes SET estado = 'revertido_parcial' WHERE id = 2")
        .execute(&pool)
        .await
        .unwrap();
}
