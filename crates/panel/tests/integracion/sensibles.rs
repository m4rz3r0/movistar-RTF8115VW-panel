//! Datos sensibles (usuarios, URL, números): en pantalla se ven en claro
//! (Biblioteca, confirmación de un cambio, Historial) y solo los secretos de
//! verdad van censurados con el ojo; en el registro (`tracing`) no aparece
//! ninguno de los dos.

use crate::ajustes_secretos::llamada;
use crate::comun::{
    Panel, PanelPrueba, elemento, esperar_final, fijar, ok, registro_global, valores_de,
};
use serde_json::{Value, json};

const MASCARA: &str = "••••••";
const URL: &str = "https://acs-sensible-4821.example/cwmp";
const USUARIO: &str = "usuario-sensible-4821";
const USUARIO_NUEVO: &str = "usuario-nuevo-4821";
const CLAVE: &str = "clave-secreta-4821";
const SIP: &str = "sip-sensible-4821";

/// Pone en el simulador la configuración de TR-069 y el usuario SIP.
async fn sembrar(p: &Panel) {
    p.state
        .hgu
        .write(
            "tr69.asp",
            "cbSetTr69Config",
            &[
                ("acs_URL", URL),
                ("acs_username", USUARIO),
                ("acs_password", CLAVE),
            ],
        )
        .await
        .unwrap();
    p.state
        .hgu
        .write(
            "voip.asp",
            "cbSetVoiceLine",
            &[("interwork_id", "0"), ("auth_username", SIP)],
        )
        .await
        .unwrap();
    let n = p.state.hgu.call("cbGetTr69Config", &[]).await.unwrap();
    assert_eq!(elemento(&n, "acs_username").as_deref(), Some(USUARIO));
}

/// El valor de la columna `col` de la primera fila del primer grupo.
fn celda(v: &Value, col: &str) -> Value {
    let g = &v["grupos"][0];
    let i = g["columnas"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c == col)
        .unwrap_or_else(|| panic!("falta la columna {col}: {v}"));
    g["filas"][0]["valores"][i].clone()
}

#[tokio::test]
async fn biblioteca_ensena_en_claro_los_sensibles_y_con_ojo_los_secretos() {
    let registro = registro_global();
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    sembrar(&p).await;

    // TR-069: la URL y el usuario, en claro; la contraseña, para el ojo.
    let v = ok(
        &p,
        "leer",
        json!({ "funcion": "cbGetTr69Config", "params": [] }),
        &cookie,
    )
    .await;
    assert_eq!(celda(&v, "acs_URL"), json!({ "Texto": URL }), "{v}");
    assert_eq!(celda(&v, "acs_username"), json!({ "Texto": USUARIO }));
    assert_eq!(celda(&v, "acs_password"), json!({ "Clave": CLAVE }));
    // El XML: los sensibles en claro y la contraseña censurada, salvo en el
    // que muestra «Mostrar claves en el XML».
    let crudo = v["crudo"].as_str().unwrap();
    assert!(crudo.contains(USUARIO) && crudo.contains(URL), "{crudo}");
    assert!(!crudo.contains(CLAVE) && crudo.contains(MASCARA), "{crudo}");
    let con_claves = v["crudo_con_claves"].as_str().unwrap();
    assert!(
        con_claves.contains(CLAVE) && !con_claves.contains(MASCARA),
        "{con_claves}"
    );

    // La línea de voz: el usuario SIP y los números, en claro; la
    // contraseña (que el router no deja leer), con el ojo.
    let v = ok(
        &p,
        "leer",
        json!({ "funcion": "cbGetVoiceLine", "params": [["interwork_id", "0"]] }),
        &cookie,
    )
    .await;
    assert_eq!(celda(&v, "auth_username"), json!({ "Texto": SIP }), "{v}");
    assert!(celda(&v, "directory_number").get("Texto").is_some(), "{v}");
    assert!(celda(&v, "auth_password").get("Clave").is_some(), "{v}");
    assert!(v["crudo"].as_str().unwrap().contains(SIP));

    // Una lectura sin secretos no trae el XML con claves.
    let v = ok(
        &p,
        "leer",
        json!({ "funcion": "cbGetSummary", "params": [] }),
        &cookie,
    )
    .await;
    assert!(v["crudo_con_claves"].is_null(), "{v}");

    let r = registro.texto();
    for dato in [URL, USUARIO, CLAVE, SIP] {
        assert!(!r.contains(dato), "{dato} en el registro");
    }
}

#[tokio::test]
async fn un_cambio_de_un_sensible_se_ve_en_claro_y_no_se_registra() {
    let registro = registro_global();
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    sembrar(&p).await;

    // El formulario: el usuario y la URL en claro; la contraseña, con su
    // valor para el ojo.
    let f = ok(
        &p,
        "formulario",
        json!({ "funcion": "cbSetTr69Config", "instancia": null, "claves": [] }),
        &cookie,
    )
    .await;
    let valor = |n: &str| {
        f["campos"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["param"]["nombre"] == n)
            .unwrap_or_else(|| panic!("falta {n}: {f}"))
            .clone()
    };
    assert_eq!(valor("acs_username")["valor"], USUARIO);
    assert_eq!(valor("acs_username")["param"]["tipo"], "Text");
    assert_eq!(valor("acs_URL")["valor"], URL);
    assert_eq!(valor("acs_password")["param"]["tipo"], "Secret");
    assert_eq!(valor("acs_password")["valor"], CLAVE);

    // La confirmación: el usuario de antes y el nuevo, en claro.
    let mut valores = valores_de(&f);
    fijar(&mut valores, "acs_username", USUARIO_NUEVO);
    let prep = ok(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbSetTr69Config", "instancia": null,
                              "valores": valores, "borrar": false } }),
        &cookie,
    )
    .await;
    assert_eq!(
        prep["campos"],
        json!([{ "nombre": "acs_username", "etiqueta": prep["campos"][0]["etiqueta"],
                 "antes": { "Texto": USUARIO }, "despues": { "Texto": USUARIO_NUEVO } }]),
        "{prep}"
    );
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
    let escrito = p.state.hgu.call("cbGetTr69Config", &[]).await.unwrap();
    assert_eq!(
        elemento(&escrito, "acs_username").as_deref(),
        Some(USUARIO_NUEVO)
    );
    assert_eq!(elemento(&escrito, "acs_password").as_deref(), Some(CLAVE));

    // La tabla de cambios no cambia: el usuario, enmascarado y cifrado.
    let diff: String = sqlx::query_scalar("SELECT diff FROM changes WHERE id = ?")
        .bind(id)
        .fetch_one(&p.state.db)
        .await
        .unwrap();
    assert!(
        !diff.contains(USUARIO) && !diff.contains(USUARIO_NUEVO),
        "{diff}"
    );
    assert!(diff.contains(MASCARA), "{diff}");

    // El Historial: en claro, sin ojo (no es un secreto).
    let h = ok(&p, "historial", json!({ "pagina": 1 }), &cookie).await;
    let campos = &h["entradas"][0]["Cambio"]["campos"];
    assert_eq!(campos[0]["antes"], json!({ "Texto": USUARIO }), "{h}");
    assert_eq!(
        campos[0]["despues"],
        json!({ "Texto": USUARIO_NUEVO }),
        "{h}"
    );
    let s = ok(&p, "secretos_del_cambio", json!({ "cambio": id }), &cookie).await;
    assert_eq!(s, json!([]), "solo los secretos de verdad van al ojo");

    // Las respuestas que los llevan, sin caché.
    let (status, cache, _) = llamada(&p, "historial", json!({ "pagina": 1 }), &cookie).await;
    assert_eq!((status, cache.as_str()), (200, "no-store"));

    let r = registro.texto();
    for dato in [URL, USUARIO, USUARIO_NUEVO, CLAVE] {
        assert!(!r.contains(dato), "{dato} en el registro");
    }
}
