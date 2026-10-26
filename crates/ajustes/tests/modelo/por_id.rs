//! La lectura por posición frente a la escritura por id: `buscar`.

use super::*;

/// Una lista que se lee por posición (`p_0i`) y se escribe por id (`p_id`),
/// como el pool DHCP del router.
const FUNCIONES_POOL: &str = r#"
[[funcion]]
name = "cbGetP"
area = "lan_dhcp"
origen = "avanzada"
tipo = "lectura"
descripcion = "Lee un pool"
riesgo = "bajo"
lista = { id_param = "p_0i", all = false, del = false }

[[funcion.parametro]]
nombre = "p_0i"
etiqueta = "Pool"
tipo = "int"

[[funcion]]
name = "cbSetP"
area = "lan_dhcp"
origen = "avanzada"
tipo = "escritura"
pagina = "p.asp"
descripcion = "Escribe un pool"
riesgo = "medio"
acceso = "no_corta"
lectura_pareada = { funcion = "cbGetP", params = [["p_0i", "{p_id}"]] }
lista = { id_param = "p_id", all = false, del = false }

[[funcion.parametro]]
nombre = "p_id"
etiqueta = "Pool"
tipo = "int"

[[funcion.parametro]]
nombre = "inicio"
etiqueta = "Inicio"
tipo = "ip"
"#;

/// El ajuste del pool, con su `leer` y sus `fijos`.
fn pool(leer: &str, fijos: &str) -> String {
    format!(
        "[[ajuste]]\nid = \"red_local.pool\"\ntema = \"red_local\"\nnivel = \"avanzado\"\n\
         leer = {leer}\nfijos = {fijos}\n\n[[ajuste.campo]]\nid = \"inicio\"\n\
         escribe = {{ funcion = \"cbSetP\", parametro = \"inicio\" }}\nlee = \"inicio\"\ntipo = \"ip\"\n"
    )
}

fn fallos_pool(toml: &str) -> Vec<String> {
    comprobar(
        &cargar(&[("red_local", toml)], ""),
        &Catalogo::from_toml(FUNCIONES_POOL).unwrap(),
        &mascara(),
    )
}

#[test]
fn leer_por_posicion_y_escribir_por_id_exige_buscar() {
    let fijos = "{ p_id = \"0\" }";
    falla_con(
        &fallos_pool(&pool(
            "{ funcion = \"cbGetP\", params = { p_0i = \"0\" } }",
            fijos,
        )),
        "red_local.pool",
        "buscar = \"p_0i\"",
    );
    let bien = fallos_pool(&pool("{ funcion = \"cbGetP\", buscar = \"p_0i\" }", fijos));
    assert!(
        !bien.iter().any(|f| f.contains("red_local.pool")),
        "{bien:#?}"
    );
}

#[test]
fn buscar_mal_declarado() {
    let fijos = "{ p_id = \"0\" }";
    // No es el parámetro de posición de la lista.
    falla_con(
        &fallos_pool(&pool("{ funcion = \"cbGetP\", buscar = \"q_0i\" }", fijos)),
        "red_local.pool",
        "q_0i no es el parámetro de posición",
    );
    // También en los parámetros.
    falla_con(
        &fallos_pool(&pool(
            "{ funcion = \"cbGetP\", params = { p_0i = \"0\" }, buscar = \"p_0i\" }",
            fijos,
        )),
        "red_local.pool",
        "p_0i va en buscar",
    );
    // Sin el id en los fijos.
    let sin_id = pool("{ funcion = \"cbGetP\", buscar = \"p_0i\" }", "{ }");
    falla_con(&fallos_pool(&sin_id), "red_local.pool", "sin su id");
}
