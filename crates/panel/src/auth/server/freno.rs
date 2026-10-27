//! Freno de fuerza bruta por equipo (IP, o prefijo /64 en IPv6) y la IP del
//! cliente tras el proxy.
//!
//! No hay freno por usuario: con él, cualquiera de la red podía dejar a un
//! usuario sin entrar fallando con su nombre una vez por minuto. Cada equipo
//! tiene su contador para todos los usuarios, así que adivinar contraseñas
//! sigue frenado y el usuario legítimo entra desde el suyo.

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

/// Clave del freno para una IP: la propia dirección en IPv4 y el prefijo
/// /64 en IPv6, porque un equipo cambia de dirección dentro de su red
/// (SLAAC, direcciones temporales) sin pedir permiso a nadie.
pub fn clave_ip(ip: IpAddr) -> String {
    match ip.to_canonical() {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => {
            let mut s = v6.segments();
            s[4..].fill(0);
            format!("{}/64", std::net::Ipv6Addr::from(s))
        }
    }
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
pub(super) async fn wait_ms(db: &SqlitePool, ip: &str) -> Result<i64, sqlx::Error> {
    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT failures, last_failure_ms FROM login_attempts WHERE kind = 'ip' AND key = ?",
    )
    .bind(ip)
    .fetch_optional(db)
    .await?;
    let now = now_ms();
    Ok(row
        .map(|(failures, last)| last + delay_secs(failures) * 1000 - now)
        .unwrap_or(0)
        .max(0))
}

pub(super) async fn record_failure(db: &SqlitePool, ip: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO login_attempts (kind, key, failures, last_failure_ms)
         VALUES ('ip', ?, 1, ?)
         ON CONFLICT (kind, key) DO UPDATE
         SET failures = failures + 1, last_failure_ms = excluded.last_failure_ms",
    )
    .bind(ip)
    .bind(now_ms())
    .execute(db)
    .await?;
    Ok(())
}

pub(super) async fn clear_failures(db: &SqlitePool, ip: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM login_attempts WHERE kind = 'ip' AND key = ?")
        .bind(ip)
        .execute(db)
        .await?;
    Ok(())
}
