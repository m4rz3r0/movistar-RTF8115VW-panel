//! Base de datos SQLite del panel.

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::{path::Path, time::Duration};

/// Abre (o crea) la base de datos en `path` y aplica las migraciones
/// pendientes de `migrations/`.
pub async fn open(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;
    // La migración 0012 (columnas `ajuste` y `cambio_id` de `reveals`) se
    // quedó sin uso cuando ver un secreto dejó de anotarse;
    // no se quita porque las bases que ya la aplicaron no arrancarían sin
    // ella (y no se toca su texto: `sqlx` comprueba su suma).
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
