//! El Historial en el idioma de quien lo mira: las notas del gestor (que se
//! guardan como claves) y las etiquetas de un cambio de la Biblioteca (las
//! del catálogo de funciones están en castellano).

use crate::comun::ConfirmarLigado;
use crate::comun::{PanelPrueba, esperar_estado, esperar_libre_gestor, ok, req};
use panel::changes::{Estado, GrupoRequest, Paso};
use serde_json::{Value, json};

/// Un grupo de dos pasos como los de la Biblioteca (sin título ni
/// etiquetas): UPnP y la IP de la reserva 0.
fn grupo_de_la_biblioteca() -> GrupoRequest {
    let paso = |r| Paso {
        req: r,
        etiqueta: None,
    };
    GrupoRequest {
        pasos: vec![
            paso(req("cbSetUpnpCfg", None, &[("adm_state", "1")])),
            paso(req(
                "cbDhcpV4SvrPoolSetStatic",
                Some("0"),
                &[
                    ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                    ("ipv4_addr", "192.168.1.80"),
                ],
            )),
        ],
        titulo: None,
    }
}

/// El primer cambio de la página 1 del Historial, en `idioma`.
async fn primer_cambio(p: &crate::comun::Panel, cookie: &str, idioma: &str) -> Value {
    let cookie = format!("{cookie}; rp_lang={idioma}");
    let pag = ok(p, "historial", json!({ "pagina": 1 }), &cookie).await;
    pag["entradas"][0]["Cambio"].clone()
}

fn etiquetas(c: &Value) -> Vec<String> {
    c["campos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["etiqueta"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn notas_y_etiquetas_en_el_idioma_de_quien_mira() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = p.state.cambios().unwrap();
    // Con la WAN caída antes del cambio, la nota lo dice.
    p.sim.control.set_wan_up(false);
    let prep = g.preparar_grupo(grupo_de_la_biblioteca()).await.unwrap();
    let id = g
        .confirmar_grupo_ligado(&prep.token, &[], "admin")
        .await
        .unwrap();
    esperar_estado(&g, id, Estado::Aplicado).await;
    esperar_libre_gestor(&g).await;

    let es = primer_cambio(&p, &cookie, "es").await;
    assert_eq!(es["id"], id);
    let nota = es["nota"].as_str().unwrap();
    assert!(
        nota.contains("la conexión a internet ya estaba caída"),
        "{nota}"
    );
    // En castellano, las de la función, como en la Biblioteca.
    assert_eq!(etiquetas(&es), ["Activado", "Dirección IPv4"]);

    let en = primer_cambio(&p, &cookie, "en").await;
    let nota = en["nota"].as_str().unwrap();
    assert!(
        nota.contains("the internet connection was already down"),
        "{nota}"
    );
    assert!(!nota.contains("caída"), "{nota}");
    // En inglés, las de Ajustes, traducidas: nada en castellano.
    let campos = etiquetas(&en);
    assert_eq!(campos.len(), 2, "{en}");
    for e in &campos {
        assert!(
            !["Activado", "Dirección IPv4"].contains(&e.as_str()),
            "{campos:?}"
        );
    }
    let resumen = en["resumen"].as_str().unwrap();
    assert!(
        !resumen.contains("Activado") && !resumen.contains("Dirección"),
        "{resumen}"
    );
}

/// Una fila de antes, que guarda la nota como texto: se muestra tal cual.
#[tokio::test]
async fn una_nota_antigua_se_muestra_tal_cual() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = p.state.cambios().unwrap();
    let prep = g.preparar_grupo(grupo_de_la_biblioteca()).await.unwrap();
    let id = g
        .confirmar_grupo_ligado(&prep.token, &[], "admin")
        .await
        .unwrap();
    esperar_estado(&g, id, Estado::Aplicado).await;
    esperar_libre_gestor(&g).await;
    let antigua = "vigilancia retomada tras un reinicio del panel";
    sqlx::query("UPDATE changes SET nota = ? WHERE id = ?")
        .bind(antigua)
        .bind(id)
        .execute(&p.state.db)
        .await
        .unwrap();
    for idioma in ["es", "en"] {
        assert_eq!(primer_cambio(&p, &cookie, idioma).await["nota"], antigua);
    }
}

/// Lo que se ve de la fila `id` del Historial: sin los datos de la página
/// ni los detalles técnicos plegados; y esos detalles.
fn fila_a_la_vista(pagina: &str, id: i64) -> (String, String) {
    use crate::comun::partir;
    let fila = &pagina[pagina
        .find(&format!("id=\"cambio-{id}\""))
        .expect("la fila")..];
    let fila = &fila[..fila
        .find("<li class=\"hist-entrada")
        // (El último `</ol>`: el primero puede ser el de los pasos.)
        .unwrap_or(fila.rfind("</ol>").unwrap())];
    let (sin_scripts, _) = partir(fila, "<script", "</script>");
    let (fuera, tecnicos) = partir(
        &sin_scripts,
        "<details class=\"detalles-tecnicos\"",
        "</details>",
    );
    (
        crate::ajustes_secretos::texto_visible(&fuera),
        tecnicos.concat(),
    )
}

/// Un grupo cuyo segundo paso rechaza el router: el error del Historial dice
/// qué pasó en lenguaje claro y en el idioma de quien mira, sin funciones
/// ni parámetros de la API fuera de los detalles técnicos, que guardan lo
/// que dijo el router.
#[tokio::test]
async fn el_error_de_un_paso_sin_nombres_de_la_api() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = p.state.cambios().unwrap();
    let prep = g.preparar_grupo(grupo_de_la_biblioteca()).await.unwrap();
    p.sim.control.fail_write_n(2);
    let r = g.confirmar_grupo_ligado(&prep.token, &[], "admin").await;
    assert!(r.is_err(), "{r:?}");
    let id = g.historial(1).await.remove(0).id;
    let c = esperar_estado(&g, id, Estado::Fallido).await;
    esperar_libre_gestor(&g).await;
    assert!(
        crate::comun::legible(c.error.as_deref().unwrap()).contains("paso 2"),
        "{c:?}"
    );
    let api = regex::Regex::new(r"\bcb[A-Z]\w*|adm_state|ipv4_addr|rtm_cfg|Result=").unwrap();
    for (idioma, error, revertido) in [
        (
            "es",
            "paso 2: el router no hizo el cambio",
            "se volvió a los valores anteriores",
        ),
        // (Sin el apóstrofo, que el HTML escapa.)
        ("en", "make the change", "the previous values were put back"),
    ] {
        let con_idioma = format!("{cookie}; rp_lang={idioma}");
        let pagina = crate::ajustes_secretos::html(&p, &con_idioma, "/historial").await;
        let (visible, tecnicos) = fila_a_la_vista(&pagina, id);
        assert!(visible.contains(error), "{idioma}: {visible}");
        assert!(visible.contains(&t_paso(idioma)), "{idioma}: {visible}");
        assert!(visible.contains(revertido), "{idioma}: {visible}");
        assert!(!api.is_match(&visible), "{idioma}: {visible}");
        // Lo que dijo el router, en los detalles técnicos.
        assert!(api.is_match(&tecnicos), "{idioma}: {tecnicos}");
    }
}

fn t_paso(idioma: &str) -> String {
    if idioma == "es" { "paso 2:" } else { "step 2:" }.to_owned()
}

/// Los secretos mostrados de la 0.2.x (la 0.3 ya no los apunta): la fila dice
/// qué campo se mostró con su nombre legible o, sin él, «una clave»; la
/// función y el parámetro, solo en los detalles técnicos.
#[tokio::test]
async fn revelados_antiguos_sin_nombres_de_la_api() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    for (ts, funcion) in [(2000, "te_wifi.wl"), (3000, "te_desconocida.asp")] {
        sqlx::query(
            "INSERT INTO reveals (ts, user_id, funcion, campo, instancia) \
             VALUES (?, (SELECT id FROM users LIMIT 1), ?, 'wlWpaPsk', NULL)",
        )
        .bind(ts)
        .bind(funcion)
        .execute(&p.state.db)
        .await
        .unwrap();
    }
    for (idioma, conocido, desconocido) in [
        ("es", "Se mostró «Clave WiFi»", "Se mostró una clave"),
        ("en", "", "A password was shown"),
    ] {
        let con_idioma = format!("{cookie}; rp_lang={idioma}");
        let pagina = crate::ajustes_secretos::html(&p, &con_idioma, "/historial").await;
        let mut visibles = Vec::new();
        for id in [1, 2] {
            let fila = &pagina[pagina
                .find(&format!("id=\"revelado-{id}\""))
                .expect("la fila")..];
            let fila = &fila[..fila.find("</li>").unwrap()];
            let (fuera, tecnicos) =
                crate::comun::partir(fila, "<details class=\"detalles-tecnicos\"", "</details>");
            let visible = crate::ajustes_secretos::texto_visible(&fuera);
            assert!(
                !visible.contains("wlWpaPsk") && !visible.contains("te_"),
                "{idioma}: {visible}"
            );
            assert!(tecnicos.concat().contains("wlWpaPsk"), "{idioma}: {fila}");
            visibles.push(visible);
        }
        // El de la función conocida (id 1), con su nombre; el otro, genérico.
        assert!(visibles[0].contains(conocido), "{idioma}: {}", visibles[0]);
        assert!(
            visibles[1].contains(desconocido),
            "{idioma}: {}",
            visibles[1]
        );
    }
}

/// Una clave guardada que este panel no conoce (p. ej. tras volver a una
/// versión anterior) y una nota ilegible: el Historial dice algo genérico y
/// traducido, nunca la clave ni el JSON, que van en los detalles técnicos.
#[tokio::test]
async fn clave_desconocida_texto_generico() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let g = p.state.cambios().unwrap();
    let prep = g.preparar_grupo(grupo_de_la_biblioteca()).await.unwrap();
    let id = g
        .confirmar_grupo_ligado(&prep.token, &[], "admin")
        .await
        .unwrap();
    esperar_estado(&g, id, Estado::Aplicado).await;
    esperar_libre_gestor(&g).await;
    for (guardado, en_lo_tecnico) in [
        (
            r#"i18n:[{"clave":"cambio-error-de-una-version-futura","tecnico":"cbSetX: Result=1"}]"#,
            "cambio-error-de-una-version-futura",
        ),
        ("i18n:{roto", "i18n:{roto"),
    ] {
        sqlx::query("UPDATE changes SET nota = ? WHERE id = ?")
            .bind(guardado)
            .bind(id)
            .execute(&p.state.db)
            .await
            .unwrap();
        for (idioma, generico) in [
            ("es", "algo no salió como se esperaba"),
            ("en", "something did not go as expected"),
        ] {
            let con_idioma = format!("{cookie}; rp_lang={idioma}");
            let pagina = crate::ajustes_secretos::html(&p, &con_idioma, "/historial").await;
            let (visible, tecnicos) = fila_a_la_vista(&pagina, id);
            assert!(visible.contains(generico), "{idioma}: {visible}");
            assert!(
                !visible.contains("version-futura")
                    && !visible.contains("i18n:")
                    && !visible.contains("cbSetX"),
                "{idioma}: {visible}"
            );
            assert!(tecnicos.contains(en_lo_tecnico), "{idioma}: {tecnicos}");
        }
    }
}
