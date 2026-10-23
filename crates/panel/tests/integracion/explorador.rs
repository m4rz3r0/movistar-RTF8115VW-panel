//! Pruebas del explorador (server functions y páginas) contra `router-sim`,
//! con el gestor de cambios del servidor y tiempos de vigilancia cortos.

use crate::comun::{
    Panel, PanelPrueba, aplicar, body_text, campo, esperar_final, fijar, get_con_cookie, llamar,
    ok, valores_de,
};
use serde_json::{Value, json};

const LEASES: &str = "cbDhcpV4SvrPoolSetStatic";
const POOL: &str = "rtm_cfg_dhcpv4_svr_pool_id";
const MASCARA: &str = "••••••";

async fn ids_de_reservas(p: &Panel, cookie: &str) -> Vec<String> {
    let i = ok(
        p,
        "instancias",
        json!({ "funcion": LEASES, "claves": [[POOL, "0"]] }),
        cookie,
    )
    .await;
    i["filas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap().to_owned())
        .collect()
}

/// Una lectura trae el valor de sus secretos para el campo censurado con el
/// ojo (`Clave`), sin pedir la contraseña ni anotar nada; el XML crudo, que
/// se lee como texto, sigue enmascarado.
#[tokio::test]
async fn leer_trae_los_secretos_para_el_ojo() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    // Página básica: variables con nombres de secretos del catálogo.
    let v = ok(
        &p,
        "leer",
        json!({ "funcion": "te_wifi.asp", "params": [] }),
        &cookie,
    )
    .await;
    let body = v.to_string();
    assert!(!body.contains("sessionKey"), "{body}");
    let par = |n: &str| {
        v["pares"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["nombre"] == n)
            .unwrap_or_else(|| panic!("falta {n}: {body}"))["valor"]
            .clone()
    };
    assert_eq!(par("wlWpaPsk"), json!({ "Clave": "psk-sim-secreta" }));
    assert_eq!(par("wlSsid"), json!({ "Texto": "RED-SIM" }));

    // Lectura XML: la tabla de instancias con el valor; el XML crudo,
    // enmascarado.
    let v = ok(
        &p,
        "leer",
        json!({ "funcion": "cbGetWifiAccPt", "params": [["dpm_cfg_wifi_acc_pt_id", "1"]] }),
        &cookie,
    )
    .await;
    let g = &v["grupos"][0];
    let col = g["columnas"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c == "key_phrase")
        .expect("columna key_phrase");
    assert_eq!(
        g["filas"][0]["valores"][col],
        json!({ "Clave": "X-REDACTED-X" })
    );
    assert_eq!(g["filas"][0]["id"], "1");
    let crudo = v["crudo"].as_str().unwrap();
    assert!(
        crudo.contains(MASCARA) && !crudo.contains("X-REDACTED-X"),
        "{crudo}"
    );
    let filas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reveals")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert_eq!(filas, 0, "ver un secreto ya no se anota");

    // Lista completa con `all=ALL`.
    let v = ok(
        &p,
        "leer",
        json!({ "funcion": "cbDhcpV4SvrPoolGetStatic",
                "params": [[POOL, "0"], ["all", "ALL"], ["rtm_dhcpv4_svr_pool_static_id", ""]] }),
        &cookie,
    )
    .await;
    assert_eq!(v["grupos"][0]["total"], 2, "{v}");

    // Las páginas que disparan acciones, las acciones y lo desconocido no se leen.
    for f in [
        "syslog.asp",
        "backup_restore.asp",
        "cbReboot",
        "te_wifirefresh.wl",
    ] {
        let (status, body) = llamar(
            &p,
            "leer",
            json!({ "funcion": f, "params": [] }),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, 403, "{f}: {body}");
    }
    let (status, _) = llamar(
        &p,
        "leer",
        json!({ "funcion": "cbGetSummary", "params": [["sessionKey", "1"]] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400);
    // Sin sesión, nada.
    let (status, _) = llamar(
        &p,
        "leer",
        json!({ "funcion": "te_wifi.asp", "params": [] }),
        None,
    )
    .await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn formulario_static_lease_rellena_valores() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": LEASES, "instancia": "0", "claves": [[POOL, "0"]] }),
        &cookie,
    )
    .await;
    assert_eq!(f["operacion"], "Editar");
    assert_eq!(campo(&f, "ipv4_addr").unwrap()["valor"], "192.168.1.68");
    assert_eq!(campo(&f, "mac_addr").unwrap()["valor"], "02:00:00:00:00:04");
    assert_eq!(campo(&f, "adm_state").unwrap()["valor"], "1");
    assert_eq!(campo(&f, "mac_addr").unwrap()["param"]["tipo"], "Mac");
    // Los ids no son campos editables.
    assert!(campo(&f, POOL).is_none());
    assert!(campo(&f, "rtm_dhcpv4_svr_pool_static_id").is_none());
    assert_eq!(f["claves"], json!([[POOL, "0"]]));

    // Crear: los mismos campos, vacíos.
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": LEASES, "instancia": null, "claves": [[POOL, "0"]] }),
        &cookie,
    )
    .await;
    assert_eq!(f["operacion"], "Crear");
    assert_eq!(campo(&f, "ipv4_addr").unwrap()["valor"], "");
    assert_eq!(ids_de_reservas(&p, &cookie).await, ["0", "1"]);

    // Un sí/no al crear empieza en «0»: el interruptor apagado se envía.
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": "cbSetTR98L2Filtering", "instancia": null, "claves": [] }),
        &cookie,
    )
    .await;
    assert_eq!(f["operacion"], "Crear");
    let c = campo(&f, "cls_ethertype").expect("cls_ethertype");
    assert_eq!(c["param"]["tipo"], "Bool");
    assert_eq!(c["valor"], "0");
    assert_eq!(campo(&f, "order").unwrap()["valor"], "");

    // Una clave que no identifica instancias se rechaza.
    let (status, _) = llamar(
        &p,
        "formulario",
        json!({ "funcion": LEASES, "instancia": "0", "claves": [["ipv4_addr", "198.51.100.4"]] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400);
}

/// Review Focus #1: el formulario trae el valor del secreto para el campo
/// censurado con el ojo (con `no-store`); enviado sin tocar (el navegador lo
/// devuelve como MASCARA, o con su mismo valor), nunca escribe MASCARA y
/// conserva el valor actual.
#[tokio::test]
async fn preparar_con_secreto_sin_tocar_no_cambia_secreto() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "1")])
        .await
        .unwrap();
    let clave_actual = antes.children[0].text_of("key_phrase").unwrap().to_owned();
    assert!(!clave_actual.is_empty() && clave_actual != MASCARA);

    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": "cbSetWifiAccPt", "instancia": null,
                "claves": [["dpm_cfg_wifi_acc_pt_id", "1"]] }),
        &cookie,
    )
    .await;
    assert_eq!(f["operacion"], "Editar");
    assert_eq!(
        campo(&f, "key_phrase").unwrap()["valor"],
        clave_actual.as_str()
    );
    assert_eq!(campo(&f, "key_phrase").unwrap()["sin_leer"], false);
    // Lo que la lectura no muestra no se ofrece (el gestor lo rechazaría).
    assert!(campo(&f, "validation_only").is_none());
    assert!(
        f["omitidos"]
            .as_array()
            .unwrap()
            .contains(&json!("validation_only"))
    );

    // Sin tocar nada: no hay cambio.
    let valores = valores_de(&f);
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetWifiAccPt", "instancia": null,
                              "valores": valores, "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("No hay nada que cambiar"), "{body}");
    // Ni devolviendo la clave tal cual llegó.
    let mut valores = valores_de(&f);
    fijar(&mut valores, "key_phrase", &clave_actual);
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetWifiAccPt", "instancia": null,
                              "valores": valores, "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");

    // Cambiando solo el SSID: el diff no incluye el secreto.
    let mut valores = valores_de(&f);
    fijar(&mut valores, "ssid", "Otra red");
    let peticion = json!({ "funcion": "cbSetWifiAccPt", "instancia": null,
                           "valores": valores, "borrar": false });
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": peticion }),
        &cookie,
    )
    .await;
    let campos: Vec<&str> = prep["campos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["nombre"].as_str().unwrap())
        .collect();
    assert_eq!(campos, ["ssid"], "{prep}");
    assert_eq!(prep["riesgo"], "Medio");
    let id = ok(
        &p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    let c = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");

    let writes = p.sim.control.writes();
    assert_eq!(writes.len(), 1, "{writes:?}");
    let (nombre, params) = &writes[0];
    assert_eq!(nombre, "cbSetWifiAccPt");
    assert!(params.iter().all(|(_, v)| v != MASCARA), "{params:?}");
    let enviado = |k: &str| params.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    assert_eq!(enviado("key_phrase"), Some(clave_actual.as_str()));
    assert_eq!(enviado("ssid"), Some("Otra red"));
    assert_eq!(enviado("validation_only"), None);
}

#[tokio::test]
async fn bloqueada_sin_formulario() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for (f, motivo) in [
        ("cbRestoreConfig", ""),
        ("te_wifi.wl", "v1"),
        ("cbSetLanIPv4Addr", "IP del router"),
        ("scbidirectionalflt.cmd?action=add", "cortafuegos"),
        ("scbidirectionalfwl.cmd?action=save", "cortafuegos"),
    ] {
        let d = ok(&p, "explorador_funcion", json!({ "nombre": f }), &cookie).await;
        let b = d["resumen"]["bloqueada"]
            .as_str()
            .unwrap_or_else(|| panic!("{f}: {d}"));
        assert!(b.contains(motivo), "{f}: {b}");
        let (status, body) = llamar(
            &p,
            "formulario",
            json!({ "funcion": f, "instancia": null, "claves": [] }),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, 403, "{f}: {body}");
        assert!(body.contains("bloqueada"), "{f}: {body}");
        let (status, _) = llamar(
            &p,
            "preparar_cambio",
            json!({ "peticion": { "funcion": f, "instancia": null, "valores": [], "borrar": false } }),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, 403, "{f}");
    }
    assert!(p.sim.control.writes().is_empty());
    // El lateral marca las bloqueadas, también las de la v1.
    let areas = ok(&p, "explorador_catalogo", json!({}), &cookie).await;
    let areas = areas.as_array().unwrap();
    assert_eq!(areas.len(), 14);
    let todas: Vec<&Value> = areas
        .iter()
        .flat_map(|a| a["funciones"].as_array().unwrap())
        .collect();
    assert!(todas.len() > 100);
    let bloqueada = |n: &str| {
        todas.iter().find(|f| f["nombre"] == n).unwrap()["bloqueada"]
            .as_str()
            .is_some()
    };
    assert!(bloqueada("te_wifi.wl") && bloqueada("upload/cfgupload.asp"));
    assert!(!bloqueada(LEASES) && !bloqueada("cbGetSummary"));
}

/// Review Focus #3: dos confirmaciones a la vez; una recibe "en curso".
#[tokio::test]
async fn confirmar_a_la_vez_uno_en_curso() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let a = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": LEASES, "instancia": "0",
                              "valores": [[POOL, "0"], ["ipv4_addr", "192.168.1.80"]], "borrar": false } }),
        &cookie,
    )
    .await;
    let b = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetUpnpCfg", "instancia": null,
                              "valores": [["adm_state", "1"]], "borrar": false } }),
        &cookie,
    )
    .await;
    let confirmar = |t: &Value| json!({ "token": t["token"], "palabra": null });
    let (ra, rb) = tokio::join!(
        llamar(&p, "confirmar_cambio", confirmar(&a), Some(&cookie)),
        llamar(&p, "confirmar_cambio", confirmar(&b), Some(&cookie)),
    );
    let (bien, mal) = match (ra.0, rb.0) {
        (200, 409) => (ra, rb),
        (409, 200) => (rb, ra),
        _ => panic!("se esperaba uno aceptado y otro en curso: {ra:?} {rb:?}"),
    };
    assert!(mal.1.contains("otro cambio en curso"), "{}", mal.1);
    assert!(mal.1.contains("espera a que termine"), "{}", mal.1);
    assert_eq!(p.sim.control.writes().len(), 1);
    let id: i64 = bien.1.parse().unwrap();
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
}

#[tokio::test]
async fn reserva_estatica_alta_edicion_y_baja() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    // Alta con el formulario de crear.
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": LEASES, "instancia": null, "claves": [[POOL, "0"]] }),
        &cookie,
    )
    .await;
    let mut valores = valores_de(&f);
    fijar(&mut valores, "adm_state", "1");
    fijar(&mut valores, "mac_addr", "02:00:00:00:00:99");
    fijar(&mut valores, "ipv4_addr", "192.168.1.99");
    // Validación del servidor: una MAC mal escrita no llega al gestor.
    let mut mala = valores.clone();
    fijar(&mut mala, "mac_addr", "02:00:00:00:99");
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": LEASES, "instancia": null, "valores": mala, "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("MAC"), "{body}");

    let id = aplicar(
        &p,
        &cookie,
        json!({ "funcion": LEASES, "instancia": null, "valores": valores, "borrar": false }),
    )
    .await;
    let c = esperar_final(&p, &cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
    let ids = ids_de_reservas(&p, &cookie).await;
    assert_eq!(ids, ["0", "1", "2"]);

    // Edición de la nueva.
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": LEASES, "instancia": "2", "claves": [[POOL, "0"]] }),
        &cookie,
    )
    .await;
    assert_eq!(campo(&f, "ipv4_addr").unwrap()["valor"], "192.168.1.99");
    let mut valores = valores_de(&f);
    fijar(&mut valores, "ipv4_addr", "192.168.1.98");
    let id = aplicar(
        &p,
        &cookie,
        json!({ "funcion": LEASES, "instancia": "2", "valores": valores, "borrar": false }),
    )
    .await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": LEASES, "instancia": "2", "claves": [[POOL, "0"]] }),
        &cookie,
    )
    .await;
    assert_eq!(campo(&f, "ipv4_addr").unwrap()["valor"], "192.168.1.98");

    // Baja (el gestor añade `del=DEL`).
    let id = aplicar(
        &p,
        &cookie,
        json!({ "funcion": LEASES, "instancia": "2", "valores": [[POOL, "0"]], "borrar": true }),
    )
    .await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");
    assert_eq!(ids_de_reservas(&p, &cookie).await, ["0", "1"]);
    let ultimo = p.sim.control.writes().pop().unwrap();
    assert!(
        ultimo.1.contains(&("del".into(), "DEL".into())),
        "{ultimo:?}"
    );
}

#[tokio::test]
async fn paginas_del_explorador_con_nombres_codificados() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let html = |uri: &'static str| {
        let app = p.app.clone();
        let cookie = cookie.clone();
        async move {
            let resp = get_con_cookie(&app, uri, &cookie).await;
            assert_eq!(resp.status(), 200, "{uri}");
            body_text(resp).await
        }
    };
    let h = html("/explorador").await;
    assert!(h.contains("LAN/DHCP"), "{h}");
    let h = html("/explorador/nat").await;
    assert!(h.contains("te_ppp_pm.cmd?action=create"), "{h}");
    assert!(
        h.contains("/explorador/nat/te_ppp_pm.cmd%3Faction%3Dcreate"),
        "{h}"
    );
    let h = html("/explorador/nat/te_ppp_pm.cmd%3Faction%3Dcreate").await;
    assert!(h.contains("<h1>te_ppp_pm.cmd?action=create</h1>"), "{h}");
    let h = html("/explorador/sistema/upload%2Fcfgupload.asp").await;
    assert!(h.contains("<h1>upload/cfgupload.asp</h1>"), "{h}");
    assert!(h.contains("Bloqueada"), "{h}");
    // Sin sesión, al login.
    let resp = crate::comun::get(&p.app, "/explorador/wan").await;
    assert_eq!(resp.status(), 303);
}

/// Las respuestas de la Biblioteca que pueden llevar secretos (una lectura,
/// las instancias de una lista, el formulario) van con `no-store`.
#[tokio::test]
async fn respuestas_con_secretos_no_store() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for (nombre, cuerpo) in [
        (
            "leer",
            json!({ "funcion": "cbGetWifiAccPt", "params": [["dpm_cfg_wifi_acc_pt_id", "1"]] }),
        ),
        (
            "instancias",
            json!({ "funcion": LEASES, "claves": [[POOL, "0"]] }),
        ),
        (
            "formulario",
            json!({ "funcion": "cbSetWifiAccPt", "instancia": null,
                    "claves": [["dpm_cfg_wifi_acc_pt_id", "1"]] }),
        ),
    ] {
        let mut req =
            crate::comun::post_json(&format!("/api/{nombre}"), &cuerpo.to_string(), "127.0.0.1");
        req.headers_mut().insert("cookie", cookie.parse().unwrap());
        let resp = crate::comun::send(&p.app, req).await;
        assert_eq!(resp.status(), 200, "{nombre}");
        let cache = resp
            .headers()
            .get(axum::http::header::CACHE_CONTROL)
            .map(|v| v.to_str().unwrap().to_owned());
        assert_eq!(cache.as_deref(), Some("no-store"), "{nombre}");
    }
}
