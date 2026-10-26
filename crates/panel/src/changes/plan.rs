//! Planificación de cambios, sin E/S: carga a enviar, diff, inversa,
//! comparación con lo que muestra el router, validación y enmascarado.

use super::{CampoDiff, ChangeRequest, Diff};
use catalogo::{
    Funcion, Parametro, Tipo, TipoParam,
    mask::{MASCARA, Mascara},
};
use hgu::{fmt::split_mac, xml::XmlNode};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::Ipv4Addr,
};

/// Palabra que hay que escribir para reiniciar el router.
pub const PALABRA_REINICIO: &str = "REINICIAR";
/// Palabra para las demás acciones.
pub const PALABRA_ACCION: &str = "CONFIRMAR";
pub(super) const REBOOT: &str = "cbReboot";

mod lectura;
pub(crate) use lectura::identidad;
pub(super) use lectura::{Lectura, lectura_de};

// ------------------------------------------------------------- planes

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Operacion {
    Editar,
    Crear,
    Borrar,
    Accion,
}

impl Operacion {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Operacion::Editar => "editar",
            Operacion::Crear => "crear",
            Operacion::Borrar => "borrar",
            Operacion::Accion => "accion",
        }
    }

    pub(super) fn parse(s: &str) -> Option<Operacion> {
        [
            Operacion::Editar,
            Operacion::Crear,
            Operacion::Borrar,
            Operacion::Accion,
        ]
        .into_iter()
        .find(|o| o.as_str() == s)
    }
}

/// Un campo que cambia, con los valores en claro (solo en memoria; su
/// `Debug` tapa los de un secreto).
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Campo {
    pub(super) nombre: String,
    pub(super) antes: String,
    pub(super) despues: String,
    pub(super) secreto: bool,
    /// El router mostraba el campo: `antes` es su valor real.
    pub(super) leido: bool,
}

impl std::fmt::Debug for Campo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let valor = |v: &str| {
            if self.secreto {
                "***".to_owned()
            } else {
                v.to_owned()
            }
        };
        f.debug_struct("Campo")
            .field("nombre", &self.nombre)
            .field("antes", &valor(&self.antes))
            .field("despues", &valor(&self.despues))
            .field("secreto", &self.secreto)
            .field("leido", &self.leido)
            .finish()
    }
}

/// Cambio listo para escribir y comprobar. Su `Debug` enseña los nombres de
/// los valores y del payload, no los valores (llevan los secretos en claro).
#[derive(Clone)]
pub(super) struct Plan {
    pub(super) funcion: Funcion,
    pub(super) operacion: Operacion,
    pub(super) instancia: Option<String>,
    /// Valores de la petición, con los secretos sin tocar sustituidos por el
    /// valor actual (o quitados si no se conoce) y el id de la instancia.
    pub(super) valores: BTreeMap<String, String>,
    /// Lo que se envía (sin `sessionKey`).
    pub(super) payload: Vec<(String, String)>,
    pub(super) campos: Vec<Campo>,
    pub(super) lectura: Lectura,
    /// Al crear: ids de la lista antes de escribir. La instancia creada es la
    /// que aparece nueva (no la primera con esos valores: puede haber una
    /// igual de antes). `None` si no se leyó (cambios anteriores a v0.1.1).
    pub(super) ids_antes: Option<BTreeSet<String>>,
}

impl std::fmt::Debug for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plan")
            .field("funcion", &self.funcion.name)
            .field("operacion", &self.operacion)
            .field("instancia", &self.instancia)
            .field("valores", &self.valores.keys().collect::<Vec<_>>())
            .field(
                "payload",
                &self.payload.iter().map(|(k, _)| k).collect::<Vec<_>>(),
            )
            .field("campos", &self.campos)
            .field("lectura", &self.lectura)
            .field("ids_antes", &self.ids_antes)
            .finish()
    }
}

/// Valor de `nombre` en `m`: primero exacto, luego sin distinguir mayúsculas.
pub(crate) fn buscar<'a>(m: &'a BTreeMap<String, String>, nombre: &str) -> Option<&'a str> {
    m.get(nombre)
        .or_else(|| {
            m.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nombre))
                .map(|(_, v)| v)
        })
        .map(String::as_str)
}

/// Iguales una vez normalizados: sin espacios en los extremos; IP y MAC sin
/// distinguir mayúsculas.
pub(super) fn iguales(p: Option<&Parametro>, a: &str, b: &str) -> bool {
    let (a, b) = (a.trim(), b.trim());
    match p.map(|p| p.tipo) {
        Some(TipoParam::Ip | TipoParam::Mac) => a.eq_ignore_ascii_case(b),
        _ => a == b,
    }
}

/// ¿El gestor trata `nombre` como secreto? Lo es por el catálogo o por la
/// máscara en cualquiera de sus dos niveles: un dato sensible (un usuario,
/// una URL) también va cifrado, enmascarado en el diff guardado y en los
/// registros, y bloqueado en la web básica, aunque en pantalla se vea en
/// claro ([`para_mostrar`]).
pub(super) fn es_secreto_param(f: &Funcion, m: &Mascara, nombre: &str) -> bool {
    es_credencial(f, m, nombre) || m.es_protegido(nombre)
}

/// ¿Es `nombre` una credencial (contraseña, clave, PIN), por el catálogo o
/// por la máscara? Solo esas van censuradas en pantalla y, vacías en una
/// petición, quieren decir «sin tocar»: un dato sensible vacío se borra.
pub(super) fn es_credencial(f: &Funcion, m: &Mascara, nombre: &str) -> bool {
    f.parametro(nombre)
        .is_some_and(|p| p.tipo == TipoParam::Secret)
        || m.es_secreto(nombre)
}

/// Comprueba el valor nuevo de un parámetro. Con `protegido` (un secreto o
/// un dato sensible, [`es_secreto_param`]) o un `secret` del catálogo, el
/// error no lleva el valor: acaba en la respuesta y puede acabar en un
/// registro.
pub(super) fn validar(p: &Parametro, v: &str, protegido: bool) -> Result<(), String> {
    let n = &p.nombre;
    let ok = match p.tipo {
        TipoParam::Ip => v.trim().parse::<Ipv4Addr>().is_ok(),
        TipoParam::Mac => split_mac(v.trim()).is_some_and(|parts| {
            parts
                .iter()
                .all(|x| x.len() == 2 && x.bytes().all(|b| b.is_ascii_hexdigit()))
        }),
        TipoParam::Bool => matches!(v.trim(), "0" | "1"),
        TipoParam::Enum => p.opciones.iter().any(|(o, _)| o == v.trim()),
        TipoParam::Int => v.trim().parse::<i64>().is_ok(),
        TipoParam::Text | TipoParam::Secret => !v.chars().any(char::is_control),
    };
    if ok {
        Ok(())
    } else if protegido || p.tipo == TipoParam::Secret {
        Err(format!("{n}: contiene caracteres no válidos"))
    } else {
        Err(format!("{n}: «{v}» no es un valor válido"))
    }
}

/// Campos hoja de un elemento.
pub(super) fn hojas(n: &XmlNode) -> BTreeMap<String, String> {
    n.children
        .iter()
        .filter(|c| c.children.is_empty() && c.tag != "Result" && c.tag != "ErrorString")
        .map(|c| (c.tag.clone(), c.text.clone()))
        .collect()
}

/// Elementos (hijos con hijos) de una respuesta; si no hay, la propia
/// respuesta.
pub(super) fn elementos(n: &XmlNode) -> Vec<BTreeMap<String, String>> {
    let v: Vec<_> = n
        .children
        .iter()
        .filter(|c| !c.children.is_empty())
        .map(hojas)
        .collect();
    if v.is_empty() { vec![hojas(n)] } else { v }
}

/// Resultado de [`componer`].
pub(super) struct Compuesto {
    /// Valores de la petición con los secretos sin tocar resueltos.
    pub(super) efectivos: BTreeMap<String, String>,
    /// Carga a enviar.
    pub(super) payload: Vec<(String, String)>,
    /// Campos que cambian.
    pub(super) campos: Vec<Campo>,
    /// Secretos que la carga reenviaría desde la lectura sin que valga como
    /// clave: uno que el catálogo marca como tapado al leer
    /// ([`catalogo::Parametro::tapado`]) o uno que llega enmascarado
    /// ([`enmascarado`]). Reenviarlos podría borrar la clave del router: el
    /// gestor no escribe. Siempre vacío con `explicito` (ahí se omiten).
    pub(super) secretos_tapados: Vec<String>,
}

/// ¿Es una máscara y no una clave? La del panel, o solo asteriscos o puntos.
pub(crate) fn enmascarado(v: &str) -> bool {
    let v = v.trim();
    !v.is_empty() && (v == MASCARA || v.chars().all(|c| matches!(c, '*' | '•' | '●')))
}

/// ¿El valor leído (o guardado de una lectura) del secreto `p` no sirve para
/// reenviarlo? Si el catálogo lo marca como tapado al leer, vacío o
/// enmascarado; si no, solo enmascarado (uno vacío, como una clave WEP sin
/// usar, se reenvía tal cual).
fn no_reenviable(p: &Parametro, v: &str) -> bool {
    enmascarado(v) || (p.tapado && v.trim().is_empty())
}

/// Une valores de la petición y valores actuales. No hace peticiones.
///
/// `explicito`: los valores son exactos (reversión o deshacer, con valores
/// anteriores reales): un secreto vacío se escribe vacío en vez de
/// entenderse como "sin tocar".
pub(super) fn componer(
    f: &Funcion,
    m: &Mascara,
    op: Operacion,
    actual: &BTreeMap<String, String>,
    valores: &BTreeMap<String, String>,
    explicito: bool,
) -> Compuesto {
    let ident = identidad(f);
    // Sin tocar (un secreto vacío o cualquier campo que llega enmascarado):
    // valor actual, o fuera si no se conoce. `MASCARA` nunca se escribe.
    let mut efectivos = BTreeMap::new();
    // Lo que se reenvía tal como lo da la lectura.
    let mut de_la_lectura = BTreeSet::new();
    for (k, v) in valores {
        let p = f.parametro(k);
        let vacio = !explicito && es_credencial(f, m, k) && v.trim().is_empty();
        if v == MASCARA || vacio {
            if let Some(cur) = buscar(actual, k) {
                efectivos.insert(k.clone(), cur.to_owned());
                de_la_lectura.insert(k.clone());
            }
            continue;
        }
        let v = match p.map(|p| p.tipo) {
            Some(TipoParam::Text | TipoParam::Secret) | None => v.clone(),
            Some(_) => v.trim().to_owned(),
        };
        efectivos.insert(k.clone(), v);
    }
    let mut payload = Vec::new();
    let mut secretos_tapados = Vec::new();
    // Con `explicito` (reversión o deshacer), un secreto que no sirve para
    // reenviarlo se omite: la vuelta atrás no se bloquea nunca por esto.
    let mut omitidos = BTreeSet::new();
    for p in &f.parametros {
        let (v, leido) = if let Some(v) = efectivos.get(&p.nombre) {
            (v.as_str(), de_la_lectura.contains(&p.nombre))
        } else if op == Operacion::Editar
            && let Some(cur) = buscar(actual, &p.nombre)
        {
            (cur, true)
        } else {
            continue;
        };
        let secreto = es_secreto_param(f, m, &p.nombre);
        // Lo que viene de una lectura: la de ahora, con `explicito` el valor
        // anterior que se guardó de la de entonces, o lo pedido si es lo
        // mismo que se lee (un dato sensible va en claro en el formulario y
        // vuelve tal cual) o una máscara (la de una foto de partida). Nadie
        // escribe `****` como usuario ni como clave a propósito.
        let como_lo_leido = buscar(actual, &p.nombre).is_some_and(|cur| cur == v);
        if secreto && (leido || explicito || como_lo_leido || enmascarado(v)) && no_reenviable(p, v)
        {
            if explicito {
                omitidos.insert(p.nombre.clone());
                continue;
            }
            secretos_tapados.push(p.nombre.clone());
        }
        payload.push((p.nombre.clone(), v.to_owned()));
    }
    for k in &omitidos {
        efectivos.remove(k);
    }
    let secreto =
        |k: &str, a: &str, b: &str| es_secreto_param(f, m, k) || m.oculta(k, a) || m.oculta(k, b);
    let mut campos = Vec::new();
    if op == Operacion::Borrar {
        payload.push(("del".into(), "DEL".into()));
        for p in &f.parametros {
            if ident.contains(&p.nombre)
                || efectivos.contains_key(&p.nombre)
                || omitidos.contains(&p.nombre)
            {
                continue;
            }
            if let Some(cur) = buscar(actual, &p.nombre) {
                campos.push(Campo {
                    nombre: p.nombre.clone(),
                    antes: cur.to_owned(),
                    despues: String::new(),
                    secreto: secreto(&p.nombre, cur, ""),
                    leido: true,
                });
            }
        }
    } else {
        for p in &f.parametros {
            let Some(v) = efectivos.get(&p.nombre) else {
                continue;
            };
            if ident.contains(&p.nombre) && op != Operacion::Accion {
                continue;
            }
            let cur = buscar(actual, &p.nombre);
            let antes = cur.unwrap_or_default();
            if op != Operacion::Accion && iguales(Some(p), antes, v) {
                continue;
            }
            campos.push(Campo {
                nombre: p.nombre.clone(),
                antes: antes.to_owned(),
                despues: v.clone(),
                secreto: secreto(&p.nombre, antes, v),
                leido: cur.is_some(),
            });
        }
    }
    Compuesto {
        efectivos,
        payload,
        campos,
        secretos_tapados,
    }
}

/// Petición que deshace `plan` (`None` si no se puede: acciones, o sin
/// valores anteriores conocidos).
pub(super) fn inversa(plan: &Plan) -> Option<ChangeRequest> {
    let f = &plan.funcion;
    let ids = identidad(f);
    let id_param = f.lista.as_ref().map(|l| l.id_param.as_str());
    // Lo que identifica la instancia (el id de la lista va en `instancia`).
    let mut valores: BTreeMap<String, String> = plan
        .valores
        .iter()
        .filter(|(k, _)| ids.contains(*k) && Some(k.as_str()) != id_param)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let anteriores: Vec<(String, String)> = plan
        .campos
        .iter()
        .filter(|c| c.leido)
        .map(|c| (c.nombre.clone(), c.antes.clone()))
        .collect();
    let (instancia, borrar) = match plan.operacion {
        Operacion::Accion => return None,
        Operacion::Editar if anteriores.is_empty() => return None,
        Operacion::Editar => {
            valores.extend(anteriores);
            (plan.instancia.clone(), false)
        }
        Operacion::Crear => (Some(plan.instancia.clone()?), true),
        Operacion::Borrar => {
            valores.extend(anteriores);
            (None, false)
        }
    };
    // Un usuario borrado vuelve con `cbAddUser`, aunque el borrado llegase
    // como `cbDeleteUser` (de la Biblioteca); la contraseña la pone quien
    // lo deshace (`especiales::con_contrasena_nueva`).
    // Igual la regla del cortafuegos, cuya baja se anota con la función
    // que se envía (la de quitar).
    let funcion = match (plan.operacion, &plan.lectura) {
        (Operacion::Borrar, Lectura::Usuarios) => hgu::fmt::ALTA_USUARIO.to_owned(),
        (Operacion::Borrar, Lectura::Cortafuegos) => hgu::cortafuegos::ALTA.to_owned(),
        _ => f.name.clone(),
    };
    // La baja de la regla «sin ping» va atada al filtro donde se creó.
    let identidad = match (plan.operacion, &plan.lectura) {
        (Operacion::Crear, Lectura::Cortafuegos) => {
            let belong = super::especiales::BELONG_CORTAFUEGOS;
            plan.valores
                .get(belong)
                .map(|fid| (belong.to_owned(), fid.clone()))
                .into_iter()
                .collect()
        }
        _ => BTreeMap::new(),
    };
    Some(ChangeRequest {
        funcion,
        instancia,
        valores,
        borrar,
        identidad,
    })
}

/// Qué muestra el router respecto a un plan.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Comprobacion {
    /// Coincide; con la instancia nueva si se creó una.
    Coincide(Option<String>),
    /// No hay forma de comprobarlo (sin lectura o sin campos comparables).
    Limitada,
    /// No coincide (motivos, secretos enmascarados).
    Difiere(Vec<String>),
    /// No se pudo leer el estado del router (error transitorio).
    SinLectura(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Lado {
    Nuevo,
    Anterior,
}

/// Compara los campos del plan (lado `lado`) con lo que muestra el router.
/// `Limitada` si el router no muestra ninguno.
pub(super) fn comparar(
    f: &Funcion,
    campos: &[Campo],
    actual: &BTreeMap<String, String>,
    lado: Lado,
) -> Comprobacion {
    let mut comparados = 0;
    let mut fallos = Vec::new();
    for c in campos {
        if lado == Lado::Anterior && !c.leido {
            continue;
        }
        let Some(leido) = buscar(actual, &c.nombre) else {
            continue;
        };
        // Uno que el router da tapado al leer no se puede comprobar: lo que
        // lee no es su valor (cuentan los demás campos del paso).
        if f.parametro(&c.nombre).is_some_and(|p| p.tapado) {
            continue;
        }
        comparados += 1;
        let esperado = match lado {
            Lado::Nuevo => &c.despues,
            Lado::Anterior => &c.antes,
        };
        if !iguales(f.parametro(&c.nombre), leido, esperado) {
            let ver = |v: &str| {
                if c.secreto {
                    MASCARA.to_owned()
                } else {
                    format!("«{v}»")
                }
            };
            fallos.push(format!(
                "{}: se esperaba {}, el router tiene {}",
                c.nombre,
                ver(esperado),
                ver(leido)
            ));
        }
    }
    if comparados == 0 {
        Comprobacion::Limitada
    } else if fallos.is_empty() {
        Comprobacion::Coincide(None)
    } else {
        Comprobacion::Difiere(fallos)
    }
}

/// ¿Hay internet? Con `Some(n)` (la conexión a internet detectada o elegida
/// a mano), la conexión `n` está arriba. Con `None` (sin detectar), alguna
/// conexión de datos (ni televisión ni telefonía) está arriba con IPv4: la
/// red de seguridad del gestor nunca se apaga por no saber cuál es.
pub fn wan_arriba(conns: &[hgu::models::WanConn], wan: Option<&str>) -> bool {
    match wan {
        Some(n) => conns.iter().any(|c| c.name == n && c.up),
        None => conns
            .iter()
            .any(|c| c.es_de_datos() && c.up && c.ipv4.is_some()),
    }
}

pub(super) fn palabra_requerida(f: &Funcion) -> Option<&'static str> {
    match f.tipo {
        Tipo::Accion if f.name == REBOOT => Some(PALABRA_REINICIO),
        Tipo::Accion => Some(PALABRA_ACCION),
        _ => None,
    }
}

/// Diff para mostrar y guardar: secretos enmascarados.
pub(super) fn visible(campos: &[Campo]) -> Diff {
    let ver = |c: &Campo, v: &str| {
        if c.secreto {
            MASCARA.to_owned()
        } else {
            v.to_owned()
        }
    };
    Diff {
        campos: campos
            .iter()
            .map(|c| CampoDiff {
                nombre: c.nombre.clone(),
                antes: ver(c, &c.antes),
                despues: ver(c, &c.despues),
                secreto: c.secreto,
                sensible: false,
            })
            .collect(),
    }
}

/// Diff para la pantalla (la confirmación de un cambio): solo las
/// credenciales enmascaradas ([`es_credencial`], o un valor que la máscara
/// censura en pantalla); los datos sensibles, en claro y marcados
/// `sensible` (su `Debug` los sigue tapando). No se guarda: lo guardado es
/// [`visible`].
pub(super) fn para_mostrar(f: &Funcion, m: &Mascara, campos: &[Campo]) -> Diff {
    let mut d = visible(campos);
    for (cd, c) in d.campos.iter_mut().zip(campos) {
        let credencial = es_credencial(f, m, &c.nombre)
            || m.oculta_en_pantalla(&c.nombre, &c.antes)
            || m.oculta_en_pantalla(&c.nombre, &c.despues);
        if c.secreto && !credencial {
            cd.antes = c.antes.clone();
            cd.despues = c.despues.clone();
            cd.secreto = false;
            cd.sensible = true;
        }
    }
    d
}

/// Resumen del diff para los registros (secretos enmascarados).
pub(super) fn resumen(campos: &[Campo]) -> String {
    visible(campos)
        .campos
        .iter()
        .map(|c| format!("{}: «{}» → «{}»", c.nombre, c.antes, c.despues))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_sensibles;
