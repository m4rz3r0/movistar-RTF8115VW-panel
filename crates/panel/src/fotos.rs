//! Fotos de la configuración del router: la respuesta de cada lectura, por
//! nombre de función, guardada cifrada con la clave maestra (tabla `fotos`).

use crate::crypto::HistoryKey;
use sqlx::SqlitePool;
use std::collections::BTreeMap;

/// Por qué se tomó la foto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoFoto {
    /// Lectura completa del asistente de instalación.
    Partida,
    /// Tomada a mano.
    Manual,
}

impl TipoFoto {
    fn como_texto(self) -> &'static str {
        match self {
            TipoFoto::Partida => "partida",
            TipoFoto::Manual => "manual",
        }
    }
}

/// Guarda una foto (función → XML de su respuesta) cifrada con `clave` y
/// devuelve su id.
pub async fn guardar(
    db: &SqlitePool,
    clave: &HistoryKey,
    tipo: TipoFoto,
    contenido: &BTreeMap<String, String>,
) -> Result<i64, String> {
    let json = serde_json::to_vec(contenido).map_err(|e| format!("foto no serializable: {e}"))?;
    sqlx::query_scalar("INSERT INTO fotos (momento, tipo, contenido) VALUES (?, ?, ?) RETURNING id")
        .bind(crate::series::now())
        .bind(tipo.como_texto())
        .bind(clave.seal(&json))
        .fetch_one(db)
        .await
        .map_err(|e| format!("no se puede guardar la foto: {e}"))
}

/// La foto más reciente de `tipo` con su id; `None` si no hay o no se puede
/// leer o descifrar (el error queda registrado, sin el contenido).
pub async fn ultima(
    db: &SqlitePool,
    clave: &HistoryKey,
    tipo: TipoFoto,
) -> Option<(i64, BTreeMap<String, String>)> {
    let fila: Option<(i64, Vec<u8>)> = match sqlx::query_as(
        "SELECT id, contenido FROM fotos WHERE tipo = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(tipo.como_texto())
    .fetch_optional(db)
    .await
    {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("no se puede leer la foto {}: {e}", tipo.como_texto());
            return None;
        }
    };
    let (id, cifrado) = fila?;
    let contenido = clave
        .open(&cifrado)
        .and_then(|plano| serde_json::from_slice(&plano).ok());
    if contenido.is_none() {
        tracing::error!(
            "no se puede descifrar la foto {id}: ¿ha cambiado la clave maestra (RP_HISTORY_KEY)?"
        );
    }
    contenido.map(|c| (id, c))
}

/// ¿Hay alguna foto de `tipo`? Sin descifrarla (`false` si no se puede
/// mirar; el error queda registrado).
pub async fn hay(db: &SqlitePool, tipo: TipoFoto) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM fotos WHERE tipo = ?)")
        .bind(tipo.como_texto())
        .fetch_one(db)
        .await
        .inspect_err(|e| tracing::error!("no se puede mirar la foto {}: {e}", tipo.como_texto()))
        .unwrap_or(false)
}

/// Cuándo se tomó la foto `id` (segundos Unix).
pub async fn momento(db: &SqlitePool, id: i64) -> Option<i64> {
    sqlx::query_scalar("SELECT momento FROM fotos WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await
        .inspect_err(|e| tracing::error!("no se puede leer la fecha de la foto {id}: {e}"))
        .ok()
        .flatten()
}
