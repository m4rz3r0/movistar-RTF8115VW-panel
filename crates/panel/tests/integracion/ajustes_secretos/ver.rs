//! Secretos a la vista: solo en el campo de las páginas que los muestran.

use super::*;

/// El valor de un secreto llega con la página (`leer_tema`, con `no-store`)
/// para el campo censurado con el ojo, también después de cambiarlo en el
/// router (sin la caché vieja). Ver un secreto ya no pide la contraseña ni
/// anota nada; lo que el router no deja leer (enmascarado o tapado al leer)
/// llega vacío.
#[tokio::test]
async fn los_secretos_llegan_con_la_pagina() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    let (status, cache, body) = llamada(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(cache, "no-store");
    let tema: Value = serde_json::from_str(&body).unwrap();
    let c = campo_de(&tema, "wifi.red_5", "contrasena");
    assert_eq!(c["valor"], "Secreto", "{c}");
    assert_eq!(c["valor_secreto"], CLAVE_1, "{c}");
    // Lo que no es secreto no lleva valor aparte.
    assert_eq!(
        campo_de(&tema, "wifi.red_5", "nombre")["valor_secreto"],
        Value::Null
    );
    // Uno que el router da enmascarado no es su valor: vacío.
    escribir_punto(&p, "1", &[("key_phrase", "********")]).await;
    p.state.lecturas.invalidar();
    let tema = ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    assert_eq!(
        campo_de(&tema, "wifi.red_5", "contrasena")["valor_secreto"],
        Value::Null
    );
    let filas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reveals")
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert_eq!(filas, 0, "ver un secreto ya no se anota");
    // Las server functions para revelar ya no existen.
    for f in ["revelar_ajuste", "revelar_anterior", "revelar"] {
        let (status, _) = llamar(&p, f, json!({}), Some(&cookie)).await;
        assert_eq!(status, 404, "{f}");
    }
}

/// Solo las páginas que enseñan secretos los llevan, y solo en el `value`
/// del campo (y en los datos de la página): nunca como texto a la vista. El
/// resto (Inicio, el Historial plegado, la Biblioteca sin abrir) no los
/// lleva, y nunca se guardan en claro.
#[tokio::test]
async fn secretos_solo_en_el_campo_de_las_paginas_que_los_muestran() {
    // El registro, antes de hacer nada: así recoge todo lo de este test.
    let registro = registro_global();
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    escribir_punto(&p, "1", &[("key_phrase", CLAVE_1)]).await;
    cambiar_clave_5(&p, &cookie, CLAVE_2).await;
    // Dos veces: la segunda sale de la caché de lecturas (sin enmascarar).
    ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    ok(&p, "leer_tema", json!({ "tema": "wifi" }), &cookie).await;
    let wifi = html(&p, &cookie, "/ajustes/wifi").await;
    // El ojo dice de qué campo es, y los gestores de contraseñas lo
    // ignoran.
    assert!(
        wifi.contains("aria-label=\"Mostrar Contraseña\""),
        "el nombre del ojo"
    );
    for marca in [
        "data-1p-ignore",
        "data-lpignore=\"true\"",
        "data-bwignore",
        "data-form-type=\"other\"",
    ] {
        assert!(wifi.contains(marca), "{marca}");
    }
    assert!(
        !wifi.contains("attr:data-"),
        "con su nombre, sin el prefijo"
    );
    assert!(
        wifi.contains(CLAVE_2),
        "la página de la WiFi lleva la clave"
    );
    let (fuera, _) = partir(&wifi, "<script", "</script>");
    let visible = texto_visible(&fuera);
    assert!(!visible.contains(CLAVE_2), "a la vista: {visible}");
    let en_value = format!("value=\"{CLAVE_2}\"");
    assert_eq!(
        fuera.matches(CLAVE_2).count(),
        fuera.matches(&en_value).count(),
        "solo en el value del campo"
    );
    for ruta in ["/", "/historial", "/explorador", "/ajustes/puertos"] {
        let pagina = html(&p, &cookie, ruta).await;
        assert!(!pagina.contains(CLAVE_1), "{ruta}: la clave vieja");
        assert!(!pagina.contains(CLAVE_2), "{ruta}: la nueva");
    }
    let claro: Vec<String> = sqlx::query_scalar(
        "SELECT CAST(valores_anteriores AS TEXT) || CAST(valores_nuevos AS TEXT) || solicitud || diff \
         FROM cambio_pasos",
    )
    .fetch_all(&p.state.db)
    .await
    .unwrap();
    assert!(
        claro
            .iter()
            .all(|c| !c.contains(CLAVE_1) && !c.contains(CLAVE_2)),
        "{claro:?}"
    );
    // Ni en el registro, que sí ha recogido lo que se ha hecho.
    let texto = registro.texto();
    assert!(
        texto.contains("cbSetWifiAccPt"),
        "el registro no recoge nada: {texto}"
    );
    assert!(
        !texto.contains(CLAVE_1) && !texto.contains(CLAVE_2),
        "{texto}"
    );
}

/// Una contraseña mala en el diálogo de reautenticación (la que se sigue
/// pidiendo para la contraseña del router y para exportar) cuenta para el
/// mismo freno que el login.
#[tokio::test]
async fn el_freno_es_el_mismo_que_el_del_login() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, _) = llamar(
        &p,
        "reautenticar",
        json!({ "contrasena": "mala" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 403);
    let resp = send(
        &p.app,
        crate::comun::post_form(
            "/api/login",
            &crate::comun::login_body(crate::comun::PANEL_USER, PANEL_PASS),
            "127.0.0.1",
        ),
    )
    .await;
    assert_eq!(resp.status(), 429);
}
