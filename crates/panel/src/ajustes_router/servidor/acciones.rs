//! Acciones sobre varios elementos: un grupo del gestor, un paso por elemento.

use super::guardar::{exigir_tapados, grupo_de, instancias_fijas, paso_vista};
use super::*;
use ajustes::{Elemento, titulo_accion};

/// ¿Es `titulo` el de un grupo de una acción ([`titulo_accion`])?
pub(super) fn es_de_accion(titulo: &str) -> bool {
    titulo.starts_with("ajuste-accion-") && titulo.ends_with("-titulo")
}

/// El ajuste de `e` con el campo `campo` escribible: donde escribe la
/// acción.
fn ajuste_de<'a>(state: &'a AppState, e: &Elemento, campo: &str) -> Option<&'a Ajuste> {
    e.ajustes
        .iter()
        .filter_map(|id| state.ajustes.ajuste(id))
        .find(|a| a.campo(campo).is_some_and(|c| c.escribible().is_some()))
}

/// Ids sin repetir, en el orden en que llegan.
fn sin_repetir(ids: Vec<String>) -> Vec<String> {
    let mut vistos = std::collections::HashSet::new();
    ids.into_iter()
        .filter(|i| vistos.insert(i.clone()))
        .collect()
}

pub(in crate::ajustes_router) async fn preparar_accion(
    tema: String,
    accion: String,
    elementos: Vec<String>,
) -> Result<GrupoVista, ServerFnError> {
    let (state, _) = preparar().await?;
    let tema = Tema::from_slug(&tema)
        .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-tema")))?;
    if elementos.len() > MAX_ELEMENTOS_ACCION {
        return Err(fallo(
            StatusCode::BAD_REQUEST,
            t!("ajustes-accion-demasiados", max = MAX_ELEMENTOS_ACCION),
        ));
    }
    let no_aplica = || fallo(StatusCode::BAD_REQUEST, t!("ajustes-accion-no-aplica"));
    let a = state
        .ajustes
        .acciones_de(tema)
        .find(|a| a.id == accion)
        .ok_or_else(no_aplica)?;
    let (Some(campo), Some(valor)) = (&a.campo, &a.valor) else {
        return Err(no_aplica());
    };
    let ids = sin_repetir(elementos);
    if ids.is_empty() {
        return Err(no_aplica());
    }
    let mut destinos: Vec<&Ajuste> = Vec::with_capacity(ids.len());
    for id in &ids {
        let e = state
            .ajustes
            .elementos_de(tema)
            .find(|e| e.id == *id)
            .ok_or_else(no_aplica)?;
        destinos.push(ajuste_de(&state, e, campo).ok_or_else(no_aplica)?);
    }
    let g = gestor(&state)?;
    libre(&g)?;
    let mut grupo = GrupoRequest {
        pasos: Vec::new(),
        titulo: Some(titulo_accion(&a.id)),
    };
    // El ajuste de cada paso, para su resumen.
    let mut de_cada_paso: Vec<&Ajuste> = Vec::new();
    for aj in destinos {
        let p = PeticionAjuste {
            ajuste: aj.id.clone(),
            instancia: None,
            borrar: false,
            valores: [(campo.clone(), valor.clone())].into(),
            esperados: BTreeMap::new(),
        };
        let invalida = |m: String| {
            tracing::warn!("ajustes: acción {} en {}: {m}", a.id, aj.id);
            fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
        };
        let mut sub = grupo_de(aj, &p).map_err(|e| invalida(e.to_string()))?;
        instancias_fijas(&state.catalogo, aj, &mut sub).map_err(invalida)?;
        // Una escritura que reenvía una clave que el router no deja leer
        // (la de una línea de teléfono) la borraría: desde la barra no se
        // puede escribir, así que esa acción no vale aquí.
        exigir_tapados(&state.catalogo, aj, std::slice::from_ref(&p), &sub).map_err(|_| {
            fallo(
                StatusCode::BAD_REQUEST,
                t!(
                    "ajustes-accion-tapado",
                    ajuste =
                        crate::ui::i18n::traducir(&format!("{}-titulo", prefijo_i18n(aj)), None)
                ),
            )
        })?;
        de_cada_paso.extend(std::iter::repeat_n(aj, sub.pasos.len()));
        grupo.pasos.extend(sub.pasos);
    }
    let pedidos: Vec<BTreeMap<String, String>> =
        grupo.pasos.iter().map(|p| p.req.valores.clone()).collect();
    let prep = g.preparar_grupo(grupo).await.map_err(|e| match e {
        CambioError::SinCambios => fallo(StatusCode::CONFLICT, t!("ajustes-accion-ya-estaba")),
        e => error_ajuste(e),
    })?;
    Ok(vista_por_pasos(
        &g,
        titulo_accion(&a.id),
        &de_cada_paso,
        prep.token,
        &prep.pasos,
        prep.riesgo,
        &|i, param| buscar(&pedidos[i], param).map(str::to_owned),
    ))
}

/// Resumen de un grupo cuyos pasos escriben en ajustes distintos (`ajustes`,
/// uno por paso): cada paso lleva el título de su ajuste, y el grupo, el
/// efecto del primero que cambia algo y lo tiene.
pub(super) fn vista_por_pasos(
    g: &ChangeManager,
    titulo: String,
    ajustes: &[&Ajuste],
    token: String,
    pasos: &[PasoPreparado],
    r: Riesgo,
    completar: &dyn Fn(usize, &str) -> Option<String>,
) -> GrupoVista {
    let efecto = pasos
        .iter()
        .filter(|p| !p.sin_cambios)
        .find_map(|p| ajustes.get(p.indice).and_then(|a| a.efecto.as_ref()))
        .map(|e| format!("efecto-{e}"));
    GrupoVista {
        token,
        titulo,
        pasos: pasos
            .iter()
            .filter_map(|paso| {
                let a = ajustes.get(paso.indice)?;
                Some(PasoVista {
                    titulo: Some(format!("{}-titulo", prefijo_i18n(a))),
                    ..paso_vista(a, paso, &|param| completar(paso.indice, param))
                })
            })
            .collect(),
        riesgo: riesgo(r),
        efecto,
        vigilancia_s: g.tiempos().vigilancia(r).as_secs(),
        detalle: None,
        aviso: None,
    }
}

/// Ajuste cuyo título es la etiqueta `etiqueta` de un paso.
pub(super) fn ajuste_de_etiqueta<'a>(
    state: &'a AppState,
    etiqueta: Option<&str>,
) -> Option<&'a Ajuste> {
    let etiqueta = etiqueta?;
    state
        .ajustes
        .todos()
        .iter()
        .find(|a| format!("{}-titulo", prefijo_i18n(a)) == etiqueta)
}

/// Resumen del deshacer preparado de una acción (`titulo`): el ajuste de
/// cada paso sale de su etiqueta, y lo que no cambia, de lo que muestra
/// ahora el router. `None` si un paso no es de ningún ajuste.
pub(super) async fn vista_deshacer(
    state: &AppState,
    g: &ChangeManager,
    titulo: String,
    prep: &crate::changes::Preparado,
) -> Option<GrupoVista> {
    let mut ajustes: Vec<&Ajuste> = Vec::with_capacity(prep.pasos.len());
    let mut actuales: Vec<BTreeMap<String, String>> = Vec::new();
    for paso in &prep.pasos {
        let a = ajuste_de_etiqueta(state, paso.etiqueta.as_deref())?;
        let leidos = if paso.sin_cambios {
            None
        } else {
            g.valores_actuales(&paso.funcion, None, &a.fijos)
                .await
                .inspect_err(|e| {
                    tracing::warn!("ajustes: {}: no se puede leer para el resumen: {e}", a.id);
                })
                .ok()
                .flatten()
        };
        ajustes.push(a);
        actuales.push(leidos.unwrap_or_default());
    }
    Some(vista_por_pasos(
        g,
        titulo,
        &ajustes,
        prep.token.clone(),
        &prep.pasos,
        prep.riesgo,
        &|i, param| buscar(&actuales[i], param).map(str::to_owned),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titulo_de_una_accion() {
        assert_eq!(
            titulo_accion("quitar_wps"),
            "ajuste-accion-quitar_wps-titulo"
        );
        assert!(es_de_accion(&titulo_accion("aislar")));
        assert!(!es_de_accion("ajuste-wifi-red_24-titulo"));
    }

    #[test]
    fn ids_sin_repetir_en_su_orden() {
        let ids = ["b", "a", "b", "c", "a"].map(String::from).to_vec();
        assert_eq!(sin_repetir(ids), ["b", "a", "c"]);
    }
}
