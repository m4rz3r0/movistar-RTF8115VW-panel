//! Reglas del catálogo de ajustes contra el de funciones, claves de
//! traducción y riesgo.

use catalogo::{Catalogo, Origen, Parametro, Riesgo, Tipo, TipoParam, mask::Mascara};
use std::collections::HashSet;

use crate::carga::CatalogoAjustes;
use crate::modelo::{Ajuste, Campo, TipoCampo};
use crate::valores::a_router;

/// Prefijo de las claves de traducción de un ajuste: `ajuste-` más el id con
/// `.` → `-` (`wifi.red_5` → `ajuste-wifi-red_5`).
pub fn prefijo_i18n(a: &Ajuste) -> String {
    format!("ajuste-{}", a.id.replace('.', "-"))
}

/// Claves de traducción que necesita un ajuste, sin repetir:
/// - `<p>-titulo`, `<p>-desc`, `<p>-ayuda` y `efecto-<efecto>`;
/// - por campo, `<p>-<campo>`, `<p>-<campo>-desc` y la de `solo_lectura`;
/// - por opción, `<p>-<campo>-<clave>`, su `-desc` y `aviso-<aviso>`;
/// - `<p>-<campo>-recomendado` si el campo tiene recomendación (el motivo).
pub fn claves_requeridas(a: &Ajuste) -> Vec<String> {
    let p = prefijo_i18n(a);
    let mut claves = vec![
        format!("{p}-titulo"),
        format!("{p}-desc"),
        format!("{p}-ayuda"),
    ];
    claves.extend(a.efecto.iter().map(|e| format!("efecto-{e}")));
    for c in &a.campos {
        let pc = format!("{p}-{}", c.id);
        claves.push(pc.clone());
        claves.push(format!("{pc}-desc"));
        claves.extend(c.solo_lectura.iter().cloned());
        for o in &c.opciones {
            claves.push(format!("{pc}-{}", o.clave));
            claves.push(format!("{pc}-{}-desc", o.clave));
            claves.extend(o.aviso.iter().map(|av| format!("aviso-{av}")));
        }
        if c.recomendado.is_some() || c.opciones.iter().any(|o| o.recomendada) {
            claves.push(format!("{pc}-recomendado"));
        }
    }
    let mut vistas = HashSet::new();
    claves.retain(|k| vistas.insert(k.clone()));
    claves
}

/// Riesgo de aplicar un ajuste: el mayor de sus funciones de escritura
/// (bajo si no escribe nada).
pub fn riesgo(a: &Ajuste, f: &Catalogo) -> Riesgo {
    a.campos
        .iter()
        .filter_map(|c| f.funcion(&c.escribe.funcion))
        .map(|func| func.riesgo)
        .max()
        .unwrap_or(Riesgo::Bajo)
}

/// Fallos del catálogo de ajustes frente al de funciones y la máscara de
/// secretos: uno legible por regla incumplida, con el id del ajuste (o de la
/// función excluida). Vacío si todo cuadra.
pub fn comprobar(c: &CatalogoAjustes, f: &Catalogo, m: &Mascara) -> Vec<String> {
    let mut fallos = Vec::new();
    let mut ids = HashSet::new();
    for a in c.todos() {
        if !ids.insert(a.id.as_str()) {
            fallos.push(format!("{}: id repetido", a.id));
        }
        comprobar_ajuste(a, f, m, &mut fallos);
    }
    comprobar_excluidas(c, f, &mut fallos);
    fallos
}

/// Identificador válido como trozo de una clave de traducción.
fn es_nombre(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn comprobar_ajuste(a: &Ajuste, f: &Catalogo, m: &Mascara, fallos: &mut Vec<String>) {
    let id = &a.id;
    // 1. Forma del id.
    match a.id.split_once('.') {
        Some((tema, nombre)) if es_nombre(tema) && es_nombre(nombre) => {
            if tema != a.tema.slug() {
                fallos.push(format!(
                    "{id}: el tema del id no coincide con `tema` ({})",
                    a.tema.slug()
                ));
            }
        }
        _ => fallos.push(format!(
            "{id}: el id debe tener la forma <tema>.<nombre> (minúsculas, cifras y _)"
        )),
    }
    // 2. La lectura.
    match f.funcion(&a.leer.funcion) {
        None => fallos.push(format!("{id}: la lectura {} no existe", a.leer.funcion)),
        Some(l) if l.tipo != Tipo::Lectura => {
            fallos.push(format!("{id}: {} no es una lectura", a.leer.funcion));
        }
        Some(_) => {}
    }
    let mut campos = HashSet::new();
    for campo in &a.campos {
        if !campos.insert(campo.id.as_str()) {
            fallos.push(format!("{id}: campo {} repetido", campo.id));
        }
        comprobar_campo(a, campo, f, m, fallos);
    }
}

fn comprobar_campo(a: &Ajuste, c: &Campo, f: &Catalogo, m: &Mascara, fallos: &mut Vec<String>) {
    let donde = format!("{}.{}", a.id, c.id);
    let mut mal = |msg: String| fallos.push(format!("{donde}: {msg}"));
    if !es_nombre(&c.id) {
        mal("el id del campo solo admite minúsculas, cifras y _".into());
    }
    if c.lee.trim().is_empty() {
        mal("`lee` vacío".into());
    }
    comprobar_forma(c, &mut mal);

    // 3. La función de escritura y su parámetro.
    let nombre_f = &c.escribe.funcion;
    let Some(func) = f.funcion(nombre_f) else {
        mal(format!("la función {nombre_f} no existe"));
        return;
    };
    if func.tipo != Tipo::Escritura {
        mal(format!("{nombre_f} no es una escritura"));
    }
    if func.bloqueada.is_some() {
        mal(format!("{nombre_f} está bloqueada"));
    }
    // 7. Listas.
    if a.lista && func.lista.is_none() {
        mal(format!("lista = true, pero {nombre_f} no es una lista"));
    }
    // 8. Secretos de la web básica.
    let basica = func.origen == Origen::Basica || !nombre_f.starts_with("cb");
    if c.tipo == TipoCampo::Secreto && basica && c.solo_lectura.is_none() {
        mal(format!(
            "secreto que solo escribe la web básica ({nombre_f}): necesita solo_lectura"
        ));
    }
    let Some(p) = func.parametro(&c.escribe.parametro) else {
        mal(format!(
            "{nombre_f} no tiene el parámetro {}",
            c.escribe.parametro
        ));
        return;
    };
    // 5. Secreto ↔ secreto; 4. tipos compatibles.
    let param_secreto = p.tipo == TipoParam::Secret || m.es_secreto(&p.nombre);
    let campo_secreto = c.tipo == TipoCampo::Secreto;
    if param_secreto && !campo_secreto {
        mal(format!(
            "el parámetro {} es secreto y el campo no es `secreto`",
            p.nombre
        ));
    } else if campo_secreto && !param_secreto {
        mal(format!(
            "campo `secreto` sobre {}, que no es un parámetro secreto",
            p.nombre
        ));
    } else if !campo_secreto && !compatibles(c.tipo, p) {
        mal(format!(
            "tipo {:?} incompatible con el parámetro {} ({:?})",
            c.tipo, p.nombre, p.tipo
        ));
    }
    // 6. Valores dentro del enum.
    if p.tipo == TipoParam::Enum {
        let validos: Vec<&str> = p.opciones.iter().map(|(v, _)| v.as_str()).collect();
        let usados: Vec<&str> = match c.tipo {
            TipoCampo::Opcion => c.opciones.iter().map(|o| o.valor.as_str()).collect(),
            TipoCampo::Booleano => vec![c.booleano.si.as_str(), c.booleano.no.as_str()],
            _ => Vec::new(),
        };
        for v in usados.into_iter().filter(|v| !validos.contains(v)) {
            mal(format!(
                "el valor {v} no está entre los del parámetro {}",
                p.nombre
            ));
        }
    }
}

/// Coherencia del campo consigo mismo: opciones, recomendación y límites.
fn comprobar_forma(c: &Campo, mal: &mut impl FnMut(String)) {
    match (c.tipo, c.opciones.is_empty()) {
        (TipoCampo::Opcion, true) => mal("campo `opcion` sin opciones".into()),
        (t, false) if t != TipoCampo::Opcion => {
            mal("opciones en un campo que no es `opcion`".into());
        }
        _ => {}
    }
    let (mut valores, mut claves) = (HashSet::new(), HashSet::new());
    for o in &c.opciones {
        if !valores.insert(o.valor.as_str()) {
            mal(format!("opción con el valor {} repetido", o.valor));
        }
        if !claves.insert(o.clave.as_str()) {
            mal(format!("opción {} repetida", o.clave));
        }
        if !es_nombre(&o.clave) {
            mal(format!(
                "la clave de opción {} solo admite minúsculas, cifras y _",
                o.clave
            ));
        }
        if o.aviso.as_deref().is_some_and(|av| !es_nombre(av)) {
            mal(format!("aviso no válido en la opción {}", o.clave));
        }
    }
    if c.opciones.iter().filter(|o| o.recomendada).count() > 1 {
        mal("más de una opción recomendada".into());
    }
    if let Some(r) = &c.recomendado {
        match c.tipo {
            TipoCampo::Secreto => mal("un secreto no lleva recomendado".into()),
            TipoCampo::Opcion => {
                mal("un campo `opcion` marca `recomendada` en la opción, no `recomendado`".into())
            }
            _ if a_router(c, r).is_err() => {
                mal(format!("recomendado «{r}» no es un valor válido del campo"));
            }
            _ => {}
        }
    }
    if c.invertido && c.tipo != TipoCampo::Booleano {
        mal("`invertido` solo vale en un campo `booleano`".into());
    }
    if let Some(v) = c.validar {
        if !matches!(
            c.tipo,
            TipoCampo::Texto | TipoCampo::Secreto | TipoCampo::Numero
        ) {
            mal("`validar` solo vale en texto, secreto o número".into());
        }
        if let (Some(min), Some(max)) = (v.min, v.max)
            && min > max
        {
            mal(format!("límites invertidos ({min} > {max})"));
        }
    }
}

/// Regla 4: el control del campo sirve para el tipo del parámetro.
fn compatibles(t: TipoCampo, p: &Parametro) -> bool {
    match t {
        TipoCampo::Texto => p.tipo == TipoParam::Text,
        TipoCampo::Secreto => p.tipo == TipoParam::Secret,
        TipoCampo::Booleano => {
            p.tipo == TipoParam::Bool || (p.tipo == TipoParam::Enum && p.opciones.len() == 2)
        }
        TipoCampo::Opcion => matches!(p.tipo, TipoParam::Enum | TipoParam::Int | TipoParam::Text),
        TipoCampo::Numero => p.tipo == TipoParam::Int,
        TipoCampo::Ip => p.tipo == TipoParam::Ip,
        TipoCampo::Mac => p.tipo == TipoParam::Mac,
    }
}

/// Regla 9: cada excluida existe, tiene motivo y ningún ajuste la usa.
fn comprobar_excluidas(c: &CatalogoAjustes, f: &Catalogo, fallos: &mut Vec<String>) {
    let mut vistas = HashSet::new();
    for e in c.excluidas() {
        let n = &e.funcion;
        if !vistas.insert(n.as_str()) {
            fallos.push(format!("excluida {n}: repetida"));
        }
        if f.funcion(n).is_none() {
            fallos.push(format!(
                "excluida {n}: no existe en el catálogo de funciones"
            ));
        }
        if e.motivo.trim().is_empty() {
            fallos.push(format!("excluida {n}: sin motivo"));
        }
        for a in c.todos() {
            for campo in a.campos.iter().filter(|campo| campo.escribe.funcion == *n) {
                fallos.push(format!(
                    "excluida {n}: también la usa {}.{}",
                    a.id, campo.id
                ));
            }
        }
    }
}
