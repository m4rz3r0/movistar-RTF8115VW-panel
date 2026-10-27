//! El editor completo del cortafuegos en el gestor: alta, edición y baja de reglas y de filtros, con la carga de la web, sus guardas y la comprobación por relectura.
//!
//! Cada petición lleva su función del catálogo y su forma ([`Pide`]):
//!
//! - regla nueva: `add.regla` sin instancia, con los valores de la regla
//!   (`belong` es su filtro); se escribe [`cf::carga_regla`] sin id (con
//!   `escape=true`) y va al final del filtro, como en la web;
//! - regla cambiada: `modify` con el RID como instancia y la regla entera;
//! - regla borrada: `add.regla` con `borrar`, el RID como instancia y, en la
//!   identidad, su filtro (`belong`); se escribe `item=<FID>|<RID>`;
//! - filtro nuevo, cambiado o borrado: `add.filtro`, `modify.filtro` (el
//!   FID como instancia) y `remove.filtro` (o `add.filtro` con `borrar`, el
//!   deshacer de un alta). Solo se borra un filtro vacío: `item=<FID>`
//!   borraría también sus reglas y el deshacer no podría devolverlas.
//!
//! Solo se cambia o se borra una regla o un filtro que el panel sabe
//! volver a escribir igual ([`cf::regla_reescribible`],
//! [`cf::filtro_reescribible`]): si no, su deshacer no podría dejarlo como
//! estaba. La identidad que manda la página (los valores que enseñaba) se
//! compara con lo que hay en el router justo antes de escribir.

use super::*;
use hgu::cortafuegos::{
    ALTA_FILTRO, ALTA_REGLA, BAJA_FILTRO, MODIFICAR, MODIFICAR_FILTRO, PARAMS_FILTRO, PARAMS_REGLA,
    Regla, Valores, es_de_filtro,
};

/// Lo que pide una petición del editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pide {
    ReglaNueva,
    ReglaCambiada,
    ReglaBorrada,
    FiltroNuevo,
    FiltroCambiado,
    FiltroBorrado,
}

/// Lo que pide `req` por su función, `borrar` y si trae instancia.
fn pide(req: &ChangeRequest) -> Option<Pide> {
    match (req.funcion.as_str(), req.borrar, req.instancia.is_some()) {
        (ALTA_REGLA, false, false) => Some(Pide::ReglaNueva),
        (ALTA_REGLA, true, true) => Some(Pide::ReglaBorrada),
        (MODIFICAR, false, true) => Some(Pide::ReglaCambiada),
        (ALTA_FILTRO, false, false) => Some(Pide::FiltroNuevo),
        (ALTA_FILTRO | BAJA_FILTRO, true, true) => Some(Pide::FiltroBorrado),
        (MODIFICAR_FILTRO, false, true) => Some(Pide::FiltroCambiado),
        _ => None,
    }
}

/// ¿Vale `v` como regla del filtro de la versión que trae (o IPv4)?
fn regla_valida(v: &BTreeMap<String, String>) -> bool {
    let version = v.get("version").map_or("", String::as_str);
    cf::normalizar_regla(v, version).is_ok()
}

/// ¿Tiene `req` la forma de una petición del editor? La instancia, un
/// número; una baja, sin valores; un alta o un cambio, con los valores
/// de una regla o un filtro que valen. Las guardas que miran el router
/// van en el plan.
pub(super) fn exento(req: &ChangeRequest) -> bool {
    let Some(p) = pide(req) else {
        return false;
    };
    if req.instancia.as_deref().is_some_and(|i| !es_numero(i)) {
        return false;
    }
    match p {
        Pide::ReglaNueva | Pide::ReglaCambiada => regla_valida(&req.valores),
        Pide::FiltroNuevo | Pide::FiltroCambiado => cf::normalizar_filtro(&req.valores).is_ok(),
        Pide::ReglaBorrada | Pide::FiltroBorrado => req.valores.is_empty(),
    }
}

/// ¿Sabe el panel volver a escribir `r` del filtro `f` tal como está
/// ([`cf::regla_reescribible`])? Si no, su deshacer no podría dejarla
/// como estaba.
fn reescribible(f: &Filtro, r: &Regla) -> bool {
    cf::regla_reescribible(f, r)
}

/// Un filtro se cambia o se borra solo si el panel sabe volver a
/// escribirlo igual (su deshacer): con un nombre que la web no deja
/// poner, no.
fn filtro_reescribible(actual: &Valores, fid: &str) -> Result<(), CambioError> {
    cf::normalizar_filtro(actual).map(drop).map_err(|e| {
        CambioError::Invalida(format!(
            "el filtro {fid} no se puede volver a escribir igual ({e}): no se toca"
        ))
    })
}

/// La regla `rid` y su filtro.
fn buscar_regla<'a>(filtros: &'a [Filtro], rid: &str) -> Option<(&'a Filtro, &'a Regla)> {
    filtros
        .iter()
        .find_map(|f| f.reglas.iter().find(|r| r.rid == rid).map(|r| (f, r)))
}

/// La carga como pares para el plan.
fn pares(carga: Vec<(&'static str, String)>) -> Vec<(String, String)> {
    carga.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

fn campo(nombre: &str, antes: &str, despues: &str, leido: bool) -> Campo {
    Campo {
        nombre: nombre.to_owned(),
        antes: antes.to_owned(),
        despues: despues.to_owned(),
        secreto: false,
        leido,
    }
}

fn valor<'a>(v: &'a Valores, k: &str) -> &'a str {
    v.get(k).map_or("", String::as_str)
}

/// Los campos de un alta: los que tienen valor.
fn campos_de_alta(params: &[&str], v: &Valores) -> Vec<Campo> {
    params
        .iter()
        .filter(|k| !valor(v, k).is_empty())
        .map(|k| campo(k, "", valor(v, k), false))
        .collect()
}

/// Los campos de un cambio o una baja: todos, con lo leído (el deshacer
/// vuelve a escribir la regla o el filtro entero).
fn campos_leidos(params: &[&str], antes: &Valores, despues: Option<&Valores>) -> Vec<Campo> {
    params
        .iter()
        .map(|k| {
            campo(
                k,
                valor(antes, k),
                despues.map_or("", |d| valor(d, k)),
                true,
            )
        })
        .collect()
}

/// `Invalida` con el campo que no vale.
fn invalida(e: cf::CampoMal) -> CambioError {
    CambioError::Invalida(e.to_string())
}

/// ¿Sigue igual en el router lo que la página enseñaba (`identidad`)? Si
/// no, `InstanciaCambiada` sin escribir.
fn misma_identidad(
    identidad: &BTreeMap<String, String>,
    actual: &Valores,
    que: &str,
) -> Result<(), CambioError> {
    let campos: Vec<&str> = identidad.keys().map(String::as_str).collect();
    if cf::coincide(actual, identidad, &campos) {
        Ok(())
    } else {
        Err(CambioError::InstanciaCambiada(format!(
            "{que} ya no es igual en el router: no se escribe"
        )))
    }
}

/// Los filtros como el gestor los ve: sus FID.
fn fids(filtros: &[Filtro]) -> BTreeSet<String> {
    filtros.iter().map(|f| f.fid.clone()).collect()
}

/// Los valores de los campos de `plan` en el lado `lado`.
fn esperado(plan: &Plan, lado: Lado) -> Valores {
    match lado {
        Lado::Nuevo => plan.valores.clone(),
        Lado::Anterior => plan
            .campos
            .iter()
            .map(|c| (c.nombre.clone(), c.antes.clone()))
            .collect(),
    }
}

impl ChangeManager {
    /// Plan de una petición del editor (ver el módulo).
    pub(super) async fn planificar_editor(
        &self,
        f: &Funcion,
        req: &ChangeRequest,
    ) -> Result<Plan, CambioError> {
        let p = pide(req).filter(|_| exento(req)).ok_or_else(|| {
            CambioError::Invalida(format!("{}: no es una petición del editor", f.name))
        })?;
        let filtros = self.filtros().await?;
        let id = req.instancia.clone().unwrap_or_default();
        let plan = |operacion: Operacion,
                    valores: Valores,
                    payload: Vec<(String, String)>,
                    campos: Vec<Campo>,
                    ids_antes: Option<BTreeSet<String>>| Plan {
            funcion: f.clone(),
            operacion,
            instancia: (operacion != Operacion::Crear).then(|| id.clone()),
            valores,
            payload,
            campos,
            lectura: Lectura::Cortafuegos,
            ids_antes,
        };
        let regla_que_no_esta = || {
            CambioError::InstanciaCambiada(format!(
                "la regla {id} ya no está en el router: no se escribe"
            ))
        };
        let filtro_que_no_esta = |fid: &str| {
            CambioError::InstanciaCambiada(format!(
                "el filtro {fid} ya no está en el router: no se escribe"
            ))
        };
        match p {
            Pide::ReglaNueva => {
                let fid = valor(&req.valores, "belong");
                let filtro = filtros
                    .iter()
                    .find(|x| x.fid == fid)
                    .ok_or_else(|| filtro_que_no_esta(fid))?;
                let v = cf::normalizar_regla(&req.valores, &filtro.version).map_err(invalida)?;
                let rids = filtros
                    .iter()
                    .flat_map(|x| &x.reglas)
                    .map(|r| r.rid.clone())
                    .collect();
                Ok(plan(
                    Operacion::Crear,
                    v.clone(),
                    pares(cf::carga_regla(&v, None)),
                    campos_de_alta(&PARAMS_REGLA, &v),
                    Some(rids),
                ))
            }
            Pide::ReglaCambiada => {
                let (filtro, r) = buscar_regla(&filtros, &id).ok_or_else(regla_que_no_esta)?;
                let actual = cf::vista_regla(filtro, r);
                misma_identidad(&req.identidad, &actual, "la regla")?;
                if !reescribible(filtro, r) {
                    return Err(CambioError::Invalida(format!(
                        "la regla {id} no se puede volver a escribir igual: no se cambia"
                    )));
                }
                if valor(&req.valores, "belong") != filtro.fid {
                    return Err(CambioError::Invalida(
                        "una regla no cambia de filtro: bórrala y créala en el otro".into(),
                    ));
                }
                let v = cf::normalizar_regla(&req.valores, &filtro.version).map_err(invalida)?;
                Ok(plan(
                    Operacion::Editar,
                    v.clone(),
                    pares(cf::carga_regla(&v, Some(&id))),
                    campos_leidos(&PARAMS_REGLA, &actual, Some(&v)),
                    None,
                ))
            }
            Pide::ReglaBorrada => {
                let (filtro, r) = buscar_regla(&filtros, &id).ok_or_else(regla_que_no_esta)?;
                if !es_numero(&filtro.fid) {
                    return Err(CambioError::Invalida(
                        "el filtro de la regla no tiene un identificador numérico: no se borra"
                            .into(),
                    ));
                }
                // La identidad lleva el filtro donde estaba (la página o el
                // deshacer del alta): con otro, sería otra regla.
                if req.identidad.get(BELONG) != Some(&filtro.fid) {
                    return Err(CambioError::InstanciaCambiada(format!(
                        "la regla {id} ya no está en el filtro donde estaba: no se borra, \
                         para no tocar otra"
                    )));
                }
                let actual = cf::vista_regla(filtro, r);
                misma_identidad(&req.identidad, &actual, "la regla")?;
                if !reescribible(filtro, r) {
                    return Err(CambioError::Invalida(format!(
                        "la regla {id} no se puede volver a escribir igual: no se borra"
                    )));
                }
                let valores: Valores = [(BELONG.to_owned(), filtro.fid.clone())].into();
                Ok(plan(
                    Operacion::Borrar,
                    valores.clone(),
                    baja(&id, &valores).1,
                    campos_leidos(&PARAMS_REGLA, &actual, None),
                    Some(filtro.reglas.iter().map(|r| r.rid.clone()).collect()),
                ))
            }
            Pide::FiltroNuevo => {
                let v = cf::normalizar_filtro(&req.valores).map_err(invalida)?;
                Ok(plan(
                    Operacion::Crear,
                    v.clone(),
                    pares(cf::carga_filtro(&v, None)),
                    campos_de_alta(&PARAMS_FILTRO, &v),
                    Some(fids(&filtros)),
                ))
            }
            Pide::FiltroCambiado => {
                let filtro = filtros
                    .iter()
                    .find(|x| x.fid == id)
                    .ok_or_else(|| filtro_que_no_esta(&id))?;
                let actual = cf::vista_filtro(filtro);
                misma_identidad(&req.identidad, &actual, "el filtro")?;
                filtro_reescribible(&actual, &id)?;
                let v = cf::normalizar_filtro(&req.valores).map_err(invalida)?;
                if !filtro.reglas.is_empty() && valor(&v, "version") != filtro.version {
                    return Err(CambioError::Invalida(
                        "un filtro con reglas no cambia de versión de IP".into(),
                    ));
                }
                Ok(plan(
                    Operacion::Editar,
                    v.clone(),
                    pares(cf::carga_filtro(&v, Some(&id))),
                    campos_leidos(&PARAMS_FILTRO, &actual, Some(&v)),
                    None,
                ))
            }
            Pide::FiltroBorrado => {
                let filtro = filtros
                    .iter()
                    .find(|x| x.fid == id)
                    .ok_or_else(|| filtro_que_no_esta(&id))?;
                let actual = cf::vista_filtro(filtro);
                misma_identidad(&req.identidad, &actual, "el filtro")?;
                if !filtro.reglas.is_empty() {
                    return Err(CambioError::InstanciaCambiada(format!(
                        "el filtro {id} tiene reglas: solo se borra vacío"
                    )));
                }
                filtro_reescribible(&actual, &id)?;
                Ok(plan(
                    Operacion::Borrar,
                    Valores::new(),
                    baja_de_filtro(&id).1,
                    campos_leidos(&PARAMS_FILTRO, &actual, None),
                    Some(fids(&filtros)),
                ))
            }
        }
    }

    /// ¿Muestra el router el lado `lado` de un plan del editor? Se relee la
    /// lista de filtros y se compara en forma de formulario
    /// ([`cf::coincide`]).
    pub(super) async fn comprobar_editor(&self, plan: &Plan, lado: Lado) -> Comprobacion {
        let filtros = match self.filtros().await {
            Ok(f) => f,
            Err(e) => return Comprobacion::SinLectura(e.to_string()),
        };
        if es_de_filtro(&plan.funcion.name) {
            comprobar_filtro(&filtros, plan, lado)
        } else {
            comprobar_regla(&filtros, plan, lado)
        }
    }
}

fn difiere(m: &str) -> Comprobacion {
    Comprobacion::Difiere(vec![m.to_owned()])
}

/// ¿Es nuevo `id` (no estaba al planificar)?
fn nuevo(plan: &Plan, id: &str) -> bool {
    plan.ids_antes
        .as_ref()
        .is_none_or(|antes| !antes.contains(id))
}

/// La única nueva de `nuevas` (las que encajan con lo escrito), o por qué
/// no.
fn la_nueva(nuevas: Vec<String>, que: &str) -> Comprobacion {
    match nuevas.as_slice() {
        [] => difiere(&format!("no aparece {que}")),
        [x] => Comprobacion::Coincide(Some(x.clone())),
        varias => difiere(&format!(
            "aparecen {} ({}): no se sabe cuál es la de este cambio; no se borra nada",
            varias.len(),
            varias.join(", ")
        )),
    }
}

fn comprobar_regla(filtros: &[Filtro], plan: &Plan, lado: Lado) -> Comprobacion {
    let espera = esperado(plan, lado);
    let encaja =
        |f: &Filtro, r: &Regla| cf::coincide(&cf::vista_regla(f, r), &espera, &PARAMS_REGLA);
    let instancia = plan.instancia.as_deref();
    let esta = instancia.and_then(|rid| buscar_regla(filtros, rid));
    match (plan.operacion, lado, instancia) {
        (Operacion::Crear, Lado::Nuevo, None) => {
            let fid = valor(&plan.valores, BELONG);
            let nuevas = filtros
                .iter()
                .filter(|f| f.fid == fid)
                .flat_map(|f| f.reglas.iter().map(move |r| (f, r)))
                .filter(|(f, r)| nuevo(plan, &r.rid) && encaja(f, r))
                .map(|(_, r)| r.rid.clone())
                .collect();
            la_nueva(nuevas, "la regla nueva")
        }
        (Operacion::Crear, Lado::Anterior, None) => {
            if filtros
                .iter()
                .flat_map(|f| &f.reglas)
                .any(|r| nuevo(plan, &r.rid))
            {
                difiere("la regla nueva existe")
            } else {
                Comprobacion::Coincide(None)
            }
        }
        (Operacion::Crear, Lado::Anterior, Some(_)) | (Operacion::Borrar, Lado::Nuevo, Some(_))
            if esta.is_some() =>
        {
            difiere("la regla existe")
        }
        (Operacion::Crear, Lado::Anterior, Some(_)) => Comprobacion::Coincide(None),
        (Operacion::Borrar, Lado::Nuevo, Some(rid)) => baja_exacta(filtros, plan, rid),
        (_, _, Some(_)) => match esta {
            Some((f, r)) if encaja(f, r) => Comprobacion::Coincide(None),
            Some(_) => difiere("la regla no tiene los valores esperados"),
            None => difiere("la regla no está"),
        },
        (_, _, None) => Comprobacion::Limitada,
    }
}

fn comprobar_filtro(filtros: &[Filtro], plan: &Plan, lado: Lado) -> Comprobacion {
    let espera = esperado(plan, lado);
    let encaja = |f: &Filtro| cf::coincide(&cf::vista_filtro(f), &espera, &PARAMS_FILTRO);
    let instancia = plan.instancia.as_deref();
    let esta = instancia.and_then(|fid| filtros.iter().find(|f| f.fid == fid));
    match (plan.operacion, lado, instancia) {
        (Operacion::Crear, Lado::Nuevo, None) => {
            let nuevos = filtros
                .iter()
                .filter(|f| nuevo(plan, &f.fid) && encaja(f))
                .map(|f| f.fid.clone())
                .collect();
            la_nueva(nuevos, "el filtro nuevo")
        }
        (Operacion::Crear, Lado::Anterior, None) => {
            if filtros.iter().any(|f| nuevo(plan, &f.fid)) {
                difiere("el filtro nuevo existe")
            } else {
                Comprobacion::Coincide(None)
            }
        }
        (Operacion::Crear, Lado::Anterior, Some(_)) | (Operacion::Borrar, Lado::Nuevo, Some(_))
            if esta.is_some() =>
        {
            difiere("el filtro existe")
        }
        (Operacion::Crear, Lado::Anterior, Some(_)) => Comprobacion::Coincide(None),
        (Operacion::Borrar, Lado::Nuevo, Some(fid)) => {
            // Solo falta el suyo: un router que borra otro no lo da por bueno.
            let quedan = fids(filtros);
            let faltan: Vec<&str> = plan
                .ids_antes
                .iter()
                .flatten()
                .map(String::as_str)
                .filter(|x| *x != fid && !quedan.contains(*x))
                .collect();
            if faltan.is_empty() {
                Comprobacion::Coincide(None)
            } else {
                difiere(&format!(
                    "faltan otros filtros ({}): la baja borró más de la cuenta",
                    faltan.join(", ")
                ))
            }
        }
        (_, _, Some(_)) => match esta {
            Some(f) if encaja(f) => Comprobacion::Coincide(None),
            Some(_) => difiere("el filtro no tiene los valores esperados"),
            None => difiere("el filtro no está"),
        },
        (_, _, None) => Comprobacion::Limitada,
    }
}

#[cfg(test)]
mod tests;
