//! Mejoras contra `router-sim`: el estado de cada lote y de sus pasos, el
//! contador, la preparación de un lote como un solo cambio con su deshacer,
//! y los casos raros (todo desmarcado). Las lecturas que fallan, en
//! [`lecturas`]; el proveedor de DNS, en [`proveedor`].

use crate::comun::{
    Panel, PanelPrueba, aplicar_ajuste, body_text, elemento, get_con_cookie, llamar, ok,
};
use panel::instancia::{self, claves};
use serde_json::{Value, json};

mod dns_propios;
mod lecturas;
mod pagina;
mod proveedor;

/// `leer_mejoras`, sin caché.
async fn mejoras(p: &Panel, cookie: &str) -> Value {
    ok(p, "leer_mejoras", json!({ "fresca": true }), cookie).await
}

/// El lote `id` de la página.
fn lote<'a>(m: &'a Value, id: &str) -> &'a Value {
    m["lotes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"] == id)
        .unwrap_or_else(|| panic!("falta el lote {id}: {m}"))
}

/// El paso `clave` del lote `id`.
fn paso<'a>(m: &'a Value, id: &str, clave: &str) -> &'a Value {
    lote(m, id)["pasos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["clave"] == clave)
        .unwrap_or_else(|| panic!("falta el paso {clave} en {id}: {m}"))
}

fn pendientes(pendientes: u32, total: u32) -> Value {
    json!({ "Pendientes": { "pendientes": pendientes, "total": total } })
}

/// `preparar_lote`: estado y cuerpo.
async fn preparar(p: &Panel, cookie: &str, id: &str, opciones: Value) -> (u16, String) {
    llamar(
        p,
        "preparar_lote",
        json!({ "id": id, "opciones": opciones }),
        Some(cookie),
    )
    .await
}

/// Como [`preparar`], exigiendo 200; devuelve el resumen.
async fn grupo(p: &Panel, cookie: &str, id: &str, opciones: Value) -> Value {
    let (status, body) = preparar(p, cookie, id, opciones).await;
    assert_eq!(status, 200, "preparar_lote {id}: {body}");
    serde_json::from_str(&body).unwrap()
}

/// El paso del grupo con la clave `clave`.
fn del_grupo<'a>(g: &'a Value, clave: &str) -> &'a Value {
    g["pasos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["clave"] == clave)
        .unwrap_or_else(|| panic!("falta {clave} en el grupo: {g}"))
}

/// Confirma el grupo con los pasos `claves` desmarcados y espera a que se
/// aplique.
async fn confirmar_sin(p: &Panel, cookie: &str, g: &Value, claves: &[&str]) -> i64 {
    let desmarcados: Vec<&Value> = claves.iter().map(|c| &del_grupo(g, c)["indice"]).collect();
    let id = ok(
        p,
        "confirmar_ajuste",
        json!({ "token": g["token"], "desmarcados": desmarcados }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap();
    let c = crate::comun::esperar_final(p, cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
    crate::comun::esperar_libre(p).await;
    id
}

/// Elemento `nombre` de la lectura `funcion` del simulador.
async fn del_sim(p: &Panel, funcion: &str, params: &[(&str, &str)], nombre: &str) -> String {
    let n = p.state.hgu.call(funcion, params).await.unwrap();
    elemento(&n, nombre).unwrap_or_else(|| panic!("falta {nombre}"))
}

async fn del_tr69(p: &Panel, nombre: &str) -> String {
    del_sim(p, "cbGetTr69Config", &[], nombre).await
}

async fn del_punto(p: &Panel, id: &str) -> String {
    del_sim(
        p,
        "cbGetWifiAccPt",
        &[("dpm_cfg_wifi_acc_pt_id", id)],
        "wps_adm_state",
    )
    .await
}

/// Elemento `nombre` del pool DHCP de casa (id 0, en la posición 1 de las
/// fixtures).
async fn del_pool(p: &Panel, nombre: &str) -> String {
    del_sim(
        p,
        "cbGetDhcpv4ServerPool",
        &[("rtm_cfg_dhcpv4_svr_pool_0i", "1")],
        nombre,
    )
    .await
}

/// Escribe en el simulador como lo haría el router (otro que no es el
/// panel) y olvida la caché de lecturas.
async fn escribir(p: &Panel, pagina: &str, funcion: &str, pares: &[(&str, &str)]) {
    p.state.hgu.write(pagina, funcion, pares).await.unwrap();
    p.state.lecturas.invalidar();
}

async fn dns_del_pool(p: &Panel, principal: &str, secundario: &str) {
    escribir(
        p,
        "dhcp.asp",
        "cbSetDhcpv4ServerPool",
        &[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("ipv4_dns_addr_0", principal),
            ("ipv4_dns_addr_1", secundario),
        ],
    )
    .await;
}

#[tokio::test]
async fn mejoras_con_las_fixtures() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let m = mejoras(&p, &cookie).await;
    assert_eq!(lote(&m, "seguridad")["estado"], pendientes(5, 11), "{m}");
    assert_eq!(lote(&m, "privacidad")["estado"], pendientes(3, 5), "{m}");
    assert_eq!(m["pendientes"], 6, "{m}");
    assert_eq!(m["solo_lectura"], false);
    assert_eq!(
        paso(&m, "seguridad", "aislar_invitados")["estado"],
        "NoAplica"
    );
    let hora = paso(&m, "privacidad", "hora_neutra");
    assert_eq!(
        (&hora["estado"], &hora["opcional"]),
        (&json!("Pendiente"), &json!(true))
    );
    let wps = paso(&m, "seguridad", "wps@red-24");
    assert_eq!(wps["texto"], "mejoras-paso-wps");
    assert_eq!(wps["detalle"], "ajuste-wifi-red_24-titulo");
    assert_eq!(paso(&m, "seguridad", "upnp")["detalle"], Value::Null);
    assert_eq!(lote(&m, "seguridad")["icono"], "escudo");
    assert_eq!(
        lote(&m, "seguridad")["avisos"],
        json!(["mejoras-aviso-tr069"])
    );
    assert_eq!(
        lote(&m, "seguridad")["consejos"].as_array().unwrap().len(),
        5
    );
    let proveedores = m["proveedores"].as_array().unwrap();
    assert_eq!(proveedores.len(), 7, "«Mantener» y los seis: {m}");
    assert_eq!(
        proveedores[6],
        json!({ "id": "cloudflare_familiar", "servidores": ["1.1.1.3", "1.0.0.3"], "defecto": false }),
        "{m}"
    );
    assert_eq!(proveedores[0]["id"], "mantener", "{m}");
    assert_eq!(
        proveedores[1],
        json!({ "id": "quad9", "servidores": ["9.9.9.9", "149.112.112.112"], "defecto": true })
    );
    assert_eq!(ok(&p, "contar_mejoras", json!({}), &cookie).await, json!(6));
}

#[tokio::test]
async fn aplicar_seguridad_y_deshacer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = grupo(&p, &cookie, "seguridad", json!({})).await;
    assert_eq!(g["titulo"], "mejoras-lote-seguridad-titulo", "{g}");
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(pasos.len(), 11, "{g}");
    let con_cambios = pasos.iter().filter(|p| p["sin_cambios"] == false).count();
    assert_eq!((con_cambios, pasos.len() - con_cambios), (5, 6), "{g}");
    // Agrupados por pestaña, cada uno con su nota y su clave.
    let pestanas: Vec<&str> = pasos
        .iter()
        .map(|p| p["pestana"].as_str().unwrap())
        .collect();
    let mut agrupadas = pestanas.clone();
    agrupadas.dedup();
    let mut sin_repetir = agrupadas.clone();
    sin_repetir.sort();
    sin_repetir.dedup();
    assert_eq!(agrupadas.len(), sin_repetir.len(), "{pestanas:?}");
    let wps = del_grupo(&g, "wps@red-24");
    assert_eq!(wps["nota"], "mejoras-paso-wps");
    assert_eq!(wps["pestana"], "ajustes-tema-wifi");
    assert_eq!(wps["desmarcado"], false);
    assert!(!g.to_string().contains("cbSet"), "{g}");

    let id = confirmar_sin(&p, &cookie, &g, &[]).await;
    assert_eq!(del_tr69(&p, "enableCWMP").await, "0");
    assert_eq!(del_tr69(&p, "periodicInformEnable").await, "0");
    for punto in ["0", "1", "2"] {
        assert_eq!(del_punto(&p, punto).await, "0", "punto {punto}");
    }
    let m = mejoras(&p, &cookie).await;
    assert_eq!(lote(&m, "seguridad")["estado"], "Aplicado", "{m}");
    assert_eq!(lote(&m, "privacidad")["estado"], pendientes(1, 5), "{m}");
    assert_eq!(m["pendientes"], 1);
    let html = body_text(get_con_cookie(&p.app, "/historial", &cookie).await).await;
    assert!(html.contains("Mejoras, seguridad recomendada"), "{html}");

    // Un solo deshacer lo devuelve todo.
    let inverso = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    assert_eq!(
        inverso["titulo"], "mejoras-lote-seguridad-titulo",
        "{inverso}"
    );
    aplicar_ajuste(&p, &cookie, &inverso).await;
    assert_eq!(del_tr69(&p, "enableCWMP").await, "1");
    for punto in ["0", "1", "2"] {
        assert_eq!(del_punto(&p, punto).await, "1", "punto {punto}");
    }
    assert_eq!(mejoras(&p, &cookie).await["pendientes"], 6);
}

#[tokio::test]
async fn privacidad_con_proveedor_y_paso_desmarcado() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = grupo(
        &p,
        &cookie,
        "privacidad",
        json!({ "proveedor_dns": "dns4eu" }),
    )
    .await;
    assert_eq!(del_grupo(&g, "hora_neutra")["desmarcado"], true, "{g}");
    assert_eq!(del_grupo(&g, "dns_casa")["desmarcado"], false);
    assert_eq!(
        del_grupo(&g, "dns_casa")["pestana"],
        "ajustes-tema-red_local"
    );
    confirmar_sin(&p, &cookie, &g, &["aviso_operador", "hora_neutra"]).await;
    assert_eq!(del_pool(&p, "ipv4_dns_addr_0").await, "86.54.11.100");
    assert_eq!(del_pool(&p, "ipv4_dns_addr_1").await, "86.54.11.200");
    assert_eq!(del_tr69(&p, "enableCWMP").await, "0");
    assert_eq!(del_tr69(&p, "periodicInformEnable").await, "1");
    assert_eq!(
        del_sim(&p, "cbGetTimeNtp", &[], "ntp_svr_1").await,
        "hora.operador.example"
    );
}

/// Privacidad máxima entera, con un solo deshacer: el pool 0 vuelve a sus
/// DNS y lo demás, como estaba.
#[tokio::test]
async fn aplicar_privacidad_y_deshacer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = grupo(&p, &cookie, "privacidad", json!({})).await;
    let id = confirmar_sin(&p, &cookie, &g, &[]).await;
    assert_eq!(del_pool(&p, "ipv4_dns_addr_0").await, "9.9.9.9");
    assert_eq!(del_pool(&p, "ipv4_dns_addr_1").await, "149.112.112.112");
    assert_eq!(del_tr69(&p, "enableCWMP").await, "0");
    assert_eq!(del_tr69(&p, "periodicInformEnable").await, "0");
    assert_eq!(
        del_sim(&p, "cbGetTimeNtp", &[], "ntp_svr_1").await,
        "hora.roa.es"
    );
    assert_eq!(
        lote(&mejoras(&p, &cookie).await, "privacidad")["estado"],
        "Aplicado"
    );

    let inverso = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    assert_eq!(
        inverso["titulo"], "mejoras-lote-privacidad-titulo",
        "{inverso}"
    );
    aplicar_ajuste(&p, &cookie, &inverso).await;
    assert_eq!(del_pool(&p, "ipv4_dns_addr_0").await, "192.168.1.1");
    assert_eq!(del_pool(&p, "ipv4_dns_addr_1").await, "192.168.1.1");
    assert_eq!(del_tr69(&p, "enableCWMP").await, "1");
    assert_eq!(del_tr69(&p, "periodicInformEnable").await, "1");
    assert_eq!(
        del_sim(&p, "cbGetTimeNtp", &[], "ntp_svr_1").await,
        "hora.operador.example"
    );
    let m = mejoras(&p, &cookie).await;
    assert_eq!(lote(&m, "privacidad")["estado"], pendientes(3, 5), "{m}");
    assert_eq!(m["pendientes"], 6);
}

#[tokio::test]
async fn lotes_seguidos_comparten_pasos() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = grupo(&p, &cookie, "seguridad", json!({})).await;
    confirmar_sin(&p, &cookie, &g, &[]).await;
    let g = grupo(&p, &cookie, "privacidad", json!({})).await;
    for clave in ["gestion_remota", "aviso_operador", "espejo", "registro"] {
        assert_eq!(del_grupo(&g, clave)["sin_cambios"], true, "{clave}: {g}");
    }
    for clave in ["dns_casa", "hora_neutra"] {
        assert_eq!(del_grupo(&g, clave)["sin_cambios"], false, "{clave}: {g}");
    }
}

/// Vigilancia 3: con todo desmarcado no se escribe nada ni queda nada en
/// el Historial.
#[tokio::test]
async fn lote_todo_desmarcado_no_escribe() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = grupo(&p, &cookie, "privacidad", json!({})).await;
    let con_cambios: Vec<&Value> = g["pasos"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["sin_cambios"] == false)
        .map(|p| &p["indice"])
        .collect();
    let escrituras = p.sim.control.writes().len();
    let (status, body) = llamar(
        &p,
        "confirmar_ajuste",
        json!({ "token": g["token"], "desmarcados": con_cambios }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("Ya está así"), "{body}");
    assert_eq!(p.sim.control.writes().len(), escrituras);
    let h = ok(&p, "historial", json!({ "pagina": 1 }), &cookie).await;
    assert_eq!(h["total"], 0, "{h}");
    // El token sigue valiendo: con algo marcado, se aplica.
    confirmar_sin(&p, &cookie, &g, &["hora_neutra"]).await;
}

/// `syslog_mode = 1` (solo envío remoto) no es «sin envío»: pendiente.
#[tokio::test]
async fn registro_solo_remoto_es_pendiente() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    escribir(
        &p,
        "syslog.asp",
        "cbSetSyslog",
        &[("syslog_mode", "1"), ("remote_ip", "192.0.2.5")],
    )
    .await;
    let m = mejoras(&p, &cookie).await;
    assert_eq!(
        paso(&m, "seguridad", "registro")["estado"],
        "Pendiente",
        "{m}"
    );
    assert_eq!(lote(&m, "seguridad")["estado"], pendientes(6, 11));
}

#[tokio::test]
async fn preparar_lote_sin_permiso_403() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    instancia::escribir(&p.state.db, claves::SOLO_LECTURA, &json!(true))
        .await
        .unwrap();
    let (status, body) = preparar(&p, &cookie, "seguridad", json!({})).await;
    assert_eq!(status, 403, "{body}");
    assert!(p.sim.control.writes().is_empty());
    assert_eq!(mejoras(&p, &cookie).await["solo_lectura"], true);
}

#[tokio::test]
async fn lote_desconocido_404() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar(&p, &cookie, "otro", json!({})).await;
    assert_eq!(status, 404, "{body}");
    assert!(body.contains("no existe"), "{body}");
}

#[tokio::test]
async fn proveedor_desconocido_400() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar(
        &p,
        &cookie,
        "privacidad",
        json!({ "proveedor_dns": "mullvad" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(p.sim.control.writes().is_empty());
}
