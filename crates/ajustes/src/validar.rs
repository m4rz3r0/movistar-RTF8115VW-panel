//! Reglas del catálogo de ajustes contra el de funciones, claves de
//! traducción y riesgo.

use catalogo::{Catalogo, Funcion, Origen, Parametro, Riesgo, Tipo, TipoParam, mask::Mascara};
use std::collections::HashSet;

use crate::carga::CatalogoAjustes;
use crate::legible::admite_opcional;
use crate::modelo::{Ajuste, Campo, SIN_ELECCION, TipoCampo};
use crate::valores::traducir;

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
        if c.sin_eleccion.is_some() {
            claves.push(format!("{pc}-{SIN_ELECCION}"));
            claves.push(format!("{pc}-{SIN_ELECCION}-desc"));
        }
        if c.recomendado.is_some() || c.opciones.iter().any(|o| o.recomendada) {
            claves.push(format!("{pc}-recomendado"));
        }
    }
    let mut vistas = HashSet::new();
    claves.retain(|k| vistas.insert(k.clone()));
    claves
}

/// Riesgo de aplicar un ajuste: el mayor de las funciones de escritura de sus
/// campos editables (bajo si no escribe nada).
pub fn riesgo(a: &Ajuste, f: &Catalogo) -> Riesgo {
    a.campos
        .iter()
        .filter_map(Campo::escribible)
        .filter_map(|e| f.funcion(&e.funcion))
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

/// Ids de campo que darían las claves del propio ajuste (`<p>-titulo`…).
const CAMPOS_RESERVADOS: [&str; 3] = ["titulo", "desc", "ayuda"];
/// Claves de opción que darían las del propio campo (`<p>-<campo>-desc`…).
const OPCIONES_RESERVADAS: [&str; 2] = ["desc", "recomendado"];

/// El parámetro es secreto por su tipo o por la máscara.
fn es_secreto(p: &Parametro, m: &Mascara) -> bool {
    p.tipo == TipoParam::Secret || m.es_secreto(&p.nombre)
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
    if a.efecto.as_deref().is_some_and(|e| !es_nombre(e)) {
        fallos.push(format!(
            "{id}: el efecto solo admite minúsculas, cifras y _"
        ));
    }
    // 2. La lectura y sus parámetros.
    match f.funcion(&a.leer.funcion) {
        None => fallos.push(format!("{id}: la lectura {} no existe", a.leer.funcion)),
        Some(l) if l.tipo != Tipo::Lectura => {
            fallos.push(format!("{id}: {} no es una lectura", a.leer.funcion));
        }
        Some(l) => {
            if a.lista {
                comprobar_id_de_lista(a, l, m, fallos);
            }
            for (k, _) in &a.leer.params {
                match l.parametro(k) {
                    None => fallos.push(format!(
                        "{id}: la lectura {} no tiene el parámetro {k}",
                        l.name
                    )),
                    Some(p) if es_secreto(p, m) => {
                        fallos.push(format!("{id}: el parámetro de lectura {k} es secreto"));
                    }
                    Some(_) => {}
                }
            }
        }
    }
    comprobar_fijos(a, f, m, fallos);
    let mut campos = HashSet::new();
    for campo in &a.campos {
        if !campos.insert(campo.id.as_str()) {
            fallos.push(format!("{id}: campo {} repetido", campo.id));
        }
        comprobar_campo(a, campo, f, m, fallos);
        comprobar_requiere(a, campo, fallos);
        let escritura = campo.escribe.as_ref().and_then(|e| f.funcion(&e.funcion));
        if let Some(func) = escritura.filter(|_| a.lista) {
            comprobar_id_de_lista(a, func, m, fallos);
        }
    }
}

/// `requiere`: el campo que lo pide se puede escribir (si no, nunca se
/// cumpliría), el exigido es otro del mismo ajuste y se puede escribir, y
/// `cuando` es un valor válido del campo que lo pide.
fn comprobar_requiere(a: &Ajuste, c: &Campo, fallos: &mut Vec<String>) {
    let Some(r) = &c.requiere else {
        return;
    };
    let donde = format!("{}.{}", a.id, c.id);
    if c.escribible().is_none() {
        fallos.push(format!(
            "{donde}: requiere en un campo que no se puede escribir"
        ));
    }
    match a.campo(&r.campo) {
        None => fallos.push(format!(
            "{donde}: requiere {}, que no es un campo del ajuste",
            r.campo
        )),
        Some(e) if e.id == c.id => {
            fallos.push(format!("{donde}: requiere el propio campo"));
        }
        Some(e) if e.escribible().is_none() => fallos.push(format!(
            "{donde}: requiere {}, que no se puede escribir",
            e.id
        )),
        Some(_) => {}
    }
    if traducir(c, &r.cuando).is_err() {
        fallos.push(format!(
            "{donde}: requiere con «{}», que no es un valor válido del campo",
            r.cuando
        ));
    }
}

/// Los fijos son parámetros de cada función de escritura del ajuste, no son
/// secretos ni los escribe también un campo.
fn comprobar_fijos(a: &Ajuste, f: &Catalogo, m: &Mascara, fallos: &mut Vec<String>) {
    if a.fijos.is_empty() {
        return;
    }
    let id = &a.id;
    let mut funciones: Vec<&str> = a
        .campos
        .iter()
        .filter_map(Campo::escribible)
        .map(|e| e.funcion.as_str())
        .collect();
    funciones.sort_unstable();
    funciones.dedup();
    if funciones.is_empty() {
        fallos.push(format!("{id}: fijos en un ajuste que no escribe"));
    }
    for k in a.fijos.keys() {
        if m.es_secreto(k) {
            fallos.push(format!("{id}: el fijo {k} es secreto"));
        }
        if a.campos
            .iter()
            .filter_map(Campo::escribible)
            .any(|e| e.parametro == *k)
        {
            fallos.push(format!("{id}: el fijo {k} también lo escribe un campo"));
        }
        // Las funciones que no existen ya las señala su campo (regla 3).
        for func in funciones.iter().filter_map(|n| f.funcion(n)) {
            match func.parametro(k) {
                None => fallos.push(format!("{id}: {} no tiene el fijo {k}", func.name)),
                Some(p) if p.tipo == TipoParam::Secret && !m.es_secreto(k) => {
                    fallos.push(format!("{id}: el fijo {k} es secreto"));
                }
                Some(_) => {}
            }
        }
    }
}

/// El parámetro de instancia de una lista (el id que identifica cada
/// instancia en la interfaz) no es secreto.
fn comprobar_id_de_lista(a: &Ajuste, func: &Funcion, m: &Mascara, fallos: &mut Vec<String>) {
    if let Some(l) = func.lista.as_ref().filter(|l| m.es_secreto(&l.id_param)) {
        let fallo = format!(
            "{}: el parámetro de instancia {} de {} es secreto",
            a.id, l.id_param, func.name
        );
        if !fallos.contains(&fallo) {
            fallos.push(fallo);
        }
    }
}

fn comprobar_campo(a: &Ajuste, c: &Campo, f: &Catalogo, m: &Mascara, fallos: &mut Vec<String>) {
    let donde = format!("{}.{}", a.id, c.id);
    let mut mal = |msg: String| fallos.push(format!("{donde}: {msg}"));
    if !es_nombre(&c.id) {
        mal("el id del campo solo admite minúsculas, cifras y _".into());
    }
    if CAMPOS_RESERVADOS.contains(&c.id.as_str()) {
        mal(format!(
            "el id de campo {} está reservado para las claves del ajuste",
            c.id
        ));
    }
    if c.lee.trim().is_empty() {
        mal("`lee` vacío".into());
    }
    let campo_secreto = c.tipo == TipoCampo::Secreto;
    // 5. Lo que se lee de un elemento secreto no sale del servidor.
    if m.es_secreto(&c.lee) && !campo_secreto {
        mal(format!(
            "el elemento leído {} es secreto y el campo no es `secreto`",
            c.lee
        ));
    }
    comprobar_forma(c, &mut mal);

    let Some(e) = &c.escribe else {
        if c.solo_lectura.is_none() {
            mal("sin `escribe`, el campo necesita `solo_lectura`".into());
        }
        return;
    };
    // 3. La función de escritura y su parámetro.
    let nombre_f = &e.funcion;
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
    // 8. Secretos de la web básica: se quedan en solo lectura. Es el único
    // caso en que un campo de solo lectura conserva su `escribe`.
    let basica = func.origen == Origen::Basica || !nombre_f.starts_with("cb");
    let secreto_basico = campo_secreto && basica;
    if secreto_basico && c.solo_lectura.is_none() {
        mal(format!(
            "secreto que solo escribe la web básica ({nombre_f}): necesita solo_lectura"
        ));
    } else if !secreto_basico && c.solo_lectura.is_some() {
        mal(
            "`solo_lectura` con `escribe`: un campo de solo lectura no lleva `escribe` \
             (salvo un secreto de la web básica)"
                .into(),
        );
    }
    let Some(p) = func.parametro(&e.parametro) else {
        mal(format!("{nombre_f} no tiene el parámetro {}", e.parametro));
        return;
    };
    // 5. Secreto ↔ secreto; 4. tipos compatibles.
    let param_secreto = es_secreto(p, m);
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
    // 6. Valores dentro del enum (y enteros en un `int`).
    match c.tipo {
        TipoCampo::Opcion => {
            for o in &c.opciones {
                comprobar_valor(p, &o.valor, &mut mal);
            }
        }
        TipoCampo::Booleano => {
            let (si, no) = c.si_no();
            comprobar_valor(p, si, &mut mal);
            comprobar_valor(p, no, &mut mal);
        }
        _ => {}
    }
    // Los parámetros extra de las opciones: reglas 3, 4, 5 y 6.
    for ad in &c.ademas {
        if ad.parametro == e.parametro {
            mal(format!("`ademas` repite el parámetro {}", ad.parametro));
            continue;
        }
        let Some(pa) = func.parametro(&ad.parametro) else {
            mal(format!("{nombre_f} no tiene el parámetro {}", ad.parametro));
            continue;
        };
        if es_secreto(pa, m) {
            mal(format!("el parámetro {} es secreto", pa.nombre));
            continue;
        }
        if m.es_secreto(&ad.lee) {
            mal(format!("el elemento leído {} es secreto", ad.lee));
        }
        if !compatibles(TipoCampo::Opcion, pa) {
            mal(format!(
                "parámetro {} ({:?}) incompatible con las opciones",
                pa.nombre, pa.tipo
            ));
        }
        for v in c
            .opciones
            .iter()
            .filter_map(|o| o.ademas.get(&ad.parametro))
        {
            comprobar_valor(pa, v, &mut mal);
        }
    }
}

/// Regla 6: un valor de un `enum` está entre los suyos; el de un `int`, es
/// un entero.
fn comprobar_valor(p: &Parametro, v: &str, mal: &mut impl FnMut(String)) {
    match p.tipo {
        TipoParam::Enum if !p.opciones.iter().any(|(valido, _)| valido == v) => {
            mal(format!(
                "el valor {v} no está entre los del parámetro {}",
                p.nombre
            ));
        }
        TipoParam::Int if v.parse::<i64>().is_err() => {
            mal(format!(
                "el valor {v} del parámetro {} no es un entero",
                p.nombre
            ));
        }
        _ => {}
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
    if !c.ademas.is_empty() && c.tipo != TipoCampo::Opcion {
        mal("`ademas` solo vale en un campo `opcion`".into());
    }
    let mut extra = HashSet::new();
    for ad in &c.ademas {
        if !extra.insert(ad.parametro.as_str()) {
            mal(format!("`ademas` repite el parámetro {}", ad.parametro));
        }
        if ad.lee.trim().is_empty() {
            mal(format!("`lee` vacío en `ademas` ({})", ad.parametro));
        }
    }
    let (mut valores, mut claves) = (HashSet::new(), HashSet::new());
    for o in &c.opciones {
        if !valores.insert((o.valor.as_str(), &o.ademas)) {
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
        if OPCIONES_RESERVADAS.contains(&o.clave.as_str()) {
            mal(format!(
                "la clave de opción {} está reservada para las claves del campo",
                o.clave
            ));
        }
        if o.aviso.as_deref().is_some_and(|av| !es_nombre(av)) {
            mal(format!("aviso no válido en la opción {}", o.clave));
        }
        for ad in c
            .ademas
            .iter()
            .filter(|ad| !o.ademas.contains_key(&ad.parametro))
        {
            mal(format!(
                "la opción {} no da valor a {}",
                o.clave, ad.parametro
            ));
        }
        for k in o.ademas.keys().filter(|k| !extra.contains(k.as_str())) {
            mal(format!(
                "la opción {} da valor a {k}, que no está en el `ademas` del campo",
                o.clave
            ));
        }
    }
    if let Some(v) = &c.sin_eleccion {
        if c.tipo != TipoCampo::Opcion {
            mal("`sin_eleccion` solo vale en un campo `opcion`".into());
        }
        if c.opciones.iter().any(|o| o.valor == *v) {
            mal(format!("`sin_eleccion` {v} es el valor de una opción"));
        }
        if c.opciones.iter().any(|o| o.clave == SIN_ELECCION) {
            mal(format!(
                "con `sin_eleccion`, la clave de opción {SIN_ELECCION} está reservada"
            ));
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
            _ if traducir(c, r).is_err() => {
                mal(format!("recomendado «{r}» no es un valor válido del campo"));
            }
            _ => {}
        }
    }
    if c.invertido && c.tipo != TipoCampo::Booleano {
        mal("`invertido` solo vale en un campo `booleano`".into());
    }
    if let Some(b) = &c.booleano {
        if c.tipo != TipoCampo::Booleano {
            mal("`booleano` solo vale en un campo `booleano`".into());
        }
        if b.si == b.no {
            mal("`booleano` con el mismo valor para sí y no".into());
        }
    }
    if c.opcional && !admite_opcional(c.tipo) {
        mal("`opcional` no vale en un campo `opcion` ni `booleano`".into());
    }
    if let Some(v) = c.validar {
        if !matches!(
            c.tipo,
            TipoCampo::Texto | TipoCampo::Secreto | TipoCampo::Numero
        ) {
            mal("`validar` solo vale en texto, secreto o número".into());
        }
        if v.ascii && !matches!(c.tipo, TipoCampo::Texto | TipoCampo::Secreto) {
            mal("`ascii` solo vale en texto o secreto".into());
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

/// Regla 9: cada excluida es una escritura no bloqueada del catálogo, tiene
/// motivo y ningún ajuste la usa.
fn comprobar_excluidas(c: &CatalogoAjustes, f: &Catalogo, fallos: &mut Vec<String>) {
    let mut vistas = HashSet::new();
    for e in c.excluidas() {
        let n = &e.funcion;
        if !vistas.insert(n.as_str()) {
            fallos.push(format!("excluida {n}: repetida"));
        }
        match f.funcion(n) {
            None => fallos.push(format!(
                "excluida {n}: no existe en el catálogo de funciones"
            )),
            Some(func) if func.tipo != Tipo::Escritura => fallos.push(format!(
                "excluida {n}: no es una escritura (solo se excluyen escrituras)"
            )),
            Some(func) if func.bloqueada.is_some() => fallos.push(format!(
                "excluida {n}: está bloqueada en el catálogo de funciones (no hace falta excluirla)"
            )),
            Some(_) => {}
        }
        if e.motivo.trim().is_empty() {
            fallos.push(format!("excluida {n}: sin motivo"));
        }
        for a in c.todos() {
            for campo in a
                .campos
                .iter()
                .filter(|campo| campo.escribe.as_ref().is_some_and(|e| e.funcion == *n))
            {
                fallos.push(format!(
                    "excluida {n}: también la usa {}.{}",
                    a.id, campo.id
                ));
            }
        }
    }
}
