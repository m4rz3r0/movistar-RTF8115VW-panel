//! Datos de la instancia del panel guardados en la base de datos: las
//! credenciales del router (la contraseña cifrada con la clave maestra) y los
//! parámetros de la instancia.

use crate::{config, crypto::HistoryKey, state::AppState};
use hgu::deteccion::RedDetectada;
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
    /// Versión de firmware (`SWVersion`) que el usuario confirmó aunque no
    /// esté probada.
    pub const FIRMWARE_CONFIRMADO: &str = "firmware_confirmado";
    /// Última detección de la red del router (JSON de
    /// `hgu::deteccion::RedDetectada`).
    pub const RED_DETECTADA: &str = "red_detectada";
    /// Conexión a internet elegida a mano (nombre, p. ej. `ip2`): manda sobre
    /// la detectada.
    pub const WAN_MANUAL: &str = "wan_manual";
    /// Marca de la migración [`super::migrar_zona_utc_por_defecto`] (ya
    /// hecha).
    pub const MIGRACION_ZONA_UTC: &str = "migracion.zona_utc_por_defecto";
}

/// Versiones de firmware con las que se ha probado el panel (`SWVersion` de
/// `cbGetSummary`). Con otra, el panel queda en solo lectura hasta que se
/// confirme (ver [`fijar_modo_firmware`]).
pub const FIRMWARES_PROBADOS: &[&str] = &["ES_g14.17_RTF_TEF001_V9.17_V034"];

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
    /// Configuración del cliente del router con los tiempos por defecto,
    /// salvo la pausa entre peticiones, que sale de `cfg`
    /// ([`config::Config::router_min_gap`]).
    pub fn client_config(&self, cfg: &config::Config) -> hgu::ClientConfig {
        hgu::ClientConfig {
            min_gap: cfg.router_min_gap,
            ..hgu::ClientConfig::new(&self.url, &self.usuario, &self.contrasena)
        }
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

/// Guarda (o sustituye) varios parámetros de la instancia a la vez: todos o
/// ninguno.
pub async fn escribir_varios(db: &SqlitePool, valores: &[(&str, Value)]) -> Result<(), String> {
    let error = |e: sqlx::Error| format!("no se pueden guardar los ajustes de la instancia: {e}");
    let mut tx = db.begin().await.map_err(error)?;
    for (clave, valor) in valores {
        sqlx::query(
            "INSERT INTO ajustes_instancia (clave, valor) VALUES (?, ?)
             ON CONFLICT (clave) DO UPDATE SET valor = excluded.valor",
        )
        .bind(clave)
        .bind(valor.to_string())
        .execute(&mut *tx)
        .await
        .map_err(error)?;
    }
    tx.commit().await.map_err(error)
}

/// Migración de una sola vez (al arrancar): antes, «Ajustes del panel»
/// mostraba `UTC` como zona por defecto y guardarlo (p. ej. al cambiar el
/// idioma) lo dejaba en la base de datos aunque las gráficas iban en hora
/// peninsular. Si aún no se ha hecho ([`claves::MIGRACION_ZONA_UTC`]) y la
/// zona guardada es exactamente `UTC`, se quita (sin guardar es la
/// peninsular). La marca se escribe en la misma transacción, se haya quitado
/// algo o no: un `UTC` elegido después se respeta. Devuelve si quitó la zona.
pub async fn migrar_zona_utc_por_defecto(db: &SqlitePool) -> Result<bool, String> {
    let error = |e: sqlx::Error| format!("no se puede migrar la zona horaria: {e}");
    let mut tx = db.begin().await.map_err(error)?;
    let hecha: Option<String> =
        sqlx::query_scalar("SELECT valor FROM ajustes_instancia WHERE clave = ?")
            .bind(claves::MIGRACION_ZONA_UTC)
            .fetch_optional(&mut *tx)
            .await
            .map_err(error)?;
    if hecha.is_some() {
        return Ok(false);
    }
    let quitada = sqlx::query("DELETE FROM ajustes_instancia WHERE clave = ? AND valor = ?")
        .bind(claves::ZONA_HORARIA)
        .bind(Value::String("UTC".into()).to_string())
        .execute(&mut *tx)
        .await
        .map_err(error)?
        .rows_affected()
        > 0;
    sqlx::query("INSERT INTO ajustes_instancia (clave, valor) VALUES (?, ?)")
        .bind(claves::MIGRACION_ZONA_UTC)
        .bind(Value::Bool(true).to_string())
        .execute(&mut *tx)
        .await
        .map_err(error)?;
    tx.commit().await.map_err(error)?;
    if quitada {
        tracing::info!("zona horaria UTC de la versión anterior quitada: hora peninsular");
    }
    Ok(quitada)
}

/// ¿Se ha probado el panel con esta versión de firmware?
pub fn firmware_probado(firmware: &str) -> bool {
    FIRMWARES_PROBADOS.contains(&firmware)
}

/// Fija el modo de escritura según el firmware del router: solo lectura si
/// no es uno de [`FIRMWARES_PROBADOS`] ni el confirmado
/// ([`claves::FIRMWARE_CONFIRMADO`] guarda la versión que se confirmó: otra
/// sin probar vuelve a solo lectura). Devuelve si el firmware está probado.
/// La aplican el asistente al guardar el router y el sondeo cada vez que lee
/// un firmware distinto.
pub async fn fijar_modo_firmware(db: &SqlitePool, firmware: &str) -> Result<bool, String> {
    let probado = firmware_probado(firmware);
    let confirmado = !firmware.is_empty()
        && leer(db, claves::FIRMWARE_CONFIRMADO)
            .await
            .as_ref()
            .and_then(Value::as_str)
            == Some(firmware);
    escribir(
        db,
        claves::SOLO_LECTURA,
        &Value::Bool(!probado && !confirmado),
    )
    .await?;
    Ok(probado)
}

/// ¿Está el panel en solo lectura (firmware no probado sin confirmar)?
pub async fn solo_lectura(db: &SqlitePool) -> bool {
    leer(db, claves::SOLO_LECTURA).await == Some(Value::Bool(true))
}

/// Detecta la red del router ([`hgu::deteccion::detectar`]), la anota en
/// [`AppState`] (la conexión elegida a mano sigue mandando) y la guarda en
/// [`claves::RED_DETECTADA`]. Registra qué cambió respecto a la anterior,
/// nunca direcciones. Error (sin tocar lo anterior) si el router no responde
/// o si, al terminar, hay un cambio aplicándose o vigilándose
/// (`ChangeManager::ocupado`): la detección se descarta para que su red de
/// seguridad no cambie de conexión a mitad (el sondeo vuelve a detectar
/// después). Pasa con las detecciones que siguen en segundo plano (la del
/// arranque, la de guardar las credenciales).
pub async fn detectar_y_guardar(state: &AppState) -> Result<RedDetectada, String> {
    let red = hgu::deteccion::detectar(&state.hgu, crate::series::now())
        .await
        .map_err(|e| format!("no se puede detectar la red del router: {e}"))?;
    if state.cambios().is_some_and(|m| m.ocupado()) {
        return Err(
            "red del router: se descarta la detección, hay un cambio en curso (se volverá a \
             detectar después)"
                .to_owned(),
        );
    }
    let anterior = state.red();
    for cambio in cambios_de_red(anterior.as_ref(), &red) {
        tracing::info!("red del router: {cambio}");
    }
    state.red_actual().fijar_detectada(red.clone());
    let valor = serde_json::to_value(&red)
        .map_err(|e| format!("no se puede guardar la red detectada: {e}"))?;
    escribir(&state.db, claves::RED_DETECTADA, &valor).await?;
    Ok(red)
}

/// Lo más que se espera, de serie, a [`detectar_y_guardar`] al arrancar sin
/// ninguna detección guardada (p. ej. al actualizar desde una versión sin
/// ella) o al guardar las credenciales; si tarda más (el router no responde),
/// sigue en segundo plano y, entretanto, vale la detección guardada o, sin
/// ella, cualquier conexión de datos. La espera de verdad es la de la
/// configuración ([`config::Config::espera_deteccion`]).
pub const ESPERA_DETECCION: std::time::Duration = std::time::Duration::from_secs(5);

/// [`detectar_y_guardar`] en segundo plano, esperando como mucho `espera`.
/// Un fallo solo se registra: el sondeo lo vuelve a intentar.
pub async fn detectar_sin_esperar_mucho(state: &AppState, espera: std::time::Duration) {
    let s = state.clone();
    let tarea = tokio::spawn(async move {
        if let Err(e) = detectar_y_guardar(&s).await {
            tracing::warn!("{e}");
        }
    });
    if !espera.is_zero() && tokio::time::timeout(espera, tarea).await.is_err() {
        tracing::info!("red del router: la detección sigue en segundo plano");
    }
}

/// Al arrancar: si ya hay una detección guardada, se renueva sin retrasar el
/// arranque; si no, se espera hasta [`config::Config::espera_deteccion`].
async fn detectar_al_arrancar(state: &AppState) {
    let espera = if state.red().is_some() {
        std::time::Duration::ZERO
    } else {
        state.cfg.espera_deteccion
    };
    detectar_sin_esperar_mucho(state, espera).await;
}

/// Qué cambió entre dos detecciones, para el registro: nombres de
/// conexión, tipos y radios, nunca direcciones (de la LAN solo dice que
/// cambió).
fn cambios_de_red(antes: Option<&RedDetectada>, ahora: &RedDetectada) -> Vec<String> {
    let texto = |o: Option<String>| o.unwrap_or_else(|| "sin detectar".to_owned());
    let tipo = |r: Option<&RedDetectada>| texto(r.and_then(|r| r.tipo).map(|t| format!("{t:?}")));
    let si_no = |b: bool| if b { "sí" } else { "no" };
    let mut v = Vec::new();
    let wan = |r: Option<&RedDetectada>| r.and_then(|r| r.wan.clone());
    if antes.is_none() || wan(antes) != ahora.wan {
        v.push(format!(
            "conexión a internet: {} → {}",
            texto(wan(antes)),
            texto(ahora.wan.clone())
        ));
    }
    if antes.and_then(|r| r.tipo) != ahora.tipo {
        v.push(format!(
            "tipo de conexión: {} → {}",
            tipo(antes),
            tipo(Some(ahora))
        ));
    }
    let lan = |r: Option<&RedDetectada>| r.and_then(|red| red.lan.clone());
    if lan(antes) != lan(Some(ahora)) {
        v.push(match lan(Some(ahora)) {
            Some(_) => "subred LAN: detectada (distinta de la anterior)".to_owned(),
            None => "subred LAN: sin detectar".to_owned(),
        });
    }
    if antes.map(|r| r.voip) != Some(ahora.voip) {
        v.push(format!("telefonía: {}", si_no(ahora.voip)));
    }
    if antes.map(|r| r.iptv) != Some(ahora.iptv) {
        v.push(format!("televisión: {}", si_no(ahora.iptv)));
    }
    if antes.map(|r| &r.radios) != Some(&ahora.radios) {
        v.push(format!("radios WiFi: {}", ahora.radios.join(", ")));
    }
    v
}

/// Configura el cliente del router al arrancar. Por orden: las credenciales
/// de la base de datos; si no hay, las de `RP_ROUTER_URL`, `RP_ROUTER_USER` y
/// `RP_ROUTER_SECRET` (las tres), que se importan a la base de datos una sola
/// vez; si tampoco, el router queda sin configurar.
///
/// Error si solo están una o dos de las tres variables (una errata no debe
/// dejar el router sin configurar sin que se note), si las credenciales
/// guardadas no se pueden descifrar, o si están las tres variables pero el
/// fichero de la contraseña no se puede leer o lo pueden leer otros (no se
/// usa ni se importa). Registra de dónde salieron las credenciales, nunca su
/// valor. Con el router configurado, detecta su red (ver
/// `detectar_al_arrancar`).
pub async fn arrancar_router(state: &AppState) -> Result<(), String> {
    let cfg = &state.cfg;
    let presentes = [
        ("RP_ROUTER_URL", cfg.router_url.is_some()),
        ("RP_ROUTER_USER", cfg.router_user.is_some()),
        ("RP_ROUTER_SECRET", cfg.router_secret_file.is_some()),
    ];
    if presentes.iter().any(|(_, p)| *p) && !presentes.iter().all(|(_, p)| *p) {
        let faltan: Vec<&str> = presentes
            .iter()
            .filter(|(_, p)| !p)
            .map(|(v, _)| *v)
            .collect();
        return Err(format!(
            "faltan {}: RP_ROUTER_URL, RP_ROUTER_USER y RP_ROUTER_SECRET van las tres juntas \
             (para importar las credenciales del router) o ninguna",
            faltan.join(" y ")
        ));
    }
    if let Some(c) = leer_credenciales(&state.db, &state.history_key).await? {
        tracing::info!("router: credenciales de la base de datos");
        state.hgu.reconfigurar(c.client_config(cfg)).await;
        detectar_al_arrancar(state).await;
        return Ok(());
    }
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
            state.hgu.reconfigurar(c.client_config(cfg)).await;
            detectar_al_arrancar(state).await;
        }
        _ => tracing::info!("router: sin configurar"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red(wan: Option<&str>, lan: Option<&str>) -> RedDetectada {
        RedDetectada {
            wan: wan.map(str::to_owned),
            tipo: Some(hgu::models::TipoConexion::Pppoe),
            lan: lan.map(str::to_owned),
            router_lan: None,
            voip: true,
            iptv: true,
            radios: vec!["0".into(), "1".into()],
            momento: 0,
        }
    }

    #[test]
    fn registro_de_la_red_sin_direcciones() {
        let a = red(Some("ip2"), Some("192.0.2.0/24"));
        assert!(cambios_de_red(Some(&a), &a).is_empty());
        let b = RedDetectada {
            voip: false,
            ..red(Some("ip5"), Some("198.51.100.0/24"))
        };
        let c = cambios_de_red(Some(&a), &b);
        assert_eq!(
            c,
            [
                "conexión a internet: ip2 → ip5",
                "subred LAN: detectada (distinta de la anterior)",
                "telefonía: no",
            ]
        );
        let primera = cambios_de_red(None, &a);
        assert_eq!(primera[0], "conexión a internet: sin detectar → ip2");
        assert!(
            !primera.join(" ").contains("192.0.2"),
            "nunca direcciones: {primera:?}"
        );
    }

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
