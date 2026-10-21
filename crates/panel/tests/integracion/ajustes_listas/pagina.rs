//! Página de la red local: dispositivos, IP fijas y avisos de cada tarjeta.

use super::*;

/// La página de Red local pinta los dos componentes (sin el hueco de
/// «todavía no está lista»).
#[tokio::test]
async fn pagina_red_local_con_dispositivos_y_reservas() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let resp = crate::comun::get_con_cookie(&p.app, "/ajustes/red_local", &cookie).await;
    assert_eq!(resp.status(), 200);
    let html = crate::comun::body_text(resp).await;
    assert!(html.contains("Fijar esta IP"), "{html}");
    assert!(html.contains("Añadir IP fija"), "{html}");
    assert!(html.contains("192.168.1.62"), "{html}");
    assert!(!html.contains("todavía no está lista"), "{html}");
}

// ------------------------------------------- ronda de corrección 1

/// El `<article>` de la página cuyo título es `titulo` (sin lo que llega
/// después por streaming).
fn tarjeta<'a>(html: &'a str, titulo: &str) -> &'a str {
    html.split("<article")
        .find(|a| a.contains(&format!(">{titulo}</h2>")))
        .unwrap_or_else(|| panic!("sin la tarjeta {titulo}"))
        .split("</article>")
        .next()
        .unwrap()
}

/// El fragmento que el servidor envía aparte (streaming fuera de orden, un
/// `<template>`) con `marca`: lo que pinta una tarjeta tras leer la red.
fn fragmento<'a>(html: &'a str, marca: &str) -> &'a str {
    html.split("<template")
        .map(|t| t.split("</template>").next().unwrap())
        .find(|t| t.contains(marca) && !t.contains("<article"))
        .unwrap_or_else(|| panic!("sin el fragmento con {marca}"))
}

/// Lo que se ve de los dispositivos y de las IP fijas, ya leídos.
fn lista_de_dispositivos(html: &str) -> &str {
    fragmento(html, r#"class="dispositivos""#)
}

fn ip_fijas(html: &str) -> &str {
    fragmento(html, "tabla-apilada")
}

/// El nombre accesible de «Fijar esta IP» empieza por lo que se ve
/// (WCAG 2.5.3): quien lo pide por voz lo encuentra.
#[tokio::test]
async fn fijar_esta_ip_se_nombra_como_se_ve() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let html = pagina(&p, &cookie, "/ajustes/red_local").await;
    assert!(
        html.contains(r#"aria-label="Fijar esta IP: 192.168.1.62 (host-2)""#),
        "{html}"
    );
}

/// Un dispositivo con IP fija dice cuál es, aunque ahora tenga otra.
#[tokio::test]
async fn ya_tiene_ip_fija_dice_la_reservada() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let d = dispositivos(&p, &cookie).await;
    assert_eq!(por_mac(&d, "02:00:00:00:00:01")["ip"], "192.168.1.60");
    assert_eq!(por_mac(&d, "02:00:00:00:00:01")["ip_fija"], "192.168.1.69");
    assert_eq!(por_mac(&d, "02:00:00:00:00:03")["ip_fija"], Value::Null);
    let html = pagina(&p, &cookie, "/ajustes/red_local").await;
    assert!(html.contains("Ya tiene IP fija: 192.168.1.69"), "{html}");
}

/// Sin «Fijar esta IP» donde el servidor la rechazaría siempre (una IP de
/// otra red, la de otra reserva) ni para un dispositivo desconectado (su última
/// IP puede tenerla ya otro): una nota breve en su lugar.
#[tokio::test]
async fn sin_fijar_si_no_se_puede() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("dispositivos-irregulares"))
        .arrancar_con_gestor()
        .await;
    // Otra reserva con la IP de host-2.
    p.state
        .hgu
        .write(
            "dhcp.asp",
            "cbDhcpV4SvrPoolSetStatic",
            &[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "-1"),
                ("mac_addr", "02:00:00:00:00:0b"),
                ("ipv4_addr", "192.168.1.62"),
                ("adm_state", "1"),
            ],
        )
        .await
        .unwrap();
    let html = pagina(&p, &cookie, "/ajustes/red_local").await;
    let dispositivos = lista_de_dispositivos(&html);
    for ip in ["198.51.100.76", "192.168.1.61", "192.168.1.62"] {
        assert!(
            !dispositivos.contains(&format!("Fijar esta IP: {ip} ")),
            "{ip}: {dispositivos}"
        );
    }
    assert!(dispositivos.contains("Fijar esta IP: 192.168.1.64 (host-4)"));
    assert_eq!(dispositivos.matches("Esta IP no se puede fijar").count(), 2);
    assert_eq!(
        dispositivos.matches("Conéctalo para fijar su IP").count(),
        1
    );
}

/// El nombre del dispositivo de una reserva se busca con las MAC normalizadas.
#[tokio::test]
async fn nombre_de_la_reserva_con_mac_normalizada() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("dispositivos-irregulares"))
        .arrancar_con_gestor()
        .await;
    let html = pagina(&p, &cookie, "/ajustes/red_local").await;
    let fijas = ip_fijas(&html);
    let fila = fijas
        .split("<tr")
        .find(|f| f.contains("02:00:00:00:00:04"))
        .unwrap_or_else(|| panic!("{fijas}"));
    assert!(fila.contains("host-3"), "{fila}");
}

/// Una sola región de alerta por tarjeta, no una por dispositivo.
#[tokio::test]
async fn una_alerta_por_tarjeta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let html = pagina(&p, &cookie, "/ajustes/red_local").await;
    let alertas = |t: &str| t.matches(r#"role="alert""#).count();
    let n =
        alertas(tarjeta(&html, "Dispositivos conectados")) + alertas(lista_de_dispositivos(&html));
    assert_eq!(n, 1, "dispositivos");
    let n = alertas(tarjeta(&html, "IP fijas")) + alertas(ip_fijas(&html));
    assert_eq!(n, 1, "IP fijas");
}

fn dispositivo(ip: &str, activo: bool) -> DispositivoVista {
    DispositivoVista {
        nombre: "host-2".into(),
        ip: ip.into(),
        mac: "02:00:00:00:00:03".into(),
        conexion: panel::ajustes_router::Conexion::Desconocida,
        activo,
        reserva: None,
        ip_fija: None,
    }
}

#[test]
fn destino_de_un_dispositivo_o_de_una_ip_fija_existe() {
    let d = [
        dispositivo("192.168.1.62", true),
        dispositivo("192.168.1.61", false),
    ];
    let r = existentes();
    assert!(!destino_inexistente("192.168.1.62", &d, &r), "conectado");
    assert!(!destino_inexistente("192.168.1.61", &d, &r), "desconectado");
    assert!(!destino_inexistente("192.168.1.69", &d, &r), "IP fija");
    assert!(
        !destino_inexistente(" 192.168.1.68 ", &[], &r),
        "con espacios"
    );
    assert!(destino_inexistente("192.168.1.75", &d, &r), "de nadie");
    assert!(destino_inexistente("192.168.1.75", &[], &[]), "sin nada");
}

/// La regla de las fixtures va a 192.168.1.75, que no es de ningún
/// dispositivo ni de ninguna IP fija: la página lo avisa.
#[tokio::test]
async fn aviso_destino_inexistente_en_la_pagina() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let resp = crate::comun::get_con_cookie(&p.app, "/ajustes/puertos", &cookie).await;
    assert_eq!(resp.status(), 200);
    let html = crate::comun::body_text(resp).await;
    assert!(
        html.contains("El dispositivo de destino ya no está en la red"),
        "{html}"
    );
    assert!(!html.contains("todavía no está lista"), "{html}");
    assert!(html.contains("Abrir un puerto"), "{html}");
}

/// Un protocolo que el panel no conoce se ve como en el resto de ajustes
/// («Valor no reconocido (47)»), nunca con su clave de traducción.
#[tokio::test]
async fn protocolo_desconocido_en_la_pagina() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("puertos-protocolo-desconocido"))
        .arrancar_con_gestor()
        .await;
    let resp = crate::comun::get_con_cookie(&p.app, "/ajustes/puertos", &cookie).await;
    let html = crate::comun::body_text(resp).await;
    assert!(html.contains("Valor no reconocido (47)"), "{html}");
    assert!(!html.contains("ajuste-puertos-reglas-protocolo-"), "{html}");
}
