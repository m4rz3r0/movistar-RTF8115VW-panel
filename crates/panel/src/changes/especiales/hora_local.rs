//! Hora del router a mano (`cbSetLocalTime`) como la manda js/ntp.js: la
//! fecha y la hora del router (`AAAA-MM-DD HH:MM`) se convierten a
//! `SysTime`, segundos desde 1970 en UTC, con la zona del router
//! (`SysTzOffset` de `cbGetSummary`: minutos, UTC menos la hora local). Un
//! valor ya en segundos va tal cual.

use std::collections::BTreeMap;

pub(in crate::changes) const FUNCION: &str = "cbSetLocalTime";
/// Lectura con la zona del router.
pub(in crate::changes) const RESUMEN: &str = "cbGetSummary";
pub(in crate::changes) const ZONA: &str = "SysTzOffset";
const HORA: &str = "SysTime";

/// Convierte `SysTime` de `valores` a segundos con la zona `zona_min`.
pub(in crate::changes) fn preparar(
    valores: &mut BTreeMap<String, String>,
    zona_min: i64,
) -> Result<(), String> {
    let Some(v) = valores.get(HORA) else {
        return Ok(());
    };
    let v = v.trim();
    let segundos = if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) {
        v.parse::<i64>().map_err(|_| no_vale(v))?
    } else {
        segundos_utc(v).ok_or_else(|| no_vale(v))? + zona_min * 60
    };
    valores.insert(HORA.to_owned(), segundos.to_string());
    Ok(())
}

fn no_vale(v: &str) -> String {
    format!("{HORA}: «{v}» no es una fecha y hora (AAAA-MM-DD HH:MM) ni segundos")
}

/// `AAAA-MM-DD HH:MM` (o con `T`) como si fuera UTC, en segundos.
fn segundos_utc(v: &str) -> Option<i64> {
    let (fecha, hora) = v.split_once([' ', 'T'])?;
    let mut f = fecha.split('-').map(|x| x.parse::<i64>().ok());
    let (a, m, d) = (f.next()??, f.next()??, f.next()??);
    let mut h = hora.split(':').map(|x| x.parse::<i64>().ok());
    let (hh, mm) = (h.next()??, h.next()??);
    if f.next().is_some() || h.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&m) || d < 1 || d > dias_del_mes(a, m) {
        return None;
    }
    if !(0..24).contains(&hh) || !(0..60).contains(&mm) {
        return None;
    }
    Some(dias_desde_1970(a, m, d) * 86_400 + hh * 3_600 + mm * 60)
}

fn dias_del_mes(a: i64, m: i64) -> i64 {
    match m {
        2 if (a % 4 == 0 && a % 100 != 0) || a % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Días del 1970-01-01 a la fecha (calendario gregoriano; el algoritmo
/// `days_from_civil` de Howard Hinnant).
fn dias_desde_1970(a: i64, m: i64, d: i64) -> i64 {
    let a = if m <= 2 { a - 1 } else { a };
    let era = a.div_euclid(400);
    let ade = a - era * 400;
    let mp = (m + 9) % 12;
    let dda = (153 * mp + 2) / 5 + d - 1;
    let dde = ade * 365 + ade / 4 - ade / 100 + dda;
    era * 146_097 + dde - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fechas_conocidas() {
        assert_eq!(segundos_utc("1970-01-01 00:00"), Some(0));
        assert_eq!(segundos_utc("2000-02-29 12:30"), Some(951_827_400));
        assert_eq!(segundos_utc("2026-01-01T00:00"), Some(1_767_225_600));
        assert_eq!(segundos_utc("2100-02-29 00:00"), None);
        assert_eq!(segundos_utc("2026-13-01 00:00"), None);
        assert_eq!(segundos_utc("2026-01-01 10:00:00"), None);
    }
}
