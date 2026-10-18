//! Parte de servidor del historial: la lista (cambios y revelados mezclados
//! por fecha) y «Deshacer», que pasa por el gestor de cambios.

use super::*;
use crate::{
    changes::{Cambio, CambioError, EstadoPaso},
    state::AppState,
    ui::explorador::{
        PreparadoVista, ruta_funcion,
        servidor::{
            Ocultar, campo_cambio, con_estado, error_cambio, estado, gestor, interno, riesgo,
            usuario_para_revelar, vista_preparado,
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

async fn fila_cambio(
    state: &AppState,
    o: &Ocultar,
    c: Cambio,
) -> Result<CambioFila, ServerFnError> {
    let g = gestor(state)?;
    // Cada campo con las etiquetas de la función de su paso.
    let campos_de = |funcion: &str, diff: crate::changes::Diff| -> Vec<CampoCambio> {
        let f = state.catalogo.funcion(funcion);
        diff.campos
            .into_iter()
            .map(|d| campo_cambio(f, o, d))
            .collect()
    };
    let traducir = |clave: &str| crate::ui::i18n::traducir(clave, None);
    let (campos, pasos) = if c.pasos.len() > 1 {
        let pasos: Vec<PasoFila> = c
            .pasos
            .into_iter()
            .map(|p| PasoFila {
                numero: p.orden + 1,
                etiqueta: p
                    .etiqueta
                    .as_deref()
                    .map_or_else(|| p.funcion.clone(), traducir),
                campos: campos_de(&p.funcion, p.diff),
                funcion: p.funcion,
                instancia: p.instancia,
                estado: estado_paso(p.estado),
                error: p.error,
            })
            .collect();
        let campos = pasos.iter().flat_map(|p| p.campos.clone()).collect();
        (campos, pasos)
    } else {
        (campos_de(&c.funcion, c.diff), Vec::new())
    };
    let (por, deshace) = g.enlaces_deshacer(c.id).await;
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
        nota: c.nota,
        error: c.error,
        deshecho_por: enlace(por).await?,
        deshace: enlace(deshace).await?,
        puede_deshacer: g.se_puede_deshacer(c.id).await,
        titulo: c.titulo.as_deref().map(traducir),
        pasos,
        anteriores: {
            let mut v = Vec::new();
            for (orden, funcion, param) in g.secretos_con_anterior(c.id).await {
                let (_, etiqueta) = etiqueta_del_secreto(state, c.id, &funcion, &param).await;
                v.push((orden, param, etiqueta));
            }
            v
        },
    })
}

/// Título del ajuste (si el cambio `cambio` se hizo desde Ajustes) y
/// etiqueta del secreto `param` de la escritura `funcion`, la misma que en
/// Ajustes. Si no es de Ajustes, la etiqueta del parámetro en el catálogo
/// de funciones (nunca su nombre, salvo que ya no esté en él).
async fn etiqueta_del_secreto(
    state: &AppState,
    cambio: i64,
    funcion: &str,
    param: &str,
) -> (Option<String>, String) {
    let traducir = |clave: &str| crate::ui::i18n::traducir(clave, None);
    let de_ajustes = match gestor(state) {
        Ok(g) => crate::ajustes_router::servidor::ajuste_del_cambio(state, &g, cambio).await,
        Err(_) => None,
    };
    if let Some((a, _)) = de_ajustes {
        let p = ajustes::prefijo_i18n(a);
        if let Some(c) = a.campos.iter().find(|c| {
            c.escribible()
                .is_some_and(|e| e.funcion == funcion && e.parametro == param)
        }) {
            return (
                Some(traducir(&format!("{p}-titulo"))),
                traducir(&format!("{p}-{}", c.id)),
            );
        }
    }
    let etiqueta = state
        .catalogo
        .funcion(funcion)
        .and_then(|f| f.parametro(param))
        .map_or_else(|| param.to_owned(), |p| p.etiqueta.clone());
    (None, etiqueta)
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

async fn fila_revelado(state: &AppState, id: i64) -> Result<Option<ReveladoFila>, ServerFnError> {
    let fila = sqlx::query(
        "SELECT r.ts, r.funcion, r.campo, r.instancia, r.ajuste, r.cambio_id, u.username \
         FROM reveals r LEFT JOIN users u ON u.id = r.user_id WHERE r.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(interno)?;
    let traducir = |clave: &str| crate::ui::i18n::traducir(clave, None);
    let Some(f) = fila else {
        return Ok(None);
    };
    let funcion: String = f.get("funcion");
    let campo: String = f.get("campo");
    let ajuste: Option<String> = f.get("ajuste");
    let cambio: Option<i64> = f.get("cambio_id");
    // Desde Ajustes: el título del ajuste y la etiqueta del campo, sin la
    // función (si el ajuste ya no está en el catálogo, sus ids). Un valor
    // anterior: la etiqueta del secreto (y su ajuste, si el cambio se hizo
    // desde Ajustes). Si no, el parámetro y la función, como en la
    // Biblioteca.
    let (ajuste, campo, ruta) = match (ajuste, cambio) {
        (Some(id), _) => match state.ajustes.ajuste(&id) {
            Some(a) => {
                let p = ajustes::prefijo_i18n(a);
                (
                    Some(traducir(&format!("{p}-titulo"))),
                    traducir(&format!("{p}-{campo}")),
                    None,
                )
            }
            None => (Some(id), campo, None),
        },
        (None, Some(cambio)) => {
            let (ajuste, etiqueta) = etiqueta_del_secreto(state, cambio, &funcion, &campo).await;
            (ajuste, etiqueta, None)
        }
        (None, None) => (None, campo, ruta(state, &funcion)),
    };
    Ok(Some(ReveladoFila {
        id,
        ts: f.get("ts"),
        usuario: f.get("username"),
        ruta,
        funcion,
        campo,
        instancia: f.get("instancia"),
        ajuste,
        cambio,
    }))
}

/// Marca la respuesta `no-store`: lleva un secreto.
pub(super) fn sin_guardar() {
    use axum::http::{HeaderValue, header};
    if let Some(resp) = leptos::prelude::use_context::<leptos_axum::ResponseOptions>() {
        resp.insert_header(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
}

pub async fn revelar_anterior(
    state: &AppState,
    user: crate::auth::User,
    cambio: i64,
    orden: i64,
    campo: &str,
    contrasena: &str,
) -> Result<String, ServerFnError> {
    // Primero la contraseña: sin ella no se descifra ni se dice si existe.
    let user = usuario_para_revelar(state, user, contrasena).await?;
    let g = gestor(state)?;
    // Solo los secretos con valor anterior de verdad (no los tapados al leer).
    let guardado = g
        .secretos_con_anterior(cambio)
        .await
        .iter()
        .any(|(o, _, p)| *o == orden && p == campo);
    let Some((funcion, valor)) = g
        .anteriores_secretos(cambio)
        .await
        .into_iter()
        .filter(|(o, _, _)| guardado && *o == orden)
        .find_map(|(_, funcion, mut m)| m.remove(campo).map(|v| (funcion, v)))
    else {
        return Err(con_estado(
            StatusCode::NOT_FOUND,
            crate::t!("historial-error-sin-anterior"),
        ));
    };
    sqlx::query(
        "INSERT INTO reveals (ts, user_id, funcion, campo, instancia, cambio_id) \
         VALUES (?, ?, ?, ?, NULL, ?)",
    )
    .bind(crate::series::now())
    .bind(user.id)
    .bind(&funcion)
    .bind(campo)
    .bind(cambio)
    .execute(&state.db)
    .await
    .map_err(interno)?;
    tracing::info!(
        usuario = %user.username,
        cambio,
        orden,
        campo,
        "valor anterior de un secreto mostrado"
    );
    Ok(valor)
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
    let mut entradas = Vec::with_capacity(claves.len());
    for c in &claves {
        let id: i64 = c.get("id");
        let entrada = match c.get::<String, _>("k").as_str() {
            "c" => match g.cambio(id).await {
                Some(c) => Some(EntradaVista::Cambio(fila_cambio(state, &o, c).await?)),
                None => None,
            },
            _ => fila_revelado(state, id).await?.map(EntradaVista::Revelado),
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

pub async fn deshacer(state: &AppState, change_id: i64) -> Result<PreparadoVista, ServerFnError> {
    let g = gestor(state)?;
    match g.deshacer(change_id).await {
        Ok(p) => Ok(vista_preparado(state, &g, p)),
        Err(CambioError::Invalida(m)) => Err(con_estado(
            StatusCode::BAD_REQUEST,
            crate::t!("historial-error-deshacer", motivo = m),
        )),
        Err(e) => Err(error_cambio(e)),
    }
}
