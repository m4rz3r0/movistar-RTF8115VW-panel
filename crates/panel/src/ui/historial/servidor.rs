//! Parte de servidor del historial: la lista (cambios y revelados mezclados
//! por fecha) y «Deshacer», que pasa por el gestor de cambios.

use super::*;
use crate::{
    changes::{Cambio, CambioError, EstadoPaso},
    state::AppState,
    ui::explorador::{
        PreparadoVista, ruta_funcion,
        servidor::{
            Ocultar, campo_cambio, con_estado, error_cambio, error_tecnico, estado, gestor,
            interno, nota_legible, riesgo, vista_preparado,
        },
    },
};
use axum::http::StatusCode;
use sqlx::Row;

/// Enlace a la página de la función en el explorador, si sigue en el catálogo.
fn ruta(state: &AppState, funcion: &str) -> Option<String> {
    state
        .catalogo
        .funcion(funcion)
        .map(|f| ruta_funcion(f.area.slug(), &f.name))
}

/// Página (desde 1) en la que está el cambio `id` con el orden de la lista:
/// fecha descendente, cambios antes que revelados a igual fecha y, dentro de
/// cada clase, el más reciente primero.
async fn vinculo(state: &AppState, id: i64) -> Result<Option<Vinculo>, ServerFnError> {
    let ts: Option<i64> = sqlx::query_scalar("SELECT ts FROM changes WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(interno)?;
    let Some(ts) = ts else {
        return Ok(None);
    };
    let antes: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM changes WHERE ts > ?1 OR (ts = ?1 AND id > ?2)) \
         + (SELECT COUNT(*) FROM reveals WHERE ts > ?1)",
    )
    .bind(ts)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(interno)?;
    Ok(Some(Vinculo {
        id,
        pagina: (antes as u32) / POR_PAGINA + 1,
    }))
}

/// Valores en claro de lo que el gestor guardó como secreto en un cambio:
/// `(orden, parámetro, antes, después, tapado)`.
type Claros = [(i64, String, Option<String>, Option<String>, bool)];

/// ¿Puede ser un dato sensible el campo del diff guardado `d`? Lo guardado
/// como secreto que, por su nombre, no se censura en pantalla.
fn puede_ser_sensible(o: &Ocultar, d: &crate::changes::CampoDiff) -> bool {
    d.secreto && !o.oculto(&d.nombre, "")
}

/// Pone en claro, en el diff del paso `orden`, los datos sensibles que el
/// gestor guardó enmascarados (sus valores van cifrados): en pantalla solo
/// se censuran los secretos de verdad. Un campo cuyo valor la pantalla
/// censuraría se queda como estaba.
fn revelar_sensibles(o: &Ocultar, diff: &mut crate::changes::Diff, orden: i64, claros: &Claros) {
    for d in diff.campos.iter_mut().filter(|d| puede_ser_sensible(o, d)) {
        let Some((_, _, antes, despues, _)) = claros
            .iter()
            .find(|(n, nombre, ..)| *n == orden && *nombre == d.nombre)
        else {
            continue;
        };
        let (antes, despues) = (
            antes.clone().unwrap_or_default(),
            despues.clone().unwrap_or_default(),
        );
        if o.oculto(&d.nombre, &antes) || o.oculto(&d.nombre, &despues) {
            continue;
        }
        d.antes = antes;
        d.despues = despues;
        d.secreto = false;
        d.sensible = true;
    }
}

async fn fila_cambio(
    state: &AppState,
    o: &Ocultar,
    de_ajustes: &Etiquetas,
    mut c: Cambio,
) -> Result<CambioFila, ServerFnError> {
    let g = gestor(state)?;
    // Los datos sensibles, en claro (se descifra este cambio solo si tiene
    // alguno).
    let con_sensibles = c
        .pasos
        .iter()
        .flat_map(|p| &p.diff.campos)
        .chain(&c.diff.campos)
        .any(|d| puede_ser_sensible(o, d));
    if con_sensibles {
        let claros = g.secretos_en_claro(c.id).await;
        for p in &mut c.pasos {
            revelar_sensibles(o, &mut p.diff, p.orden, &claros);
        }
        if let [p] = c.pasos.as_slice() {
            let orden = p.orden;
            revelar_sensibles(o, &mut c.diff, orden, &claros);
        }
    }
    let (por, deshace) = g.enlaces_deshacer(c.id).await;
    // Un deshacer guardado sin título (antes de que lo llevaran) toma el del
    // cambio que deshace: así no se queda con el nombre de la función.
    let titulo = match (&c.titulo, deshace) {
        (Some(t), _) => Some(t.clone()),
        (None, Some(d)) => g.cambio(d).await.and_then(|d| d.titulo),
        (None, None) => None,
    };
    let de_ajuste = titulo.is_some();
    // Las etiquetas del catálogo de funciones están en castellano: en otro
    // idioma, las de Ajustes (traducidas) donde las haya.
    let en_otro_idioma = crate::ui::i18n::idioma_actual() != i18n::Idioma::Es;
    // Cada campo con la etiqueta de su campo en Ajustes o, si no, la de la
    // función de su paso.
    let campos_de = |funcion: &str, diff: crate::changes::Diff| -> Vec<CampoCambio> {
        let f = state.catalogo.funcion(funcion);
        diff.campos
            .into_iter()
            .map(|d| {
                let mut c = campo_cambio(f, o, d);
                // Un cambio hecho desde Ajustes (lleva su título). Uno de la
                // Biblioteca se queda en castellano con las etiquetas de su
                // función, como en la Biblioteca; en otro idioma, toma la de
                // Ajustes si el catálogo de ajustes la trae.
                if (de_ajuste || en_otro_idioma)
                    && let Some(e) = de_ajustes.get(&(funcion.to_owned(), c.nombre.clone()))
                {
                    c.etiqueta = e.clone();
                }
                c
            })
            .collect()
    };
    let traducir = |clave: &str| crate::ui::i18n::traducir(clave, None);
    let (campos, pasos) = if c.pasos.len() > 1 {
        let pasos: Vec<PasoFila> = c
            .pasos
            .into_iter()
            .map(|p| PasoFila {
                numero: p.orden + 1,
                // Sin etiqueta, un texto genérico: nunca el nombre de la
                // función (va en los detalles técnicos).
                etiqueta: p
                    .etiqueta
                    .as_deref()
                    .map_or_else(|| crate::t!("historial-cambio-router"), traducir),
                campos: campos_de(&p.funcion, p.diff),
                funcion: p.funcion,
                instancia: p.instancia,
                estado: estado_paso(p.estado),
                error_tecnico: p.error.as_deref().and_then(error_tecnico),
                error: p.error.as_deref().map(nota_legible),
            })
            .collect();
        let campos = pasos.iter().flat_map(|p| p.campos.clone()).collect();
        (campos, pasos)
    } else {
        (campos_de(&c.funcion, c.diff), Vec::new())
    };
    let puede_deshacer = g.se_puede_deshacer(c.id).await;
    // Aplicado y sin deshacer, pero sin nada que volver atrás: solo cambió
    // lo que el router no deja leer. La fila dice por qué.
    let sin_deshacer =
        if !puede_deshacer && por.is_none() && c.estado == crate::changes::Estado::Aplicado {
            match g.tapados_del_cambio(c.id).await {
                (true, tapados) => Some(crate::t!(
                    "cambio-sin-deshacer-tapado",
                    campos = nombres(de_ajustes, state, &tapados)
                )),
                _ => None,
            }
        } else {
            None
        };
    let enlace = |id: Option<i64>| async move {
        match id {
            Some(id) => vinculo(state, id).await,
            None => Ok(None),
        }
    };
    Ok(CambioFila {
        id: c.id,
        ts: c.ts,
        updated: c.updated,
        usuario: c.usuario,
        ruta: ruta(state, &c.funcion),
        funcion: c.funcion,
        instancia: c.instancia,
        riesgo: riesgo(c.riesgo),
        estado: estado(c.estado),
        resumen: resumen(&campos),
        campos,
        nota: c.nota.as_deref().map(nota_legible),
        // Lo técnico del error o, sin él, de la nota (la de una escritura
        // sin respuesta, o una clave que este panel no conoce).
        error_tecnico: c
            .error
            .as_deref()
            .and_then(error_tecnico)
            .or_else(|| c.nota.as_deref().and_then(error_tecnico)),
        error: c.error.as_deref().map(nota_legible),
        deshecho_por: enlace(por).await?,
        deshace: enlace(deshace).await?,
        puede_deshacer,
        sin_deshacer,
        titulo: titulo.as_deref().map(traducir),
        pasos,
    })
}

/// Etiquetas de los campos de Ajustes por (función, parámetro).
type Etiquetas = std::collections::HashMap<(String, String), String>;

/// Las etiquetas de `tapados` (`(función, parámetro)`), juntas: las de
/// Ajustes o, si no, las del catálogo de funciones.
pub(crate) fn nombres(e: &Etiquetas, state: &AppState, tapados: &[(String, String)]) -> String {
    let mut v: Vec<String> = tapados
        .iter()
        .map(|(f, p)| {
            e.get(&(f.clone(), p.clone())).cloned().unwrap_or_else(|| {
                state
                    .catalogo
                    .funcion(f)
                    .and_then(|func| func.parametro(p))
                    .map_or_else(|| p.clone(), |x| x.etiqueta.clone())
            })
        })
        .collect();
    v.dedup();
    v.join(", ")
}

/// Los secretos del cambio `cambio` con su valor anterior y el nuevo, para
/// su detalle desplegado (descifra solo este cambio).
pub async fn secretos_del_cambio(
    state: &AppState,
    cambio: i64,
) -> Result<Vec<SecretoDelCambio>, ServerFnError> {
    let g = gestor(state)?;
    let o = Ocultar::new(state);
    // Solo los secretos de verdad: los datos sensibles ya van en claro en la
    // fila.
    let secreto = |nombre: &str, v: &Option<String>| o.oculto(nombre, v.as_deref().unwrap_or(""));
    Ok(g.secretos_en_claro(cambio)
        .await
        .into_iter()
        .filter(|(_, nombre, antes, despues, _)| secreto(nombre, antes) || secreto(nombre, despues))
        .map(|(orden, nombre, antes, despues, tapado)| SecretoDelCambio {
            orden,
            nombre,
            antes,
            despues,
            tapado,
        })
        .collect())
}

fn estado_paso(e: EstadoPaso) -> EstadoPasoVista {
    match e {
        EstadoPaso::Pendiente => EstadoPasoVista::Pendiente,
        EstadoPaso::Omitido => EstadoPasoVista::Omitido,
        EstadoPaso::Aplicando => EstadoPasoVista::Aplicando,
        EstadoPaso::Aplicado => EstadoPasoVista::Aplicado,
        EstadoPaso::Fallido => EstadoPasoVista::Fallido,
        EstadoPaso::Revertido => EstadoPasoVista::Revertido,
    }
}

/// El nombre legible del campo `campo` de `funcion` para un revelado: en
/// castellano, el del catálogo de funciones; en otro idioma, el de Ajustes
/// si lo trae. Sin ninguno, `None` (la fila dice «una clave»).
fn etiqueta_revelado(
    state: &AppState,
    de_ajustes: &Etiquetas,
    funcion: &str,
    campo: &str,
) -> Option<String> {
    let del_catalogo = || {
        state
            .catalogo
            .funcion(funcion)
            .and_then(|f| f.parametro(campo))
            // Sin la aclaración técnica del final («Clave WiFi (XOR 0x1f)»).
            .map(|p| match p.etiqueta.rsplit_once(" (") {
                Some((nombre, _)) if p.etiqueta.ends_with(')') => nombre.to_owned(),
                _ => p.etiqueta.clone(),
            })
    };
    let de_ajustes = || {
        de_ajustes
            .get(&(funcion.to_owned(), campo.to_owned()))
            .cloned()
    };
    if crate::ui::i18n::idioma_actual() == i18n::Idioma::Es {
        del_catalogo().or_else(de_ajustes)
    } else {
        de_ajustes()
    }
}

async fn fila_revelado(
    state: &AppState,
    de_ajustes: &Etiquetas,
    id: i64,
) -> Result<Option<ReveladoFila>, ServerFnError> {
    let fila = sqlx::query(
        "SELECT r.ts, r.funcion, r.campo, r.instancia, u.username \
         FROM reveals r LEFT JOIN users u ON u.id = r.user_id WHERE r.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(interno)?;
    Ok(fila.map(|f| {
        let funcion: String = f.get("funcion");
        let campo: String = f.get("campo");
        ReveladoFila {
            id,
            ts: f.get("ts"),
            usuario: f.get("username"),
            ruta: ruta(state, &funcion),
            etiqueta: etiqueta_revelado(state, de_ajustes, &funcion, &campo),
            funcion,
            campo,
            instancia: f.get("instancia"),
        }
    }))
}

pub async fn pagina(state: &AppState, pagina: u32) -> Result<PaginaHistorial, ServerFnError> {
    let g = gestor(state)?;
    let pagina = pagina.max(1);
    let total: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM changes) + (SELECT COUNT(*) FROM reveals)",
    )
    .fetch_one(&state.db)
    .await
    .map_err(interno)?;
    let desde = u64::from(pagina - 1) * u64::from(POR_PAGINA);
    let claves = sqlx::query(
        "SELECT k, id FROM (\
            SELECT 'c' AS k, id, ts FROM changes \
            UNION ALL SELECT 'r' AS k, id, ts FROM reveals) \
         ORDER BY ts DESC, k, id DESC LIMIT ? OFFSET ?",
    )
    .bind(i64::from(POR_PAGINA))
    .bind(desde as i64)
    .fetch_all(&state.db)
    .await
    .map_err(interno)?;
    let o = Ocultar::new(state);
    // Una vez por página (no por fila): las etiquetas de Ajustes.
    let de_ajustes = crate::ajustes_router::servidor::etiquetas_de_campos(state);
    let mut entradas = Vec::with_capacity(claves.len());
    for c in &claves {
        let id: i64 = c.get("id");
        let entrada = match c.get::<String, _>("k").as_str() {
            "c" => match g.cambio(id).await {
                Some(c) => Some(EntradaVista::Cambio(Box::new(
                    fila_cambio(state, &o, &de_ajustes, c).await?,
                ))),
                None => None,
            },
            _ => fila_revelado(state, &de_ajustes, id)
                .await?
                .map(EntradaVista::Revelado),
        };
        entradas.extend(entrada);
    }
    let total = total as u64;
    Ok(PaginaHistorial {
        entradas,
        pagina,
        total: total as u32,
        hay_mas: desde + u64::from(POR_PAGINA) < total,
    })
}

/// Deshacer un alta o una baja de usuarios del router exige la ventana de
/// reautenticación, como crearlos o borrarlos; si no, 401.
pub async fn exigir_reauth_si_usuarios(
    state: &AppState,
    change_id: i64,
) -> Result<(), ServerFnError> {
    if gestor(state)?.es_de_usuarios_el_cambio(change_id).await {
        crate::auth::require_reauth().await?;
    }
    Ok(())
}

pub async fn deshacer(state: &AppState, change_id: i64) -> Result<PreparadoVista, ServerFnError> {
    let g = gestor(state)?;
    match g.deshacer(change_id).await {
        Ok(p) => {
            let mut v = vista_preparado(state, &g, p);
            // Lo que el router no deja leer se queda como se escribió.
            let (_, tapados) = g.tapados_del_cambio(change_id).await;
            if !tapados.is_empty() {
                let e = crate::ajustes_router::servidor::etiquetas_de_campos(state);
                v.aviso = Some(crate::t!(
                    "cambio-aviso-tapado",
                    campos = nombres(&e, state, &tapados)
                ));
            }
            Ok(v)
        }
        Err(CambioError::Invalida(m)) if m == crate::changes::DESHACER_SOLO_TAPADOS => {
            let (_, tapados) = g.tapados_del_cambio(change_id).await;
            let e = crate::ajustes_router::servidor::etiquetas_de_campos(state);
            Err(con_estado(
                StatusCode::BAD_REQUEST,
                crate::t!(
                    "cambio-sin-deshacer-tapado",
                    campos = nombres(&e, state, &tapados)
                ),
            ))
        }
        Err(CambioError::Invalida(m)) => Err(con_estado(
            StatusCode::BAD_REQUEST,
            crate::t!("historial-error-deshacer", motivo = m),
        )),
        Err(e) => Err(error_cambio(e)),
    }
}
