//! Freno de fuerza bruta por IP y por usuario, y la IP del cliente tras el proxy.

use super::*;

/// IP del cliente: la del socket, salvo que el socket sea uno de los
/// proxies de confianza (`proxies`). Entonces se recorre
/// `X-Forwarded-For` de derecha a izquierda (del proxy más cercano hacia
/// el cliente) saltando las direcciones de confianza: la primera que no
/// lo es, es el cliente. Lo que queda a su izquierda lo escribió el
/// cliente y no cuenta. Si se llega a un valor que no es una IP, o todos
/// son de confianza, se usa la última dirección de confianza vista (la
/// más a la izquierda; el propio socket si la cabecera falta).
pub fn client_ip(peer: IpAddr, headers: &HeaderMap, proxies: &[RedIp]) -> IpAddr {
    let de_confianza = |ip: IpAddr| proxies.iter().any(|red| red.contiene(ip));
    let mut ultima = peer.to_canonical();
    if !de_confianza(ultima) {
        return ultima;
    }
    let valores: Vec<&str> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .collect();
    for valor in valores.into_iter().rev() {
        let Ok(ip) = valor.trim().parse::<IpAddr>() else {
            return ultima;
        };
        let ip = ip.to_canonical();
        if !de_confianza(ip) {
            return ip;
        }
        ultima = ip;
    }
    ultima
}

/// Espera tras `failures` fallos seguidos: `min(2^(fallos-1), 60)` s.
pub fn delay_secs(failures: i64) -> i64 {
    match failures {
        ..=0 => 0,
        1..=6 => (1 << (failures - 1)).min(MAX_DELAY_SECS),
        _ => MAX_DELAY_SECS,
    }
}

/// Milisegundos que faltan para poder intentar de nuevo (0 si ya se puede).
pub(super) async fn wait_ms(db: &SqlitePool, ip: &str, user: &str) -> Result<i64, sqlx::Error> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT failures, last_failure_ms FROM login_attempts
         WHERE (kind = 'ip' AND key = ?) OR (kind = 'user' AND key = ?)",
    )
    .bind(ip)
    .bind(user)
    .fetch_all(db)
    .await?;
    let now = now_ms();
    Ok(rows
        .into_iter()
        .map(|(failures, last)| last + delay_secs(failures) * 1000 - now)
        .max()
        .unwrap_or(0)
        .max(0))
}

pub(super) async fn record_failure(
    db: &SqlitePool,
    ip: &str,
    user: &str,
) -> Result<(), sqlx::Error> {
    let now = now_ms();
    for (kind, key) in [("ip", ip), ("user", user)] {
        sqlx::query(
            "INSERT INTO login_attempts (kind, key, failures, last_failure_ms)
             VALUES (?, ?, 1, ?)
             ON CONFLICT (kind, key) DO UPDATE
             SET failures = failures + 1, last_failure_ms = excluded.last_failure_ms",
        )
        .bind(kind)
        .bind(key)
        .bind(now)
        .execute(db)
        .await?;
    }
    Ok(())
}

pub(super) async fn clear_failures(
    db: &SqlitePool,
    ip: &str,
    user: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM login_attempts
         WHERE (kind = 'ip' AND key = ?) OR (kind = 'user' AND key = ?)",
    )
    .bind(ip)
    .bind(user)
    .execute(db)
    .await?;
    Ok(())
}
