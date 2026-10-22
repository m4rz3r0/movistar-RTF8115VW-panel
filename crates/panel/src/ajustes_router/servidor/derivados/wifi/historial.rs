//! «Apagada desde»: cuándo apagó el panel cada red, según su Historial.

use super::*;

/// `apagada desde` de una red: si está apagada y lo último que hizo el
/// panel con ella fue apagarla, cuándo.
pub(in super::super) fn apagada_desde(ctx: &Contexto, e: &Elemento) -> Option<i64> {
    let (red, _) = red_de(e)?;
    if ctx.vista(red)?.red_wifi != Some(EmisionRed::Apagada) {
        return None;
    }
    let punto = ctx.ajuste(red)?.fijos.get(ID_PUNTO)?;
    ctx.apagadas.get(punto).copied()
}

/// Un paso aplicado que escribe en un punto de acceso: los valores que
/// envió (con su id) y su diff (JSON), y cuándo se aplicó su cambio.
pub(in super::super) struct PasoDeRed {
    pub valores: Leidos,
    pub diff: String,
    pub cuando: i64,
}

/// Desde cuándo está apagado cada punto de acceso según los pasos
/// aplicados por el panel, en orden: el último que tocó su `adm_state`
/// manda (si lo apagó, cuándo; si lo encendió, nada). Pura.
pub(in super::super) fn ultimo_apagado(pasos: &[PasoDeRed]) -> HashMap<String, i64> {
    let mut out = HashMap::new();
    for p in pasos {
        let Some(punto) = p.valores.get(ID_PUNTO) else {
            continue;
        };
        let diff: Vec<crate::changes::CampoDiff> =
            serde_json::from_str(&p.diff).unwrap_or_default();
        let Some(d) = diff.iter().find(|d| d.nombre == "adm_state") else {
            continue;
        };
        if d.despues.trim() == "0" {
            out.insert(punto.clone(), p.cuando);
        } else {
            out.remove(punto);
        }
    }
    out
}

/// [`ultimo_apagado`] con los pasos del Historial (los de los cambios
/// aplicados o vigilándose). Sin Historial legible, nada.
pub(in crate::ajustes_router) async fn apagadas_desde(
    db: &sqlx::SqlitePool,
) -> HashMap<String, i64> {
    use sqlx::Row;
    let filas = sqlx::query(
        "SELECT p.valores_nuevos, p.solicitud, p.diff, c.updated FROM cambio_pasos p \
         JOIN changes c ON c.id = p.cambio_id \
         WHERE p.funcion = 'cbSetWifiAccPt' AND p.estado = 'aplicado' \
         AND c.estado IN ('aplicado', 'vigilando') \
         ORDER BY c.updated, p.cambio_id, p.orden",
    )
    .fetch_all(db)
    .await
    .inspect_err(|e| tracing::warn!("ajustes: no se puede leer el Historial: {e}"))
    .unwrap_or_default();
    let pasos: Vec<PasoDeRed> = filas
        .iter()
        .map(|f| {
            let mut valores: Leidos =
                serde_json::from_slice(&f.get::<Vec<u8>, _>("valores_nuevos")).unwrap_or_default();
            let pedidos: Leidos =
                serde_json::from_str(&f.get::<String, _>("solicitud")).unwrap_or_default();
            valores.extend(pedidos);
            PasoDeRed {
                valores,
                diff: f.get("diff"),
                cuando: f.get("updated"),
            }
        })
        .collect();
    ultimo_apagado(&pasos)
}
