//! Fechas en la zona horaria de la instancia (calculadas aquí para que
//! servidor y navegador pinten lo mismo al hidratar), ticks de tiempo y
//! duraciones. El día y el mes se escriben según el idioma («08/10» o
//! «8 Oct»).
//!
//! La zona ([`Zona`]) es la de la instancia: Península, Canarias o un desfase
//! fijo respecto a UTC. El servidor la pone en `<html data-zona>` (como el
//! idioma en `lang`) y el navegador la lee de ahí al hidratar.

use crate::t;
use leptos::prelude::{provide_context, use_context};

// ---------------------------------------------------------------------------
// Zona horaria
// ---------------------------------------------------------------------------

/// Zona horaria de las fechas del panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Zona {
    /// Hora peninsular española (`Europe/Madrid`): UTC+1, +2 en verano.
    #[default]
    Peninsular,
    /// Hora de Canarias (`Atlantic/Canary`): una hora menos que la
    /// Península, con el mismo horario de verano.
    Canarias,
    /// Desfase fijo respecto a UTC, en segundos (sin horario de verano).
    Fija(i32),
}

/// Lo más lejos de UTC que puede estar un desfase fijo: 14 h.
const MAX_DESFASE: i32 = 14 * 3600;

impl Zona {
    /// `Europe/Madrid`, `Atlantic/Canary`, `UTC` o `UTC±HH:MM` (como mucho
    /// 14:00 de diferencia).
    pub fn desde_texto(s: &str) -> Option<Zona> {
        match s {
            "Europe/Madrid" => return Some(Zona::Peninsular),
            "Atlantic/Canary" => return Some(Zona::Canarias),
            "UTC" => return Some(Zona::Fija(0)),
            _ => {}
        }
        let (signo, resto) = if let Some(r) = s.strip_prefix("UTC+") {
            (1, r)
        } else {
            (-1, s.strip_prefix("UTC-")?)
        };
        let (h, m) = resto.split_once(':')?;
        if h.len() != 2 || m.len() != 2 || !h.bytes().chain(m.bytes()).all(|b| b.is_ascii_digit()) {
            return None;
        }
        let (h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
        let segundos = h * 3600 + m * 60;
        (m < 60 && segundos <= MAX_DESFASE).then_some(Zona::Fija(signo * segundos))
    }

    /// El texto con el que se guarda: el de [`Zona::desde_texto`] (un
    /// desfase cero es `UTC`).
    pub fn codigo(&self) -> String {
        match *self {
            Zona::Peninsular => "Europe/Madrid".to_owned(),
            Zona::Canarias => "Atlantic/Canary".to_owned(),
            Zona::Fija(0) => "UTC".to_owned(),
            Zona::Fija(s) => {
                let signo = if s < 0 { '-' } else { '+' };
                let m = s.unsigned_abs() / 60;
                format!("UTC{signo}{:02}:{:02}", m / 60, m % 60)
            }
        }
    }

    /// Desfase respecto a UTC en el instante `ts`, en segundos.
    pub fn desfase(&self, ts: i64) -> i64 {
        let verano = i64::from(verano_europeo(ts)) * 3600;
        match *self {
            Zona::Peninsular => 3600 + verano,
            Zona::Canarias => verano,
            Zona::Fija(s) => i64::from(s),
        }
    }

    /// Fecha y hora en esta zona del instante `ts` (segundos Unix).
    pub fn hora_local(&self, ts: i64) -> HoraLocal {
        let local = ts + self.desfase(ts);
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
}

/// Zona de la página en el contexto.
#[derive(Clone, Copy)]
struct ZonaContexto(Zona);

/// Zona de la página en el navegador, para el código que corre sin contexto
/// (manejadores de eventos, tareas).
#[cfg(feature = "hydrate")]
static ZONA_PAGINA: std::sync::OnceLock<Zona> = std::sync::OnceLock::new();

/// Pone `zona` en el contexto de la aplicación.
pub fn provide_zona(zona: Zona) {
    #[cfg(feature = "hydrate")]
    let _ = ZONA_PAGINA.set(zona);
    provide_context(ZonaContexto(zona));
}

/// Zona de las fechas: la del contexto; sin él, la de la instancia en el
/// servidor y la de la página en el navegador; si no, la peninsular.
pub fn zona_actual() -> Zona {
    if let Some(ZonaContexto(z)) = use_context() {
        return z;
    }
    #[cfg(feature = "ssr")]
    if let Some(s) = use_context::<crate::state::AppState>() {
        return s.zona();
    }
    #[cfg(feature = "hydrate")]
    if let Some(z) = ZONA_PAGINA.get() {
        return *z;
    }
    Zona::default()
}

/// Zona con la que arranca la aplicación: la de la instancia en el servidor;
/// la de `<html data-zona>` (que puso el servidor) al hidratar.
pub fn zona_inicial() -> Zona {
    if let Some(ZonaContexto(z)) = use_context() {
        return z;
    }
    #[cfg(feature = "ssr")]
    if let Some(s) = use_context::<crate::state::AppState>() {
        return s.zona();
    }
    #[cfg(feature = "hydrate")]
    if let Some(z) = leptos::prelude::document()
        .document_element()
        .and_then(|e| e.get_attribute("data-zona"))
        .and_then(|z| Zona::desde_texto(&z))
    {
        return z;
    }
    Zona::default()
}

// ---------------------------------------------------------------------------
// Fechas civiles
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

/// Horario de verano europeo en el instante `ts`: del último domingo de
/// marzo al último de octubre, a la 01:00 UTC.
fn verano_europeo(ts: i64) -> bool {
    let (y, _, _) = civil_de_dias(ts.div_euclid(86_400));
    let verano = ultimo_domingo(y, 3) * 86_400 + 3600;
    let invierno = ultimo_domingo(y, 10) * 86_400 + 3600;
    (verano..invierno).contains(&ts)
}

/// Fecha y hora de un instante en una zona.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoraLocal {
    pub anio: i64,
    pub mes: u32,
    pub dia: u32,
    pub hora: u32,
    pub minuto: u32,
    pub segundo: u32,
}

/// Fecha y hora del instante `ts` (segundos Unix) en la zona actual.
pub fn hora_local(ts: i64) -> HoraLocal {
    zona_actual().hora_local(ts)
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

/// Día y mes: `"08/10"` en español, `"8 Oct"` en inglés.
pub fn formato_dia_mes(h: &HoraLocal) -> String {
    t!(
        "comun-fecha-dia-mes",
        dia = format!("{:02}", h.dia),
        mes = format!("{:02}", h.mes),
        n_dia = h.dia,
        n_mes = h.mes
    )
}

/// Día, mes y año: `"09/10/2026"` en español, `"9 Oct 2026"` en inglés.
pub fn formato_fecha(h: &HoraLocal) -> String {
    t!(
        "comun-fecha-dia-mes-anio",
        dia = format!("{:02}", h.dia),
        mes = format!("{:02}", h.mes),
        anio = h.anio.to_string(),
        n_dia = h.dia,
        n_mes = h.mes
    )
}

/// `"08/10 14:05"` (con segundos si se piden; `"8 Oct 14:05"` en inglés).
pub fn formato_fecha_hora(ts: i64, con_segundos: bool) -> String {
    format!(
        "{} {}",
        formato_dia_mes(&hora_local(ts)),
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
/// horas redondas de la zona actual, con su texto (`"14:00"`, o `"08/10"` si
/// el paso es de días o el tick cae a medianoche).
pub fn ticks_tiempo(desde: i64, hasta: i64, max_ticks: usize) -> Vec<(i64, String)> {
    ticks_tiempo_en(zona_actual(), desde, hasta, max_ticks)
}

/// [`ticks_tiempo`] en la zona `zona`.
pub fn ticks_tiempo_en(zona: Zona, desde: i64, hasta: i64, max_ticks: usize) -> Vec<(i64, String)> {
    if hasta <= desde || max_ticks == 0 {
        return Vec::new();
    }
    let span = hasta - desde;
    let paso = PASOS_TIEMPO
        .iter()
        .copied()
        .find(|p| (span / p + 1) as usize <= max_ticks)
        .unwrap_or(PASOS_TIEMPO[PASOS_TIEMPO.len() - 1]);
    let off = zona.desfase(desde);
    let mut local = (desde + off).div_euclid(paso) * paso;
    if local < desde + off {
        local += paso;
    }
    let mut ticks = Vec::new();
    loop {
        let t = local - zona.desfase(local - off);
        if t > hasta {
            break;
        }
        // La noche del cambio de marzo, las 02:00 locales no existen y caen en
        // el mismo instante que las 01:00: un solo tick.
        if t >= desde && ticks.last().is_none_or(|u: &(i64, String)| u.0 != t) {
            let h = zona.hora_local(t);
            let texto = if paso >= 86_400 || (h.hora == 0 && h.minuto == 0) {
                formato_dia_mes(&h)
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
        assert_eq!(Zona::Peninsular.desfase(octubre - 1), 7200);
        assert_eq!(Zona::Peninsular.desfase(octubre), 3600);
        // Cambio de marzo de 2026: domingo 29.
        let marzo = dias_de_civil(2026, 3, 29) * 86_400 + 3600;
        assert_eq!(Zona::Peninsular.desfase(marzo - 1), 3600);
        assert_eq!(Zona::Peninsular.desfase(marzo), 7200);
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
    fn ticks_de_tiempo_en_el_cambio_de_marzo() {
        // 29/03/2026 00:00 CET → 05:00 CEST (4 h reales): las 02:00 no existen.
        let desde = dias_de_civil(2026, 3, 28) * 86_400 + 23 * 3600;
        let ticks = ticks_tiempo(desde, desde + 4 * 3600, 6);
        let textos: Vec<&str> = ticks.iter().map(|t| t.1.as_str()).collect();
        assert_eq!(textos, ["29/03", "01:00", "03:00", "04:00", "05:00"]);
        assert!(ticks.windows(2).all(|w| w[1].0 > w[0].0), "{ticks:?}");
        assert!(ticks.len() <= 6);
    }

    #[test]
    fn fechas_en_ingles() {
        use crate::ui::i18n::provide_idioma;
        use leptos::reactive::owner::Owner;
        Owner::new().with(|| {
            provide_idioma(::i18n::Idioma::En);
            assert_eq!(
                formato_fecha_hora(D2026_10_08 + 12 * 3600 + 5, true),
                "8 Oct 14:00:05"
            );
            assert_eq!(formato_fecha(&hora_local(D2026_10_08)), "8 Oct 2026");
        });
        assert_eq!(formato_fecha(&hora_local(D2026_10_08)), "08/10/2026");
    }

    /// 2026-07-01 12:00 UTC (verano).
    const VERANO: i64 = 1_782_907_200;
    /// 2026-12-01 12:00 UTC (invierno).
    const INVIERNO: i64 = 1_796_126_400;

    #[test]
    fn canarias_una_hora_menos() {
        assert_eq!(Zona::Canarias.desfase(VERANO), 3600);
        assert_eq!(Zona::Peninsular.desfase(VERANO), 7200);
        assert_eq!(Zona::Canarias.desfase(INVIERNO), 0);
        assert_eq!(Zona::Peninsular.desfase(INVIERNO), 3600);
        // Cambia el mismo instante que la Península (01:00 UTC).
        let octubre = dias_de_civil(2026, 10, 25) * 86_400 + 3600;
        assert_eq!(Zona::Canarias.desfase(octubre - 1), 3600);
        assert_eq!(Zona::Canarias.desfase(octubre), 0);
        let h = Zona::Canarias.hora_local(VERANO);
        assert_eq!((h.hora, h.minuto), (13, 0));
    }

    #[test]
    fn fija_sin_horario_de_verano() {
        for ts in [VERANO, INVIERNO] {
            assert_eq!(Zona::Fija(0).desfase(ts), 0);
            assert_eq!(Zona::Fija(-3 * 3600 - 1800).desfase(ts), -12_600);
            assert_eq!(Zona::Fija(5 * 3600 + 45 * 60).desfase(ts), 20_700);
        }
        assert_eq!(Zona::Fija(0).hora_local(VERANO).hora, 12);
        // Las marcas del eje, en horas redondas de la zona: con UTC+05:45,
        // de 06:52 a 07:52 (01:07 a 02:07 UTC).
        let desde = D2026_10_08 + 3600 + 7 * 60;
        let ticks = ticks_tiempo_en(Zona::Fija(20_700), desde, desde + 3600, 5);
        let textos: Vec<&str> = ticks.iter().map(|t| t.1.as_str()).collect();
        assert_eq!(textos, ["07:00", "07:15", "07:30", "07:45"]);
    }

    #[test]
    fn zona_por_defecto_peninsular() {
        assert_eq!(Zona::default(), Zona::Peninsular);
        assert_eq!(zona_actual(), Zona::Peninsular);
        assert_eq!(zona_inicial(), Zona::Peninsular);
        // Con la zona de la página en el contexto, manda esa.
        use leptos::reactive::owner::Owner;
        Owner::new().with(|| {
            provide_zona(Zona::Canarias);
            assert_eq!(zona_actual(), Zona::Canarias);
            assert_eq!(formato_hora(VERANO, false), "13:00");
        });
        assert_eq!(formato_hora(VERANO, false), "14:00");
    }

    #[test]
    fn zonas_desde_texto_y_codigo() {
        for (texto, zona, codigo) in [
            ("Europe/Madrid", Zona::Peninsular, "Europe/Madrid"),
            ("Atlantic/Canary", Zona::Canarias, "Atlantic/Canary"),
            ("UTC", Zona::Fija(0), "UTC"),
            ("UTC+00:00", Zona::Fija(0), "UTC"),
            ("UTC-00:00", Zona::Fija(0), "UTC"),
            ("UTC+01:00", Zona::Fija(3600), "UTC+01:00"),
            ("UTC-03:30", Zona::Fija(-12_600), "UTC-03:30"),
            ("UTC+05:45", Zona::Fija(20_700), "UTC+05:45"),
            ("UTC+14:00", Zona::Fija(50_400), "UTC+14:00"),
            ("UTC-14:00", Zona::Fija(-50_400), "UTC-14:00"),
        ] {
            assert_eq!(Zona::desde_texto(texto), Some(zona), "{texto}");
            assert_eq!(zona.codigo(), codigo, "{texto}");
        }
        for texto in [
            "",
            "utc",
            "UTC+1",
            "UTC+14:30",
            "UTC-14:01",
            "UTC+01:60",
            "UTC 01:00",
            "UTC+1:00",
            "europe/madrid",
            "America/New_York",
        ] {
            assert_eq!(Zona::desde_texto(texto), None, "{texto}");
        }
    }

    #[test]
    fn duraciones_legibles() {
        assert_eq!(formato_duracion(45), "45 s");
        assert_eq!(formato_duracion(7 * 60 + 3), "7 min");
        assert_eq!(formato_duracion(3600 + 5 * 60), "1 h 05 min");
        assert_eq!(formato_duracion(2 * 86_400 + 3 * 3600), "2 d 3 h");
    }
}
