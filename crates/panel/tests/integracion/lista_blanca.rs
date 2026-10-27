//! La 1.0 tal cual (sin `habilitar_todo`): lo no comprobado en el router real y las funciones desactivadas solo se ven, y el servidor no las escribe.

use crate::comun::{
    ConfirmarLigado, Panel, PanelPrueba, aplicar_ajuste, esperar_estado, esperar_final,
    esperar_libre_gestor, leer, llamar, ok, reautenticar, tiempos_rapidos,
};
use panel::changes::{ChangeManager, ChangeRequest, Estado};
use serde_json::{Value, json};
use std::sync::Arc;

/// Texto del 409 de lo que no está comprobado en el router real.
const SIN_COMPROBAR: &str = "no se ha comprobado en este router";

/// Panel con el gestor y todo como en la 1.0, y la cookie. UPnP
/// (`cbSetUpnpCfg`) y las reglas de puertos (`cbSetNatPortMapping`) están
/// comprobadas en el router real, pero aquí sirven de ejemplo fijo de lo no
/// comprobado.
async fn como_la_1_0() -> (Panel, String) {
    let (p, cookie) = PanelPrueba::nuevo()
        .config(|c| c.habilitar_todo = false)
        .sin_comprobar("cbSetUpnpCfg")
        .sin_comprobar("cbSetNatPortMapping")
        .arrancar_con_gestor()
        .await;
    reautenticar(&p, &cookie).await;
    (p, cookie)
}

fn elemento<'a>(pagina: &'a Value, id: &str) -> &'a Value {
    pagina["elementos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .unwrap_or_else(|| panic!("falta el elemento {id}: {pagina}"))
}

fn upnp(activo: bool) -> Value {
    json!({ "p": { "ajuste": "puertos.upnp", "instancia": null, "borrar": false,
                   "valores": { "activo": activo.to_string() } } })
}

/// UPnP (`cbSetUpnpCfg`, sin comprobar): ni desde Ajustes ni desde la
/// Biblioteca; 409 con su motivo y sin escribir.
#[tokio::test]
async fn lo_no_comprobado_no_se_escribe() {
    let (p, cookie) = como_la_1_0().await;
    let (status, body) = llamar(&p, "preparar_ajuste", upnp(true), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains(SIN_COMPROBAR), "{body}");
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetUpnpCfg", "instancia": null,
            "valores": [["adm_state", "1"]], "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains(SIN_COMPROBAR), "{body}");
    assert!(p.sim.control.writes().is_empty());
}

/// En la página, UPnP solo se ve: sin interruptor, sus campos sin editar y
/// la recomendación como información (sin botón ni lotes); las reglas de
/// puertos, sin acciones. Las comprobadas (la WiFi) siguen editables.
#[tokio::test]
async fn lo_no_comprobado_solo_se_ve() {
    let (p, cookie) = como_la_1_0().await;
    p.state
        .hgu
        .write("upnp.asp", "cbSetUpnpCfg", &[("adm_state", "1")])
        .await
        .unwrap();
    let pagina = leer(&p, &cookie, "puertos").await;
    let u = elemento(&pagina, "upnp");
    assert!(u["interruptor"].is_null(), "{u}");
    for c in u["ajustes"][0]["campos"].as_array().unwrap() {
        assert_eq!(c["solo_lectura"], "funcion-sin-comprobar", "{c}");
    }
    let r = &u["recomendacion"];
    assert_eq!(r["informativa"], true, "{r}");
    assert_eq!(r["boton"], "", "{r}");
    assert_eq!(r["lotes"], json!([]), "{r}");
    assert_eq!(elemento(&pagina, "reglas")["acciones"], json!([]));
    let wifi = leer(&p, &cookie, "wifi").await;
    let red = elemento(&wifi, "red-24");
    assert!(!red["interruptor"].is_null(), "{red}");
}

/// Mejoras no ofrece los pasos sin comprobar (UPnP), sí los demás.
#[tokio::test]
async fn mejoras_sin_lo_no_comprobado() {
    let (p, cookie) = como_la_1_0().await;
    let m = ok(&p, "leer_mejoras", json!({ "fresca": true }), &cookie).await;
    let claves: Vec<&str> = m["lotes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|l| l["pasos"].as_array().unwrap())
        .map(|p| p["clave"].as_str().unwrap())
        .collect();
    assert!(!claves.contains(&"upnp"), "{claves:?}");
    assert!(claves.contains(&"registro"), "{claves:?}");
}

/// Lo hecho antes con una función que ya no está comprobada (aquí, con un
/// gestor aparte que lo tiene todo habilitado) se sigue deshaciendo.
#[tokio::test]
async fn lo_hecho_se_deshace() {
    let (p, cookie) = como_la_1_0().await;
    let mut st = p.state.clone();
    let mut cat = (*st.catalogo).clone();
    assert!(cat.habilitar_todo_para_pruebas());
    st.catalogo = Arc::new(cat);
    let otro = ChangeManager::new(st, tiempos_rapidos());
    otro.reanudar_pendientes().await;
    let req = ChangeRequest {
        funcion: "cbSetUpnpCfg".into(),
        instancia: None,
        valores: [("adm_state".to_owned(), "1".to_owned())].into(),
        borrar: false,
        identidad: Default::default(),
    };
    let prep = otro.preparar(req).await.unwrap();
    let id = otro
        .confirmar_ligado(&prep.token, None, "admin")
        .await
        .unwrap();
    esperar_estado(&otro, id, Estado::Aplicado).await;
    esperar_libre_gestor(&otro).await;
    let d = ok(&p, "deshacer", json!({ "change_id": id }), &cookie).await;
    let deshecho = ok(
        &p,
        "confirmar_cambio",
        json!({ "token": d["token"], "palabra": null }),
        &cookie,
    )
    .await;
    let c = esperar_final(&p, &cookie, deshecho.as_i64().unwrap()).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
}

/// «Sin ping» y cambiar otra cuenta, desactivadas: la página no las ofrece
/// (sin acción propia, con su motivo en `desactivadas`) y el servidor
/// responde 409 sin escribir. Crear una cuenta sigue.
#[tokio::test]
async fn funciones_desactivadas_de_la_1_0() {
    let (p, cookie) = como_la_1_0().await;
    let pagina = leer(&p, &cookie, "seguridad").await;
    let cf = elemento(&pagina, "cortafuegos");
    assert!(cf["accion_propia"].is_null(), "{cf}");
    assert_eq!(cf["desactivadas"]["sin_ping"], true, "{cf}");
    let u = elemento(&pagina, "usuarios");
    assert_eq!(
        u["desactivadas"],
        json!({ "usuarios": false, "contrasena_otro": true, "sin_ping": true })
    );
    let (status, body) = llamar(&p, "preparar_sin_ping", json!({}), Some(&cookie)).await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("otra regla del operador"), "{body}");
    let (status, body) = llamar(
        &p,
        "cambiar_usuario_router",
        json!({ "usuario": "invitado1", "nueva": "clave-nueva-1", "grupo": "user" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("solo la cuenta 1234"), "{body}");
    assert!(p.sim.control.writes().is_empty());
    let g = ok(
        &p,
        "preparar_usuario_router",
        json!({ "op": { "Crear": { "usuario": "prueba1", "contrasena": "clave-prueba-1",
            "grupo": "user" } } }),
        &cookie,
    )
    .await;
    aplicar_ajuste(&p, &cookie, &g).await;
}
