//! Ayudas comunes: el catálogo de funciones y los ajustes sintéticos, y cómo cargarlos.

use super::*;

/// Catálogo de funciones mínimo: una lectura, una escritura de la web
/// avanzada y otra de la básica.
pub(crate) const FUNCIONES: &str = r#"
[[funcion]]
name = "cbGetX"
area = "wifi"
origen = "avanzada"
tipo = "lectura"
descripcion = "Lee X"
riesgo = "bajo"

[[funcion.parametro]]
nombre = "x_id"
etiqueta = "Instancia"
tipo = "int"

[[funcion.parametro]]
nombre = "x_psk"
etiqueta = "Clave"
tipo = "text"

[[funcion]]
name = "cbSetX"
area = "wifi"
origen = "avanzada"
tipo = "escritura"
pagina = "x.asp"
descripcion = "Escribe X"
riesgo = "medio"
lectura_pareada = { funcion = "cbGetX" }

[[funcion.parametro]]
nombre = "x_id"
etiqueta = "Instancia"
tipo = "int"

[[funcion.parametro]]
nombre = "nombre"
etiqueta = "Nombre"
tipo = "text"

[[funcion.parametro]]
nombre = "seguridad"
etiqueta = "Seguridad"
tipo = "enum"
opciones = [["11", "WPA2/WPA3"], ["3", "WPA2"], ["0", "Abierta"]]

[[funcion.parametro]]
nombre = "cifrado"
etiqueta = "Cifrado"
tipo = "enum"
opciones = [["4", "AES"], ["2", "TKIP"], ["0", "Ninguno"]]

[[funcion.parametro]]
nombre = "oculta"
etiqueta = "Oculta"
tipo = "bool"

[[funcion.parametro]]
nombre = "key_phrase"
etiqueta = "Clave"
tipo = "secret"

[[funcion.parametro]]
nombre = "wlPassphrase"
etiqueta = "Clave sin marcar"
tipo = "text"

[[funcion.parametro]]
nombre = "max_clientes"
etiqueta = "Máximo"
tipo = "int"

[[funcion.parametro]]
nombre = "puerta"
etiqueta = "Puerta"
tipo = "ip"

[[funcion.parametro]]
nombre = "mac"
etiqueta = "MAC"
tipo = "mac"

[[funcion]]
name = "te_x.wl"
area = "wifi"
origen = "basica"
tipo = "escritura"
pagina = "te_x.asp"
descripcion = "Escribe X desde la web básica"
riesgo = "alto"
lectura_pareada = { funcion = "cbGetX" }

[[funcion.parametro]]
nombre = "wlWpaPsk"
etiqueta = "Clave"
tipo = "secret"

[[funcion.parametro]]
nombre = "wlSsid"
etiqueta = "Nombre"
tipo = "text"
"#;

pub(crate) const SECRETOS: &str = r#"
fragmentos = ["key", "psk", "passphrase"]
exactos = []
sufijos_numericos = ["_id", "_type"]
"#;

/// Ajustes válidos contra [`FUNCIONES`].
pub(crate) const WIFI: &str = r#"
[[ajuste]]
id = "wifi.red"
tema = "wifi"
nivel = "basico"
efecto = "wifi_reinicia"
leer = { funcion = "cbGetX", params = { x_id = "1" } }
fijos = { x_id = "1" }

[[ajuste.campo]]
id = "nombre"
escribe = { funcion = "cbSetX", parametro = "nombre" }
lee = "nombre"
tipo = "texto"
validar = { min = 1, max = 32 }

[[ajuste.campo]]
id = "seguridad"
escribe = { funcion = "cbSetX", parametro = "seguridad" }
lee = "seguridad"
tipo = "opcion"
opciones = [
  { valor = "11", clave = "wpa2_wpa3", recomendada = true },
  { valor = "3", clave = "wpa2" },
  { valor = "0", clave = "abierta", aviso = "inseguro" },
]

[[ajuste.campo]]
id = "visible"
escribe = { funcion = "cbSetX", parametro = "oculta" }
lee = "oculta"
tipo = "booleano"
invertido = true
recomendado = "true"

[[ajuste.campo]]
id = "clave"
escribe = { funcion = "cbSetX", parametro = "key_phrase" }
lee = "key_phrase"
tipo = "secreto"
validar = { min = 8, max = 63 }

[[ajuste.campo]]
id = "clave_basica"
escribe = { funcion = "te_x.wl", parametro = "wlWpaPsk" }
lee = "key_phrase"
tipo = "secreto"
solo_lectura = "ajustes-solo-lectura-basica"

[[ajuste]]
id = "wifi.avanzado"
tema = "wifi"
nivel = "avanzado"
leer = { funcion = "cbGetX" }

[[ajuste.campo]]
id = "maximo"
escribe = { funcion = "cbSetX", parametro = "max_clientes" }
lee = "max_clientes"
tipo = "numero"
validar = { min = 1, max = 64 }
"#;

pub(crate) const RED_LOCAL: &str = r#"
[[ajuste]]
id = "red_local.direcciones"
tema = "red_local"
nivel = "oculto"
componente = "reservas"
leer = { funcion = "cbGetX" }

[[ajuste.campo]]
id = "puerta"
escribe = { funcion = "cbSetX", parametro = "puerta" }
lee = "puerta"
tipo = "ip"

[[ajuste.campo]]
id = "mac"
escribe = { funcion = "cbSetX", parametro = "mac" }
lee = "mac"
tipo = "mac"
"#;

pub(crate) const EXCLUIDAS: &str = r#"
[[excluida]]
funcion = "te_x.wl"
motivo = "duplicado de la web básica de wifi.red"
"#;

pub(crate) fn funciones() -> Catalogo {
    Catalogo::from_toml(FUNCIONES).expect("catálogo sintético válido")
}

pub(crate) fn mascara() -> Mascara {
    Mascara::from_toml(SECRETOS).expect("secretos sintéticos válidos")
}

pub(crate) fn cargar(ficheros: &[(&str, &str)], excluidas: &str) -> CatalogoAjustes {
    CatalogoAjustes::from_toml(ficheros, excluidas).expect("ajustes sintéticos legibles")
}

/// Fallos de `comprobar` para un único fichero `wifi.toml`.
pub(crate) fn fallos(wifi: &str, excluidas: &str) -> Vec<String> {
    comprobar(
        &cargar(&[("wifi", wifi)], excluidas),
        &funciones(),
        &mascara(),
    )
}

/// `fallos` no está vacío y alguno menciona `id` y `texto`.
pub(crate) fn falla_con(fallos: &[String], id: &str, texto: &str) {
    assert!(
        fallos.iter().any(|f| f.contains(id) && f.contains(texto)),
        "se esperaba un fallo de {id} con «{texto}»: {fallos:#?}"
    );
}
