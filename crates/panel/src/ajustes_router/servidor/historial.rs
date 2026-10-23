//! Ajustes en el historial: etiquetas de campos, ajuste de un cambio y deshacer.

use super::guardar::vista_grupo;

use super::*;

/// Ajuste e instancia del cambio `id`, hecho desde Ajustes: por su
/// título o, si es un deshacer (que no lo lleva), por el del cambio que
/// deshace.
/// Etiqueta (traducida) de cada campo que se escribe del catálogo de
/// ajustes, por (función, parámetro): la que se ve en Ajustes. El primer
/// ajuste que lo escribe da la suya.
pub(crate) fn etiquetas_de_campos(state: &AppState) -> HashMap<(String, String), String> {
    let mut out = HashMap::new();
    for a in state.ajustes.todos() {
        let p = prefijo_i18n(a);
        for c in &a.campos {
            if let Some(e) = &c.escribe {
                out.entry((e.funcion.clone(), e.parametro.clone()))
                    .or_insert_with(|| crate::ui::i18n::traducir(&format!("{p}-{}", c.id), None));
            }
        }
    }
    out
}

pub(crate) async fn ajuste_del_cambio<'a>(
    state: &'a AppState,
    g: &ChangeManager,
    id: i64,
) -> Option<(&'a Ajuste, Option<String>)> {
    let mut actual = id;
    // Una cadena de deshacer no debería ser larga; el tope evita un
    // bucle si el historial tuviera un ciclo.
    for _ in 0..32 {
        let c = g.cambio(actual).await?;
        if let Some(titulo) = &c.titulo {
            let a = state
                .ajustes
                .todos()
                .iter()
                .find(|a| format!("{}-titulo", prefijo_i18n(a)) == *titulo)?;
            let instancia = c.pasos.iter().find_map(|p| p.instancia.clone());
            return Some((a, instancia.or(c.instancia)));
        }
        actual = g.enlaces_deshacer(actual).await.1?;
    }
    None
}

pub(in crate::ajustes_router) async fn deshacer_ajuste(
    cambio: i64,
) -> Result<GrupoVista, ServerFnError> {
    let (state, _) = preparar().await?;
    let g = gestor(&state)?;
    // De un ajuste o de una acción sobre varios elementos (un paso por
    // elemento, cada uno con el título de su ajuste).
    let de_ajuste = ajuste_del_cambio(&state, &g, cambio).await;
    let de_accion = match de_ajuste {
        Some(_) => None,
        None => g
            .cambio(cambio)
            .await
            .and_then(|c| c.titulo)
            .filter(|t| super::acciones::es_de_accion(t)),
    };
    if de_ajuste.is_none() && de_accion.is_none() {
        return Err(fallo(StatusCode::NOT_FOUND, t!("ajustes-error-cambio")));
    }
    // Antes que «otro cambio en curso»: un cambio que aún se vigila (el
    // activo) o que ya se deshizo no se puede deshacer.
    let (solo_tapados, tapados) = g.tapados_del_cambio(cambio).await;
    let nombres_tapados = || {
        let e = etiquetas_de_campos(&state);
        crate::ui::historial::servidor::nombres(&e, &state, &tapados)
    };
    if !g.se_puede_deshacer(cambio).await {
        // Primero el estado (aún se vigila, ya se deshizo…); solo un
        // cambio aplicado y sin deshacer en el que solo cambió lo que el
        // router no deja leer da ese motivo.
        let aplicado = g
            .cambio(cambio)
            .await
            .is_some_and(|c| c.estado == crate::changes::Estado::Aplicado);
        let deshecho = g.enlaces_deshacer(cambio).await.0.is_some();
        let msg = if solo_tapados && aplicado && !deshecho {
            t!("cambio-sin-deshacer-tapado", campos = nombres_tapados())
        } else {
            t!("ajustes-error-no-deshacer")
        };
        return Err(fallo(StatusCode::BAD_REQUEST, msg));
    }
    libre(&g)?;
    let prep = g.deshacer(cambio).await.map_err(error_ajuste)?;
    // Lo que el router no deja leer se queda como se escribió.
    let aviso =
        (!tapados.is_empty()).then(|| t!("cambio-aviso-tapado", campos = nombres_tapados()));
    let Some((a, instancia)) = de_ajuste else {
        let titulo = de_accion.unwrap_or_default();
        let vista = super::acciones::vista_deshacer(&state, &g, titulo, &prep)
            .await
            .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-cambio")))?;
        return Ok(GrupoVista { aviso, ..vista });
    };
    // Los usuarios del router tienen su propio resumen. Un deshacer que
    // vuelve a crear un usuario borrado le da una contraseña nueva.
    if a.componente == Some(Componente::UsuariosRouter) {
        let recrea = prep
            .pasos
            .iter()
            .any(|p| p.diff.campos.iter().any(|c| c.nombre == "rqPwd"));
        let borra = prep.borrar;
        let detalle = prep.instancia.clone().or(instancia);
        let vista =
            crate::usuarios_router::servidor::vista(&g, prep.token, &prep.pasos, prep.riesgo);
        return Ok(GrupoVista {
            aviso: aviso.or_else(|| recrea.then(|| t!("usuarios-router-aviso-deshacer"))),
            borra,
            detalle,
            ..vista
        });
    }
    // La regla «sin ping» del cortafuegos, también: su deshacer la quita.
    if a.componente == Some(Componente::Cortafuegos) {
        let vista = super::cortafuegos::vista(&g, prep.token, &prep.pasos, prep.riesgo);
        return Ok(GrupoVista { aviso, ..vista });
    }
    // El ajuste de cada paso (el detalle de un elemento guarda varios en un
    // cambio: cada paso lleva el título del suyo) y lo que no cambia en
    // él, como lo muestra ahora el router (una lectura por paso con
    // cambios).
    let mut de_cada_paso: Vec<&Ajuste> = Vec::new();
    let mut actuales: Vec<BTreeMap<String, String>> = Vec::new();
    for paso in &prep.pasos {
        let del_paso =
            super::acciones::ajuste_de_etiqueta(&state, paso.etiqueta.as_deref()).unwrap_or(a);
        let leidos = if paso.sin_cambios {
            None
        } else {
            g.valores_actuales(&paso.funcion, instancia.as_deref(), &del_paso.fijos)
                .await
                .inspect_err(|e| {
                    tracing::warn!("ajustes: {}: no se puede leer para el resumen: {e}", a.id);
                })
                .ok()
                .flatten()
        };
        de_cada_paso.push(del_paso);
        actuales.push(leidos.unwrap_or_default());
    }
    let completar = |i: usize, param: &str| buscar(&actuales[i], param).map(str::to_owned);
    let vista = if de_cada_paso.iter().all(|x| x.id == a.id) {
        vista_grupo(&g, a, prep.token, &prep.pasos, prep.riesgo, &completar)
    } else {
        super::acciones::vista_por_pasos(
            &g,
            format!("{}-titulo", prefijo_i18n(a)),
            &de_cada_paso,
            prep.token,
            &prep.pasos,
            prep.riesgo,
            &completar,
        )
    };
    Ok(GrupoVista { aviso, ..vista })
}
