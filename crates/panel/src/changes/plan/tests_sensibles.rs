//! Pruebas internas de los datos sensibles en el gestor: se guardan y se
//! registran como los secretos (cifrados, enmascarados), pero la pantalla
//! los ve en claro y, vacíos, se borran en vez de quedarse sin tocar.

use super::*;
use catalogo::Catalogo;

fn mapa(pares: &[(&str, &str)]) -> BTreeMap<String, String> {
    pares
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn sensible_en_claro_en_pantalla_y_enmascarado_al_guardar() {
    let c = Catalogo::load().unwrap();
    let m = Mascara::load().unwrap();
    let f = c.funcion("cbSetTr69Config").unwrap();
    let actual = mapa(&[
        ("acs_URL", "https://acs.example"),
        ("acs_username", "usuario-viejo"),
        ("acs_password", "clave-vieja"),
    ]);
    let valores = mapa(&[
        ("acs_URL", "https://otro-acs.example"),
        ("acs_username", "usuario-nuevo"),
        ("acs_password", "clave-nueva"),
    ]);
    let campos = componer(f, &m, Operacion::Editar, &actual, &valores, false).campos;
    assert_eq!(campos.len(), 3, "{campos:?}");
    // Para el gestor, los tres son secretos: van cifrados y enmascarados.
    assert!(campos.iter().all(|c| c.secreto), "{campos:?}");
    let guardado = visible(&campos);
    assert!(
        guardado
            .campos
            .iter()
            .all(|d| d.antes == MASCARA && d.despues == MASCARA && d.secreto && !d.sensible)
    );
    let r = resumen(&campos);
    for v in ["usuario-nuevo", "usuario-viejo", "otro-acs", "clave-nueva"] {
        assert!(!r.contains(v), "{r}");
    }
    // En pantalla, el usuario y la URL en claro; la contraseña, no.
    let d = para_mostrar(f, &m, &campos);
    let de = |n: &str| d.campos.iter().find(|c| c.nombre == n).unwrap();
    assert_eq!(
        (
            de("acs_username").antes.as_str(),
            de("acs_username").despues.as_str()
        ),
        ("usuario-viejo", "usuario-nuevo")
    );
    assert!(de("acs_username").sensible && !de("acs_username").secreto);
    assert_eq!(de("acs_URL").despues, "https://otro-acs.example");
    let clave = de("acs_password");
    assert!(clave.secreto && !clave.sensible);
    assert_eq!(
        (clave.antes.as_str(), clave.despues.as_str()),
        (MASCARA, MASCARA)
    );
    // Ni en un registro con `{:?}`.
    let txt = format!("{d:?}");
    assert!(
        !txt.contains("usuario-nuevo") && !txt.contains("clave-nueva"),
        "{txt}"
    );
}

#[test]
fn sensible_vacio_se_borra_y_credencial_vacia_no_se_toca() {
    let c = Catalogo::load().unwrap();
    let m = Mascara::load().unwrap();
    let f = c.funcion("cbSetTr69Config").unwrap();
    let actual = mapa(&[("acs_username", "usuario"), ("acs_password", "clave")]);
    let valores = mapa(&[("acs_username", ""), ("acs_password", "")]);
    let r = componer(f, &m, Operacion::Editar, &actual, &valores, false);
    let p: BTreeMap<_, _> = r.payload.into_iter().collect();
    assert_eq!(p["acs_username"], "", "el usuario se borra");
    assert_eq!(p["acs_password"], "clave", "la contraseña, sin tocar");
    assert_eq!(r.campos.len(), 1);
    assert_eq!(r.campos[0].nombre, "acs_username");
    assert!(r.campos[0].secreto);
}

/// Un dato sensible que el router da tapado (`****`) no se reenvía: ni si
/// vuelve tal cual del formulario (donde va en claro) ni si llega como
/// máscara (de una foto de partida). La escritura se bloquea como la de un
/// secreto tapado.
#[test]
fn sensible_tapado_no_se_reenvia() {
    let c = Catalogo::load().unwrap();
    let m = Mascara::load().unwrap();
    let f = c.funcion("cbSetTr69Config").unwrap();
    let actual = mapa(&[("acs_username", "****"), ("acs_password", "clave")]);
    let valores = mapa(&[
        ("acs_username", "****"),
        ("acs_password", MASCARA),
        ("acs_URL", "https://x.example"),
    ]);
    let r = componer(f, &m, Operacion::Editar, &actual, &valores, false);
    assert_eq!(r.secretos_tapados, ["acs_username"], "{:?}", r.payload);
    // Una máscara que no es lo leído (el router ya tiene otro usuario).
    let actual = mapa(&[("acs_username", "usuario-nuevo")]);
    let valores = mapa(&[("acs_username", "••••")]);
    let r = componer(f, &m, Operacion::Editar, &actual, &valores, false);
    assert_eq!(r.secretos_tapados, ["acs_username"]);
    // Un usuario de verdad, sí.
    let valores = mapa(&[("acs_username", "otro-usuario")]);
    let r = componer(f, &m, Operacion::Editar, &actual, &valores, false);
    assert!(r.secretos_tapados.is_empty());
}

/// El error de validación de un dato sensible no lleva su valor (va a la
/// respuesta y puede ir a un registro).
#[test]
fn error_de_un_sensible_sin_su_valor() {
    let c = Catalogo::load().unwrap();
    let f = c.funcion("cbSetTr69Config").unwrap();
    let p = f.parametro("acs_username").unwrap();
    let e = validar(p, "usuario-4821\u{9}x", true).unwrap_err();
    assert!(!e.contains("usuario-4821"), "{e}");
    assert!(e.contains("acs_username"), "{e}");
}
