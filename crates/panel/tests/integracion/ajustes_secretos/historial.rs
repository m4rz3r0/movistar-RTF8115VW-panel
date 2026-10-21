//! Secretos en el Historial: detalle descifrado y sin nombres de la API.

use super::*;

/// El detalle de un cambio del Historial trae, al desplegarlo, el valor
/// anterior y el nuevo de cada secreto (descifrados solo para ese cambio,
/// con `no-store`); la lista del Historial no los trae nunca.
#[tokio::test]
async fn detalle_de_un_cambio_con_sus_secretos() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    let id = cambiar_clave_5(&p, &cookie, CLAVE_2).await;
    let h = ok(&p, "historial", json!({ "pagina": 1 }), &cookie)
        .await
        .to_string();
    assert!(!h.contains(CLAVE_1) && !h.contains(CLAVE_2), "{h}");
    let (status, cache, body) =
        llamada(&p, "secretos_del_cambio", json!({ "cambio": id }), &cookie).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        v,
        json!([{ "orden": 0, "nombre": "key_phrase", "antes": CLAVE_1, "despues": CLAVE_2,
                 "tapado": false }]),
        "{v}"
    );
    // Sin sesión, nada; un cambio que no existe, vacío.
    let (status, _) = llamar(&p, "secretos_del_cambio", json!({ "cambio": id }), None).await;
    assert_eq!(status, 401);
    assert_eq!(
        ok(
            &p,
            "secretos_del_cambio",
            json!({ "cambio": id + 50 }),
            &cookie
        )
        .await,
        json!([])
    );
    let filas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reveals")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert_eq!(filas, 0);
}

/// La tabla de diferencias del Historial no enseña parámetros de la API: de
/// un cambio hecho desde Ajustes, la etiqueta del campo (la de Ajustes) en
/// los dos idiomas; de uno de la Biblioteca, el nombre técnico solo dentro
/// de los detalles técnicos plegados.
#[tokio::test]
async fn historial_con_las_etiquetas_de_ajustes() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let id = cambiar_clave_5(&p, &cookie, CLAVE_2).await;
    for (idioma, ajuste, campo) in [
        ("es", "Red WiFi 5 GHz", "Contraseña"),
        ("en", "5 GHz WiFi network", "Password"),
    ] {
        let con_idioma = format!("{cookie}; rp_lang={idioma}");
        let h = ok(&p, "historial", json!({ "pagina": 1 }), &con_idioma).await;
        let c = h["entradas"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|e| e.get("Cambio").filter(|c| c["id"] == id))
            .unwrap();
        assert_eq!(c["campos"][0]["etiqueta"], campo, "{idioma}: {c}");
        assert_eq!(c["titulo"], ajuste, "{idioma}: {c}");
        let pagina = html(&p, &con_idioma, "/historial").await;
        let fila = &pagina[pagina
            .find(&format!("id=\"cambio-{id}\""))
            .expect("la fila")..];
        let fila = &fila[..fila
            .find("<li class=\"hist-entrada")
            .unwrap_or(fila.find("</ol>").unwrap())];
        let (fuera, _) = partir(fila, "<details class=\"detalles-tecnicos\"", "</details>");
        let visible = texto_visible(&fuera);
        assert!(
            visible.contains(campo) && visible.contains(ajuste),
            "{idioma}: {visible}"
        );
        assert!(
            !visible.contains("key_phrase") && !visible.contains("cbSet"),
            "{idioma}: {visible}"
        );
    }
    // Uno de la Biblioteca: el nombre técnico, solo en los detalles.
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetUpnpCfg", "instancia": null,
                "valores": [["adm_state", "1"]], "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let prep: Value = serde_json::from_str(&body).unwrap();
    let otro = ok(
        &p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    esperar_final(&p, &cookie, otro).await;
    let pagina = html(&p, &cookie, "/historial").await;
    let fila = &pagina[pagina
        .find(&format!("id=\"cambio-{otro}\""))
        .expect("la fila")..];
    let fila = &fila[..fila
        .find("<li class=\"hist-entrada")
        .unwrap_or(fila.find("</ol>").unwrap())];
    let (fuera, tecnicos) = partir(fila, "<details class=\"detalles-tecnicos\"", "</details>");
    assert!(!texto_visible(&fuera).contains("adm_state"), "{fuera}");
    assert!(
        tecnicos.iter().any(|t| t.contains("adm_state")),
        "{tecnicos:?}"
    );
}

/// Lo que se ve de una fila del Historial: fuera de los datos de la página
/// y de los detalles técnicos plegados.
fn a_la_vista(fila: &str) -> String {
    let (sin_scripts, _) = partir(fila, "<script", "</script>");
    let (fuera, _) = partir(
        &sin_scripts,
        "<details class=\"detalles-tecnicos\"",
        "</details>",
    );
    texto_visible(&fuera)
}

/// La fila `id="cambio-{id}"` del HTML del Historial.
fn fila_del_cambio(pagina: &str, id: i64) -> String {
    let fila = &pagina[pagina
        .find(&format!("id=\"cambio-{id}\""))
        .expect("la fila")..];
    fila[..fila
        .find("<li class=\"hist-entrada")
        .unwrap_or(fila.find("</ol>").unwrap())]
        .to_owned()
}

/// El deshacer de un cambio hecho desde Ajustes lleva su título, y sus
/// campos las etiquetas de Ajustes, en los dos idiomas; ninguna fila enseña
/// el nombre de una función. Una fila de deshacer guardada sin título (de
/// antes) lo toma del cambio que deshace, y uno de la Biblioteca, sin
/// título, sale como «Cambio en el router».
#[tokio::test]
async fn historial_sin_nombres_de_la_api_al_deshacer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = preparar(&p, &cookie, "wifi.red_5", json!({ "nombre": "otra-red" })).await;
    assert_eq!(status, 200, "{body}");
    let id = aplicar_ajuste(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let g = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    let deshecho = aplicar_ajuste(&p, &cookie, &g).await;
    let titulo: Option<String> = sqlx::query_scalar("SELECT titulo FROM changes WHERE id = ?")
        .bind(deshecho)
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert_eq!(
        titulo.as_deref(),
        Some("ajuste-wifi-red_5-titulo"),
        "el deshacer lleva título"
    );
    // Uno de la Biblioteca, sin título.
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetUpnpCfg", "instancia": null,
                "valores": [["adm_state", "1"]], "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let prep: Value = serde_json::from_str(&body).unwrap();
    let biblioteca = ok(
        &p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        &cookie,
    )
    .await
    .as_i64()
    .unwrap();
    esperar_final(&p, &cookie, biblioteca).await;
    let comprobar = |fase: &'static str| {
        let (p, cookie) = (&p, &cookie);
        async move {
            for (idioma, ajuste, campo, generico) in [
                (
                    "es",
                    "Red WiFi 5 GHz",
                    "Nombre de la red",
                    "Cambio en el router",
                ),
                ("en", "5 GHz WiFi network", "Network name", "Router change"),
            ] {
                let con_idioma = format!("{cookie}; rp_lang={idioma}");
                let h = ok(p, "historial", json!({ "pagina": 1 }), &con_idioma).await;
                let c = h["entradas"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find_map(|e| e.get("Cambio").filter(|c| c["id"] == deshecho))
                    .unwrap();
                assert_eq!(c["titulo"], ajuste, "{fase} {idioma}: {c}");
                assert_eq!(c["campos"][0]["etiqueta"], campo, "{fase} {idioma}: {c}");
                let pagina = html(p, &con_idioma, "/historial").await;
                for (n, esperado) in [(deshecho, ajuste), (biblioteca, generico)] {
                    let vista = a_la_vista(&fila_del_cambio(&pagina, n));
                    assert!(vista.contains(esperado), "{fase} {idioma} {n}: {vista}");
                    assert!(!vista.contains("cbSet"), "{fase} {idioma} {n}: {vista}");
                }
                let vista = a_la_vista(&fila_del_cambio(&pagina, deshecho));
                assert!(vista.contains(campo), "{fase} {idioma}: {vista}");
            }
        }
    };
    comprobar("con título").await;
    // Una fila de deshacer guardada antes, sin título: lo toma del cambio
    // que deshace.
    sqlx::query("UPDATE changes SET titulo = NULL WHERE id = ?")
        .bind(deshecho)
        .execute(&p.state.db)
        .await
        .unwrap();
    comprobar("sin título guardado").await;
}
