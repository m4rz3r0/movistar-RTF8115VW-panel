//! Páginas de Red local y Puertos: sus listas (dispositivos, IP fijas,
//! reglas) y el detalle de cada fila.

use super::*;

/// Ids de las filas de la lista cuyo título es `titulo` (`id="fila-…"`).
fn filas(html: &str, titulo: &str) -> Vec<String> {
    let desde = html
        .find(&format!(">{titulo}</h2>"))
        .unwrap_or_else(|| panic!("sin la lista {titulo}"));
    let lista = &html[desde..];
    let lista = lista.split("</section>").next().unwrap();
    lista
        .split(r#"id="fila-"#)
        .skip(1)
        .map(|f| f.split('"').next().unwrap().to_owned())
        .collect()
}

/// El detalle abierto en la página (lo que hay dentro de su `<section>`).
fn detalle(html: &str) -> &str {
    html.split(r#"class="detalle detalle-lateral""#)
        .nth(1)
        .unwrap_or_else(|| panic!("sin detalle: {html}"))
        .split("</section>")
        .next()
        .unwrap()
}

/// Los detalles de todos los dispositivos de Red local, uno por fila.
async fn detalles_de_dispositivos(p: &Panel, cookie: &str) -> Vec<String> {
    let html = pagina(p, cookie, "/ajustes/red_local").await;
    let mut out = Vec::new();
    for id in filas(&html, "Dispositivos conectados") {
        let html = pagina(p, cookie, &format!("/ajustes/red_local/{id}")).await;
        out.push(detalle(&html).to_owned());
    }
    out
}

/// La página de Red local pinta las dos listas (sin el hueco de «todavía no
/// está lista») y, abierto un dispositivo, «Fijar esta IP».
#[tokio::test]
async fn pagina_red_local_con_dispositivos_y_reservas() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let html = pagina(&p, &cookie, "/ajustes/red_local").await;
    assert!(html.contains("Añadir IP fija"), "{html}");
    assert!(html.contains("192.168.1.62"), "{html}");
    assert!(!html.contains("todavía no está lista"), "{html}");
    assert_eq!(filas(&html, "IP fijas"), ["0", "1"]);
    let html = pagina(&p, &cookie, "/ajustes/red_local/02-00-00-00-00-03").await;
    assert!(detalle(&html).contains("Fijar esta IP"), "{html}");
}

// ------------------------------------------- ronda de corrección 1

/// El nombre accesible de «Fijar esta IP» empieza por lo que se ve
/// (WCAG 2.5.3): quien lo pide por voz lo encuentra.
#[tokio::test]
async fn fijar_esta_ip_se_nombra_como_se_ve() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let html = pagina(&p, &cookie, "/ajustes/red_local/02-00-00-00-00-03").await;
    assert!(
        detalle(&html).contains(r#"aria-label="Fijar esta IP: 192.168.1.62 (host-2)""#),
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
    let html = pagina(&p, &cookie, "/ajustes/red_local/02-00-00-00-00-01").await;
    // Un dato, no una acción: una etiqueta, no un botón desactivado.
    let d = detalle(&html);
    assert!(
        d.contains("<span class=\"chip chip-ya-fija\">Ya tiene IP fija: 192.168.1.69</span>"),
        "{d}"
    );
    assert!(!d.contains("Fijar esta IP"), "{d}");
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
    let detalles = detalles_de_dispositivos(&p, &cookie).await.join("\n");
    for ip in ["198.51.100.76", "192.168.1.61", "192.168.1.62"] {
        assert!(
            !detalles.contains(&format!("Fijar esta IP: {ip} ")),
            "{ip}: {detalles}"
        );
    }
    assert!(detalles.contains("Fijar esta IP: 192.168.1.64 (host-4)"));
    assert_eq!(detalles.matches("Esta IP no se puede fijar").count(), 2);
    assert_eq!(detalles.matches("Conéctalo para fijar su IP").count(), 1);
}

/// El nombre del dispositivo de una reserva se busca con las MAC normalizadas.
#[tokio::test]
async fn nombre_de_la_reserva_con_mac_normalizada() {
    let (p, cookie) = PanelPrueba::nuevo()
        .capa(&variante("dispositivos-irregulares"))
        .arrancar_con_gestor()
        .await;
    let html = pagina(&p, &cookie, "/ajustes/red_local").await;
    let fijas = html.split(">IP fijas</h2>").nth(1).unwrap();
    let fila = fijas
        .split("<tr")
        .find(|f| f.contains("02:00:00:00:00:04"))
        .unwrap_or_else(|| panic!("{fijas}"));
    assert!(fila.contains("host-3"), "{fila}");
}

/// Una sola región de alerta en el detalle de un dispositivo, no una por
/// dato ni por acción.
#[tokio::test]
async fn una_alerta_por_detalle() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for d in detalles_de_dispositivos(&p, &cookie).await {
        assert_eq!(d.matches(r#"role="alert""#).count(), 1, "{d}");
    }
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
    let html = pagina(&p, &cookie, "/ajustes/puertos").await;
    assert!(!html.contains("todavía no está lista"), "{html}");
    assert!(html.contains("Abrir un puerto"), "{html}");
    let ids = filas(&html, "Puertos abiertos");
    assert_eq!(ids.len(), 1, "{html}");
    let fila = html
        .split(&format!(r#"id="fila-{}""#, ids[0]))
        .nth(1)
        .unwrap()
        .split("</tr>")
        .next()
        .unwrap();
    assert!(fila.contains("Sin dispositivo"), "{fila}");
    // Su detalle dice qué hacer.
    let html = pagina(&p, &cookie, &format!("/ajustes/puertos/{}", ids[0])).await;
    assert!(
        detalle(&html).contains("El router no conoce ningún dispositivo"),
        "{html}"
    );
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
