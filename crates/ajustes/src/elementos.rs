//! Reglas de los elementos y las acciones de cada pestaña (spec §5.1 y §5.5).

use std::collections::{HashMap, HashSet};

use crate::carga::CatalogoAjustes;
use crate::modelo::{Accion, Ajuste, Campo, Componente, Elemento, TipoCampo};
use crate::valores::traducir;

/// Datos calculados que puede pedir un elemento (`derivado:<nombre>` en
/// `datos`): el panel los resuelve en `ajustes_router::servidor::derivados`.
pub const DERIVADOS_DATO: &[&str] = &["canal_y_ancho", "conectados", "nombres_ddns", "cuentas"];

/// Reglas de estado que puede pedir un elemento (`derivado:<nombre>` en
/// `estado`).
pub const DERIVADOS_ESTADO: &[&str] = &[
    "estado_red",
    "estado_dns",
    "estado_ddns",
    "estado_gestion_remota",
    "estado_linea",
    "estado_voz",
    "estado_paso_directo",
    "estado_hora",
    "estado_diagnostico",
    "segun_lo_recomendado",
];

/// Prefijo de las referencias a un dato o una regla calculados.
pub const PREFIJO_DERIVADO: &str = "derivado:";

/// Como mucho, datos en la tarjeta de un elemento.
pub const MAX_DATOS: usize = 6;

/// Clave de traducción del papel de un elemento.
pub fn clave_papel(papel: &str) -> String {
    format!("ajustes-papel-{papel}")
}

/// Clave de traducción del botón de una acción.
pub fn clave_accion(id: &str) -> String {
    format!("ajustes-accion-{id}")
}

/// Clave de traducción del título del grupo de cambios de una acción (en el
/// resumen y en el Historial).
pub fn titulo_accion(id: &str) -> String {
    format!("ajuste-accion-{id}-titulo")
}

/// Claves de traducción que piden los elementos y las acciones (papeles,
/// botones y títulos de sus cambios), sin repetir.
pub fn claves_de_elementos(c: &CatalogoAjustes) -> Vec<String> {
    let mut claves: Vec<String> = c
        .elementos()
        .iter()
        .filter_map(|e| e.papel.as_deref().map(clave_papel))
        .chain(c.acciones().iter().map(|a| clave_accion(&a.id)))
        .chain(c.acciones().iter().map(|a| titulo_accion(&a.id)))
        .collect();
    claves.sort();
    claves.dedup();
    claves
}

/// Separa `<ajuste>.<campo>` (el ajuste ya lleva su propio punto).
pub fn partir_referencia(r: &str) -> Option<(&str, &str)> {
    r.rsplit_once('.')
}

/// Id de elemento válido en una URL: minúsculas, cifras y `-`.
fn es_id_de_url(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Identificador válido como trozo de una clave de traducción.
fn es_nombre(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Fallos de los elementos y las acciones del catálogo, uno legible por
/// regla incumplida con el id del elemento o de la acción.
pub(crate) fn comprobar_elementos(c: &CatalogoAjustes, fallos: &mut Vec<String>) {
    let mut ids = HashSet::new();
    let mut duenos: HashMap<&str, &str> = HashMap::new();
    for e in c.elementos() {
        if !ids.insert((e.tema, e.id.as_str())) {
            fallos.push(format!("elemento {}: id repetido", e.id));
        }
        comprobar_elemento(c, e, &mut duenos, fallos);
    }
    let mut ids = HashSet::new();
    for a in c.acciones() {
        if !ids.insert((a.tema, a.id.as_str())) {
            fallos.push(format!("acción {}: id repetido", a.id));
        }
        comprobar_accion(c, a, fallos);
    }
    for a in c.todos() {
        comprobar_columnas(a, fallos);
    }
}

/// ¿Existe la columna `col` en la lista `a`? Un campo suyo o un rango
/// (`<col>_desde` y `<col>_hasta`).
fn existe_columna(a: &Ajuste, col: &str) -> bool {
    a.campo(col).is_some()
        || (a.campo(&format!("{col}_desde")).is_some()
            && a.campo(&format!("{col}_hasta")).is_some())
}

/// Las columnas de un ajuste: solo en una lista (o en los dispositivos, con
/// las suyas), sin repetir y que existan.
fn comprobar_columnas(a: &Ajuste, fallos: &mut Vec<String>) {
    if a.columnas.is_empty() {
        return;
    }
    let mut mal = |msg: String| fallos.push(format!("{}: {msg}", a.id));
    let dispositivos = a.componente == Some(Componente::Dispositivos);
    if !a.lista && !dispositivos {
        mal("solo una lista lleva `columnas`".into());
        return;
    }
    let mut vistas = HashSet::new();
    for col in &a.columnas {
        if !vistas.insert(col.as_str()) {
            mal(format!("columna {col} repetida"));
        }
        let existe = if dispositivos {
            crate::COLUMNAS_DISPOSITIVOS.contains(&col.as_str())
        } else {
            existe_columna(a, col)
        };
        if !existe {
            mal(format!("la columna {col} no existe"));
        }
    }
}

fn comprobar_elemento<'c>(
    c: &'c CatalogoAjustes,
    e: &'c Elemento,
    duenos: &mut HashMap<&'c str, &'c str>,
    fallos: &mut Vec<String>,
) {
    let mut mal = |msg: String| fallos.push(format!("elemento {}: {msg}", e.id));
    if !es_id_de_url(&e.id) {
        mal("el id solo admite minúsculas, cifras y -".into());
    }
    if e.ajustes.is_empty() {
        mal("sin ajustes".into());
    }
    if e.papel.as_deref().is_some_and(|p| !es_nombre(p)) {
        mal("el papel solo admite minúsculas, cifras y _".into());
    }
    for id in &e.ajustes {
        match c.ajuste(id) {
            None => mal(format!("el ajuste {id} no existe")),
            Some(a) if a.tema != e.tema => {
                mal(format!("el ajuste {id} es de otro tema"));
            }
            Some(_) => {}
        }
        if let Some(otro) = duenos.insert(id.as_str(), e.id.as_str()) {
            mal(format!("el ajuste {id} ya está en el elemento {otro}"));
        }
    }
    if let Some(r) = &e.interruptor {
        match campo_del_elemento(c, e, r) {
            Err(m) => mal(format!("interruptor {r}: {m}")),
            Ok(campo) if campo.tipo != TipoCampo::Booleano || campo.escribible().is_none() => {
                mal(format!(
                    "interruptor {r}: tiene que ser un campo booleano que se pueda escribir"
                ));
            }
            Ok(_) => {}
        }
    }
    if e.datos.len() > MAX_DATOS {
        mal(format!("{} datos: como mucho {MAX_DATOS}", e.datos.len()));
    }
    for d in &e.datos {
        match d.strip_prefix(PREFIJO_DERIVADO) {
            Some(n) if !DERIVADOS_DATO.contains(&n) => {
                mal(format!("el dato derivado {n} no existe"));
            }
            Some(_) => {}
            None => {
                if let Err(m) = campo_del_elemento(c, e, d) {
                    mal(format!("dato {d}: {m}"));
                }
            }
        }
    }
    if let Some(i) = e.icono.as_deref().filter(|i| !crate::ICONOS.contains(i)) {
        mal(format!("el icono {i} no existe"));
    }
    if let Some(r) = &e.estado {
        match r.strip_prefix(PREFIJO_DERIVADO) {
            Some(n) if DERIVADOS_ESTADO.contains(&n) => {}
            Some(n) => mal(format!("la regla de estado {n} no existe")),
            None => mal(format!("estado {r}: tiene que ser derivado:<nombre>")),
        }
    }
}

/// Campo `<ajuste>.<campo>` de uno de los ajustes de `e`.
fn campo_del_elemento<'c>(
    c: &'c CatalogoAjustes,
    e: &Elemento,
    r: &str,
) -> Result<&'c Campo, String> {
    let (ajuste, campo) = partir_referencia(r).ok_or("no tiene la forma <ajuste>.<campo>")?;
    if !e.ajustes.iter().any(|a| a == ajuste) {
        return Err(format!("el ajuste {ajuste} no es del elemento"));
    }
    c.ajuste(ajuste)
        .ok_or_else(|| format!("el ajuste {ajuste} no existe"))?
        .campo(campo)
        .ok_or_else(|| format!("el campo {campo} no existe"))
}

/// Campos que una acción cambia: el campo `campo` de cada ajuste de los
/// elementos de su tema que lo tiene.
fn campos_de_la_accion<'c>(
    c: &'c CatalogoAjustes,
    a: &Accion,
    campo: &str,
) -> Vec<(&'c Ajuste, &'c Campo)> {
    c.elementos_de(a.tema)
        .flat_map(|e| e.ajustes.iter())
        .filter_map(|id| c.ajuste(id))
        .filter_map(|aj| aj.campo(campo).map(|x| (aj, x)))
        .collect()
}

fn comprobar_accion(c: &CatalogoAjustes, a: &Accion, fallos: &mut Vec<String>) {
    let mut mal = |msg: String| fallos.push(format!("acción {}: {msg}", a.id));
    if !es_nombre(&a.id) {
        mal("el id solo admite minúsculas, cifras y _".into());
    }
    if a.borrar {
        if a.lista.is_none() {
            mal("borrar necesita `lista`".into());
        }
        if a.campo.is_some() || a.valor.is_some() {
            mal("borrar va sin `campo` ni `valor`".into());
        }
        if !a.peligro {
            mal("borrar lleva el botón de peligro (`peligro = true`)".into());
        }
    }
    let lista = match a.lista.as_deref().map(|id| (id, c.ajuste(id))) {
        None => None,
        Some((id, None)) => return mal(format!("la lista {id} no existe")),
        Some((id, Some(l))) if !l.lista || l.tema != a.tema => {
            return mal(format!("{id} no es una lista de su tema"));
        }
        Some((_, Some(l))) => Some(l),
    };
    if a.borrar {
        return;
    }
    let (Some(campo), Some(valor)) = (&a.campo, &a.valor) else {
        mal("necesita `campo` y `valor`".into());
        return;
    };
    let campos = match lista {
        Some(l) => l.campo(campo).map(|x| (l, x)).into_iter().collect(),
        None => campos_de_la_accion(c, a, campo),
    };
    if !campos.iter().any(|(_, x)| x.escribible().is_some()) {
        mal(format!(
            "ningún elemento de su tema tiene el campo {campo} y se puede escribir"
        ));
    }
    for (aj, x) in campos {
        if x.tipo == TipoCampo::Secreto {
            mal(format!("{}.{campo} es un secreto", aj.id));
        } else if traducir(x, valor).is_err() {
            mal(format!(
                "«{valor}» no es un valor válido de {}.{campo}",
                aj.id
            ));
        }
    }
}
