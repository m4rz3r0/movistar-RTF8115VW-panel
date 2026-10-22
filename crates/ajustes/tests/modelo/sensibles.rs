//! Datos sensibles (usuarios, URL, números): un campo `texto`, que se ve en
//! claro; solo las credenciales van en un campo `secreto`. En la web
//! básica, el gestor los bloquea como a los secretos: solo lectura.

use super::*;

/// [`FUNCIONES`] con un usuario en `cbSetX` (web avanzada) y otro en
/// `te_x.wl` (web básica).
fn con_usuarios() -> Catalogo {
    let f = FUNCIONES
        .replace(
            "[[funcion.parametro]]\nnombre = \"max_clientes\"",
            "[[funcion.parametro]]\nnombre = \"x_username\"\netiqueta = \"Usuario\"\ntipo = \"text\"\n\n\
             [[funcion.parametro]]\nnombre = \"max_clientes\"",
        )
        .replace(
            "[[funcion.parametro]]\nnombre = \"wlSsid\"",
            "[[funcion.parametro]]\nnombre = \"wlUsername\"\netiqueta = \"Usuario\"\ntipo = \"text\"\n\n\
             [[funcion.parametro]]\nnombre = \"wlSsid\"",
        );
    Catalogo::from_toml(&f).expect("catálogo sintético válido")
}

fn fallos_con(ajustes: &str) -> Vec<String> {
    comprobar(
        &cargar(&[("wifi", ajustes)], ""),
        &con_usuarios(),
        &mascara(),
    )
}

const USUARIO: &str = r#"
[[ajuste]]
id = "wifi.cuenta"
tema = "wifi"
nivel = "avanzado"
leer = { funcion = "cbGetX" }

[[ajuste.campo]]
id = "usuario"
escribe = { funcion = "cbSetX", parametro = "x_username" }
lee = "x_username"
tipo = "texto"
"#;

#[test]
fn un_usuario_es_texto_y_no_secreto() {
    assert_eq!(fallos_con(USUARIO), Vec::<String>::new());
    let secreto = USUARIO.replace("tipo = \"texto\"", "tipo = \"secreto\"");
    falla_con(
        &fallos_con(&secreto),
        "wifi.cuenta.usuario",
        "no es un parámetro secreto",
    );
}

#[test]
fn un_usuario_de_la_web_basica_es_de_solo_lectura() {
    let basica = USUARIO.replace(
        "{ funcion = \"cbSetX\", parametro = \"x_username\" }",
        "{ funcion = \"te_x.wl\", parametro = \"wlUsername\" }",
    );
    falla_con(
        &fallos_con(&basica),
        "wifi.cuenta.usuario",
        "necesita solo_lectura",
    );
    let solo = basica.replace(
        "tipo = \"texto\"\n",
        "tipo = \"texto\"\nsolo_lectura = \"ajustes-solo-lectura-basica\"\n",
    );
    assert_eq!(fallos_con(&solo), Vec::<String>::new());
}
