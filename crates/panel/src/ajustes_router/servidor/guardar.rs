//! Guardado de un ajuste: del formulario al grupo de cambios y su resumen.

use super::lectura::tapado;
use super::puertos::{comprobar_regla, partes};
use super::red_local::comprobar_reserva;

use super::*;

/// Pares `(parámetro, valor)` que escribe el campo `c` con el legible
/// `v`, validado; `None` si es un secreto sin tocar ([`MASCARA`]). Un
/// campo de solo lectura no vale ni con la máscara: la página no lo
/// envía.
fn pares(c: &Campo, v: &str) -> Result<Option<Vec<(String, String)>>, ErrorValor> {
    if c.escribible().is_none() {
        return Err(ErrorValor::SoloLectura);
    }
    if c.tipo == TipoCampo::Secreto && v == MASCARA {
        return Ok(None);
    }
    a_router(c, v).map(Some)
}

/// Grupo de cambios que guarda `p` en el ajuste `a`: un paso por función
/// de escritura de los campos tocados, en el orden de los campos (al
/// borrar, de todos los campos que se escriben). Cada paso lleva los
/// `fijos` del ajuste y los parámetros de sus campos tocados en formato
/// del router; lo no tocado lo completa el gestor con lo que lea. Los
/// ids de `p.valores` que no son campos del ajuste no cuentan (los
/// rechaza quien llama). Sin campos tocados, el grupo no tiene pasos.
pub(crate) fn grupo_de(a: &Ajuste, p: &PeticionAjuste) -> Result<GrupoRequest, ErrorValor> {
    let titulo = format!("{}-titulo", prefijo_i18n(a));
    let mut pasos: Vec<(String, BTreeMap<String, String>)> = Vec::new();
    for c in &a.campos {
        let tocados = match p.valores.get(&c.id) {
            Some(v) if !p.borrar => pares(c, v)?,
            _ => None,
        };
        let Some(e) = c.escribible() else {
            continue;
        };
        if tocados.is_none() && !p.borrar {
            continue;
        }
        let i = match pasos.iter().position(|(f, _)| *f == e.funcion) {
            Some(i) => i,
            None => {
                pasos.push((e.funcion.clone(), a.fijos.clone()));
                pasos.len() - 1
            }
        };
        pasos[i].1.extend(tocados.unwrap_or_default());
    }
    Ok(GrupoRequest {
        pasos: pasos
            .into_iter()
            .map(|(funcion, valores)| Paso {
                req: ChangeRequest {
                    funcion,
                    instancia: p.instancia.clone().filter(|_| a.lista),
                    valores,
                    borrar: p.borrar && a.lista,
                    identidad: Default::default(),
                },
                etiqueta: Some(titulo.clone()),
            })
            .collect(),
        titulo: Some(titulo),
    })
}

/// En un ajuste que no es lista pero escribe con una función de lista
/// (un pool DHCP concreto), la instancia de cada paso es el id de la
/// lista que fijan sus `fijos`: sin ella, el gestor crearía una
/// instancia nueva. `Err` con la función si los `fijos` no lo traen.
pub(super) fn instancias_fijas(
    f: &Catalogo,
    a: &Ajuste,
    g: &mut GrupoRequest,
) -> Result<(), String> {
    if a.lista {
        return Ok(());
    }
    for paso in &mut g.pasos {
        let Some(l) = f.funcion(&paso.req.funcion).and_then(|f| f.lista.as_ref()) else {
            continue;
        };
        paso.req.instancia = Some(
            a.fijos
                .get(&l.id_param)
                .cloned()
                .ok_or_else(|| paso.req.funcion.clone())?,
        );
    }
    Ok(())
}

/// Texto legible de un valor para el resumen.
fn texto(v: ValorLegible) -> String {
    match v {
        ValorLegible::Texto(t) | ValorLegible::Opcion(t) | ValorLegible::Desconocido(t) => t,
        ValorLegible::Booleano(b) => b.to_string(),
        ValorLegible::Secreto => MASCARA.to_owned(),
    }
}

/// Cambio legible del campo `c` según el diff de su paso, o `None` si
/// ninguno de sus parámetros cambia. Un parámetro del campo que no está
/// en el diff no cambia: su valor lo da `completar` (lo pedido o lo que
/// muestra el router), así una opción con `ademas` se reconoce entera.
pub(super) fn cambio_legible(
    prefijo: &str,
    c: &Campo,
    diff: &Diff,
    completar: &dyn Fn(&str) -> Option<String>,
) -> Option<CambioLegible> {
    let e = c.escribe.as_ref()?;
    let params: Vec<(&str, &str)> = std::iter::once((e.parametro.as_str(), c.lee.as_str()))
        .chain(
            c.ademas
                .iter()
                .map(|ad| (ad.parametro.as_str(), ad.lee.as_str())),
        )
        .collect();
    let en_diff = |param: &str| {
        diff.campos
            .iter()
            .find(|d| d.nombre.eq_ignore_ascii_case(param))
    };
    if !params.iter().any(|(param, _)| en_diff(param).is_some()) {
        return None;
    }
    let campo = format!("{prefijo}-{}", c.id);
    let secreto = c.tipo == TipoCampo::Secreto
        || params
            .iter()
            .any(|(param, _)| en_diff(param).is_some_and(|d| d.secreto));
    if secreto {
        return Some(CambioLegible {
            campo,
            antes: MASCARA.to_owned(),
            despues: MASCARA.to_owned(),
            secreto: true,
            tipo: c.tipo,
        });
    }
    let (mut antes, mut despues) = (Leidos::new(), Leidos::new());
    for (param, lee) in params {
        let (a, d) = match en_diff(param) {
            Some(d) => (d.antes.clone(), d.despues.clone()),
            None => {
                let v = completar(param).unwrap_or_default();
                (v.clone(), v)
            }
        };
        antes.insert(lee.to_owned(), a);
        despues.insert(lee.to_owned(), d);
    }
    Some(CambioLegible {
        campo,
        antes: texto(a_legible(c, &antes)),
        despues: texto(a_legible(c, &despues)),
        secreto: false,
        tipo: c.tipo,
    })
}

/// Lo que cambia en el paso preparado `paso`, que escribe en el ajuste
/// `a`. `completar(parámetro)` da el valor de un parámetro que no cambia.
pub(super) fn paso_vista(
    a: &Ajuste,
    paso: &PasoPreparado,
    completar: &dyn Fn(&str) -> Option<String>,
) -> PasoVista {
    let prefijo = prefijo_i18n(a);
    PasoVista {
        indice: paso.indice,
        sin_cambios: paso.sin_cambios,
        campos: a
            .campos
            .iter()
            .filter(|c| {
                c.escribe
                    .as_ref()
                    .is_some_and(|e| e.funcion == paso.funcion)
            })
            .filter_map(|c| cambio_legible(&prefijo, c, &paso.diff, completar))
            .collect(),
        titulo: None,
        nota: None,
        pestana: None,
        desmarcado: false,
        clave: None,
    }
}

/// Resumen de un grupo preparado del ajuste `a`. `completar(paso,
/// parámetro)` da el valor de un parámetro que no cambia en ese paso.
pub(super) fn vista_grupo(
    g: &ChangeManager,
    a: &Ajuste,
    token: String,
    pasos: &[PasoPreparado],
    r: Riesgo,
    completar: &dyn Fn(usize, &str) -> Option<String>,
) -> GrupoVista {
    GrupoVista {
        token,
        titulo: format!("{}-titulo", prefijo_i18n(a)),
        pasos: pasos
            .iter()
            .map(|paso| paso_vista(a, paso, &|param| completar(paso.indice, param)))
            .collect(),
        riesgo: riesgo(r),
        efecto: a.efecto.as_ref().map(|e| format!("efecto-{e}")),
        vigilancia_s: g.tiempos().vigilancia(r).as_secs(),
        detalle: None,
        aviso: None,
        borra: false,
        proveedor_dns: None,
    }
}

/// Valida lo que pide `p` en el ajuste `a` antes de leer el router: el
/// ajuste admite borrar, los campos existen, lo exigido viene y cada valor
/// vale (400 con su mensaje).
fn validar(a: &Ajuste, p: &PeticionAjuste) -> Result<(), ServerFnError> {
    if p.borrar && !a.lista {
        return Err(fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida")));
    }
    let prefijo = prefijo_i18n(a);
    let campos = partes(a, p)
        .into_iter()
        .flat_map(|q| q.valores)
        .map(|(id, v)| {
            a.campo(&id)
                .map(|c| (c, v))
                .ok_or_else(|| fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-campo")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    // Antes que los valores: «sin elegir» en el campo exigido es que
    // falta, no una opción que no vale.
    if !p.borrar
        && let Some((pide, falta)) = falta_requerido(a, &p.valores)
    {
        return Err(error_requiere(&prefijo, pide, falta));
    }
    for (c, v) in campos {
        pares(c, &v).map_err(|e| error_valor(&prefijo, c, &e))?;
    }
    Ok(())
}

/// El grupo que guarda `p` en el ajuste `a` (ya validado), con el título
/// de cada paso: con varias partes («TCP y UDP»), su protocolo. Exige la
/// clave que el router no deja leer donde se reenvía (400).
fn armar(
    state: &AppState,
    a: &Ajuste,
    p: &PeticionAjuste,
) -> Result<(GrupoRequest, Vec<Option<String>>), ServerFnError> {
    let prefijo = prefijo_i18n(a);
    let partes = partes(a, p);
    let mut grupo = GrupoRequest {
        pasos: Vec::new(),
        titulo: None,
    };
    let mut titulos: Vec<Option<String>> = Vec::new();
    for q in &partes {
        let g = grupo_de(a, q).map_err(|e| {
            tracing::warn!("ajustes: {}: valor no válido tras validarlo: {e}", a.id);
            fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
        })?;
        let titulo = (partes.len() > 1)
            .then(|| q.valores.get("protocolo"))
            .flatten()
            .map(|v| format!("{prefijo}-protocolo-{v}"));
        titulos.extend(std::iter::repeat_n(titulo, g.pasos.len()));
        grupo.pasos.extend(g.pasos);
        grupo.titulo = g.titulo;
    }
    instancias_fijas(&state.catalogo, a, &mut grupo).map_err(|funcion| {
        tracing::warn!(
            "ajustes: {}: los fijos no dan la instancia de {funcion}",
            a.id
        );
        fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
    })?;
    exigir_tapados(&state.catalogo, a, &partes, &grupo)?;
    Ok((grupo, titulos))
}

pub(in crate::ajustes_router) async fn preparar_ajuste(
    p: PeticionAjuste,
) -> Result<GrupoVista, ServerFnError> {
    // `preparar()` ya pone `no-store` (el resumen lleva los datos sensibles
    // en claro).
    let (state, _) = preparar().await?;
    let a = ajuste_por_id(&state, &p.ajuste)?;
    validar(a, &p)?;
    let g = gestor(&state)?;
    libre(&g)?;
    let detalle = match a.componente {
        Some(Componente::Reservas) => comprobar_reserva(&state, a, &p).await?,
        Some(Componente::Puertos) => comprobar_regla(&state, a, &p).await?,
        _ => None,
    };
    let (grupo, titulos) = armar(&state, a, &p)?;
    if grupo.pasos.is_empty() {
        return Err(error_ajuste(CambioError::SinCambios));
    }
    let pedidos: Vec<BTreeMap<String, String>> =
        grupo.pasos.iter().map(|p| p.req.valores.clone()).collect();
    let prep = g.preparar_grupo(grupo).await.map_err(error_ajuste)?;
    let mut vista = vista_grupo(&g, a, prep.token, &prep.pasos, prep.riesgo, &|i, param| {
        buscar(&pedidos[i], param).map(str::to_owned)
    });
    titular_partes(&mut vista.pasos, &titulos, &prefijo_i18n(a));
    Ok(GrupoVista {
        detalle,
        borra: p.borrar,
        ..vista
    })
}

/// Prepara el guardado del detalle de un elemento: varios ajustes suyos
/// (lo esencial y lo avanzado de una red) en un solo grupo, con un paso por
/// escritura titulado con su ajuste. Solo ajustes que no son listas ni
/// tienen componente propio (400). Con uno solo, es [`preparar_ajuste`].
pub(in crate::ajustes_router) async fn preparar_elemento(
    peticiones: Vec<PeticionAjuste>,
) -> Result<GrupoVista, ServerFnError> {
    let invalida = || fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"));
    if peticiones.len() <= 1 {
        let p = peticiones.into_iter().next().ok_or_else(invalida)?;
        return preparar_ajuste(p).await;
    }
    let (state, _) = preparar().await?;
    let mut ajustes: Vec<&Ajuste> = Vec::with_capacity(peticiones.len());
    for p in &peticiones {
        let a = ajuste_por_id(&state, &p.ajuste)?;
        if a.lista || a.componente.is_some() || ajustes.iter().any(|x| x.id == a.id) {
            return Err(invalida());
        }
        validar(a, p)?;
        ajustes.push(a);
    }
    let g = gestor(&state)?;
    libre(&g)?;
    let mut grupo = GrupoRequest {
        pasos: Vec::new(),
        titulo: Some(format!("{}-titulo", prefijo_i18n(ajustes[0]))),
    };
    let mut de_cada_paso: Vec<&Ajuste> = Vec::new();
    for (a, p) in ajustes.iter().zip(&peticiones) {
        let (sub, _) = armar(&state, a, p)?;
        de_cada_paso.extend(std::iter::repeat_n(*a, sub.pasos.len()));
        grupo.pasos.extend(sub.pasos);
    }
    if grupo.pasos.is_empty() {
        return Err(error_ajuste(CambioError::SinCambios));
    }
    let titulo = grupo.titulo.clone().unwrap_or_default();
    let pedidos: Vec<BTreeMap<String, String>> =
        grupo.pasos.iter().map(|p| p.req.valores.clone()).collect();
    let prep = g.preparar_grupo(grupo).await.map_err(error_ajuste)?;
    Ok(super::acciones::vista_por_pasos(
        &g,
        titulo,
        &de_cada_paso,
        prep.token,
        &prep.pasos,
        prep.riesgo,
        &|i, param| buscar(&pedidos[i], param).map(str::to_owned),
    ))
}

/// Cada paso que escribe una función con un secreto tapado al leer lo
/// reenvía: tiene que venir escrito en la petición (si no, 400 junto al
/// campo, con el motivo). Al borrar no se reenvía.
pub(super) fn exigir_tapados(
    f: &Catalogo,
    a: &Ajuste,
    partes: &[PeticionAjuste],
    g: &GrupoRequest,
) -> Result<(), ServerFnError> {
    let prefijo = prefijo_i18n(a);
    for c in a.campos.iter().filter(|c| tapado(f, c)) {
        let Some(e) = c.escribible() else {
            continue;
        };
        let reenvia = g
            .pasos
            .iter()
            .any(|p| p.req.funcion == e.funcion && !p.req.borrar);
        let escrito = partes.iter().all(|q| {
            q.valores
                .get(&c.id)
                .is_some_and(|v| !v.is_empty() && v != MASCARA)
        });
        if reenvia && !escrito {
            let etiqueta = crate::ui::i18n::traducir(&format!("{prefijo}-{}", c.id), None);
            return Err(fallo(
                StatusCode::BAD_REQUEST,
                format!("{etiqueta}: {}", t!("ajustes-secreto-tapado-pedir")),
            ));
        }
    }
    Ok(())
}

/// Pone a cada paso su título (`titulos`, por índice). Los pasos
/// titulados después del primero solo enseñan lo que cambia respecto a
/// él (en «TCP y UDP», el protocolo), con una nota que lo dice.
fn titular_partes(pasos: &mut [PasoVista], titulos: &[Option<String>], prefijo: &str) {
    let Some(primero) = pasos.first().map(|p| p.campos.clone()) else {
        return;
    };
    for (i, paso) in pasos.iter_mut().enumerate() {
        paso.titulo = titulos.get(paso.indice).cloned().flatten();
        if i > 0 && paso.titulo.is_some() {
            paso.campos.retain(|c| !primero.contains(c));
            paso.nota = Some(format!("{prefijo}-paso-igual"));
        }
    }
}

pub(in crate::ajustes_router) async fn confirmar_ajuste(
    token: String,
    desmarcados: Vec<usize>,
) -> Result<i64, ServerFnError> {
    let (state, user) = preparar().await?;
    let g = gestor(&state)?;
    // Todo lo que cambia, desmarcado: no se escribe nada ni queda nada en
    // el Historial, y el token sigue valiendo para volver a marcar.
    if g.nada_que_aplicar(&token, &desmarcados) {
        return Err(error_ajuste(CambioError::SinCambios));
    }
    g.confirmar_grupo(&token, &desmarcados, &user.username)
        .await
        .map_err(error_ajuste)
}
