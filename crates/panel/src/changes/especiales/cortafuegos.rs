//! La regla «sin ping» del cortafuegos en el gestor: la exención del bloqueo, las guardas, el plan, la baja y la comprobación.
//!
//! Las funciones del cortafuegos están bloqueadas en el catálogo (la
//! Biblioteca no las escribe: `escribible` comprueba solo el nombre). El
//! gestor solo las escribe dentro de `planificar`, y solo con dos
//! peticiones ([`exento_de_bloqueo`]):
//!
//! - el alta, `scbidirectionalflt.cmd?action=add` con la carga exacta de
//!   [`cf::carga_sin_ping`] en el filtro de entrada IPv4 de la conexión a
//!   internet; si ya hay una regla con la huella, 409
//!   ([`SIN_PING_YA_PUESTA`]);
//! - su baja (el deshacer o la reversión): la misma función con `borrar` y
//!   el RID como instancia, sin valores. Solo si esa regla existe, tiene la
//!   huella ([`cf::es_sin_ping`]) y está en un filtro de entrada IPv4, con
//!   un FID numérico y el mismo en el que se creó (la identidad `belong`
//!   que pone el deshacer); si no, `InstanciaCambiada` (409) sin escribir. Se escribe
//!   `scbidirectionalflt.cmd?action=remove` con `item=<FID>|<RID>` y
//!   `escape=true`: nunca un FID suelto, que borraría el filtro entero.
//!
//! `modify` y las aperturas IPv6 (`scbidirectionalfwl`) no se eximen nunca.

use super::*;
use hgu::cortafuegos::{self as cf, ALTA, BAJA, Filtro};

/// Motivo de `InstanciaCambiada` cuando la regla «sin ping» ya está en el
/// router. Es la clave i18n del texto (dice «ya puesta»).
pub(crate) const SIN_PING_YA_PUESTA: &str = "ajuste-seguridad-cortafuegos-ya-puesta";

/// La carga del alta en el filtro `fid`.
fn carga(fid: &str) -> Vec<(&'static str, String)> {
    let molde = Filtro {
        fid: fid.to_owned(),
        name: String::new(),
        interface: String::new(),
        intf_id: String::new(),
        tipo: String::new(),
        version: String::new(),
        default: String::new(),
        reglas: Vec::new(),
    };
    cf::carga_sin_ping(&molde)
}

/// ¿Es `v` exactamente la carga del alta (en algún filtro)?
fn es_carga_sin_ping(v: &BTreeMap<String, String>) -> bool {
    let Some(fid) = v.get("belong").filter(|f| es_numero(f)) else {
        return false;
    };
    let esperada: BTreeMap<String, String> = carga(fid)
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
    *v == esperada
}

fn es_numero(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// ¿Se escribe `req` aunque su función esté bloqueada? Solo el alta de la
/// regla «sin ping» con su carga exacta o la baja de una regla por su RID
/// (ver el módulo).
pub(crate) fn exento_de_bloqueo(req: &ChangeRequest) -> bool {
    if req.funcion != ALTA {
        return false;
    }
    match (req.borrar, req.instancia.as_deref()) {
        (false, None) => es_carga_sin_ping(&req.valores),
        (true, Some(rid)) => req.valores.is_empty() && es_numero(rid),
        _ => false,
    }
}

/// Función y parámetros de la baja de la regla `rid` (su filtro, en
/// `valores["belong"]`).
pub(in crate::changes) fn baja(
    rid: &str,
    valores: &BTreeMap<String, String>,
) -> (&'static str, Vec<(String, String)>) {
    let fid = valores
        .get("belong")
        .map(String::as_str)
        .unwrap_or_default();
    (
        BAJA,
        vec![
            ("item".to_owned(), cf::item(fid, rid)),
            ("escape".to_owned(), "true".to_owned()),
        ],
    )
}

/// El filtro de la regla `rid` si es la del panel: con su huella
/// ([`cf::es_sin_ping`]) y en un filtro de entrada IPv4 (Ruling G).
fn filtro_de_la_regla<'a>(filtros: &'a [Filtro], rid: &str) -> Option<&'a Filtro> {
    filtros.iter().find(|x| {
        x.tipo == "0"
            && x.version == "0"
            && x.reglas.iter().any(|r| r.rid == rid && cf::es_sin_ping(r))
    })
}

/// El filtro de la baja que pide `req` (la regla `req.instancia`): el de
/// la regla del panel ([`filtro_de_la_regla`]), que tiene que ser el mismo
/// en el que se creó (`req.identidad["belong"]`, que pone el deshacer del
/// alta) y tener un FID numérico (con un `|` el `item` borraría otra cosa).
fn filtro_para_la_baja<'a>(
    filtros: &'a [Filtro],
    req: &ChangeRequest,
) -> Result<&'a Filtro, CambioError> {
    let rid = req.instancia.as_deref().unwrap_or_default();
    let filtro = filtro_de_la_regla(filtros, rid).ok_or_else(|| {
        CambioError::InstanciaCambiada(format!(
            "la regla {rid} ya no es la del panel (no está o tiene otra huella): \
             no se borra, para no tocar otra"
        ))
    })?;
    if !es_numero(&filtro.fid) {
        return Err(CambioError::Invalida(
            "el filtro de la regla no tiene un identificador numérico: no se borra".into(),
        ));
    }
    if req.identidad.get(BELONG) != Some(&filtro.fid) {
        return Err(CambioError::InstanciaCambiada(format!(
            "la regla {rid} ya no está en el filtro donde se creó: no se borra, para no \
             tocar otra"
        )));
    }
    Ok(filtro)
}

/// Parámetro del filtro de la regla en el alta; en el deshacer, la
/// identidad que ata la baja a ese filtro.
pub(in crate::changes) const BELONG: &str = "belong";

/// Cada regla como la ve el gestor, `{RID, belong, version, enabled,
/// protocol, specificAction, icmpType}` (los parámetros del alta), y si
/// tiene la huella de la regla «sin ping».
pub(crate) fn filas_del_cortafuegos(filtros: &[Filtro]) -> Vec<(BTreeMap<String, String>, bool)> {
    filtros
        .iter()
        .flat_map(|f| f.reglas.iter().map(move |r| (f, r)))
        .map(|(f, r)| {
            let fila = [
                ("RID", &r.rid),
                ("belong", &f.fid),
                ("version", &f.version),
                ("enabled", &r.enable),
                ("protocol", &r.protocol),
                ("specificAction", &r.action),
                ("icmpType", &r.icmp_type),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.clone()))
            .collect();
            (fila, cf::es_sin_ping(r))
        })
        .collect()
}

/// ¿Quedó la baja de la regla `rid` como se pidió? Su filtro
/// (`plan.valores["belong"]`) sigue en el router, ya no tiene la regla y,
/// de las que tenía al planificar (`plan.ids_antes`), solo falta ella. Un
/// router que borra el filtro entero u otra regla no da la baja por buena.
fn baja_exacta(filtros: &[Filtro], plan: &Plan, rid: &str) -> Comprobacion {
    let difiere = |m: String| Comprobacion::Difiere(vec![m]);
    let fid = plan.valores.get("belong").map(String::as_str);
    let Some(filtro) = filtros.iter().find(|f| Some(f.fid.as_str()) == fid) else {
        return difiere(format!(
            "ya no está el filtro de la regla {rid}: la baja borró más de la cuenta"
        ));
    };
    let quedan: BTreeSet<&str> = filtro.reglas.iter().map(|r| r.rid.as_str()).collect();
    if quedan.contains(rid) {
        return difiere("la regla sigue existiendo".to_owned());
    }
    let faltan: Vec<&str> = plan
        .ids_antes
        .iter()
        .flatten()
        .map(String::as_str)
        .filter(|x| *x != rid && !quedan.contains(x))
        .collect();
    if faltan.is_empty() {
        Comprobacion::Coincide(None)
    } else {
        difiere(format!(
            "faltan otras reglas del filtro ({}): la baja borró más de la cuenta",
            faltan.join(", ")
        ))
    }
}

impl ChangeManager {
    /// Como [`ChangeManager::funcion_escribible`], pero viendo la petición
    /// entera: la regla «sin ping» del cortafuegos se escribe aunque su
    /// función esté bloqueada, solo con su carga exacta o con la baja de
    /// una regla por su RID (las guardas de `especiales::cortafuegos` miran
    /// después su huella). La exención salta solo el bloqueo del catálogo
    /// de la función: el resto de guardas se comprueba siempre. Cualquier
    /// otra petición, como siempre.
    pub(in crate::changes) fn funcion_escribible_para(
        &self,
        req: &ChangeRequest,
    ) -> Result<&Funcion, CambioError> {
        let f = self.funcion_del_catalogo(&req.funcion)?;
        crate::changes::router::guardas(f, f.bloqueada.is_some() && exento_de_bloqueo(req))
    }

    /// Los filtros del router; si no se pueden leer, `Router`.
    async fn filtros(&self) -> Result<Vec<Filtro>, CambioError> {
        cf::leer(&self.state.hgu)
            .await
            .map_err(|e| CambioError::Router(format!("no se puede leer el cortafuegos: {e}")))
    }

    /// Los filtros y el de entrada IPv4 de la conexión a internet (la que
    /// vigila el gestor, [`RedActual::wan_en`]; sin ella, la de la ruta por
    /// defecto o la única de datos con IPv4 de internet). `Router` si no se
    /// pueden leer; `Invalida` si no se sabe cuál es o el cortafuegos no
    /// tiene su filtro.
    async fn filtros_e_internet(&self) -> Result<(Vec<Filtro>, Filtro), CambioError> {
        let filtros = self.filtros().await?;
        let c = &self.state.hgu;
        let no_responde =
            |e: hgu::Error| CambioError::Router(format!("el router no responde: {e}"));
        let conns = read::wan(c).await.map_err(no_responde)?;
        let conn = match self.state.red.wan_en(&conns) {
            Some(n) => conns.iter().find(|x| x.name == n),
            None => {
                let ruta = read::ruta_por_defecto(c).await.ok().flatten();
                hgu::deteccion::elegir_wan(&conns, ruta.as_deref())
            }
        }
        .ok_or_else(|| CambioError::Invalida("no se sabe cuál es la conexión a internet".into()))?;
        let filtro = cf::filtro_de_entrada_v4(&filtros, &conn.ip_intf_id)
            .cloned()
            .ok_or_else(|| {
                CambioError::Invalida(
                    "el cortafuegos no tiene el filtro de entrada de la conexión a internet".into(),
                )
            })?;
        Ok((filtros, filtro))
    }

    /// El filtro de entrada IPv4 de la conexión a internet: donde va la
    /// regla «sin ping» (ver [`ChangeManager::filtros_e_internet`]).
    pub(crate) async fn filtro_de_internet(&self) -> Result<Filtro, CambioError> {
        self.filtros_e_internet().await.map(|(_, f)| f)
    }

    /// Plan del alta o de la baja de la regla «sin ping» (ver el módulo).
    pub(in crate::changes) async fn planificar_cortafuegos(
        &self,
        f: &Funcion,
        req: &ChangeRequest,
    ) -> Result<Plan, CambioError> {
        if !exento_de_bloqueo(req) {
            return Err(CambioError::Invalida(format!(
                "{}: solo la regla «sin ping» del panel",
                f.name
            )));
        }
        let campo = |nombre: &str, antes: &str, despues: &str, leido: bool| Campo {
            nombre: nombre.to_owned(),
            antes: antes.to_owned(),
            despues: despues.to_owned(),
            secreto: false,
            leido,
        };
        if req.borrar {
            let rid = req.instancia.clone().unwrap_or_default();
            let filtros = self.filtros().await?;
            let filtro = filtro_para_la_baja(&filtros, req)?;
            let valores: BTreeMap<String, String> =
                [("belong".to_owned(), filtro.fid.clone())].into();
            return Ok(Plan {
                funcion: f.clone(),
                operacion: Operacion::Borrar,
                payload: baja(&rid, &valores).1,
                campos: carga(&filtro.fid)
                    .iter()
                    .map(|(k, v)| campo(k, v, "", true))
                    .collect(),
                instancia: Some(rid),
                valores,
                lectura: Lectura::Cortafuegos,
                // Las reglas del filtro antes de la baja: al comprobarla,
                // tiene que faltar solo la suya ([`baja_exacta`]).
                ids_antes: Some(filtro.reglas.iter().map(|r| r.rid.clone()).collect()),
            });
        }
        let (filtros, filtro) = self.filtros_e_internet().await?;
        if req.valores.get("belong") != Some(&filtro.fid) {
            return Err(CambioError::Invalida(
                "la regla va en el filtro de entrada de la conexión a internet".into(),
            ));
        }
        if filtro.reglas.iter().any(cf::es_sin_ping) {
            return Err(CambioError::InstanciaCambiada(SIN_PING_YA_PUESTA.into()));
        }
        let carga = cf::carga_sin_ping(&filtro);
        Ok(Plan {
            funcion: f.clone(),
            operacion: Operacion::Crear,
            instancia: None,
            valores: carga
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
            payload: carga
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
            campos: carga.iter().map(|(k, v)| campo(k, "", v, false)).collect(),
            lectura: Lectura::Cortafuegos,
            ids_antes: Some(
                filtros
                    .iter()
                    .flat_map(|x| &x.reglas)
                    .map(|r| r.rid.clone())
                    .collect(),
            ),
        })
    }

    /// ¿Muestra el router el lado `lado` de un plan de la regla «sin
    /// ping»? Al crear, aparece una regla nueva (un RID que no estaba) con
    /// la huella y lo escrito; al borrar, su RID ya no tiene la huella y
    /// el filtro solo ha perdido esa regla ([`baja_exacta`]).
    pub(in crate::changes) async fn comprobar_cortafuegos(
        &self,
        plan: &Plan,
        lado: Lado,
    ) -> Comprobacion {
        let filtros = match self.filtros().await {
            Ok(f) => f,
            Err(e) => return Comprobacion::SinLectura(e.to_string()),
        };
        let filas = filas_del_cortafuegos(&filtros);
        let rid = |fila: &BTreeMap<String, String>| fila.get("RID").cloned().unwrap_or_default();
        let con_huella = |r: &str| filas.iter().any(|(fila, h)| *h && rid(fila) == r);
        let difiere = |m: &str| Comprobacion::Difiere(vec![m.to_owned()]);
        match (plan.operacion, lado, plan.instancia.as_deref()) {
            (Operacion::Crear, Lado::Nuevo, Some(r)) if con_huella(r) => {
                Comprobacion::Coincide(None)
            }
            (Operacion::Crear, Lado::Nuevo, Some(_)) => difiere("no aparece la regla nueva"),
            (Operacion::Crear, Lado::Nuevo, None) => {
                let nuevas: Vec<String> = filas
                    .iter()
                    .filter(|(fila, h)| {
                        *h && plan
                            .ids_antes
                            .as_ref()
                            .is_none_or(|antes| !antes.contains(&rid(fila)))
                            && comparar(&plan.funcion, &plan.campos, fila, Lado::Nuevo)
                                == Comprobacion::Coincide(None)
                    })
                    .map(|(fila, _)| rid(fila))
                    .collect();
                match nuevas.as_slice() {
                    [] => difiere("no aparece la regla nueva"),
                    [r] => Comprobacion::Coincide(Some(r.clone())),
                    varias => difiere(&format!(
                        "aparecen {} reglas nuevas ({}): no se sabe cuál es la de este cambio; \
                         no se borra nada",
                        varias.len(),
                        varias.join(", ")
                    )),
                }
            }
            (Operacion::Crear, Lado::Anterior, Some(r)) if con_huella(r) => {
                difiere("la regla nueva existe")
            }
            (Operacion::Crear, Lado::Anterior, Some(_)) => Comprobacion::Coincide(None),
            (Operacion::Crear, Lado::Anterior, None) => {
                let nueva = filas.iter().any(|(fila, h)| {
                    *h && plan
                        .ids_antes
                        .as_ref()
                        .is_none_or(|antes| !antes.contains(&rid(fila)))
                });
                if nueva {
                    difiere("la regla nueva existe")
                } else {
                    Comprobacion::Coincide(None)
                }
            }
            (Operacion::Borrar, Lado::Nuevo, Some(r)) if con_huella(r) => {
                difiere("la regla sigue existiendo")
            }
            (Operacion::Borrar, Lado::Nuevo, Some(r)) => baja_exacta(&filtros, plan, r),
            (Operacion::Borrar, Lado::Nuevo, None) => Comprobacion::Coincide(None),
            (Operacion::Borrar, Lado::Anterior, Some(r)) if con_huella(r) => {
                Comprobacion::Coincide(None)
            }
            (Operacion::Borrar, Lado::Anterior, _) => difiere("la regla ya no existe"),
            (Operacion::Editar | Operacion::Accion, _, _) => Comprobacion::Limitada,
        }
    }
}

#[cfg(test)]
mod tests;
