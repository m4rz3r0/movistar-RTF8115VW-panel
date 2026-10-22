//! Reglas de `lotes.toml`: pasos, elecciones y lotes.

use std::collections::HashSet;

use super::expandir::{accion, campos_del_elemento, expandir, redes_wifi};
use super::{Eleccion, Lote, PasoLote};
use crate::carga::CatalogoAjustes;
use crate::elementos::partir_referencia;
use crate::modelo::{Ajuste, Campo, Tema, TipoCampo};
use crate::valores::traducir;

/// Identificador válido como trozo de una clave de traducción.
fn es_nombre(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Fallos de los lotes, sus pasos y sus elecciones, uno legible por regla
/// incumplida: «paso <id>: …», «elección <id>: …» o «lote <id>: …».
pub(crate) fn comprobar_lotes(c: &CatalogoAjustes, fallos: &mut Vec<String>) {
    let mut ids = HashSet::new();
    for p in c.pasos() {
        if !ids.insert(p.id.as_str()) {
            fallos.push(format!("paso {}: id repetido", p.id));
        }
        let mut mal = |msg: String| fallos.push(format!("paso {}: {msg}", p.id));
        comprobar_paso(c, p, &mut mal);
    }
    let mut ids = HashSet::new();
    for e in c.elecciones() {
        if !ids.insert(e.id.as_str()) {
            fallos.push(format!("elección {}: id repetido", e.id));
        }
        comprobar_eleccion(e, fallos);
    }
    let mut ids = HashSet::new();
    for l in c.lotes() {
        if !ids.insert(l.id.as_str()) {
            fallos.push(format!("lote {}: id repetido", l.id));
        }
        comprobar_lote(c, l, fallos);
    }
}

/// `<ajuste>.<campo>` del catálogo.
fn campo_de<'c>(c: &'c CatalogoAjustes, r: &str) -> Option<(&'c Ajuste, &'c Campo)> {
    let (a, x) = partir_referencia(r)?;
    let a = c.ajuste(a)?;
    a.campo(x).map(|x| (a, x))
}

/// Un lote puede escribir `legible` en el campo: se puede escribir, no es un
/// secreto y el valor es válido.
fn comprobar_destino(a: &Ajuste, x: &Campo, legible: &str, mal: &mut impl FnMut(String)) {
    let r = format!("{}.{}", a.id, x.id);
    if x.escribible().is_none() {
        mal(format!("{r} no se puede escribir"));
    } else if x.tipo == TipoCampo::Secreto {
        mal(format!("{r} es un secreto"));
    } else if traducir(x, legible).is_err() {
        mal(format!("«{legible}» no es un valor válido de {r}"));
    }
}

fn comprobar_paso(c: &CatalogoAjustes, p: &PasoLote, mal: &mut impl FnMut(String)) {
    if !es_nombre(&p.id) {
        mal("el id solo admite minúsculas, cifras y _".into());
    }
    let destinos = [p.accion.is_some(), p.campo.is_some(), p.ajuste.is_some()];
    if destinos.iter().filter(|d| **d).count() != 1 {
        return mal("necesita uno y solo uno de `accion`, `campo` o `ajuste`".into());
    }
    if p.para_cada.is_some() && p.tema != Tema::Wifi {
        mal("para_cada = \"red_wifi\" solo vale en el tema wifi".into());
    }
    if p.valor.is_some() != p.campo.is_some() {
        mal("`valor` va con `campo`, y solo con él".into());
    }
    if p.elegir.is_some() != p.ajuste.is_some() {
        mal("`elegir` va con `ajuste`, y solo con él".into());
    }
    if p.elegir.is_some() && p.para_cada.is_some() {
        mal("una elección no se repite con para_cada".into());
    }
    if let Some(si) = &p.si {
        match campo_de(c, &si.campo) {
            None => mal(format!("si: el campo {} no existe", si.campo)),
            Some((a, x)) if traducir(x, &si.valor).is_err() => mal(format!(
                "si: «{}» no es un valor válido de {}.{}",
                si.valor, a.id, x.id
            )),
            Some(_) => {}
        }
    }
    if let Some(id) = &p.accion {
        match accion(c, p.tema, id) {
            None => return mal(format!("la acción {id} no existe en su tema")),
            Some(a) if a.lista.is_some() || a.borrar => {
                return mal(format!("la acción {id} es de una lista"));
            }
            // Su campo y su valor los comprueba la propia acción.
            Some(_) => {}
        }
    }
    if let (Some(campo), Some(valor)) = (&p.campo, &p.valor) {
        comprobar_campo(c, p, campo, valor, mal);
    }
    if let (Some(a), Some(e)) = (&p.ajuste, &p.elegir) {
        comprobar_elegir(c, p, a, e, mal);
    }
    let vacio = expandir(c, p).iter().all(|x| x.destinos.is_empty());
    if vacio {
        mal("no cambia ningún campo".into());
    }
}

/// El campo de un paso: `<ajuste>.<campo>` de su tema o, con `para_cada`,
/// el id del campo en cada red WiFi que lo tiene.
fn comprobar_campo(
    c: &CatalogoAjustes,
    p: &PasoLote,
    campo: &str,
    valor: &str,
    mal: &mut impl FnMut(String),
) {
    if p.para_cada.is_some() {
        let redes: Vec<_> = redes_wifi(c)
            .flat_map(|e| campos_del_elemento(c, e, campo))
            .collect();
        if redes.is_empty() {
            mal(format!("ninguna red WiFi tiene el campo {campo}"));
        }
        for (a, x) in redes {
            comprobar_destino(a, x, valor, mal);
        }
        return;
    }
    match campo_de(c, campo) {
        None => mal(format!("el campo {campo} no existe")),
        Some((a, _)) if a.tema != p.tema => mal(format!("el campo {campo} es de otro tema")),
        Some((a, x)) => comprobar_destino(a, x, valor, mal),
    }
}

/// Los valores de cada opción de la elección valen en los campos del
/// ajuste.
fn comprobar_elegir(
    c: &CatalogoAjustes,
    p: &PasoLote,
    ajuste: &str,
    eleccion: &str,
    mal: &mut impl FnMut(String),
) {
    let a = match c.ajuste(ajuste) {
        None => return mal(format!("el ajuste {ajuste} no existe")),
        Some(a) if a.tema != p.tema => return mal(format!("el ajuste {ajuste} es de otro tema")),
        Some(a) => a,
    };
    let Some(e) = c.eleccion(eleccion) else {
        return mal(format!("la elección {eleccion} no existe"));
    };
    for o in &e.opciones {
        let mut de_la_opcion = |m: String| mal(format!("opción {}: {m}", o.id));
        for (campo, valor) in &o.valores {
            match a.campo(campo) {
                None => de_la_opcion(format!("el campo {ajuste}.{campo} no existe")),
                Some(x) => comprobar_destino(a, x, valor, &mut de_la_opcion),
            }
        }
    }
}

fn comprobar_eleccion(e: &Eleccion, fallos: &mut Vec<String>) {
    let mut mal = |msg: String| fallos.push(format!("elección {}: {msg}", e.id));
    if !es_nombre(&e.id) {
        mal("el id solo admite minúsculas, cifras y _".into());
    }
    if e.opciones.is_empty() {
        mal("sin opciones".into());
    }
    let mut ids = HashSet::new();
    for o in &e.opciones {
        if !es_nombre(&o.id) {
            mal(format!(
                "la opción {}: el id solo admite minúsculas, cifras y _",
                o.id
            ));
        }
        if !ids.insert(o.id.as_str()) {
            mal(format!("la opción {} está repetida", o.id));
        }
        if o.valores.is_empty() {
            mal(format!("la opción {} no da ningún valor", o.id));
        }
    }
    // Todas dan valor a los mismos campos: si no, al aplicar una de ellas
    // un campo que dan las otras se quedaría sin valor.
    if let Some((primera, resto)) = e.opciones.split_first() {
        for o in resto
            .iter()
            .filter(|o| !o.valores.keys().eq(primera.valores.keys()))
        {
            mal(format!(
                "la opción {} no da valor a los mismos campos que {}",
                o.id, primera.id
            ));
        }
    }
    match e.opciones.iter().filter(|o| o.defecto).count() {
        0 if !e.opciones.is_empty() => mal("ninguna opción por defecto".into()),
        0 | 1 => {}
        _ => mal("más de una opción por defecto".into()),
    }
}

fn comprobar_lote(c: &CatalogoAjustes, l: &Lote, fallos: &mut Vec<String>) {
    let mut mal = |msg: String| fallos.push(format!("lote {}: {msg}", l.id));
    if !es_nombre(&l.id) {
        mal("el id solo admite minúsculas, cifras y _".into());
    }
    if !crate::ICONOS.contains(&l.icono.as_str()) {
        mal(format!("el icono {} no existe", l.icono));
    }
    if l.pasos.is_empty() {
        mal("sin pasos".into());
    }
    let mut vistos = HashSet::new();
    for p in &l.pasos {
        if !vistos.insert(p.as_str()) {
            mal(format!("el paso {p} está repetido"));
        }
        if c.paso(p).is_none() {
            mal(format!("el paso {p} no existe"));
        }
    }
    for (que, ids) in [("aviso", &l.avisos), ("consejo", &l.consejos)] {
        let mut vistos = HashSet::new();
        for id in ids {
            if !es_nombre(id) {
                mal(format!("el {que} {id}: solo admite minúsculas, cifras y _"));
            }
            if !vistos.insert(id.as_str()) {
                mal(format!("el {que} {id} está repetido"));
            }
        }
    }
}
