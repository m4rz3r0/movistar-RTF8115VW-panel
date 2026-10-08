//! Series temporales del estado del router en SQLite.
//!
//! Muestras crudas (cada 10 s) durante 24 h; después se agregan a 1 min hasta
//! los 7 d y a 15 min hasta los 30 d, y se purgan. Las marcas de tiempo son
//! segundos Unix del reloj del servidor.

use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

/// Muestras crudas: 24 h.
const RAW_KEEP: i64 = 24 * 3600;
/// Agregado a 1 min: 7 d.
const M1_KEEP: i64 = 7 * 24 * 3600;
/// Agregado a 15 min (y eventos de corte): 30 d.
const M15_KEEP: i64 = 30 * 24 * 3600;

/// Milisegundos Unix según el reloj del servidor (0 si el reloj marca una
/// fecha anterior a 1970: nunca entra en pánico).
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Segundos Unix según el reloj del servidor.
pub fn now() -> i64 {
    now_ms() / 1000
}

/// Tasa en bit/s entre dos lecturas `(ts_ms, bytes)` de un contador, con las
/// marcas de tiempo en milisegundos.
///
/// `None` si el contador ha bajado (reinicio del router, desbordamiento) o
/// si no ha pasado tiempo entre las lecturas.
pub fn rate(prev: (i64, u64), cur: (i64, u64)) -> Option<f64> {
    let dt_ms = cur.0 - prev.0;
    if cur.1 < prev.1 || dt_ms <= 0 {
        return None;
    }
    Some((cur.1 - prev.1) as f64 * 8.0 * 1000.0 / dt_ms as f64)
}

/// Una muestra del estado.
#[derive(Clone, Debug, PartialEq, sqlx::FromRow)]
pub struct Sample {
    pub ts: i64,
    /// Bajada de la WAN (`ip2`) en bit/s; `None` sin lectura anterior válida.
    pub rx_bps: Option<f64>,
    /// Subida de la WAN (`ip2`) en bit/s.
    pub tx_bps: Option<f64>,
    pub load1: f32,
    pub ram_used_pct: f32,
    /// Hosts LAN activos; `None` hasta la primera lectura de hosts.
    pub clients: Option<u32>,
    pub wan_up: bool,
}

/// Guarda una muestra cruda (si ya hay una en el mismo segundo, la sustituye).
pub async fn insert(pool: &SqlitePool, s: &Sample) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO samples_raw \
         (ts, rx_bps, tx_bps, load1, ram_used_pct, clients, wan_up) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(s.ts)
    .bind(s.rx_bps)
    .bind(s.tx_bps)
    .bind(s.load1)
    .bind(s.ram_used_pct)
    .bind(s.clients)
    .bind(s.wan_up)
    .execute(pool)
    .await?;
    Ok(())
}

/// Agrega en `to` (cubos de `bucket` s, medias de los valores no nulos;
/// `wan_up` falso si alguna muestra del cubo lo era) las filas de `from`
/// anteriores a `before`, y las borra de `from`. Solo toma cubos completos,
/// así que repetirlo no cambia nada.
async fn aggregate(
    tx: &mut sqlx::SqliteConnection,
    from: &str,
    to: &str,
    bucket: i64,
    before: i64,
) -> Result<(), sqlx::Error> {
    let cutoff = before.div_euclid(bucket) * bucket;
    sqlx::query(&format!(
        "INSERT OR REPLACE INTO {to} \
         (ts, rx_bps, tx_bps, load1, ram_used_pct, clients, wan_up) \
         SELECT (ts / ?1) * ?1, AVG(rx_bps), AVG(tx_bps), AVG(load1), AVG(ram_used_pct), \
                CAST(ROUND(AVG(clients)) AS INTEGER), MIN(wan_up) \
         FROM {from} WHERE ts < ?2 GROUP BY ts / ?1"
    ))
    .bind(bucket)
    .bind(cutoff)
    .execute(&mut *tx)
    .await?;
    sqlx::query(&format!("DELETE FROM {from} WHERE ts < ?"))
        .bind(cutoff)
        .execute(&mut *tx)
        .await?;
    Ok(())
}

/// Compacta las series: crudo de más de 24 h a `samples_1m`, 1 min de más de
/// 7 d a `samples_15m`, y borra lo de más de 30 d (también los cortes de la
/// WAN ya cerrados). Idempotente.
pub async fn compact(pool: &SqlitePool, now: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    aggregate(&mut tx, "samples_raw", "samples_1m", 60, now - RAW_KEEP).await?;
    aggregate(&mut tx, "samples_1m", "samples_15m", 900, now - M1_KEEP).await?;
    sqlx::query("DELETE FROM samples_15m WHERE ts < ?")
        .bind(now - M15_KEEP)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM wan_events WHERE hasta < ?")
        .bind(now - M15_KEEP)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

/// Intervalo de las gráficas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rango {
    /// Última hora, muestras crudas.
    Hora,
    /// Últimas 24 h, por minuto.
    Dia,
    /// Últimos 7 d, por 15 min.
    Semana,
}

impl Rango {
    /// Duración y tamaño de cubo, en segundos.
    fn span_bucket(self) -> (i64, i64) {
        match self {
            Rango::Hora => (3600, 1),
            Rango::Dia => (24 * 3600, 60),
            Rango::Semana => (7 * 24 * 3600, 900),
        }
    }
}

/// Muestras del `rango` hasta ahora, en orden.
pub async fn query(pool: &SqlitePool, rango: Rango) -> Result<Vec<Sample>, sqlx::Error> {
    query_at(pool, rango, now()).await
}

/// Muestras del `rango` que acaba en `now`. Toma las tres tablas (cada una
/// tiene su tramo de antigüedad) y las agrega al cubo del rango; para la
/// última hora solo hay muestras crudas.
pub async fn query_at(
    pool: &SqlitePool,
    rango: Rango,
    now: i64,
) -> Result<Vec<Sample>, sqlx::Error> {
    let (span, bucket) = rango.span_bucket();
    let tables = match rango {
        Rango::Hora => "SELECT * FROM samples_raw",
        Rango::Dia | Rango::Semana => {
            "SELECT * FROM samples_raw UNION ALL SELECT * FROM samples_1m \
             UNION ALL SELECT * FROM samples_15m"
        }
    };
    sqlx::query_as(&format!(
        "SELECT (ts / ?1) * ?1 AS ts, AVG(rx_bps) AS rx_bps, AVG(tx_bps) AS tx_bps, \
                AVG(load1) AS load1, AVG(ram_used_pct) AS ram_used_pct, \
                CAST(ROUND(AVG(clients)) AS INTEGER) AS clients, MIN(wan_up) AS wan_up \
         FROM ({tables}) WHERE ts >= ?2 AND ts <= ?3 GROUP BY ts / ?1 ORDER BY ts"
    ))
    .bind(bucket)
    .bind(now - span)
    .bind(now)
    .fetch_all(pool)
    .await
}

/// Corte de la WAN (`ip2`).
#[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct WanEvento {
    pub desde: i64,
    /// `None` si sigue caída.
    pub hasta: Option<i64>,
}

/// Registra el estado de la WAN: abre un corte si está caída y no hay uno
/// abierto; cierra el abierto si está arriba.
pub async fn wan_state(pool: &SqlitePool, up: bool, now: i64) -> Result<(), sqlx::Error> {
    let sql = if up {
        "UPDATE wan_events SET hasta = ?1 WHERE hasta IS NULL"
    } else {
        "INSERT INTO wan_events (desde) SELECT ?1 \
         WHERE NOT EXISTS (SELECT 1 FROM wan_events WHERE hasta IS NULL)"
    };
    sqlx::query(sql).bind(now).execute(pool).await?;
    Ok(())
}

/// El corte más reciente (abierto o cerrado), si hay alguno registrado.
pub async fn ultimo_corte(pool: &SqlitePool) -> Result<Option<WanEvento>, sqlx::Error> {
    sqlx::query_as("SELECT desde, hasta FROM wan_events ORDER BY desde DESC, id DESC LIMIT 1")
        .fetch_optional(pool)
        .await
}

/// Cortes que tocan el `rango` hasta ahora, en orden.
pub async fn cortes(pool: &SqlitePool, rango: Rango) -> Result<Vec<WanEvento>, sqlx::Error> {
    cortes_at(pool, rango, now()).await
}

/// Cortes que tocan el `rango` que acaba en `now`.
pub async fn cortes_at(
    pool: &SqlitePool,
    rango: Rango,
    now: i64,
) -> Result<Vec<WanEvento>, sqlx::Error> {
    let (span, _) = rango.span_bucket();
    sqlx::query_as(
        "SELECT desde, hasta FROM wan_events \
         WHERE desde <= ?2 AND (hasta IS NULL OR hasta >= ?1) ORDER BY desde",
    )
    .bind(now - span)
    .bind(now)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_calcula_bits_por_segundo() {
        // Marcas de tiempo en milisegundos.
        assert_eq!(rate((100_000, 5_000), (101_000, 6_000)), Some(8_000.0));
        assert_eq!(rate((100_000, 0), (110_000, 10_000)), Some(8_000.0));
        assert_eq!(rate((100_000, 7), (110_000, 7)), Some(0.0));
        // 10,4 s no se redondean a 10 s: 13 000 B ⇒ 10 000 bit/s.
        assert_eq!(rate((100_000, 0), (110_400, 13_000)), Some(10_000.0));
    }

    #[test]
    fn rate_descarta_reinicio_de_contador() {
        assert_eq!(rate((100_000, 6_000), (110_000, 5_000)), None);
        assert_eq!(rate((100_000, 5_000), (100_000, 6_000)), None);
        assert_eq!(rate((110_000, 5_000), (100_000, 6_000)), None);
    }

    async fn pool() -> (SqlitePool, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::open(&dir.path().join("t.db")).await.unwrap();
        (pool, dir)
    }

    fn muestra(ts: i64, rx: Option<f64>, clients: Option<u32>, wan_up: bool) -> Sample {
        Sample {
            ts,
            rx_bps: rx,
            tx_bps: rx.map(|v| v / 2.0),
            load1: 1.0,
            ram_used_pct: 50.0,
            clients,
            wan_up,
        }
    }

    async fn filas(pool: &SqlitePool, table: &str) -> Vec<Sample> {
        sqlx::query_as(&format!("SELECT * FROM {table} ORDER BY ts"))
            .fetch_all(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn compact_agrega_y_purga() {
        let (pool, _dir) = pool().await;
        const DAY: i64 = 86_400;
        // `now` alineado a 15 min más 7 s.
        let now = 1_000 * DAY + 7;
        let m = now - 25 * 3600 - 7; // múltiplo de 900 (y de 60), > 24 h

        // Crudo: un minuto completo de hace 25 h y una muestra reciente.
        for s in [
            muestra(m, Some(100.0), Some(2), true),
            muestra(m + 10, None, None, false),
            muestra(m + 20, Some(300.0), Some(5), true),
            muestra(now - 3600, Some(1.0), Some(1), true),
        ] {
            insert(&pool, &s).await.unwrap();
        }
        // 1 min: dos minutos de un cuarto de hora de hace 8 d, uno de hace 2 d.
        let q = now - 8 * DAY - 7;
        for (ts, rx) in [(q, 10.0), (q + 60, 30.0), (now - 2 * DAY - 7, 5.0)] {
            sqlx::query(
                "INSERT INTO samples_1m (ts, rx_bps, tx_bps, load1, ram_used_pct, clients, wan_up) \
                 VALUES (?, ?, NULL, 2.0, 40.0, 3, 1)",
            )
            .bind(ts)
            .bind(rx)
            .execute(&pool)
            .await
            .unwrap();
        }
        // 15 min: uno de hace 31 d (se purga) y otro de hace 10 d.
        for ts in [now - 31 * DAY - 7, now - 10 * DAY - 7] {
            sqlx::query(
                "INSERT INTO samples_15m (ts, rx_bps, tx_bps, load1, ram_used_pct, clients, wan_up) \
                 VALUES (?, NULL, NULL, 0.5, 30.0, NULL, 1)",
            )
            .bind(ts)
            .execute(&pool)
            .await
            .unwrap();
        }
        // Cortes: uno cerrado hace 31 d (se purga) y otro abierto.
        sqlx::query("INSERT INTO wan_events (desde, hasta) VALUES (?, ?), (?, NULL)")
            .bind(now - 32 * DAY)
            .bind(now - 31 * DAY)
            .bind(now - 3600)
            .execute(&pool)
            .await
            .unwrap();

        for _ in 0..2 {
            compact(&pool, now).await.unwrap();

            assert_eq!(
                filas(&pool, "samples_raw").await,
                vec![muestra(now - 3600, Some(1.0), Some(1), true)]
            );
            let m1 = filas(&pool, "samples_1m").await;
            assert_eq!(m1.len(), 2, "{m1:?}");
            assert_eq!(m1[0].ts, now - 2 * DAY - 7);
            assert_eq!(
                m1[1],
                Sample {
                    ts: m,
                    rx_bps: Some(200.0),
                    tx_bps: Some(100.0),
                    load1: 1.0,
                    ram_used_pct: 50.0,
                    clients: Some(4), // media de 2 y 5, redondeada
                    wan_up: false,
                }
            );
            let m15 = filas(&pool, "samples_15m").await;
            assert_eq!(m15.len(), 2, "{m15:?}");
            assert_eq!(m15[0].ts, now - 10 * DAY - 7);
            assert_eq!(
                m15[1],
                Sample {
                    ts: q,
                    rx_bps: Some(20.0),
                    tx_bps: None,
                    load1: 2.0,
                    ram_used_pct: 40.0,
                    clients: Some(3),
                    wan_up: true,
                }
            );
            let ev: Vec<WanEvento> = sqlx::query_as("SELECT desde, hasta FROM wan_events")
                .fetch_all(&pool)
                .await
                .unwrap();
            assert_eq!(
                ev,
                vec![WanEvento {
                    desde: now - 3600,
                    hasta: None
                }]
            );
        }
    }

    #[tokio::test]
    async fn query_elige_cubo_segun_rango() {
        let (pool, _dir) = pool().await;
        let now = 1_000 * 86_400;
        for (ts, rx) in [(now - 30, 10.0), (now - 20, 30.0), (now - 2 * 3600, 50.0)] {
            insert(&pool, &muestra(ts, Some(rx), Some(1), true))
                .await
                .unwrap();
        }
        sqlx::query(
            "INSERT INTO samples_1m (ts, rx_bps, tx_bps, load1, ram_used_pct, clients, wan_up) \
             VALUES (?, 7.0, NULL, 1.0, 50.0, NULL, 0)",
        )
        .bind(now - 3 * 86_400)
        .execute(&pool)
        .await
        .unwrap();

        let hora = query_at(&pool, Rango::Hora, now).await.unwrap();
        assert_eq!(
            hora.iter().map(|s| s.ts).collect::<Vec<_>>(),
            [now - 30, now - 20]
        );
        let dia = query_at(&pool, Rango::Dia, now).await.unwrap();
        assert_eq!(dia.len(), 2, "{dia:?}");
        assert_eq!(dia[1].ts, now - 60);
        assert_eq!(dia[1].rx_bps, Some(20.0));
        let semana = query_at(&pool, Rango::Semana, now).await.unwrap();
        assert_eq!(semana.len(), 3, "{semana:?}");
        assert!(!semana[0].wan_up);
        assert_eq!(semana[0].clients, None);
    }

    #[tokio::test]
    async fn cortes_abre_y_cierra() {
        let (pool, _dir) = pool().await;
        let now = 1_000 * 86_400;
        wan_state(&pool, true, now - 100).await.unwrap();
        wan_state(&pool, false, now - 50).await.unwrap();
        wan_state(&pool, false, now - 40).await.unwrap();
        assert_eq!(
            cortes_at(&pool, Rango::Hora, now).await.unwrap(),
            vec![WanEvento {
                desde: now - 50,
                hasta: None
            }]
        );
        wan_state(&pool, true, now - 10).await.unwrap();
        wan_state(&pool, true, now - 5).await.unwrap();
        assert_eq!(
            cortes_at(&pool, Rango::Hora, now).await.unwrap(),
            vec![WanEvento {
                desde: now - 50,
                hasta: Some(now - 10)
            }]
        );
        assert!(
            cortes_at(&pool, Rango::Hora, now + 7200)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn ultimo_corte_es_el_mas_reciente() {
        let (pool, _dir) = pool().await;
        let now = 1_000 * 86_400;
        assert_eq!(ultimo_corte(&pool).await.unwrap(), None);
        wan_state(&pool, false, now - 500).await.unwrap();
        wan_state(&pool, true, now - 400).await.unwrap();
        wan_state(&pool, false, now - 50).await.unwrap();
        assert_eq!(
            ultimo_corte(&pool).await.unwrap(),
            Some(WanEvento {
                desde: now - 50,
                hasta: None
            })
        );
        wan_state(&pool, true, now - 10).await.unwrap();
        assert_eq!(
            ultimo_corte(&pool).await.unwrap(),
            Some(WanEvento {
                desde: now - 50,
                hasta: Some(now - 10)
            })
        );
    }
}
