//! Fechas en hora peninsular (calculada aquí para que servidor y navegador
//! pinten lo mismo al hidratar), ticks de tiempo y duraciones.

// ---------------------------------------------------------------------------
// Fechas en hora peninsular (Europe/Madrid)
// ---------------------------------------------------------------------------

/// Fecha civil (año, mes, día) de un número de días desde 1970-01-01
/// (algoritmo de H. Hinnant).
fn civil_de_dias(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// Días desde 1970-01-01 de una fecha civil.
fn dias_de_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Día (desde 1970-01-01) del último domingo del mes.
fn ultimo_domingo(y: i64, m: u32) -> i64 {
    let (sy, sm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let ultimo = dias_de_civil(sy, sm, 1) - 1;
    // 1970-01-01 fue jueves: (días + 4) mód 7 = 0 es domingo.
    ultimo - (ultimo + 4).rem_euclid(7)
}

/// Desfase de la hora peninsular respecto a UTC en el instante `ts`: +2 h
/// del último domingo de marzo al último de octubre (a la 01:00 UTC), +1 h el
/// resto.
pub fn desfase_madrid(ts: i64) -> i64 {
    let (y, _, _) = civil_de_dias(ts.div_euclid(86_400));
    let verano = ultimo_domingo(y, 3) * 86_400 + 3600;
    let invierno = ultimo_domingo(y, 10) * 86_400 + 3600;
    if (verano..invierno).contains(&ts) {
        7200
    } else {
        3600
    }
}

/// Fecha y hora peninsulares de un instante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoraLocal {
    pub anio: i64,
    pub mes: u32,
    pub dia: u32,
    pub hora: u32,
    pub minuto: u32,
    pub segundo: u32,
}

/// Hora peninsular del instante `ts` (segundos Unix).
pub fn hora_local(ts: i64) -> HoraLocal {
    let local = ts + desfase_madrid(ts);
    let (anio, mes, dia) = civil_de_dias(local.div_euclid(86_400));
    let s = local.rem_euclid(86_400) as u32;
    HoraLocal {
        anio,
        mes,
        dia,
        hora: s / 3600,
        minuto: s / 60 % 60,
        segundo: s % 60,
    }
}

/// `"14:05"` o, con segundos, `"14:05:09"`.
pub fn formato_hora(ts: i64, con_segundos: bool) -> String {
    let h = hora_local(ts);
    if con_segundos {
        format!("{:02}:{:02}:{:02}", h.hora, h.minuto, h.segundo)
    } else {
        format!("{:02}:{:02}", h.hora, h.minuto)
    }
}

/// `"08/10 14:05"` (con segundos si se piden).
pub fn formato_fecha_hora(ts: i64, con_segundos: bool) -> String {
    let h = hora_local(ts);
    format!(
        "{:02}/{:02} {}",
        h.dia,
        h.mes,
        formato_hora(ts, con_segundos)
    )
}

/// Duración legible: `"45 s"`, `"7 min"`, `"1 h 05 min"`, `"2 d 3 h"`.
pub fn formato_duracion(segundos: i64) -> String {
    let s = segundos.max(0);
    match s {
        0..60 => format!("{s} s"),
        60..3600 => format!("{} min", s / 60),
        3600..86_400 => format!("{} h {:02} min", s / 3600, s / 60 % 60),
        _ => format!("{} d {} h", s / 86_400, s / 3600 % 24),
    }
}

/// Pasos posibles entre ticks de tiempo, en segundos.
const PASOS_TIEMPO: [i64; 14] = [
    60, 120, 300, 600, 900, 1800, 3600, 7200, 10_800, 21_600, 43_200, 86_400, 172_800, 604_800,
];

/// Ticks de tiempo en `[desde, hasta]`, como mucho `max_ticks`, alineados a
/// horas redondas peninsulares, con su texto (`"14:00"`, o `"08/10"` si el
/// paso es de días o el tick cae a medianoche).
pub fn ticks_tiempo(desde: i64, hasta: i64, max_ticks: usize) -> Vec<(i64, String)> {
    if hasta <= desde || max_ticks == 0 {
        return Vec::new();
    }
    let span = hasta - desde;
    let paso = PASOS_TIEMPO
        .iter()
        .copied()
        .find(|p| (span / p + 1) as usize <= max_ticks)
        .unwrap_or(PASOS_TIEMPO[PASOS_TIEMPO.len() - 1]);
    let off = desfase_madrid(desde);
    let mut local = (desde + off).div_euclid(paso) * paso;
    if local < desde + off {
        local += paso;
    }
    let mut ticks = Vec::new();
    loop {
        let t = local - desfase_madrid(local - off);
        if t > hasta {
            break;
        }
        if t >= desde {
            let h = hora_local(t);
            let texto = if paso >= 86_400 || (h.hora == 0 && h.minuto == 0) {
                format!("{:02}/{:02}", h.dia, h.mes)
            } else {
                format!("{:02}:{:02}", h.hora, h.minuto)
            };
            ticks.push((t, texto));
        }
        local += paso;
    }
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-08 00:00:00 UTC.
    const D2026_10_08: i64 = 1_791_417_600;

    #[test]
    fn hora_peninsular_verano_e_invierno() {
        // 2026-07-01 12:00 UTC → 14:00 (CEST).
        assert_eq!(formato_hora(1_782_907_200, false), "14:00");
        // 2026-12-01 12:00 UTC → 13:00 (CET).
        assert_eq!(formato_hora(1_796_126_400, false), "13:00");
        // Cambio de octubre de 2026: domingo 25 a la 01:00 UTC.
        let octubre = dias_de_civil(2026, 10, 25) * 86_400 + 3600;
        assert_eq!(desfase_madrid(octubre - 1), 7200);
        assert_eq!(desfase_madrid(octubre), 3600);
        // Cambio de marzo de 2026: domingo 29.
        let marzo = dias_de_civil(2026, 3, 29) * 86_400 + 3600;
        assert_eq!(desfase_madrid(marzo - 1), 3600);
        assert_eq!(desfase_madrid(marzo), 7200);
        assert_eq!(
            formato_fecha_hora(D2026_10_08 + 12 * 3600 + 5, true),
            "08/10 14:00:05"
        );
    }

    #[test]
    fn fechas_civiles_ida_y_vuelta() {
        for dias in [-1, 0, 59, 365, 11_016, 20_734, 40_000] {
            let (y, m, d) = civil_de_dias(dias);
            assert_eq!(dias_de_civil(y, m, d), dias);
        }
        assert_eq!(civil_de_dias(D2026_10_08 / 86_400), (2026, 10, 8));
    }

    #[test]
    fn ticks_de_tiempo_alineados_a_hora_local() {
        // 13:07 → 14:07 peninsulares: cada 15 min en punto.
        let desde = D2026_10_08 + 11 * 3600 + 7 * 60;
        let ticks = ticks_tiempo(desde, desde + 3600, 5);
        let textos: Vec<&str> = ticks.iter().map(|t| t.1.as_str()).collect();
        assert_eq!(textos, ["13:15", "13:30", "13:45", "14:00"]);
        // Una semana: días.
        let semana = ticks_tiempo(D2026_10_08 - 7 * 86_400, D2026_10_08, 8);
        assert!(semana.iter().all(|t| t.1.contains('/')), "{semana:?}");
        assert!(ticks_tiempo(10, 10, 5).is_empty());
    }

    #[test]
    fn duraciones_legibles() {
        assert_eq!(formato_duracion(45), "45 s");
        assert_eq!(formato_duracion(7 * 60 + 3), "7 min");
        assert_eq!(formato_duracion(3600 + 5 * 60), "1 h 05 min");
        assert_eq!(formato_duracion(2 * 86_400 + 3 * 3600), "2 d 3 h");
    }
}
