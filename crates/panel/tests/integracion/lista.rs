//! Listas con columnas (Puertos y Red local): lo que trae la página para
//! pintarlas y las acciones sobre varias instancias marcadas (un grupo, un
//! paso por instancia, un solo deshacer).

use crate::comun::{Panel, PanelPrueba, aplicar_ajuste, llamar, ok};
use serde_json::{Value, json};

/// La página del tema `tema`, leída del router (sin la caché corta).
async fn pagina(p: &Panel, cookie: &str, tema: &str) -> Value {
    ok(
        p,
        "leer_tema",
        json!({ "tema": tema, "fresca": true }),
        cookie,
    )
    .await
}

/// El elemento `id` de la página.
fn elemento<'a>(pagina: &'a Value, id: &str) -> &'a Value {
    pagina["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .unwrap_or_else(|| panic!("falta el elemento {id}: {pagina}"))
}

/// Valor legible del campo `campo` de una instancia.
fn valor(i: &Value, campo: &str) -> String {
    let c = i["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == campo)
        .unwrap();
    let v = &c["valor"];
    v["Texto"]
        .as_str()
        .or(v["Opcion"].as_str())
        .map(str::to_owned)
        .or(v["Booleano"].as_bool().map(|b| b.to_string()))
        .unwrap_or_default()
}

/// Las reglas de puertos como `(id, nombre, puerto externo)`.
async fn reglas(p: &Panel, cookie: &str) -> Vec<(String, String, String)> {
    let pag = pagina(p, cookie, "puertos").await;
    elemento(&pag, "reglas")["ajustes"][0]["instancias"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["id"].as_str().unwrap().to_owned(),
                valor(i, "nombre"),
                valor(i, "externo_desde"),
            )
        })
        .collect()
}

/// `preparar_accion` en `tema`: estado y cuerpo.
async fn accion(
    p: &Panel,
    cookie: &str,
    tema: &str,
    accion: &str,
    elementos: &[&str],
) -> (u16, String) {
    llamar(
        p,
        "preparar_accion",
        json!({ "tema": tema, "accion": accion, "elementos": elementos }),
        Some(cookie),
    )
    .await
}

/// Abre el 9090 TCP hacia host-2 con el nombre `nombre` y lo aplica.
async fn abrir_regla(p: &Panel, cookie: &str, nombre: &str, puerto: &str) {
    let g = ok(
        p,
        "preparar_ajuste",
        json!({ "p": {
            "ajuste": "puertos.reglas",
            "instancia": null,
            "borrar": false,
            "valores": {
                "nombre": nombre,
                "protocolo": "tcp",
                "externo_desde": puerto,
                "externo_hasta": puerto,
                "destino": "192.168.1.62",
                "interno_desde": puerto,
                "interno_hasta": puerto,
                "activa": "true",
            },
        } }),
        cookie,
    )
    .await;
    aplicar_ajuste(p, cookie, &g).await;
}

/// Borrar dos reglas a la vez es un solo cambio con un paso por regla, que
/// dice que borra (botón de peligro); deshacerlo las vuelve a crear las dos.
#[tokio::test]
async fn accion_borrar_dos_reglas_un_deshacer() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    abrir_regla(&p, &cookie, "Consola", "9090").await;
    let antes = reglas(&p, &cookie).await;
    assert_eq!(antes.len(), 2, "{antes:?}");
    let ids: Vec<&str> = antes.iter().map(|(id, _, _)| id.as_str()).collect();

    let (status, body) = accion(&p, &cookie, "puertos", "borrar_regla", &ids).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(g["titulo"], "ajuste-accion-borrar_regla-titulo", "{g}");
    assert_eq!(
        g["borra"], true,
        "el diálogo pide confirmar con peligro: {g}"
    );
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(pasos.len(), 2, "un paso por regla: {g}");
    assert!(pasos.iter().all(|p| p["sin_cambios"] == false), "{g}");
    // Dice a qué reglas se refiere, sin nombres de la API.
    let detalle = g["detalle"].as_str().unwrap_or_default();
    assert!(
        detalle.contains("desc-1") && detalle.contains("Consola"),
        "{g}"
    );
    assert!(!g.to_string().contains("cbSet"), "{g}");
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    assert!(reglas(&p, &cookie).await.is_empty());

    // Un solo deshacer las recrea las dos, con sus datos.
    let inverso = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    assert_eq!(inverso["pasos"].as_array().unwrap().len(), 2, "{inverso}");
    aplicar_ajuste(&p, &cookie, &inverso).await;
    let mut despues: Vec<(String, String)> = reglas(&p, &cookie)
        .await
        .into_iter()
        .map(|(_, n, puerto)| (n, puerto))
        .collect();
    let mut esperadas: Vec<(String, String)> = antes
        .into_iter()
        .map(|(_, n, puerto)| (n, puerto))
        .collect();
    despues.sort();
    esperadas.sort();
    assert_eq!(despues, esperadas);
}

/// Desactivar dos IP fijas: un paso por reserva; la que ya estaba
/// desactivada sale como «ya estaba así» y no se escribe. Una instancia que
/// no existe, o una acción de otra lista, no vale (400) y no escribe nada.
#[tokio::test]
async fn accion_sobre_las_ip_fijas() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let pag = pagina(&p, &cookie, "red_local").await;
    let reservas = elemento(&pag, "reservas");
    let acciones: Vec<&str> = reservas["acciones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert_eq!(
        acciones,
        ["activar_reserva", "desactivar_reserva", "borrar_reserva"]
    );
    let instancias = reservas["ajustes"][0]["instancias"].as_array().unwrap();
    assert!(instancias.len() >= 2, "{reservas}");
    let ids: Vec<&str> = instancias
        .iter()
        .take(2)
        .map(|i| i["id"].as_str().unwrap())
        .collect();

    let (status, body) = accion(&p, &cookie, "red_local", "desactivar_reserva", &ids[..1]).await;
    assert_eq!(status, 200, "{body}");
    aplicar_ajuste(&p, &cookie, &serde_json::from_str(&body).unwrap()).await;
    let antes = p.sim.control.writes().len();
    let (status, body) = accion(&p, &cookie, "red_local", "desactivar_reserva", &ids).await;
    assert_eq!(status, 200, "{body}");
    let g: Value = serde_json::from_str(&body).unwrap();
    let pasos = g["pasos"].as_array().unwrap();
    assert_eq!(pasos.len(), 2, "{g}");
    assert_eq!(pasos[0]["sin_cambios"], true, "ya estaba desactivada: {g}");
    assert_eq!(pasos[1]["sin_cambios"], false, "{g}");
    assert_eq!(g["borra"], false, "{g}");
    assert_eq!(p.sim.control.writes().len(), antes, "preparar no escribe");

    for (tema, a, ids) in [
        ("red_local", "desactivar_reserva", vec!["no-existe"]),
        ("red_local", "borrar_regla", vec![ids[0]]),
        ("puertos", "borrar_regla", vec!["dhcp"]),
    ] {
        let (status, body) = accion(&p, &cookie, tema, a, &ids).await;
        assert_eq!(status, 400, "{tema} {a} {ids:?}: {body}");
        assert!(!body.contains("cbSet"), "{body}");
    }
}

/// La página de una lista trae sus columnas, y la de Puertos y Red local,
/// los dispositivos y la red de casa (para nombrar el destino de cada regla
/// y validar antes de enviar) en la misma lectura.
#[tokio::test]
async fn pagina_de_una_lista() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let pag = pagina(&p, &cookie, "puertos").await;
    let reglas = &elemento(&pag, "reglas")["ajustes"][0];
    assert_eq!(
        reglas["columnas"],
        json!(["protocolo", "externo", "destino", "activa"])
    );
    assert!(elemento(&pag, "upnp")["interruptor"].is_object(), "{pag}");
    let red = &pag["red"];
    assert!(
        red["dispositivos"]
            .as_array()
            .is_some_and(|d| d.iter().any(|d| d["ip"] == "192.168.1.62")),
        "{red}"
    );
    assert_eq!(red["lan"], json!(["192.168.1.1", "255.255.255.0"]), "{red}");
    assert!(red["ips_fijas"].is_array(), "{red}");

    let pag = pagina(&p, &cookie, "red_local").await;
    assert_eq!(
        elemento(&pag, "dispositivos")["ajustes"][0]["columnas"],
        json!(["ip", "mac", "conexion", "ip_fija"])
    );
    assert!(pag["red"]["dispositivos"].is_array(), "{pag}");
    // En los temas sin listas, nada de la red.
    let pag = pagina(&p, &cookie, "wifi").await;
    assert!(pag["red"].is_null(), "{}", pag["red"]);
}
