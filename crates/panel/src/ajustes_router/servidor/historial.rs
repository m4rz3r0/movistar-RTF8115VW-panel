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
    let (a, instancia) = ajuste_del_cambio(&state, &g, cambio)
        .await
        .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-cambio")))?;
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
    // Lo que no cambia en cada paso, como lo muestra ahora el router
    // (una lectura por paso con cambios).
    let mut actuales: Vec<BTreeMap<String, String>> = Vec::new();
    for paso in &prep.pasos {
        let leidos = if paso.sin_cambios {
            None
        } else {
            g.valores_actuales(&paso.funcion, instancia.as_deref(), &a.fijos)
                .await
                .inspect_err(|e| {
                    tracing::warn!("ajustes: {}: no se puede leer para el resumen: {e}", a.id);
                })
                .ok()
                .flatten()
        };
        actuales.push(leidos.unwrap_or_default());
    }
    let vista = vista_grupo(&g, a, prep.token, &prep.pasos, prep.riesgo, &|i, param| {
        buscar(&actuales[i], param).map(str::to_owned)
    });
    // Lo que el router no deja leer se queda como se escribió.
    let aviso =
        (!tapados.is_empty()).then(|| t!("cambio-aviso-tapado", campos = nombres_tapados()));
    Ok(GrupoVista { aviso, ..vista })
}
