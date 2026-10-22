//! Lecturas para mostrar (`leer_tema`, con la red de casa en Puertos y Red
//! local): caché corta que se invalida al escribir y tanda sin
//! lecturas de otros intercaladas. Lo que prepara un cambio sigue leyendo el
//! router.

use crate::comun::{Panel, PanelPrueba, Registro, esperar_final, esperar_libre, leer, ok, sesion};
use panel::poller::{Poller, PollerConfig};
use serde_json::{Value, json};
use std::time::Duration;

use crate::comun::USER;
use panel::{
    changes::{ChangeManager, ChangeRequest, Estado, Tiempos},
    lecturas::{CADUCIDAD_LECTURAS, CacheLecturas, Clave},
};

/// Lecturas que ha recibido el simulador desde `desde`.
fn lecturas_desde(p: &Panel, desde: usize) -> Vec<(String, Vec<(String, String)>)> {
    p.sim.control.reads().into_iter().skip(desde).collect()
}

/// Comprueba que `tema` sale de la caché: leerlo no llega al simulador.
async fn sin_leer_el_router(p: &Panel, cookie: &str, tema: &str) {
    let antes = p.sim.control.reads().len();
    leer(p, cookie, tema).await;
    assert_eq!(
        lecturas_desde(p, antes),
        vec![],
        "{tema}: no salió de la caché"
    );
}

/// Valor legible del campo `campo` del ajuste `ajuste` en un tema leído.
fn valor(tema: &Value, ajuste: &str, campo: &str) -> Value {
    let a = crate::comun::ajustes_de(tema)
        .into_iter()
        .find(|a| a["id"] == ajuste)
        .unwrap_or_else(|| panic!("falta {ajuste}: {tema}"));
    a["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == campo)
        .unwrap_or_else(|| panic!("falta {campo}: {a}"))["valor"]
        .clone()
}

#[tokio::test]
async fn volver_a_un_tema_no_lee_el_router() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    for tema in ["wifi", "red_local", "seguridad"] {
        let antes = p.sim.control.reads().len();
        let primera = leer(&p, &cookie, tema).await;
        assert!(!lecturas_desde(&p, antes).is_empty(), "{tema}");
        let antes = p.sim.control.reads().len();
        let segunda = leer(&p, &cookie, tema).await;
        assert_eq!(lecturas_desde(&p, antes), vec![], "{tema}");
        assert_eq!(primera, segunda, "{tema}");
    }
}

/// La página de Red local lee en su misma tanda los dispositivos (con las
/// estaciones de cada radio) y la red de casa: una sola server fn por
/// página. La segunda vez, todo de la caché.
#[tokio::test]
async fn dispositivos_y_red_de_casa_tambien_de_la_cache() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let antes = p.sim.control.reads().len();
    let pagina = leer(&p, &cookie, "red_local").await;
    let del_tema: Vec<String> = lecturas_desde(&p, antes)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    for n in [
        "cbListWifiAccPtAssociation",
        "cbGetLanIPv4Addr",
        "cbGetLanHostList",
    ] {
        assert!(
            del_tema.iter().any(|x| x == n),
            "la tanda de Red local no lee {n}: {del_tema:?}"
        );
    }
    assert!(pagina["red"]["dispositivos"].is_array(), "{pagina}");
    let antes = p.sim.control.reads().len();
    assert_eq!(leer(&p, &cookie, "red_local").await["red"], pagina["red"]);
    assert_eq!(lecturas_desde(&p, antes), vec![]);
}

/// Tras un cambio por el gestor, la siguiente lectura va al router y trae
/// el valor nuevo.
#[tokio::test]
async fn tras_escribir_vuelve_a_leer_el_valor_nuevo() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = leer(&p, &cookie, "puertos").await;
    sin_leer_el_router(&p, &cookie, "puertos").await;
    assert_eq!(
        valor(&antes, "puertos.upnp", "activo"),
        json!({ "Booleano": false })
    );
    let g = ok(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "puertos.upnp", "instancia": null, "borrar": false,
                       "valores": { "activo": "true" } } }),
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
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    esperar_libre(&p).await;
    let desde = p.sim.control.reads().len();
    let despues = leer(&p, &cookie, "puertos").await;
    assert!(
        lecturas_desde(&p, desde)
            .iter()
            .any(|(n, _)| n == "cbGetUpnpCfg"),
        "no volvió a leer"
    );
    assert_eq!(
        valor(&despues, "puertos.upnp", "activo"),
        json!({ "Booleano": true })
    );
}

/// Una escritura por el cliente del panel (aunque no pase por el gestor)
/// invalida lo guardado.
#[tokio::test]
async fn cualquier_escritura_del_cliente_invalida() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    leer(&p, &cookie, "puertos").await;
    sin_leer_el_router(&p, &cookie, "puertos").await;
    p.state
        .hgu
        .write("upnp.asp", "cbSetUpnpCfg", &[("adm_state", "1")])
        .await
        .unwrap();
    let despues = leer(&p, &cookie, "puertos").await;
    assert_eq!(
        valor(&despues, "puertos.upnp", "activo"),
        json!({ "Booleano": true })
    );
}

#[tokio::test]
async fn tras_caducar_vuelve_a_leer() {
    let mut p = PanelPrueba::nuevo().arrancar().await;
    p.state.lecturas = std::sync::Arc::new(panel::lecturas::CacheLecturas::new(
        Duration::from_millis(300),
    ));
    p.app = panel::build_router(p.state.clone());
    let cookie = sesion(&p).await;
    leer(&p, &cookie, "sistema").await;
    let antes = p.sim.control.reads().len();
    leer(&p, &cookie, "sistema").await;
    assert_eq!(lecturas_desde(&p, antes), vec![]);
    tokio::time::sleep(Duration::from_millis(400)).await;
    leer(&p, &cookie, "sistema").await;
    assert_eq!(lecturas_desde(&p, antes).len(), 2);
}

/// `preparar_ajuste` lee el router aunque la página se pintase de la caché:
/// el «antes» es lo que tiene el router ahora, cambiado fuera del panel.
#[tokio::test]
async fn preparar_ajuste_no_usa_la_cache() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let pagina = leer(&p, &cookie, "wifi").await;
    assert_ne!(
        valor(&pagina, "wifi.red_24", "nombre"),
        json!({ "Texto": "cambiado-fuera" })
    );
    // Otro cliente (como la app del operador): el panel no se entera.
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, crate::comun::USER, crate::comun::PASS);
    cfg.min_gap = Duration::from_millis(1);
    hgu::Client::new(cfg)
        .write(
            "wifi.asp",
            "cbSetWifiAccPt",
            &[("dpm_cfg_wifi_acc_pt_id", "0"), ("ssid", "cambiado-fuera")],
        )
        .await
        .unwrap();
    // Para mostrar, aún lo guardado…
    let pagina = leer(&p, &cookie, "wifi").await;
    assert_ne!(
        valor(&pagina, "wifi.red_24", "nombre"),
        json!({ "Texto": "cambiado-fuera" })
    );
    // …pero el resumen del cambio parte de lo que hay en el router.
    let g = ok(
        &p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": "wifi.red_24", "instancia": null, "borrar": false,
                       "valores": { "nombre": "casa-nueva" } } }),
        &cookie,
    )
    .await;
    let antes = &g["pasos"][0]["campos"][0]["antes"];
    assert_eq!(antes, "cambiado-fuera", "{g}");
}

/// Con el sondeo en marcha y un router lento, las lecturas de un tema llegan
/// seguidas: el sondeo no se cuela entre ellas.
#[tokio::test]
async fn las_lecturas_de_un_tema_van_seguidas() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    p.sim.control.set_retardo(Duration::from_millis(20));
    // Un sondeo que siempre quiere el turno.
    let sondeo = Poller::spawn_with(
        p.state.clone(),
        PollerConfig {
            fast: Duration::from_millis(10),
            slow: Duration::from_millis(10),
            ..PollerConfig::default()
        },
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    // Puertos: tres lecturas que el sondeo no hace.
    let del_tema: std::collections::BTreeSet<String> = p
        .state
        .ajustes
        .por_tema(ajustes::Tema::from_slug("puertos").unwrap())
        .map(|a| a.leer.funcion.clone())
        .collect();
    assert_eq!(del_tema.len(), 3);
    let antes = p.sim.control.reads().len();
    leer(&p, &cookie, "puertos").await;
    let leidas: Vec<String> = lecturas_desde(&p, antes)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    drop(sondeo);
    assert!(
        leidas.iter().any(|n| !del_tema.contains(n)),
        "el sondeo no leyó nada mientras: la prueba no prueba nada ({leidas:?})"
    );
    let primera = leidas
        .iter()
        .position(|n| del_tema.contains(n))
        .expect("leyó el tema");
    let seguidas = &leidas[primera..primera + del_tema.len()];
    assert!(
        seguidas.iter().all(|n| del_tema.contains(n)),
        "lecturas del sondeo entre las del tema: {leidas:?}"
    );
}

/// Cliente del router como el del panel, con un plazo por petición `plazo`
/// y sin pausa entre peticiones.
fn cliente(p: &Panel, plazo: Duration) -> hgu::Client {
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, USER, crate::comun::PASS);
    cfg.timeout = plazo;
    cfg.min_gap = Duration::from_millis(1);
    hgu::Client::new(cfg)
}

fn claves(nombres: &[&str]) -> Vec<Clave> {
    nombres
        .iter()
        .map(|n| Clave::new::<&str, &str>(n, &[]))
        .collect()
}

/// Si el router deja de responder, la tanda no espera el plazo de cada
/// lectura: tras la primera que no responde, las demás fallan al momento y
/// el turno queda libre para el gestor y el sondeo.
#[tokio::test]
async fn sin_respuesta_del_router_la_tanda_se_corta() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let plazo = Duration::from_millis(300);
    let c = cliente(&p, plazo);
    c.call("cbGetSummary", &[]).await.unwrap();
    let cache = CacheLecturas::new(CADUCIDAD_LECTURAS);
    let k = claves(&["cbGetUpnpCfg", "cbGetNatConfig", "cbGetNatPortMapping"]);
    p.sim.control.set_unresponsive(true);
    let inicio = std::time::Instant::now();
    let r = cache.leer(&c, &k).await;
    let tardo = inicio.elapsed();
    p.sim.control.set_unresponsive(false);
    assert!(
        matches!(&r[&k[0]], Err(hgu::Error::Network(_))),
        "{:?}",
        r[&k[0]]
    );
    for clave in &k[1..] {
        assert!(
            matches!(&r[clave], Err(hgu::Error::Network(e)) if e.contains("no se intentó")),
            "{}: {:?}",
            clave.funcion,
            r[clave]
        );
    }
    // Una sola espera del plazo (sin cortar serían tres).
    assert!(tardo < plazo * 2, "{tardo:?}");
    // El router vuelve: la siguiente lectura entra sin esperar a nadie.
    assert!(c.call("cbGetSummary", &[]).await.is_ok());
}

/// Un error de la propia función (el router responde con `Result` distinto
/// de 0) no corta la tanda.
#[tokio::test]
async fn un_error_de_la_funcion_no_corta_la_tanda() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let c = cliente(&p, Duration::from_secs(5));
    let cache = CacheLecturas::new(CADUCIDAD_LECTURAS);
    let k = claves(&["cbGetNoExiste", "cbGetUpnpCfg"]);
    let r = cache.leer(&c, &k).await;
    assert!(
        matches!(&r[&k[0]], Err(hgu::Error::CallFailed { .. })),
        "{:?}",
        r[&k[0]]
    );
    assert!(r[&k[1]].is_ok(), "{:?}", r[&k[1]]);
}

/// Cada lectura que falla deja una sola línea en el registro (la de la caché
/// de lecturas), también las de la red de casa de una página, que responde
/// igual (sin ellas).
#[tokio::test]
async fn una_linea_por_lectura_fallida() {
    let mut p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    p.state.hgu = cliente(&p, Duration::from_millis(200));
    p.state.hgu.call("cbGetSummary", &[]).await.unwrap();
    p.app = panel::build_router(p.state.clone());
    let registro = Registro::default();
    let _registro = tracing::subscriber::set_default(
        tracing_subscriber::fmt()
            .with_writer(registro.clone())
            .with_ansi(false)
            .finish(),
    );
    p.sim.control.set_unresponsive(true);
    let (estado, _) = crate::comun::llamar(
        &p,
        "leer_tema",
        json!({ "tema": "red_local", "fresca": true }),
        Some(&cookie),
    )
    .await;
    p.sim.control.set_unresponsive(false);
    assert_eq!(estado, 200);
    let texto = registro.texto();
    let lineas: Vec<&str> = texto.lines().filter(|l| l.contains("WARN")).collect();
    assert!(
        lineas.iter().any(|l| l.contains("cbGetLanIPv4Addr")),
        "{texto}"
    );
    // Una por lectura, la de la caché de lecturas: la página no añade otra.
    assert!(
        lineas.iter().all(|l| l.contains("panel::lecturas:")),
        "{texto}"
    );
}

/// Una escritura que llega mientras la tanda espera su turno: lo que lee la
/// tanda después no se guarda (empezó antes de la escritura) y la siguiente
/// lectura vuelve al router.
#[tokio::test]
async fn escritura_mientras_la_tanda_espera_el_turno() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let c = cliente(&p, Duration::from_secs(5));
    let cache = std::sync::Arc::new(CacheLecturas::new(CADUCIDAD_LECTURAS));
    let k = claves(&["cbGetUpnpCfg"]);
    // Alguien tiene el turno; detrás esperan una escritura y luego la tanda.
    let turno = c.turno().await;
    let escritura = {
        let c = c.clone();
        tokio::spawn(async move {
            c.write("upnp.asp", "cbSetUpnpCfg", &[("adm_state", "1")])
                .await
                .unwrap();
        })
    };
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    let tanda = {
        let (c, cache, k) = (c.clone(), cache.clone(), k.clone());
        tokio::spawn(async move { cache.leer(&c, &k).await })
    };
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    drop(turno);
    escritura.await.unwrap();
    let r = tanda.await.unwrap();
    assert!(r[&k[0]].is_ok());
    let antes = p.sim.control.reads().len();
    cache.leer(&c, &k).await;
    assert_eq!(
        lecturas_desde(&p, antes).len(),
        1,
        "guardó una lectura que empezó antes de la escritura"
    );
}

/// Ajuste de la reserva `id` (pool 0): su IP.
fn reserva(id: &str, ip: &str) -> ChangeRequest {
    ChangeRequest {
        funcion: "cbDhcpV4SvrPoolSetStatic".into(),
        instancia: Some(id.to_owned()),
        valores: [("rtm_cfg_dhcpv4_svr_pool_id", "0"), ("ipv4_addr", ip)]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
        borrar: false,
        identidad: Default::default(),
    }
}

/// IP de la reserva `id` en lo que la caché da para mostrar.
async fn ip_mostrada(p: &Panel, id: &str) -> String {
    let clave = Clave::new("cbDhcpV4SvrPoolGetStatic", &hgu::read::POOL);
    let r = p
        .state
        .lecturas
        .leer(&p.state.hgu, std::slice::from_ref(&clave))
        .await;
    let n = r[&clave].as_ref().unwrap();
    hgu::models::StaticLease::parse_all(n)
        .into_iter()
        .find(|l| l.id == id)
        .unwrap()
        .ip
}

/// Cambia la reserva 1 con otro cliente (como la app del operador, o el
/// router aplicando algo tarde): el contador de escrituras del panel no se
/// entera.
async fn cambiar_fuera(p: &Panel, ip: &str) {
    cliente(p, Duration::from_secs(5))
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "1"),
                ("ipv4_addr", ip),
            ],
        )
        .await
        .unwrap();
}

fn tiempos(vigilancia: Duration) -> Tiempos {
    Tiempos {
        bajo: vigilancia,
        medio: vigilancia,
        alto: vigilancia,
        intervalo: Duration::from_millis(30),
        fallos_para_revertir: 3,
        pausa_max: Duration::from_secs(2),
    }
}

/// Al comprobar en el router un cambio (aquí, al retomarlo tras un reinicio
/// del panel, sin escribir nada) la caché se invalida: mientras sigue la
/// vigilancia, la página ya ve lo que el router tiene.
#[tokio::test]
async fn comprobar_un_cambio_invalida_lo_guardado() {
    let mut p = PanelPrueba::nuevo().arrancar().await;
    p.state.hgu = cliente(&p, Duration::from_millis(500));
    let lento = tiempos(Duration::from_secs(60));
    let a = ChangeManager::new(p.state.clone(), lento.clone());
    a.reanudar_pendientes().await;
    let prep = a.preparar(reserva("0", "192.168.1.80")).await.unwrap();
    let id = a.confirmar(&prep.token, None, "admin").await.unwrap();
    assert_eq!(a.cambio(id).await.unwrap().estado, Estado::Vigilando);
    // El proceso muere a mitad de la vigilancia.
    a.detener();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let antes = ip_mostrada(&p, "1").await;
    cambiar_fuera(&p, "192.168.1.99").await;
    assert_eq!(ip_mostrada(&p, "1").await, antes, "aún de la caché");
    let b = ChangeManager::new(p.state.clone(), lento);
    b.reanudar_pendientes().await;
    // Retomado y vigilando (el gestor sigue ocupado): ya comprobó.
    assert_eq!(b.activo(), Some(id));
    assert_eq!(ip_mostrada(&p, "1").await, "192.168.1.99");
    b.detener();
}

/// Al soltar el gestor (fin de un cambio, de un grupo o de lo que lo
/// apartó), la caché se invalida.
#[tokio::test]
async fn soltar_el_gestor_invalida_lo_guardado() {
    let mut p = PanelPrueba::nuevo().arrancar().await;
    p.state.hgu = cliente(&p, Duration::from_secs(5));
    let m = ChangeManager::new(p.state.clone(), tiempos(Duration::from_millis(300)));
    m.reanudar_pendientes().await;
    let antes = ip_mostrada(&p, "1").await;
    let apartado = m.excluir_cambios().unwrap();
    cambiar_fuera(&p, "192.168.1.99").await;
    assert_eq!(ip_mostrada(&p, "1").await, antes, "aún de la caché");
    drop(apartado);
    assert_eq!(ip_mostrada(&p, "1").await, "192.168.1.99");
}
