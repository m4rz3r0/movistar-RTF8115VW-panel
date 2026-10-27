//! Pruebas del historial (server functions y página) contra `router-sim`:
//! orden y mezcla de cambios y reveladores, paginación, enlaces de deshacer y
//! que no sale ningún secreto.

use crate::comun::ConfirmarLigado;
use crate::comun::{
    Panel, PanelPrueba, aplicar, body_text, esperar_final, esperar_hasta, fijar, get_con_cookie,
    llamar, ok, valores_de,
};
use serde_json::{Value, json};
use std::time::Duration;

const LEASES: &str = "cbDhcpV4SvrPoolSetStatic";
const POOL: &str = "rtm_cfg_dhcpv4_svr_pool_id";

/// Pide la página `n` del historial.
async fn pagina(p: &Panel, cookie: &str, n: u32) -> Value {
    ok(p, "historial", json!({ "pagina": n }), cookie).await
}

/// Alta de una reserva estática; devuelve el id del cambio ya aplicado.
async fn alta_reserva(p: &Panel, cookie: &str, ip: &str, mac: &str) -> i64 {
    let f = ok(
        p,
        "formulario",
        json!({ "funcion": LEASES, "instancia": null, "claves": [[POOL, "0"]] }),
        cookie,
    )
    .await;
    let mut v = valores_de(&f);
    fijar(&mut v, "adm_state", "1");
    fijar(&mut v, "mac_addr", mac);
    fijar(&mut v, "ipv4_addr", ip);
    let id = aplicar(
        p,
        cookie,
        json!({ "funcion": LEASES, "instancia": null, "valores": v, "borrar": false }),
    )
    .await;
    assert_eq!(esperar_final(p, cookie, id).await["estado"], "Aplicado");
    id
}

fn cambios(pag: &Value) -> Vec<&Value> {
    pag["entradas"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e.get("Cambio"))
        .collect()
}

#[tokio::test]
async fn historial_lista_cambios_en_orden() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let vacia = pagina(&p, &cookie, 1).await;
    assert_eq!(vacia["entradas"], json!([]));
    assert_eq!(vacia["total"], 0);
    assert_eq!(vacia["hay_mas"], false);

    let a = alta_reserva(&p, &cookie, "192.168.1.91", "02:00:00:00:00:91").await;
    let b = alta_reserva(&p, &cookie, "192.168.1.92", "02:00:00:00:00:92").await;
    // Un revelado entre los dos (a mano: así se anotaban en versiones antiguas).
    sqlx::query("UPDATE changes SET ts = 1000 WHERE id = ?")
        .bind(a)
        .execute(&p.state.db)
        .await
        .unwrap();
    sqlx::query("UPDATE changes SET ts = 3000 WHERE id = ?")
        .bind(b)
        .execute(&p.state.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO reveals (ts, user_id, funcion, campo, instancia) \
         VALUES (2000, (SELECT id FROM users LIMIT 1), 'te_wifi.asp', 'wlWpaPsk', NULL)",
    )
    .execute(&p.state.db)
    .await
    .unwrap();

    let pag = pagina(&p, &cookie, 1).await;
    assert_eq!(pag["total"], 3);
    assert_eq!(pag["hay_mas"], false);
    let e = pag["entradas"].as_array().unwrap();
    assert_eq!(e.len(), 3, "{pag}");
    assert_eq!(e[0]["Cambio"]["id"], b);
    assert_eq!(e[1]["Revelado"]["campo"], "wlWpaPsk");
    assert_eq!(e[2]["Cambio"]["id"], a);

    let c = &e[0]["Cambio"];
    assert_eq!(c["funcion"], LEASES);
    assert_eq!(c["usuario"], crate::comun::PANEL_USER);
    assert_eq!(c["estado"], "Aplicado");
    assert_eq!(c["puede_deshacer"], true);
    assert_eq!(c["ruta"], "/explorador/lan_dhcp/cbDhcpV4SvrPoolSetStatic");
    assert!(
        c["resumen"].as_str().unwrap().contains("campos:"),
        "{}",
        c["resumen"]
    );
    // El diff lleva etiquetas de opción.
    let activado = c["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["nombre"] == "adm_state")
        .unwrap();
    assert_eq!(activado["etiqueta"], "Activado");
    assert_eq!(activado["despues"]["Opcion"]["etiqueta"], "Activado");
}

#[tokio::test]
async fn deshacer_desde_historial() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let id = alta_reserva(&p, &cookie, "192.168.1.93", "02:00:00:00:00:93").await;
    assert_eq!(
        crate::comun::ok(
            &p,
            "instancias",
            json!({ "funcion": LEASES, "claves": [[POOL, "0"]] }),
            &cookie
        )
        .await["filas"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    // Deshacer una alta es borrar la instancia: el diálogo lo sabe.
    let prep = ok(&p, "deshacer", json!({ "change_id": id }), &cookie).await;
    assert_eq!(prep["funcion"], LEASES);
    assert_eq!(prep["borrar"], true);
    assert!(prep["instancia"].is_string());
    assert!(!prep["campos"].as_array().unwrap().is_empty());
    let nuevo = ok(
        &p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    assert_eq!(
        esperar_final(&p, &cookie, nuevo).await["estado"],
        "Aplicado"
    );

    let pag = pagina(&p, &cookie, 1).await;
    let c = cambios(&pag);
    assert_eq!(c.len(), 2, "{pag}");
    assert_eq!(c[0]["id"], nuevo);
    assert_eq!(c[0]["deshace"], json!({ "id": id, "pagina": 1 }));
    assert_eq!(c[0]["deshecho_por"], Value::Null);
    assert_eq!(c[1]["id"], id);
    assert_eq!(c[1]["deshecho_por"], json!({ "id": nuevo, "pagina": 1 }));
    assert_eq!(c[1]["puede_deshacer"], false, "ya deshecho");
    // El borrado que deshizo el alta se puede deshacer a su vez (se vuelve a
    // crear con los valores anteriores).
    assert_eq!(c[0]["puede_deshacer"], true, "{}", c[0]);

    // El servidor lo impone, no solo el botón.
    let (status, body) = llamar(&p, "deshacer", json!({ "change_id": id }), Some(&cookie)).await;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("deshecho"), "{body}");
    let (status, _) = llamar(&p, "deshacer", json!({ "change_id": 999 }), Some(&cookie)).await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn sin_boton_en_lo_que_no_se_puede_deshacer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let id = alta_reserva(&p, &cookie, "192.168.1.94", "02:00:00:00:00:94").await;
    for estado in ["fallido", "revertido", "vigilando", "aplicando"] {
        sqlx::query("UPDATE changes SET estado = ? WHERE id = ?")
            .bind(estado)
            .bind(id)
            .execute(&p.state.db)
            .await
            .unwrap();
        let pag = pagina(&p, &cookie, 1).await;
        assert_eq!(cambios(&pag)[0]["puede_deshacer"], false, "{estado}");
        let (status, body) =
            llamar(&p, "deshacer", json!({ "change_id": id }), Some(&cookie)).await;
        assert_eq!(status, 400, "{estado}: {body}");
    }
    sqlx::query("UPDATE changes SET estado = 'aplicado' WHERE id = ?")
        .bind(id)
        .execute(&p.state.db)
        .await
        .unwrap();
    let pag = pagina(&p, &cookie, 1).await;
    assert_eq!(cambios(&pag)[0]["puede_deshacer"], true);
}

#[tokio::test]
async fn secretos_enmascarados_y_deshacer_los_restaura() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = p
        .state
        .hgu
        .call("cbGetWifiAccPt", &[("dpm_cfg_wifi_acc_pt_id", "1")])
        .await
        .unwrap();
    let clave = antes.children[0].text_of("key_phrase").unwrap().to_owned();
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": "cbSetWifiAccPt", "instancia": null,
                "claves": [["dpm_cfg_wifi_acc_pt_id", "1"]] }),
        &cookie,
    )
    .await;
    let mut v = valores_de(&f);
    fijar(&mut v, "key_phrase", "clave-nueva-muy-secreta");
    let id = aplicar(
        &p,
        &cookie,
        json!({ "funcion": "cbSetWifiAccPt", "instancia": null, "valores": v, "borrar": false }),
    )
    .await;
    assert_eq!(esperar_final(&p, &cookie, id).await["estado"], "Aplicado");

    let pag = pagina(&p, &cookie, 1).await;
    let texto = pag.to_string();
    assert!(!texto.contains("clave-nueva-muy-secreta"), "{texto}");
    assert!(!texto.contains(&clave), "{texto}");
    let c = cambios(&pag)[0];
    assert_eq!(c["campos"][0]["despues"], "Secreto");
    assert_eq!(c["campos"][0]["antes"], "Secreto");

    let prep = ok(&p, "deshacer", json!({ "change_id": id }), &cookie).await;
    let texto = prep.to_string();
    assert!(!texto.contains("clave-nueva-muy-secreta"), "{texto}");
    assert!(!texto.contains(&clave), "{texto}");
}

#[tokio::test]
async fn revelados_listados_sin_valor() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    // Un revelado antiguo (las versiones antiguas anotaban cada uno).
    sqlx::query(
        "INSERT INTO reveals (ts, user_id, funcion, campo, instancia) \
         VALUES (2000, (SELECT id FROM users LIMIT 1), 'te_wifi.asp', 'wlWpaPsk', NULL)",
    )
    .execute(&p.state.db)
    .await
    .unwrap();

    let pag = pagina(&p, &cookie, 1).await;
    let e = pag["entradas"].as_array().unwrap();
    assert_eq!(e.len(), 1, "{pag}");
    let r = &e[0]["Revelado"];
    assert_eq!(r["funcion"], "te_wifi.asp");
    assert_eq!(r["campo"], "wlWpaPsk");
    assert_eq!(r["usuario"], crate::comun::PANEL_USER);
    assert!(r["ts"].as_i64().unwrap() > 0);
    assert!(r["ruta"].is_string());
    assert!(!pag.to_string().contains("psk-sim"), "{pag}");
}

#[tokio::test]
async fn paginacion_en_los_limites() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for i in 0..60 {
        sqlx::query(
            "INSERT INTO reveals (ts, user_id, funcion, campo, instancia) \
             VALUES (?, NULL, 'te_wifi.asp', 'wlWpaPsk', ?)",
        )
        .bind(10_000 + i)
        .bind(format!("i{i}"))
        .execute(&p.state.db)
        .await
        .unwrap();
    }
    let instancia = |e: &Value| e["Revelado"]["instancia"].as_str().unwrap().to_owned();
    let p1 = pagina(&p, &cookie, 1).await;
    assert_eq!(p1["total"], 60);
    assert_eq!(p1["pagina"], 1);
    assert_eq!(p1["entradas"].as_array().unwrap().len(), 25);
    assert_eq!(p1["hay_mas"], true);
    assert_eq!(instancia(&p1["entradas"][0]), "i59");
    let p2 = pagina(&p, &cookie, 2).await;
    assert_eq!(p2["entradas"].as_array().unwrap().len(), 25);
    assert_eq!(instancia(&p2["entradas"][0]), "i34");
    assert_eq!(p2["hay_mas"], true);
    let p3 = pagina(&p, &cookie, 3).await;
    assert_eq!(p3["entradas"].as_array().unwrap().len(), 10);
    assert_eq!(instancia(&p3["entradas"][9]), "i0");
    assert_eq!(p3["hay_mas"], false);
    // Más allá del final: vacía, sin error. La 0 es la primera.
    let p4 = pagina(&p, &cookie, 4).await;
    assert_eq!(p4["entradas"], json!([]));
    assert_eq!(p4["hay_mas"], false);
    let p0 = pagina(&p, &cookie, 0).await;
    assert_eq!(p0["pagina"], 1);
    assert_eq!(p0["entradas"], p1["entradas"]);
    // Un revelado sin usuario (borrado) se lista igualmente.
    assert_eq!(p1["entradas"][0]["Revelado"]["usuario"], Value::Null);
}

#[tokio::test]
async fn exige_sesion_y_sirve_la_pagina() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, _) = llamar(&p, "historial", json!({ "pagina": 1 }), None).await;
    assert_eq!(status, 401);
    let (status, _) = llamar(&p, "deshacer", json!({ "change_id": 1 }), None).await;
    assert_eq!(status, 401);
    let resp = get_con_cookie(&p.app, "/historial", &cookie).await;
    assert_eq!(resp.status(), 200);
    let html = body_text(resp).await;
    assert!(html.contains("Historial"), "{html}");
    assert_eq!(crate::comun::get(&p.app, "/historial").await.status(), 303);
}

/// Un grupo es una sola fila con la lista de sus pasos: resumen humano
/// (traducido; sin etiqueta, la función), estado y diff de cada uno.
#[tokio::test]
async fn grupo_es_una_fila_con_sus_pasos() {
    use panel::changes::{ChangeManager, ChangeRequest, Estado, GrupoRequest, Paso, Tiempos};
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let solo = alta_reserva(&p, &cookie, "192.168.1.93", "02:00:00:00:00:93").await;
    let m = ChangeManager::instalar(&p.state, Tiempos::default());
    let paso = |funcion: &str, instancia: Option<&str>, v: &[(&str, &str)], e: Option<&str>| Paso {
        req: ChangeRequest {
            funcion: funcion.into(),
            instancia: instancia.map(str::to_owned),
            valores: v
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            borrar: false,
            identidad: Default::default(),
        },
        etiqueta: e.map(str::to_owned),
    };
    let prep = m
        .preparar_grupo(GrupoRequest {
            pasos: vec![
                paso(
                    "cbSetUpnpCfg",
                    None,
                    &[("adm_state", "1")],
                    Some("historial-deshacer"),
                ),
                paso(
                    LEASES,
                    Some("0"),
                    &[(POOL, "0"), ("ipv4_addr", "192.168.1.80")],
                    None,
                ),
            ],
            titulo: Some("historial-titulo".into()),
        })
        .await
        .unwrap();
    let id = m
        .confirmar_grupo_ligado(&prep.token, &[], "admin")
        .await
        .unwrap();
    esperar_hasta(Duration::from_secs(10), "aplicado", async || {
        m.cambio(id).await.unwrap().estado == Estado::Aplicado
    })
    .await;
    let pag = pagina(&p, &cookie, 1).await;
    let c = cambios(&pag);
    assert_eq!(c.len(), 2, "una fila por grupo");
    let g = c[0];
    assert_eq!(g["id"], id);
    assert_eq!(g["titulo"], "Historial");
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(pasos.len(), 2);
    assert_eq!(pasos[0]["numero"], 1);
    assert_eq!(pasos[0]["etiqueta"], "Deshacer");
    assert_eq!(pasos[0]["funcion"], "cbSetUpnpCfg");
    assert_eq!(pasos[0]["estado"], "Aplicado");
    // Sin etiqueta, un texto genérico: nunca el nombre de la
    // función, que sigue en los datos para los detalles técnicos.
    assert_eq!(pasos[1]["etiqueta"], "Cambio en el router");
    assert_eq!(pasos[1]["funcion"], LEASES);
    assert_eq!(pasos[1]["instancia"], "0");
    assert_eq!(pasos[1]["campos"][0]["nombre"], "ipv4_addr");
    assert_eq!(g["campos"].as_array().unwrap().len(), 2);
    assert_eq!(g["puede_deshacer"], true);
    // Un cambio de un paso sigue como siempre, sin lista de pasos.
    assert_eq!(c[1]["id"], solo);
    assert_eq!(c[1]["pasos"], json!([]));

    // Deshacer el grupo: un solo diálogo con los campos de los dos pasos.
    let d = ok(&p, "deshacer", json!({ "change_id": id }), &cookie).await;
    let campos: Vec<&str> = d["campos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["nombre"].as_str().unwrap())
        .collect();
    assert_eq!(campos, ["ipv4_addr", "adm_state"], "en orden inverso");
}
