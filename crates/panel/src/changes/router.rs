//! Planificación de las escrituras y E/S con el router.

use super::especiales::{self, Especial};
use super::*;

impl ChangeManager {
    pub(super) fn funcion_escribible(&self, nombre: &str) -> Result<&Funcion, CambioError> {
        guardas(self.funcion_del_catalogo(nombre)?, false)
    }

    /// La función `nombre` del catálogo; `Invalida` si no está.
    pub(super) fn funcion_del_catalogo(&self, nombre: &str) -> Result<&Funcion, CambioError> {
        self.state
            .catalogo
            .funcion(nombre)
            .ok_or_else(|| CambioError::Invalida(format!("función desconocida: {nombre}")))
    }

    /// `explicito`: valores exactos (reversión/deshacer; ver [`componer`]).
    pub(super) async fn planificar(
        &self,
        req: &ChangeRequest,
        explicito: bool,
    ) -> Result<Plan, CambioError> {
        let f = self.funcion_escribible_para(req)?;
        // La lista blanca: solo lo comprobado en el router real (el
        // catálogo). El deshacer y la reversión (`explicito`) vuelven a lo
        // que ya había y siguen.
        if !explicito && !f.comprobada() {
            return Err(CambioError::Bloqueada(
                crate::validacion_real::SIN_COMPROBAR.into(),
            ));
        }
        if let Some(e) = Especial::de(f) {
            return self.planificar_especial(e, f, req, explicito).await;
        }
        for k in req.valores.keys() {
            if f.parametro(k).is_none() {
                return Err(CambioError::Invalida(format!(
                    "{}: parámetro desconocido {k}",
                    f.name
                )));
            }
        }
        let op = match (f.tipo, &f.lista, req.borrar, &req.instancia) {
            (Tipo::Accion, _, false, _) => Operacion::Accion,
            (Tipo::Escritura, Some(l), true, Some(_)) if l.del => Operacion::Borrar,
            (_, _, true, _) => {
                return Err(CambioError::Invalida(format!(
                    "{}: no se pueden borrar instancias así",
                    f.name
                )));
            }
            (_, Some(_), false, None) => Operacion::Crear,
            _ => Operacion::Editar,
        };
        let mut valores = req.valores.clone();
        if let Some(l) = &f.lista
            && op != Operacion::Accion
            && f.parametro(&l.id_param).is_some()
        {
            let id = req.instancia.as_deref().unwrap_or("-1");
            if valores.get(&l.id_param).is_some_and(|v| v != id) {
                return Err(CambioError::Invalida(format!(
                    "{} no coincide con la instancia",
                    l.id_param
                )));
            }
            valores.insert(l.id_param.clone(), id.to_owned());
        }
        let lectura = lectura_de(f, &valores, req.instancia.as_deref())?;
        let actual = match op {
            Operacion::Crear | Operacion::Accion => BTreeMap::new(),
            Operacion::Editar | Operacion::Borrar => self.leer(&lectura).await.map_err(|e| {
                CambioError::Router(format!("no se puede leer el estado actual: {e}"))
            })?,
        };
        if let Some(k) = identidad_cambiada(&req.identidad, &actual) {
            return Err(CambioError::InstanciaCambiada(format!(
                "la instancia {} ya no es la misma que al preparar el cambio ({k} es otro): \
                 no se escribe, para no tocar otra",
                req.instancia.as_deref().unwrap_or("-")
            )));
        }
        // Activar una condición sin valor deja sin IP nueva a todos.
        if !explicito {
            condicion_vacia(f, &actual, &req.valores).map_err(CambioError::Invalida)?;
        }
        let Compuesto {
            efectivos,
            payload,
            campos,
            secretos_tapados,
        } = componer(f, &self.state.mascara, op, &actual, &valores, explicito);
        if !secretos_tapados.is_empty() {
            tracing::warn!(
                funcion = %f.name,
                parametros = ?secretos_tapados,
                "el router da vacíos o tapados unos secretos que la escritura reenviaría: \
                 no se escribe"
            );
            return Err(CambioError::Bloqueada(BLOQUEO_SECRETO_TAPADO.into()));
        }
        if op == Operacion::Editar
            && let Some(c) = campos.iter().find(|c| !c.leido)
        {
            return Err(CambioError::Invalida(format!(
                "no se puede leer el valor actual de {}; sin valor anterior no se podría deshacer",
                c.nombre
            )));
        }
        // Lo que viene del router (reversión/deshacer) no se valida: puede
        // estar fuera de las opciones que conoce el catálogo.
        if op != Operacion::Borrar && !explicito {
            for c in &campos {
                if let Some(p) = f.parametro(&c.nombre) {
                    validar(p, &c.despues, c.secreto).map_err(CambioError::Invalida)?;
                }
            }
        }
        if campos.is_empty() && matches!(op, Operacion::Editar | Operacion::Crear) {
            return Err(CambioError::SinCambios);
        }
        if payload.iter().any(|(_, v)| v == MASCARA) {
            return Err(CambioError::Invalida(
                "un valor enmascarado no se puede escribir en el router".into(),
            ));
        }
        Ok(Plan {
            funcion: f.clone(),
            operacion: op,
            instancia: req.instancia.clone(),
            valores: efectivos,
            payload,
            campos,
            lectura,
            ids_antes: None,
        })
    }

    /// Ids de las instancias de la lista de `plan` (para saber cuál se crea).
    pub(super) async fn ids_de_la_lista(
        &self,
        plan: &Plan,
    ) -> Result<BTreeSet<String>, hgu::Error> {
        let Some(id_param) = plan.funcion.lista.as_ref().map(|l| l.id_param.as_str()) else {
            return Ok(BTreeSet::new());
        };
        Ok(self
            .listar(&plan.lectura)
            .await?
            .iter()
            .filter_map(|e| buscar(e, id_param).map(str::to_owned))
            .collect())
    }

    /// Valores de la instancia que muestra `l` (vacío si no hay lectura).
    pub(super) async fn leer(&self, l: &Lectura) -> Result<BTreeMap<String, String>, hgu::Error> {
        let c = &self.state.hgu;
        match l {
            Lectura::Xml {
                funcion,
                params,
                buscar,
                ..
            } => {
                let p: Vec<(&str, &str)> = params
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                let n = match buscar {
                    None => c.call(funcion, &p).await?,
                    Some(b) => {
                        let ident = [(b.campo.as_str(), b.valor.as_str())];
                        c.call_por_id(funcion, &p, &b.posicion, &ident).await?
                    }
                };
                Ok(elementos(&n).into_iter().next().unwrap_or_default())
            }
            Lectura::Pagina(page) => {
                let mut vars = page_vars(&c.basic_page(page).await?);
                vars.remove("sessionKey");
                Ok(vars)
            }
            // Una lista sin instancia que leer sola: se lista.
            Lectura::Ninguna | Lectura::Usuarios | Lectura::Cortafuegos => Ok(BTreeMap::new()),
        }
    }

    /// Todas las instancias de la lista (lectura sin el id, con `all=ALL`).
    pub(super) async fn listar(
        &self,
        l: &Lectura,
    ) -> Result<Vec<BTreeMap<String, String>>, hgu::Error> {
        if *l == Lectura::Usuarios {
            let n = self
                .state
                .hgu
                .call(especiales::LECTURA_USUARIOS, &[])
                .await?;
            return Ok(especiales::usuarios_de(&n));
        }
        if *l == Lectura::Cortafuegos {
            let filtros = hgu::cortafuegos::leer(&self.state.hgu).await?;
            return Ok(especiales::filas_del_cortafuegos(&filtros)
                .into_iter()
                .map(|(fila, _)| fila)
                .collect());
        }
        let Lectura::Xml {
            funcion,
            params,
            clave_id,
            ..
        } = l
        else {
            return Ok(Vec::new());
        };
        let mut p: Vec<(&str, &str)> = params
            .iter()
            .filter(|(k, _)| Some(k) != clave_id.as_ref())
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        if !p.iter().any(|(k, _)| *k == "all") {
            p.push(("all", "ALL"));
        }
        let n = self.state.hgu.call(funcion, &p).await?;
        Ok(n.children
            .iter()
            .filter(|c| !c.children.is_empty())
            .map(hojas)
            .collect())
    }

    /// Envía la carga del plan (las claves de la UI básica, ofuscadas con
    /// XOR 0x1f como hace la página).
    pub(super) async fn enviar(&self, plan: &Plan) -> Result<(), hgu::Error> {
        let f = &plan.funcion;
        let pagina = f.pagina.as_deref().unwrap_or_default();
        // Borrar una instancia especial es otra escritura (un usuario:
        // `cbDeleteUser` con su nombre; la regla del cortafuegos, la baja
        // de la UI básica con `item=<FID>|<RID>`, desde su lista).
        if plan.operacion == Operacion::Borrar
            && let Some(e) = Especial::de(f)
        {
            let (funcion, params) =
                e.baja(plan.instancia.as_deref().unwrap_or_default(), &plan.valores);
            let p: Vec<(&str, &str)> = params
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let c = &self.state.hgu;
            return match e {
                Especial::Cortafuegos => c
                    .basic_write(hgu::cortafuegos::PAGINA_LISTA, funcion, &p)
                    .await
                    .map(drop),
                Especial::Usuarios => c
                    .write(pagina, funcion, &p)
                    .await
                    .map(drop)
                    .map_err(super::especiales::sin_texto_del_router),
            };
        }
        let basica = !f.name.starts_with("cb");
        let m = &self.state.mascara;
        let params: Vec<(String, String)> = plan
            .payload
            .iter()
            .map(|(k, v)| {
                if basica && es_secreto_param(f, m, k) {
                    (k.clone(), mess_userpass(v))
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect();
        let p: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let c = &self.state.hgu;
        let r = if basica {
            c.basic_write(pagina, &f.name, &p).await.map(drop)
        } else {
            c.write(pagina, &f.name, &p).await.map(drop)
        };
        match Especial::de(f) {
            Some(Especial::Usuarios) => r.map_err(super::especiales::sin_texto_del_router),
            _ => r,
        }
    }

    /// ¿Muestra el router el lado `lado` del plan?
    pub(super) async fn comprobar(&self, plan: &Plan, lado: Lado) -> Comprobacion {
        let c = self.comprobar_en_el_router(plan, lado).await;
        // El router puede aplicar una escritura un poco después de
        // responder: lo leído para mostrar hasta ahora quizá sea de antes.
        self.state.lecturas.invalidar();
        c
    }

    async fn comprobar_en_el_router(&self, plan: &Plan, lado: Lado) -> Comprobacion {
        if plan.lectura == Lectura::Usuarios {
            return self.comprobar_usuarios(plan, lado).await;
        }
        if plan.lectura == Lectura::Cortafuegos {
            return self.comprobar_cortafuegos(plan, lado).await;
        }
        let f = &plan.funcion;
        let no_leido =
            |e: hgu::Error| Comprobacion::SinLectura(format!("no se puede releer el estado: {e}"));
        match (plan.operacion, &plan.lectura) {
            (Operacion::Accion, _) => Comprobacion::Coincide(None),
            (_, Lectura::Ninguna) => Comprobacion::Limitada,
            (Operacion::Editar, l) => match self.leer(l).await {
                Ok(actual) => comparar(f, &plan.campos, &actual, lado),
                Err(e) => no_leido(e),
            },
            (Operacion::Crear, l @ Lectura::Xml { .. }) => {
                let lista = match self.listar(l).await {
                    Ok(v) => v,
                    Err(e) => return no_leido(e),
                };
                let id_param = f.lista.as_ref().map(|l| l.id_param.as_str());
                let id_de = |e: &BTreeMap<String, String>| {
                    id_param.and_then(|p| buscar(e, p)).map(str::to_owned)
                };
                let con_valores = |e: &BTreeMap<String, String>| {
                    comparar(f, &plan.campos, e, Lado::Nuevo) == Comprobacion::Coincide(None)
                };
                // La instancia del cambio: la de id conocido; si no, la que
                // no estaba antes; sin ids de antes, por valores.
                let candidatas: Vec<&BTreeMap<String, String>> =
                    match (&plan.instancia, &plan.ids_antes, id_param) {
                        (Some(id), _, Some(_)) => lista
                            .iter()
                            .filter(|e| id_de(e).as_ref() == Some(id))
                            .collect(),
                        (None, Some(antes), Some(_)) => lista
                            .iter()
                            .filter(|e| id_de(e).is_some_and(|i| !antes.contains(&i)))
                            .collect(),
                        _ => lista.iter().filter(|e| con_valores(e)).collect(),
                    };
                match (lado, candidatas.as_slice()) {
                    (Lado::Nuevo, []) => {
                        Comprobacion::Difiere(vec!["no aparece la instancia nueva".into()])
                    }
                    (Lado::Nuevo, [e]) if con_valores(e) => {
                        Comprobacion::Coincide(plan.instancia.is_none().then(|| id_de(e)).flatten())
                    }
                    (Lado::Nuevo, [_]) => Comprobacion::Difiere(vec![
                        "la instancia nueva no tiene los valores escritos".into(),
                    ]),
                    (Lado::Nuevo, varias) => {
                        let ids: Vec<String> = varias.iter().filter_map(|e| id_de(e)).collect();
                        Comprobacion::Difiere(vec![format!(
                            "aparecen {} instancias nuevas ({}): no se sabe cuál es la de este \
                             cambio; no se borra nada",
                            varias.len(),
                            ids.join(", ")
                        )])
                    }
                    (Lado::Anterior, []) => Comprobacion::Coincide(None),
                    (Lado::Anterior, _) => {
                        Comprobacion::Difiere(vec!["la instancia nueva existe".into()])
                    }
                }
            }
            (Operacion::Crear, _) => Comprobacion::Limitada,
            (Operacion::Borrar, l) => {
                let existe = match self.leer(l).await {
                    Ok(a) if a.is_empty() => false,
                    Ok(a) => {
                        comparar(f, &plan.campos, &a, Lado::Anterior)
                            == Comprobacion::Coincide(None)
                    }
                    Err(hgu::Error::CallFailed { .. } | hgu::Error::SinInstancia(_)) => false,
                    Err(e) => return no_leido(e),
                };
                match (lado, existe) {
                    (Lado::Nuevo, false) | (Lado::Anterior, true) => Comprobacion::Coincide(None),
                    (Lado::Nuevo, true) => {
                        Comprobacion::Difiere(vec!["la instancia sigue existiendo".into()])
                    }
                    (Lado::Anterior, false) => {
                        Comprobacion::Difiere(vec!["la instancia ya no existe".into()])
                    }
                }
            }
        }
    }
}

/// Primer elemento de `identidad` que la instancia leída (`actual`) ya no
/// tiene igual (sin espacios ni mayúsculas: una MAC puede llegar de otra
/// forma); `None` si sigue siendo la misma.
pub(super) fn identidad_cambiada<'a>(
    identidad: &'a BTreeMap<String, String>,
    actual: &BTreeMap<String, String>,
) -> Option<&'a str> {
    identidad
        .iter()
        .find(|(k, v)| {
            !actual
                .get(*k)
                .is_some_and(|a| a.trim().eq_ignore_ascii_case(v.trim()))
        })
        .map(|(k, _)| k.as_str())
}

/// Las guardas de escritura de `f`. `exento` salta solo su bloqueo del
/// catálogo (`f.bloqueada`): las demás se comprueban siempre.
pub(super) fn guardas(f: &Funcion, exento: bool) -> Result<&Funcion, CambioError> {
    let nombre = &f.name;
    if let Some(m) = f.bloqueada.as_ref().filter(|_| !exento) {
        return Err(CambioError::Bloqueada(m.clone()));
    }
    if f.tipo == Tipo::Lectura {
        return Err(CambioError::Invalida(format!("{nombre} es una lectura")));
    }
    if f.pagina.is_none() {
        return Err(CambioError::Invalida(format!("{nombre}: sin página")));
    }
    if CAMBIAN_IP_LAN.contains(&f.name.as_str()) {
        return Err(CambioError::Bloqueada(BLOQUEO_IP_LAN.into()));
    }
    let basica = f.origen == catalogo::Origen::Basica || !f.name.starts_with("cb");
    if basica
        && f.parametros
            .iter()
            .any(|p| p.tipo == catalogo::TipoParam::Secret)
    {
        return Err(CambioError::Bloqueada(BLOQUEO_SECRETOS_BASICA.into()));
    }
    Ok(f)
}

#[cfg(test)]
mod tests_identidad;
