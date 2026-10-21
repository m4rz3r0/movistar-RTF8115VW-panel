//! Preferencias del panel: secciones, instancia y zona horaria.

use super::*;

#[tokio::test]
async fn instancia_valida_dns_y_macs() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let mut c = campos();
    c["dns_preferidos"] = json!(["9.9.9.9", " 2620:fe::fe ", "194.242.2.2"]);
    c["dispositivos_confianza"] =
        json!(["AA-BB-CC-00-11-22", "aabb.cc00.1133", "AA:BB:CC:00:11:44"]);
    c["mantenimiento"] = json!("03:00-05:30");
    c["servidor_propio"] = json!("servidor.home.arpa");
    c["idioma"] = json!({ "Fijo": "En" });
    ok(&p, "guardar_instancia", json!({ "campos": c }), &cookie).await;
    let v = ok(&p, "leer_preferencias", json!({}), &cookie).await;
    assert_eq!(
        v["dns_preferidos"],
        json!(["9.9.9.9", "2620:fe::fe", "194.242.2.2"])
    );
    assert_eq!(
        v["dispositivos_confianza"],
        json!([
            "aa:bb:cc:00:11:22",
            "aa:bb:cc:00:11:33",
            "aa:bb:cc:00:11:44"
        ])
    );
    assert_eq!(v["mantenimiento"], "03:00-05:30");
    assert_eq!(v["servidor_propio"], "servidor.home.arpa");
    let panel = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(panel["zona_horaria"], "UTC+01:00");
    assert_eq!(panel["idioma"], json!({ "Fijo": "En" }));

    let malos: Vec<(&str, Value)> = vec![
        ("dns_preferidos", json!(["9.9.9.9x"])),
        ("dns_preferidos", json!(["dns.quad9.net"])),
        (
            "dns_preferidos",
            json!([
                "1.1.1.1",
                "1.0.0.1",
                "9.9.9.9",
                "149.112.112.112",
                "194.242.2.2"
            ]),
        ),
        ("dispositivos_confianza", json!(["aa:bb:cc:00:11"])),
        ("dispositivos_confianza", json!(["aa:bb:cc:00:11:zz"])),
        ("mantenimiento", json!("3-5")),
        ("mantenimiento", json!("25:00-05:00")),
        ("mantenimiento", json!("03:00-03:60")),
        ("zona_horaria", json!("America/New_York")),
        ("zona_horaria", json!("UTC+15:00")),
        ("zona_horaria", json!("UTC+1")),
        ("servidor_propio", json!("http://servidor.home.arpa/")),
    ];
    for (campo, valor) in malos {
        let mut c = campos();
        c[campo] = valor.clone();
        let (status, body) = llamar(
            &p,
            "guardar_instancia",
            json!({ "campos": c }),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, 400, "{campo} = {valor}: {body}");
    }
    // Nada de lo malo se ha guardado.
    assert_eq!(v, ok(&p, "leer_preferencias", json!({}), &cookie).await);
    assert_eq!(panel, ok(&p, "leer_panel", json!({}), &cookie).await);
    assert_eq!(
        instancia::leer(&p.state.db, claves::MANTENIMIENTO).await,
        Some(json!("03:00-05:30"))
    );
    // Vacío: sin mantenimiento ni servidor propio.
    let mut c = campos();
    c["mantenimiento"] = json!("");
    c["servidor_propio"] = json!("  ");
    ok(&p, "guardar_instancia", json!({ "campos": c }), &cookie).await;
    let v = ok(&p, "leer_preferencias", json!({}), &cookie).await;
    assert_eq!(v["mantenimiento"], Value::Null);
    assert_eq!(v["servidor_propio"], Value::Null);
}

#[tokio::test]
async fn cada_seccion_guarda_solo_lo_suyo() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": campos() }),
        &cookie,
    )
    .await;
    // Guardar solo los parámetros de las recomendaciones no toca la zona
    // ni el idioma.
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "dns_preferidos": ["9.9.9.9"], "dispositivos_confianza": [],
            "mantenimiento": "", "servidor_propio": "" } }),
        &cookie,
    )
    .await;
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["zona_horaria"], "UTC+01:00");
    let pref = ok(&p, "leer_preferencias", json!({}), &cookie).await;
    assert_eq!(pref["dns_preferidos"], json!(["9.9.9.9"]));
    // «Ajustes del panel» no manda los DNS.
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "idioma": { "Fijo": "En" }, "zona_horaria": "UTC-03:00" } }),
        &cookie,
    )
    .await;
    let v = ok(&p, "leer_panel", json!({}), &cookie).await;
    assert_eq!(v["zona_horaria"], "UTC-03:00");
    assert_eq!(v["idioma"], json!({ "Fijo": "En" }));
    let pref = ok(&p, "leer_preferencias", json!({}), &cookie).await;
    assert_eq!(pref["dns_preferidos"], json!(["9.9.9.9"]));
}

#[tokio::test]
async fn leer_preferencias_exige_sesion() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let (status, _) = llamar(&p, "leer_preferencias", json!({}), None).await;
    assert_eq!(status, 401);
    let cookie = sesion(&p).await;
    let resp = send(&p.app, {
        let mut req = post_json("/api/leer_preferencias", "{}", "127.0.0.1");
        req.headers_mut().insert("cookie", cookie.parse().unwrap());
        req
    })
    .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()["cache-control"], "no-store");
    let v: Value = serde_json::from_str(&body_text(resp).await).unwrap();
    assert_eq!(v["dns_preferidos"], json!([]));
    assert_eq!(v["dispositivos_confianza"], json!([]));
    assert_eq!(v["mantenimiento"], Value::Null);
    assert_eq!(v["servidor_propio"], Value::Null);
}

/// Los parámetros que usarán las recomendaciones no están en «Ajustes del
/// panel», y su subtítulo no enlaza con Recomendaciones (vuelve en la fase 4
/// con sus tarjetas).
#[tokio::test]
async fn preferencias_fuera_de_ajustes_del_panel() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let html = body_text(get_con_cookie(&p.app, "/panel", &cookie).await).await;
    assert!(html.contains(r#"id="panel-idioma""#), "{html}");
    assert!(!html.contains(r#"id="panel-dns""#), "{html}");
    assert!(!html.contains("/recomendaciones"), "{html}");
    assert!(!html.contains("Tus preferencias"), "{html}");
}

/// Zonas horarias que se aceptan (Península, Canarias, UTC y desfases fijos
/// de hasta 14 h) y que pasan a usarse en el acto; las demás, 400 sin tocar
/// nada.
#[tokio::test]
async fn zona_valida_y_no_valida() {
    use panel::ui::charts::Zona;
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    assert_eq!(p.state.zona(), Zona::Peninsular);
    for (zona, guardada, en_memoria) in [
        ("Atlantic/Canary", "Atlantic/Canary", Zona::Canarias),
        ("UTC", "UTC", Zona::Fija(0)),
        (" UTC+00:00 ", "UTC", Zona::Fija(0)),
        ("UTC-03:30", "UTC-03:30", Zona::Fija(-12_600)),
        ("UTC+14:00", "UTC+14:00", Zona::Fija(50_400)),
        ("Europe/Madrid", "Europe/Madrid", Zona::Peninsular),
        ("", "Europe/Madrid", Zona::Peninsular),
    ] {
        let c = ok(
            &p,
            "guardar_instancia",
            json!({ "campos": { "zona_horaria": zona } }),
            &cookie,
        )
        .await;
        assert_eq!(c["zona_horaria"], guardada, "{zona}");
        let v = ok(&p, "leer_panel", json!({}), &cookie).await;
        assert_eq!(v["zona_horaria"], guardada, "{zona}");
        assert_eq!(p.state.zona(), en_memoria, "{zona}");
    }
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "zona_horaria": "Atlantic/Canary" } }),
        &cookie,
    )
    .await;
    for zona in [
        "America/New_York",
        "europe/madrid",
        "UTC+15:00",
        "UTC-14:30",
        "UTC+1",
    ] {
        let (status, body) = llamar(
            &p,
            "guardar_instancia",
            json!({ "campos": { "zona_horaria": zona } }),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, 400, "{zona}: {body}");
        assert_eq!(p.state.zona(), Zona::Canarias, "{zona}");
    }
    // Al arrancar, la guardada.
    let dir = p.state.cfg.db_path.parent().unwrap();
    let otro = crate::comun::estado((*p.state.cfg).clone(), dir).await;
    assert_eq!(otro.zona(), Zona::Canarias);
}

/// La zona va en `<html data-zona>` (el navegador la lee de ahí al hidratar)
/// y las fechas de la página salen en ella.
#[tokio::test]
async fn html_lleva_data_zona() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let html = body_text(get_con_cookie(&p.app, "/panel", &cookie).await).await;
    assert!(html.contains(r#"data-zona="Europe/Madrid""#), "{html}");
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "zona_horaria": "Atlantic/Canary" } }),
        &cookie,
    )
    .await;
    for ruta in ["/panel", "/", "/historial", "/login"] {
        // /login, sin sesión: con ella lleva a la portada.
        let resp = if ruta == "/login" {
            crate::comun::get(&p.app, ruta).await
        } else {
            get_con_cookie(&p.app, ruta, &cookie).await
        };
        let html = body_text(resp).await;
        assert!(
            html.contains(r#"data-zona="Atlantic/Canary""#),
            "{ruta}: {html}"
        );
    }
    // El selector, en «Canarias».
    let html = body_text(get_con_cookie(&p.app, "/panel", &cookie).await).await;
    assert!(
        html.contains(r#"<option value="Atlantic/Canary" selected"#),
        "{html}"
    );
    // Un desfase fijo distinto de UTC va en «Otra», con su propio selector.
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "zona_horaria": "UTC+05:45" } }),
        &cookie,
    )
    .await;
    let html = body_text(get_con_cookie(&p.app, "/panel", &cookie).await).await;
    assert!(html.contains(r#"data-zona="UTC+05:45""#), "{html}");
    assert!(html.contains(r#"<option value="otra" selected"#), "{html}");
    assert!(html.contains(r#"id="panel-zona-otra""#), "{html}");
    assert!(
        html.contains(r#"<option value="UTC+05:45" selected"#),
        "{html}"
    );
    // Uno válido que no está en la lista, también se ve elegido.
    ok(
        &p,
        "guardar_instancia",
        json!({ "campos": { "zona_horaria": "UTC-14:00" } }),
        &cookie,
    )
    .await;
    let html = body_text(get_con_cookie(&p.app, "/panel", &cookie).await).await;
    assert!(
        html.contains(r#"<option value="UTC-14:00" selected"#),
        "{html}"
    );
}

/// Migración al arrancar: un `UTC` guardado por la versión anterior (era lo
/// que mostraba el selector por defecto) se quita una sola vez; un `UTC`
/// elegido después se respeta y cualquier otra zona no se toca.
#[tokio::test]
async fn migracion_zona_utc_por_defecto() {
    use panel::ui::charts::Zona;
    let p = PanelPrueba::nuevo().arrancar().await;
    let db = &p.state.db;
    let dir = p.state.cfg.db_path.parent().unwrap();
    let arrancar = || crate::comun::estado((*p.state.cfg).clone(), dir);
    // Base de datos de la versión anterior: sin la marca de la migración.
    let anterior = |zona: &'static str| async move {
        sqlx::query("DELETE FROM ajustes_instancia WHERE clave = ?")
            .bind(claves::MIGRACION_ZONA_UTC)
            .execute(db)
            .await
            .unwrap();
        instancia::escribir(db, claves::ZONA_HORARIA, &json!(zona))
            .await
            .unwrap();
    };

    // `UTC` sin la marca: se quita (la peninsular) y queda la marca.
    anterior("UTC").await;
    let s = arrancar().await;
    assert_eq!(s.zona(), Zona::Peninsular);
    assert_eq!(instancia::leer(db, claves::ZONA_HORARIA).await, None);
    assert!(
        instancia::leer(db, claves::MIGRACION_ZONA_UTC)
            .await
            .is_some()
    );

    // Un `UTC` guardado después sobrevive a otro arranque.
    instancia::escribir(db, claves::ZONA_HORARIA, &json!("UTC"))
        .await
        .unwrap();
    let s = arrancar().await;
    assert_eq!(s.zona(), Zona::Fija(0));
    assert_eq!(
        instancia::leer(db, claves::ZONA_HORARIA).await,
        Some(json!("UTC"))
    );

    // Otra zona sin la marca: no se toca.
    for zona in ["Atlantic/Canary", "UTC+01:00", "Europe/Madrid"] {
        anterior(zona).await;
        arrancar().await;
        assert_eq!(
            instancia::leer(db, claves::ZONA_HORARIA).await,
            Some(json!(zona)),
            "{zona}"
        );
        assert!(
            instancia::leer(db, claves::MIGRACION_ZONA_UTC)
                .await
                .is_some()
        );
    }
}
