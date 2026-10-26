//! Opciones DHCP de un pool (`cbDhcpV4SvrPoolSetOpt`) como las manda la
//! web oficial (`js/dhcp.js`): el código por debajo de 255 y, sin longitud,
//! la de los bytes del valor. El valor va en hexadecimal sin espacios (el
//! tipo `hex` del catálogo): para los códigos 224 a 254 la web convierte el
//! texto a hexadecimal; el panel lo pide ya convertido, que es lo que
//! guarda y devuelve el router (así la relectura lo puede comparar).

use super::super::plan::{Operacion, buscar, sin_espacios};
use catalogo::Funcion;
use std::collections::BTreeMap;

const FUNCION: &str = "cbDhcpV4SvrPoolSetOpt";
const CODIGO: &str = "opt_code";
const LONGITUD: &str = "opt_len";
const VALOR: &str = "opt_value";
/// El código más alto (la web exige menos de 255).
const CODIGO_MAX: i64 = 254;

/// Comprueba el código y completa la longitud vacía de una opción que se
/// crea o se edita (sobre la lectura `actual`). Otras funciones, sin tocar.
pub(in crate::changes) fn preparar(
    f: &Funcion,
    op: Operacion,
    actual: &BTreeMap<String, String>,
    valores: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    if f.name != FUNCION || !matches!(op, Operacion::Crear | Operacion::Editar) {
        return Ok(());
    }
    if let Some(c) = valores.get(CODIGO)
        && !c
            .trim()
            .parse::<i64>()
            .is_ok_and(|n| (0..=CODIGO_MAX).contains(&n))
    {
        return Err(format!(
            "{CODIGO}: «{c}» no es un código de opción (de 0 a {CODIGO_MAX})"
        ));
    }
    if valores.get(LONGITUD).is_some_and(|v| v.trim().is_empty()) {
        let valor = valores
            .get(VALOR)
            .map(String::as_str)
            .or_else(|| buscar(actual, VALOR).filter(|_| op == Operacion::Editar))
            .unwrap_or_default();
        let bytes = sin_espacios(valor).len() / 2;
        valores.insert(LONGITUD.to_owned(), bytes.to_string());
    }
    Ok(())
}
