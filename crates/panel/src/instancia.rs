//! Datos de la instancia del panel guardados en la base de datos: las
//! credenciales del router (la contraseña cifrada con la clave maestra) y los
//! parámetros de la instancia.

use crate::{config, crypto::HistoryKey, state::AppState};
use serde_json::Value;
use sqlx::SqlitePool;

/// Claves de los parámetros de la instancia (`ajustes_instancia`).
pub mod claves {
    pub const SOLO_LECTURA: &str = "solo_lectura";
    pub const IDIOMA: &str = "idioma";
    pub const ZONA_HORARIA: &str = "zona_horaria";
    pub const DNS_PREFERIDOS: &str = "dns_preferidos";
    pub const DISPOSITIVOS_CONFIANZA: &str = "dispositivos_confianza";
    pub const MANTENIMIENTO: &str = "mantenimiento";
    pub const SERVIDOR_PROPIO: &str = "servidor_propio";
    pub const FIRMWARE_CONFIRMADO: &str = "firmware_confirmado";
}

/// URL, usuario y contraseña del router. `Debug` no muestra la contraseña.
#[derive(Clone, PartialEq, Eq)]
pub struct CredencialesRouter {
    pub url: String,
    pub usuario: String,
    pub contrasena: String,
}

impl std::fmt::Debug for CredencialesRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredencialesRouter")
            .field("url", &self.url)
            .field("usuario", &self.usuario)
            .field("contrasena", &"***")
            .finish()
    }
}

impl CredencialesRouter {
    fn client_config(&self) -> hgu::ClientConfig {
        hgu::ClientConfig::new(&self.url, &self.usuario, &self.contrasena)
    }
}

/// Credenciales del router guardadas; `None` si aún no hay. Error si no se
/// pueden leer o descifrar (p. ej. con otra clave maestra); el mensaje nunca
/// lleva la contraseña.
pub async fn leer_credenciales(
    db: &SqlitePool,
    clave: &HistoryKey,
) -> Result<Option<CredencialesRouter>, String> {
    let fila: Option<(String, String, Vec<u8>)> =
        sqlx::query_as("SELECT url, usuario, contrasena FROM router_credenciales WHERE id = 1")
            .fetch_optional(db)
            .await
            .map_err(|e| format!("no se pueden leer las credenciales del router: {e}"))?;
    let Some((url, usuario, cifrada)) = fila else {
        return Ok(None);
    };
    let contrasena = clave
        .open(&cifrada)
        .and_then(|plano| String::from_utf8(plano).ok())
        .ok_or_else(|| {
            "no se puede descifrar la contraseña del router guardada: ¿ha cambiado la clave \
             maestra (RP_HISTORY_KEY)?"
                .to_owned()
        })?;
    Ok(Some(CredencialesRouter {
        url,
        usuario,
        contrasena,
    }))
}

/// Guarda (o sustituye) las credenciales del router, con la contraseña
/// cifrada con `clave`.
pub async fn guardar_credenciales(
    db: &SqlitePool,
    clave: &HistoryKey,
    c: &CredencialesRouter,
) -> Result<(), String> {
    let cifrada = clave.seal(c.contrasena.as_bytes());
    sqlx::query(
        "INSERT INTO router_credenciales (id, url, usuario, contrasena, actualizado)
         VALUES (1, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET url = excluded.url, usuario = excluded.usuario,
             contrasena = excluded.contrasena, actualizado = excluded.actualizado",
    )
    .bind(&c.url)
    .bind(&c.usuario)
    .bind(cifrada)
    .bind(crate::series::now())
    .execute(db)
    .await
    .map(|_| ())
    .map_err(|e| format!("no se pueden guardar las credenciales del router: {e}"))
}

/// Valor del parámetro de la instancia `clave` (ver [`claves`]); `None` si
/// no está guardado o no se puede leer (el error queda registrado).
pub async fn leer(db: &SqlitePool, clave: &str) -> Option<Value> {
    let valor: Option<String> =
        match sqlx::query_scalar("SELECT valor FROM ajustes_instancia WHERE clave = ?")
            .bind(clave)
            .fetch_optional(db)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("no se puede leer el ajuste {clave} de la instancia: {e}");
                return None;
            }
        };
    let valor = valor?;
    match serde_json::from_str(&valor) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::error!("el ajuste {clave} de la instancia no es JSON válido: {e}");
            None
        }
    }
}

/// Guarda (o sustituye) el parámetro de la instancia `clave`.
pub async fn escribir(db: &SqlitePool, clave: &str, valor: &Value) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO ajustes_instancia (clave, valor) VALUES (?, ?)
         ON CONFLICT (clave) DO UPDATE SET valor = excluded.valor",
    )
    .bind(clave)
    .bind(valor.to_string())
    .execute(db)
    .await
    .map(|_| ())
    .map_err(|e| format!("no se puede guardar el ajuste {clave} de la instancia: {e}"))
}

/// Configura el cliente del router al arrancar. Por orden: las credenciales
/// de la base de datos; si no hay, las de `RP_ROUTER_URL`, `RP_ROUTER_USER` y
/// `RP_ROUTER_SECRET` (las tres), que se importan a la base de datos una sola
/// vez; si tampoco, el router queda sin configurar.
///
/// Error si las credenciales guardadas no se pueden descifrar, o si están
/// las tres variables pero el fichero de la contraseña no se puede leer o lo
/// pueden leer otros (no se usa ni se importa). Registra de dónde salieron
/// las credenciales, nunca su valor.
pub async fn arrancar_router(state: &AppState) -> Result<(), String> {
    if let Some(c) = leer_credenciales(&state.db, &state.history_key).await? {
        tracing::info!("router: credenciales de la base de datos");
        state.hgu.reconfigurar(c.client_config()).await;
        return Ok(());
    }
    let cfg = &state.cfg;
    match (&cfg.router_url, &cfg.router_user, &cfg.router_secret_file) {
        (Some(url), Some(usuario), Some(fichero)) => {
            let c = CredencialesRouter {
                url: url.clone(),
                usuario: usuario.clone(),
                contrasena: config::read_router_secret(fichero)?,
            };
            guardar_credenciales(&state.db, &state.history_key, &c).await?;
            tracing::info!(
                "router: credenciales importadas del entorno (RP_ROUTER_URL, RP_ROUTER_USER, \
                 RP_ROUTER_SECRET) a la base de datos; a partir de ahora mandan las guardadas"
            );
            state.hgu.reconfigurar(c.client_config()).await;
        }
        (None, None, None) => tracing::info!("router: sin configurar"),
        _ => tracing::warn!(
            "router: sin configurar; para importar las credenciales del entorno hacen falta \
             RP_ROUTER_URL, RP_ROUTER_USER y RP_ROUTER_SECRET"
        ),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_oculta_la_contrasena() {
        let c = CredencialesRouter {
            url: "http://192.0.2.1".into(),
            usuario: "admin".into(),
            contrasena: "no-debe-salir".into(),
        };
        let texto = format!("{c:?}");
        assert!(texto.contains("admin"), "{texto}");
        assert!(!texto.contains("no-debe-salir"), "{texto}");
    }
}
