//! Parte de servidor del historial: la lista (cambios y revelados mezclados
//! por fecha) y «Deshacer», que pasa por el gestor de cambios.

use super::*;
use crate::{
    changes::{Cambio, CambioError},
    state::AppState,
    ui::explorador::{
        PreparadoVista, ruta_funcion,
        servidor::{
            Ocultar, campo_cambio, con_estado, error_cambio, estado, gestor, interno, riesgo,
            vista_preparado,
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
    let f = state.catalogo.funcion(&c.funcion);
    let campos: Vec<CampoCambio> = c
        .diff
        .campos
        .into_iter()
        .map(|d| campo_cambio(f, o, d))
        .collect();
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
    })
}

async fn fila_revelado(state: &AppState, id: i64) -> Result<Option<ReveladoFila>, ServerFnError> {
    let fila = sqlx::query(
        "SELECT r.ts, r.funcion, r.campo, r.instancia, u.username \
         FROM reveals r LEFT JOIN users u ON u.id = r.user_id WHERE r.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(interno)?;
    Ok(fila.map(|f| ReveladoFila {
        id,
        ts: f.get("ts"),
        usuario: f.get("username"),
        ruta: ruta(state, &f.get::<String, _>("funcion")),
        funcion: f.get("funcion"),
        campo: f.get("campo"),
        instancia: f.get("instancia"),
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
