//! Lectura de los ajustes: vistas con valores legibles y detalles técnicos.

use super::*;

/// Vista de un ajuste con la respuesta de su lectura (`None` si falló).
pub(super) fn vista(f: &Catalogo, a: &Ajuste, nodo: Option<&XmlNode>) -> AjusteVista {
    let prefijo = prefijo_i18n(a);
    let sin_leer = || {
        (
            EstadoLectura::SinLeer(t!("ajustes-error-lectura")),
            Vec::new(),
            Vec::new(),
        )
    };
    let (estado, campos, instancias) = match nodo {
        None => sin_leer(),
        Some(n) if a.lista => match lista(f, a, &prefijo, n) {
            Some(instancias) => {
                let plantilla = a
                    .campos
                    .iter()
                    .map(|c| campo(f, &prefijo, c, valor_inicial(c), None))
                    .collect();
                (EstadoLectura::Leido, plantilla, instancias)
            }
            None => {
                tracing::warn!(
                    "ajustes: {} ({}) trae instancias sin su id: no se ofrecen",
                    a.id,
                    a.leer.funcion
                );
                sin_leer()
            }
        },
        Some(n) => {
            let mut leidos = Leidos::new();
            hojas(n, &mut leidos);
            let campos = a
                .campos
                .iter()
                .map(|c| {
                    campo(
                        f,
                        &prefijo,
                        c,
                        a_legible(c, &leidos),
                        valor_secreto(f, c, &leidos),
                    )
                })
                .collect();
            (EstadoLectura::Leido, campos, Vec::new())
        }
    };
    AjusteVista {
        id: a.id.clone(),
        nivel: a.nivel,
        riesgo: riesgo(ajustes::riesgo(a, f)),
        efecto: a.efecto.as_ref().map(|e| format!("efecto-{e}")),
        lista: a.lista,
        componente: a.componente.map(|c| c.slug().to_owned()),
        estado,
        campos,
        instancias,
        tecnico: tecnico(a),
        clave: prefijo,
        red_wifi: None,
        nota_radio: None,
        columnas: a.columnas.clone(),
    }
}

/// Instancias de una lista con sus valores. El id de cada una es el del
/// router (el elemento de [`id_instancia`]); solo en una lista que no se
/// escribe (como los usuarios del router) cae a su posición. En una que
/// se escribe, `None` si alguna instancia no trae el suyo: la Tarea 7
/// escribe por ese id, y una posición tomada por id editaría o borraría
/// otra instancia.
pub(super) fn lista(
    f: &Catalogo,
    a: &Ajuste,
    prefijo: &str,
    n: &XmlNode,
) -> Option<Vec<InstanciaVista>> {
    Some(
        instancias_con_id(f, a, n)?
            .into_iter()
            .map(|(id, leidos)| InstanciaVista {
                id,
                campos: a
                    .campos
                    .iter()
                    .map(|c| {
                        campo(
                            f,
                            prefijo,
                            c,
                            a_legible(c, &leidos),
                            valor_secreto(f, c, &leidos),
                        )
                    })
                    .collect(),
            })
            .collect(),
    )
}

/// Instancias de una lista con su id (ver [`lista`]) y sus elementos tal
/// cual los da el router.
pub(super) fn instancias_con_id(
    f: &Catalogo,
    a: &Ajuste,
    n: &XmlNode,
) -> Option<Vec<(String, Leidos)>> {
    let se_escribe = a.campos.iter().any(|c| c.escribible().is_some());
    let id_param = id_instancia(f, a);
    let mut elementos: Vec<&str> = a
        .campos
        .iter()
        .flat_map(|c| std::iter::once(&c.lee).chain(c.ademas.iter().map(|ad| &ad.lee)))
        .map(String::as_str)
        .collect();
    elementos.extend(id_param);
    instancias(n, &elementos)
        .into_iter()
        .enumerate()
        .map(|(i, leidos)| {
            let id = match id_param
                .and_then(|p| leidos.get(p))
                .filter(|v| !v.is_empty())
            {
                Some(id) => id.clone(),
                None if !se_escribe => i.to_string(),
                None => return None,
            };
            Some((id, leidos))
        })
        .collect()
}

/// Valor con que empieza un campo en la plantilla de una instancia
/// nueva: la opción recomendada o la primera; en un sí/no, el
/// recomendado o no; en el resto, el recomendado o vacío. Nunca
/// [`ValorLegible::Desconocido`] (no hay valor del router) ni
/// [`ValorLegible::Secreto`] (no hay nada oculto).
fn valor_inicial(c: &Campo) -> ValorLegible {
    match c.tipo {
        TipoCampo::Opcion => c
            .opciones
            .iter()
            .find(|o| o.recomendada)
            .or_else(|| c.opciones.first())
            .map_or_else(
                || ValorLegible::Texto(String::new()),
                |o| ValorLegible::Opcion(o.clave.clone()),
            ),
        TipoCampo::Booleano => ValorLegible::Booleano(c.recomendado.as_deref() == Some("true")),
        _ => ValorLegible::Texto(c.recomendado.clone().unwrap_or_default()),
    }
}

/// Vista de un campo con su valor. Las claves de traducción siguen las
/// de `ajustes::claves_requeridas`.
/// ¿El campo `c` escribe un secreto que el router da tapado al leer?
pub(crate) fn tapado(f: &Catalogo, c: &Campo) -> bool {
    c.tipo == TipoCampo::Secreto
        && c.escribible().is_some_and(|e| {
            f.funcion(&e.funcion)
                .and_then(|func| func.parametro(&e.parametro))
                .is_some_and(|p| p.tapado)
        })
}

/// Valor leído del secreto `c` para su campo con el ojo; `None` si no es
/// un secreto o el router no deja leerlo (lo da enmascarado, o vacío
/// uno tapado al leer).
pub(super) fn valor_secreto(f: &Catalogo, c: &Campo, leidos: &Leidos) -> Option<String> {
    if c.tipo != TipoCampo::Secreto {
        return None;
    }
    let v = leidos.get(&c.lee).cloned().or_else(|| {
        leidos
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&c.lee))
            .map(|(_, v)| v.clone())
    })?;
    let no_vale = crate::changes::plan::enmascarado(&v) || (tapado(f, c) && v.trim().is_empty());
    (!no_vale).then_some(v)
}

pub(super) fn campo(
    f: &Catalogo,
    prefijo: &str,
    c: &Campo,
    valor: ValorLegible,
    valor_secreto: Option<String>,
) -> CampoVista {
    let clave = format!("{prefijo}-{}", c.id);
    let recomendado = c
        .opciones
        .iter()
        .find(|o| o.recomendada)
        .map(|o| o.clave.clone())
        .or_else(|| c.recomendado.clone());
    CampoVista {
        id: c.id.clone(),
        tipo: c.tipo,
        valor,
        opciones: c
            .opciones
            .iter()
            .map(|o| OpcionVista {
                clave: o.clave.clone(),
                recomendada: o.recomendada,
                aviso: o.aviso.as_ref().map(|av| format!("aviso-{av}")),
            })
            .collect(),
        min: c.validar.and_then(|v| v.min),
        max: c.validar.and_then(|v| v.max),
        ascii: c.validar.is_some_and(|v| v.ascii),
        opcional: c.opcional,
        solo_lectura: c.solo_lectura.clone(),
        tapado: tapado(f, c),
        valor_secreto,
        motivo_recomendado: recomendado.as_ref().map(|_| format!("{clave}-recomendado")),
        recomendado,
        aceptados: c.aceptados.clone(),
        requiere: c.requiere.clone(),
        sin_eleccion: c.sin_eleccion.is_some(),
        clave,
    }
}

/// Elementos hoja de `n` a cualquier profundidad; si uno se repite, vale
/// el primero.
pub(super) fn hojas(n: &XmlNode, out: &mut Leidos) {
    for c in &n.children {
        if c.tag == "Result" || c.tag == "ErrorString" {
            continue;
        }
        if c.children.is_empty() {
            out.entry(c.tag.clone()).or_insert_with(|| c.text.clone());
        } else {
            hojas(c, out);
        }
    }
}

/// Elementos de cada instancia de una lista, en orden. Normalmente cada
/// instancia es un elemento con hijos. Si la respuesta trae los valores
/// seguidos sin elemento que los agrupe (como los pares nombre/papel de
/// los usuarios del router), solo cuentan las hojas de `elementos` (los
/// que leen los campos y el id) y empieza una instancia cada vez que se
/// repite una. Supone que el router da los elementos de cada instancia
/// juntos y antes que los de la siguiente; si a una le faltase alguno,
/// ese campo se vería vacío o desconocido, sin mezclarse con otra.
pub(super) fn instancias(n: &XmlNode, elementos: &[&str]) -> Vec<Leidos> {
    let grupos: Vec<&XmlNode> = n
        .children
        .iter()
        .filter(|c| !c.children.is_empty())
        .collect();
    if !grupos.is_empty() {
        return grupos
            .into_iter()
            .map(|g| {
                let mut leidos = Leidos::new();
                hojas(g, &mut leidos);
                leidos
            })
            .collect();
    }
    let mut out = Vec::new();
    let mut actual = Leidos::new();
    for c in &n.children {
        let es_suyo =
            c.children.is_empty() && elementos.iter().any(|e| e.eq_ignore_ascii_case(&c.tag));
        if !es_suyo {
            continue;
        }
        if actual.contains_key(&c.tag) {
            out.push(std::mem::take(&mut actual));
        }
        actual.insert(c.tag.clone(), c.text.clone());
    }
    if !actual.is_empty() {
        out.push(actual);
    }
    out
}

/// Elemento que da el id de cada instancia de una lista: el parámetro de
/// instancia de su función de escritura o, si no escribe, el de su
/// lectura.
fn id_instancia<'f>(f: &'f Catalogo, a: &Ajuste) -> Option<&'f str> {
    a.campos
        .iter()
        .filter_map(|c| c.escribe.as_ref())
        .map(|e| e.funcion.as_str())
        .chain(std::iter::once(a.leer.funcion.as_str()))
        .filter_map(|nombre| f.funcion(nombre))
        .find_map(|func| func.lista.as_ref())
        .map(|l| l.id_param.as_str())
}

fn tecnico(a: &Ajuste) -> DetallesTecnicos {
    let mut lectura = a.leer.funcion.clone();
    if !a.leer.params.is_empty() {
        let params: Vec<String> = a
            .leer
            .params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        lectura = format!("{lectura} ({})", params.join(", "));
    }
    let mut escrituras: Vec<String> = Vec::new();
    let mut parametros = Vec::new();
    for c in &a.campos {
        match &c.escribe {
            Some(e) => {
                if !escrituras.contains(&e.funcion) {
                    escrituras.push(e.funcion.clone());
                }
                parametros.push((c.id.clone(), e.parametro.clone()));
            }
            None => parametros.push((c.id.clone(), c.lee.clone())),
        }
        parametros.extend(
            c.ademas
                .iter()
                .map(|ad| (c.id.clone(), ad.parametro.clone())),
        );
    }
    DetallesTecnicos {
        lectura,
        escrituras,
        parametros,
    }
}
