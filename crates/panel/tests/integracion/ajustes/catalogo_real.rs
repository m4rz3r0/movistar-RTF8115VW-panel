//! Comprobaciones del catálogo real: claves de la vista y plantillas de listas.

use super::*;

/// Toda clave de traducción que da la vista existe en los dos idiomas.
#[tokio::test]
async fn claves_de_la_vista_existen() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let mut claves = Vec::new();
    let de_campo = |c: &Value, claves: &mut Vec<String>| {
        let k = c["clave"].as_str().unwrap();
        claves.push(k.to_owned());
        claves.push(format!("{k}-desc"));
        for o in c["opciones"].as_array().unwrap() {
            let op = o["clave"].as_str().unwrap();
            claves.push(format!("{k}-{op}"));
            claves.push(format!("{k}-{op}-desc"));
            claves.extend(o["aviso"].as_str().map(str::to_owned));
        }
        claves.extend(c["solo_lectura"].as_str().map(str::to_owned));
        claves.extend(c["motivo_recomendado"].as_str().map(str::to_owned));
    };
    for tema in [
        "internet",
        "wifi",
        "red_local",
        "puertos",
        "seguridad",
        "telefono",
        "sistema",
    ] {
        let t = leer(&p, &cookie, tema).await;
        assert!(!crate::comun::ajustes_de(&t).is_empty(), "{tema}");
        for a in crate::comun::ajustes_de(&t) {
            assert_eq!(a["estado"], "Leido", "{a}");
            for sufijo in ["titulo", "desc", "ayuda"] {
                claves.push(format!("{}-{sufijo}", a["clave"].as_str().unwrap()));
            }
            claves.extend(a["efecto"].as_str().map(str::to_owned));
            for c in a["campos"].as_array().unwrap() {
                de_campo(c, &mut claves);
            }
            for i in a["instancias"].as_array().unwrap() {
                for c in i["campos"].as_array().unwrap() {
                    de_campo(c, &mut claves);
                }
            }
        }
    }
    for idioma in i18n::Idioma::TODOS {
        let faltan: Vec<&String> = claves.iter().filter(|k| !i18n::existe(idioma, k)).collect();
        assert!(faltan.is_empty(), "faltan en {idioma:?}: {faltan:#?}");
    }
}

/// La plantilla de una lista nunca dice «valor no reconocido»: una opción
/// empieza en la recomendada o la primera, un sí/no en el recomendado o en
/// no, y el resto vacío.
#[tokio::test]
async fn plantilla_de_lista_sin_desconocidos() {
    let p = PanelPrueba::nuevo().arrancar().await;
    let cookie = sesion(&p).await;
    let tema = leer(&p, &cookie, "puertos").await;
    let reglas = ajuste(&tema, "puertos.reglas");
    assert_eq!(
        campo(reglas, "protocolo")["valor"],
        json!({ "Opcion": "tcp" })
    );
    // Una regla nueva nace activa (su recomendado).
    assert_eq!(
        campo(reglas, "activa")["valor"],
        json!({ "Booleano": true })
    );
    assert_eq!(campo(reglas, "nombre")["valor"], json!({ "Texto": "" }));
    assert_eq!(campo(reglas, "destino")["valor"], json!({ "Texto": "" }));
    for tema in [
        "internet",
        "wifi",
        "red_local",
        "puertos",
        "seguridad",
        "telefono",
        "sistema",
    ] {
        let t = leer(&p, &cookie, tema).await;
        for a in crate::comun::ajustes_de(&t) {
            if a["lista"] != true {
                continue;
            }
            for c in a["campos"].as_array().unwrap() {
                assert!(c["valor"].get("Desconocido").is_none(), "{}: {c}", a["id"]);
            }
        }
    }
}

/// Un catálogo de ajustes que no cuadra con el de funciones no deja
/// arrancar el panel, y cada fallo se dice.
#[test]
fn catalogo_roto_no_arranca() {
    let roto = ajustes::CatalogoAjustes::from_toml(
        &[(
            "wifi",
            r#"
[[ajuste]]
id = "wifi.prueba"
tema = "wifi"
nivel = "basico"
leer = { funcion = "cbNoExiste" }
"#,
        )],
        "",
    )
    .unwrap();
    let f = catalogo::Catalogo::load().unwrap();
    let m = catalogo::mask::Mascara::load().unwrap();
    let e = panel::state::comprobar_ajustes(roto, &f, &m).unwrap_err();
    assert!(e.contains("wifi.prueba"), "{e}");
    let real = ajustes::CatalogoAjustes::load().unwrap();
    assert!(panel::state::comprobar_ajustes(real, &f, &m).is_ok());
}
