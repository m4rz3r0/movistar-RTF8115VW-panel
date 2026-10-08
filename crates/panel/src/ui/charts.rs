//! Gráficos SVG propios (sin librerías JS).
//!
//! La primera mitad son funciones puras (escalas, ticks, formatos, huecos,
//! geometría) que se prueban sin navegador; la segunda, los componentes
//! [`SerieTemporal`], [`Minigrafica`], [`FranjaDisponibilidad`] e
//! [`Indicador`].
//!
//! Criterios: una sola escala Y por gráfico y desde cero (son magnitudes),
//! líneas de 2 px, rejilla y ejes en filetes discretos, huecos sin datos como
//! cortes de línea (nunca se interpola), cruceta con lectura de todas las
//! series al pasar el puntero o tocar, y una tabla con los mismos datos para
//! quien no pueda ver el gráfico. Los colores de las series salen de las clases
//! `tono-1`/`tono-2` de la hoja de estilos (sin estilos en línea por la CSP);
//! los textos nunca llevan el color de la serie.
//!
//! Las horas se pintan en hora peninsular española, calculada aquí mismo para
//! que el servidor y el navegador generen el mismo HTML al hidratar.

use crate::ui::{
    Estado, IconoEstado, Insignia,
    table::{Celda, Columna, Tabla},
};
use leptos::{ev, html, prelude::*, svg};

// ---------------------------------------------------------------------------
// Escalas y ticks
// ---------------------------------------------------------------------------

/// Escala lineal que lleva `dominio` a `rango`. Con un dominio degenerado
/// (inicio = fin) devuelve el centro del rango.
pub fn escala(dominio: (f64, f64), rango: (f64, f64)) -> impl Fn(f64) -> f64 + Copy {
    let (d0, d1) = dominio;
    let (r0, r1) = rango;
    let k = if d1 == d0 {
        None
    } else {
        Some((r1 - r0) / (d1 - d0))
    };
    move |v| match k {
        Some(k) => r0 + (v - d0) * k,
        None => (r0 + r1) / 2.0,
    }
}

/// Paso «redondo» (1, 2 o 5 por una potencia de 10) más cercano por arriba a
/// `bruto`.
fn paso_bonito(bruto: f64) -> f64 {
    let exp = bruto.log10().floor();
    let base = 10f64.powf(exp);
    let f = bruto / base;
    let nf = if f <= 1.0 {
        1.0
    } else if f <= 2.0 {
        2.0
    } else if f <= 5.0 {
        5.0
    } else {
        10.0
    };
    nf * base
}

/// Unos `n` intervalos de ticks con paso redondo que cubren `[min, max]`
/// (el primero ≤ `min`, el último ≥ `max`).
pub fn ticks_bonitos(min: f64, max: f64, n: usize) -> Vec<f64> {
    if !min.is_finite() || !max.is_finite() || n == 0 {
        return Vec::new();
    }
    let (min, max) = if max < min { (max, min) } else { (min, max) };
    if max == min {
        return vec![min];
    }
    let paso = paso_bonito((max - min) / n as f64);
    // Redondeo a la precisión del paso: evita 0.30000000000000004.
    let decimales = (-paso.log10().floor()).max(0.0) as i32 + 1;
    let m = 10f64.powi(decimales);
    let inicio = (min / paso).floor() as i64;
    let fin = (max / paso).ceil() as i64;
    (inicio..=fin)
        .map(|i| (i as f64 * paso * m).round() / m)
        .collect()
}

// ---------------------------------------------------------------------------
// Formato de números (coma decimal) y unidades
// ---------------------------------------------------------------------------

/// `v` con `decimales` cifras decimales y coma decimal. Nunca «-0».
pub fn formato_decimal(v: f64, decimales: usize) -> String {
    let s = format!("{v:.decimales$}");
    let s = match s.strip_prefix('-') {
        Some(resto) if resto.chars().all(|c| c == '0' || c == '.') => resto.to_owned(),
        _ => s,
    };
    s.replace('.', ",")
}

/// Número corto para ejes: hasta dos decimales, sin ceros a la derecha.
pub fn formato_corto(v: f64) -> String {
    let s = formato_decimal(v, 2);
    if s.contains(',') {
        s.trim_end_matches('0').trim_end_matches(',').to_owned()
    } else {
        s
    }
}

/// Prefijos de bit/s en base 1000, de mayor a menor.
const PREFIJOS_BPS: [(f64, &str); 4] = [
    (1e9, "Gbit/s"),
    (1e6, "Mbit/s"),
    (1e3, "kbit/s"),
    (1.0, "bit/s"),
];

fn indice_prefijo(v: f64) -> usize {
    PREFIJOS_BPS
        .iter()
        .position(|(d, _)| v.abs() >= *d)
        .unwrap_or(PREFIJOS_BPS.len() - 1)
}

/// Tasa en bit/s con el prefijo adecuado (base 1000): `"850 bit/s"`,
/// `"1,5 kbit/s"`, `"12,3 Mbit/s"`, `"230 Mbit/s"`.
pub fn formato_bps(v: f64) -> String {
    if !v.is_finite() {
        return "—".to_owned();
    }
    let mut i = indice_prefijo(v);
    loop {
        let (div, unidad) = PREFIJOS_BPS[i];
        let x = v / div;
        let decimales = if div == 1.0 || x.abs() >= 99.95 { 0 } else { 1 };
        let m = 10f64.powi(decimales as i32);
        // 999 960 bit/s no es «1000 kbit/s» sino «1,0 Mbit/s».
        if i > 0 && ((x * m).round() / m).abs() >= 1000.0 {
            i -= 1;
            continue;
        }
        return format!("{} {unidad}", formato_decimal(x, decimales));
    }
}

/// Unidad de los valores de un gráfico.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unidad {
    /// Tasa en bit/s.
    Bps,
    /// Porcentaje (eje fijo de 0 a 100).
    Porcentaje,
    /// Cantidad entera (clientes).
    Entero,
    /// Número con decimales (carga).
    Decimal,
}

/// Valor con su unidad, para lecturas y etiquetas.
pub fn formato_valor(unidad: Unidad, v: f64) -> String {
    if !v.is_finite() {
        return "—".to_owned();
    }
    match unidad {
        Unidad::Bps => formato_bps(v),
        Unidad::Porcentaje => format!("{} %", formato_decimal(v, 1)),
        Unidad::Entero => formato_decimal(v.round(), 0),
        Unidad::Decimal => formato_decimal(v, 2),
    }
}

/// Eje Y: máximo del dominio (el último tick), ticks con su texto y la unidad
/// que se rotula sobre el eje.
#[derive(Clone, Debug, PartialEq)]
pub struct EjeY {
    pub max: f64,
    pub ticks: Vec<(f64, String)>,
    pub unidad: Option<&'static str>,
}

/// Eje Y desde 0 para un máximo de los datos `max_dato`.
pub fn eje_y(unidad: Unidad, max_dato: f64) -> EjeY {
    let dato = if max_dato.is_finite() && max_dato > 0.0 {
        Some(max_dato)
    } else {
        None
    };
    let max = match unidad {
        Unidad::Porcentaje => dato.map_or(100.0, |d| d.max(100.0)),
        Unidad::Entero => dato.unwrap_or(0.0).max(4.0),
        Unidad::Bps => dato.unwrap_or(1e3),
        Unidad::Decimal => dato.unwrap_or(1.0),
    };
    let ticks = ticks_bonitos(0.0, max, 5);
    let max = ticks.last().copied().unwrap_or(max);
    let (div, rotulo) = match unidad {
        Unidad::Bps => {
            let (d, u) = PREFIJOS_BPS[indice_prefijo(max)];
            (d, Some(u))
        }
        Unidad::Porcentaje => (1.0, Some("%")),
        Unidad::Entero | Unidad::Decimal => (1.0, None),
    };
    EjeY {
        max,
        ticks: ticks
            .into_iter()
            .map(|t| (t, formato_corto(t / div)))
            .collect(),
        unidad: rotulo,
    }
}

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

// ---------------------------------------------------------------------------
// Huecos y trazados
// ---------------------------------------------------------------------------

/// Paso de muestreo: la mediana de las diferencias positivas entre marcas de
/// tiempo consecutivas (`None` con menos de dos marcas).
pub fn paso_muestreo(ts: impl IntoIterator<Item = i64>) -> Option<i64> {
    let ts: Vec<i64> = ts.into_iter().collect();
    let mut difs: Vec<i64> = ts
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|d| *d > 0)
        .collect();
    if difs.is_empty() {
        return None;
    }
    difs.sort_unstable();
    Some(difs[difs.len() / 2])
}

/// Tramos continuos de una serie: se corta en cada valor ausente (o no
/// finito) y donde faltan marcas de tiempo por más de dos pasos de muestreo.
pub fn segmentos(puntos: &[(i64, Option<f64>)]) -> Vec<Vec<(i64, f64)>> {
    let hueco_max = paso_muestreo(puntos.iter().map(|p| p.0)).map(|p| 2 * p);
    let mut tramos: Vec<Vec<(i64, f64)>> = Vec::new();
    let mut actual: Vec<(i64, f64)> = Vec::new();
    for &(t, v) in puntos {
        let salto = match (actual.last(), hueco_max) {
            (Some(&(prev, _)), Some(max)) => t - prev > max,
            _ => false,
        };
        if salto {
            tramos.push(std::mem::take(&mut actual));
        }
        match v {
            Some(v) if v.is_finite() => actual.push((t, v)),
            _ => {
                if !actual.is_empty() {
                    tramos.push(std::mem::take(&mut actual));
                }
            }
        }
    }
    if !actual.is_empty() {
        tramos.push(actual);
    }
    tramos
}

/// Trazado SVG (`M x,y L x,y …`) de una polilínea en píxeles.
fn ruta_linea(px: &[(f64, f64)]) -> String {
    let mut d = String::with_capacity(px.len() * 14);
    for (i, (x, y)) in px.iter().enumerate() {
        d.push(if i == 0 { 'M' } else { 'L' });
        d.push_str(&format!("{x:.1},{y:.1}"));
    }
    d
}

/// Trazado SVG del área bajo una polilínea, cerrada sobre `y0`.
fn ruta_area(px: &[(f64, f64)], y0: f64) -> String {
    let (Some(primero), Some(ultimo)) = (px.first(), px.last()) else {
        return String::new();
    };
    format!(
        "{}L{:.1},{y0:.1}L{:.1},{y0:.1}Z",
        ruta_linea(px),
        ultimo.0,
        primero.0
    )
}

/// Índice de la marca de `ts` (ordenadas) más cercana a `t`.
pub fn indice_mas_cercano(ts: &[i64], t: f64) -> Option<usize> {
    if ts.is_empty() {
        return None;
    }
    let i = ts.partition_point(|&x| (x as f64) < t);
    Some(if i == 0 {
        0
    } else if i == ts.len() {
        ts.len() - 1
    } else if t - ts[i - 1] as f64 <= ts[i] as f64 - t {
        i - 1
    } else {
        i
    })
}

/// Valor de una serie exactamente en `t` (`None` si no hay muestra o falta).
fn valor_en(puntos: &[(i64, Option<f64>)], t: i64) -> Option<f64> {
    puntos
        .binary_search_by_key(&t, |p| p.0)
        .ok()
        .and_then(|i| puntos[i].1)
        .filter(|v| v.is_finite())
}

// ---------------------------------------------------------------------------
// Series y geometría del gráfico temporal
// ---------------------------------------------------------------------------

/// Color de una serie (clases `tono-1`/`tono-2` de la hoja de estilos):
/// primario azul `#3399E0` (bajada, y las series únicas) y secundario ocre
/// `#C27F00` (subida). El ámbar de la interfaz no se usa para datos.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tono {
    #[default]
    Primario,
    Secundario,
}

impl Tono {
    fn clase(self) -> &'static str {
        match self {
            Tono::Primario => "tono-1",
            Tono::Secundario => "tono-2",
        }
    }
}

/// Serie temporal: puntos `(segundos Unix, valor)` en orden; `None` es «sin
/// dato» y corta la línea.
#[derive(Clone, Debug, PartialEq)]
pub struct Serie {
    pub nombre: String,
    pub tono: Tono,
    pub puntos: Vec<(i64, Option<f64>)>,
    /// Relleno suave bajo la línea.
    pub area: bool,
}

impl Serie {
    pub fn new(nombre: impl Into<String>, tono: Tono, puntos: Vec<(i64, Option<f64>)>) -> Self {
        Serie {
            nombre: nombre.into(),
            tono,
            puntos,
            area: false,
        }
    }

    /// Con relleno suave bajo la línea.
    pub fn con_area(mut self) -> Self {
        self.area = true;
        self
    }

    fn ultimo(&self) -> Option<(i64, f64)> {
        self.puntos
            .iter()
            .rev()
            .find_map(|&(t, v)| v.filter(|v| v.is_finite()).map(|v| (t, v)))
    }

    fn maximo(&self) -> Option<f64> {
        self.puntos
            .iter()
            .filter_map(|p| p.1)
            .filter(|v| v.is_finite())
            .reduce(f64::max)
    }
}

const MARGEN_ARRIBA: f64 = 24.0;
const MARGEN_DERECHA: f64 = 14.0;
const MARGEN_ABAJO: f64 = 26.0;
/// Anchura estimada de un carácter del texto de los ejes (11–12 px, cifras
/// tabulares).
const ANCHO_CARACTER: f64 = 7.0;
/// Separación mínima entre ticks de tiempo.
const SEPARACION_TICKS_X: f64 = 84.0;

/// Tick de un eje ya colocado en píxeles.
#[derive(Clone, Debug, PartialEq)]
pub struct Tick {
    pub pos: f64,
    pub texto: String,
    /// `text-anchor` del rótulo.
    pub ancla: &'static str,
}

/// Lo que se dibuja de una serie.
#[derive(Clone, Debug, PartialEq)]
pub struct Trazo {
    pub clase: &'static str,
    /// Un trazado por tramo continuo de dos o más puntos.
    pub lineas: Vec<String>,
    pub areas: Vec<String>,
    /// Puntos aislados entre dos huecos (se dibujan como marcador).
    pub sueltos: Vec<(f64, f64)>,
    /// Último dato.
    pub fin: Option<(f64, f64)>,
    /// Etiqueta directa del último valor: `(x, y, texto)`.
    pub etiqueta: Option<(f64, f64, String)>,
}

impl Trazo {
    fn vacio(clase: &'static str) -> Self {
        Trazo {
            clase,
            lineas: Vec::new(),
            areas: Vec::new(),
            sueltos: Vec::new(),
            fin: None,
            etiqueta: None,
        }
    }
}

/// Geometría completa de un gráfico temporal a un tamaño dado.
#[derive(Clone, Debug, PartialEq)]
pub struct Geometria {
    pub ancho: f64,
    pub alto: f64,
    pub izq: f64,
    pub der: f64,
    pub arriba: f64,
    pub abajo: f64,
    pub t0: i64,
    pub t1: i64,
    pub y_max: f64,
    pub ticks_y: Vec<Tick>,
    pub unidad_eje: Option<&'static str>,
    pub ticks_x: Vec<Tick>,
    pub trazos: Vec<Trazo>,
    /// Marcas de tiempo de todas las series, ordenadas y sin repetir.
    pub tiempos: Vec<i64>,
    /// Paso de muestreo, para decidir si las horas llevan segundos.
    pub paso: Option<i64>,
}

impl Geometria {
    fn x(&self, t: i64) -> f64 {
        escala((self.t0 as f64, self.t1 as f64), (self.izq, self.der))(t as f64)
    }

    fn y(&self, v: f64) -> f64 {
        escala((0.0, self.y_max), (self.abajo, self.arriba))(v)
    }

    fn con_segundos(&self) -> bool {
        self.paso.is_some_and(|p| p < 60)
    }
}

/// Ancla del texto de un tick centrado en `pos` para que no se salga de
/// `[0, ancho]`.
fn ancla_dentro(pos: f64, texto: &str, ancho: f64) -> &'static str {
    let medio = texto.chars().count() as f64 * ANCHO_CARACTER / 2.0;
    if pos + medio > ancho {
        "end"
    } else if pos - medio < 0.0 {
        "start"
    } else {
        "middle"
    }
}

/// Calcula la geometría de `series` en un lienzo de `ancho × alto` píxeles.
/// `ventana` fija el intervalo del eje X; sin ella, el de los datos.
pub fn geometria(
    series: &[Serie],
    unidad: Unidad,
    ancho: f64,
    alto: f64,
    ventana: Option<(i64, i64)>,
) -> Geometria {
    let mut tiempos: Vec<i64> = series
        .iter()
        .flat_map(|s| s.puntos.iter().map(|p| p.0))
        .collect();
    tiempos.sort_unstable();
    tiempos.dedup();
    let (t0, t1) = ventana.unwrap_or(match (tiempos.first(), tiempos.last()) {
        (Some(&a), Some(&b)) => (a, b),
        _ => (0, 1),
    });
    let t1 = if t1 <= t0 { t0 + 1 } else { t1 };

    let max_dato = series
        .iter()
        .filter_map(Serie::maximo)
        .fold(f64::NAN, f64::max);
    let eje = eje_y(unidad, max_dato);
    let ancho_etiquetas = eje
        .ticks
        .iter()
        .map(|(_, s)| s.chars().count())
        .max()
        .unwrap_or(1) as f64
        * ANCHO_CARACTER;
    let izq = (ancho_etiquetas + 12.0).max(28.0);
    let der = (ancho - MARGEN_DERECHA).max(izq + 1.0);
    let arriba = MARGEN_ARRIBA;
    let abajo = (alto - MARGEN_ABAJO).max(arriba + 1.0);

    let mut geo = Geometria {
        ancho,
        alto,
        izq,
        der,
        arriba,
        abajo,
        t0,
        t1,
        y_max: eje.max,
        ticks_y: Vec::new(),
        unidad_eje: eje.unidad,
        ticks_x: Vec::new(),
        trazos: Vec::new(),
        paso: paso_muestreo(tiempos.iter().copied()),
        tiempos,
    };
    geo.ticks_y = eje
        .ticks
        .into_iter()
        .map(|(v, texto)| Tick {
            pos: geo.y(v),
            texto,
            ancla: "end",
        })
        .collect();
    let max_ticks_x = ((der - izq) / SEPARACION_TICKS_X).floor().max(2.0) as usize;
    geo.ticks_x = ticks_tiempo(t0, t1, max_ticks_x)
        .into_iter()
        .map(|(t, texto)| {
            let pos = geo.x(t);
            Tick {
                pos,
                ancla: ancla_dentro(pos, &texto, ancho),
                texto,
            }
        })
        .collect();

    let y0 = geo.y(0.0);
    geo.trazos = series
        .iter()
        .map(|s| {
            let mut trazo = Trazo::vacio(s.tono.clase());
            for tramo in segmentos(&s.puntos) {
                let px: Vec<(f64, f64)> =
                    tramo.iter().map(|&(t, v)| (geo.x(t), geo.y(v))).collect();
                if px.len() == 1 {
                    trazo.sueltos.push(px[0]);
                    continue;
                }
                if s.area {
                    trazo.areas.push(ruta_area(&px, y0));
                }
                trazo.lineas.push(ruta_linea(&px));
            }
            if let Some((t, v)) = s.ultimo() {
                let (x, y) = (geo.x(t), geo.y(v));
                trazo.fin = Some((x, y));
                // Encima del punto; debajo si no cabe.
                let ye = if y - 10.0 < arriba + 4.0 {
                    y + 18.0
                } else {
                    y - 10.0
                };
                trazo.etiqueta = Some((x - 8.0, ye, formato_valor(unidad, v)));
            }
            trazo
        })
        .collect();
    // Etiquetas directas solo con hasta 4 series y si no chocan: si dos
    // quedan a menos de 14 px, se quitan todas (las llevan la leyenda y la
    // lectura).
    let alturas: Vec<f64> = geo
        .trazos
        .iter()
        .filter_map(|t| t.etiqueta.as_ref().map(|e| e.1))
        .collect();
    let chocan = alturas
        .iter()
        .enumerate()
        .any(|(i, a)| alturas[i + 1..].iter().any(|b| (a - b).abs() < 14.0));
    if series.len() > 4 || chocan {
        for t in &mut geo.trazos {
            t.etiqueta = None;
        }
    }
    geo
}

/// Lectura de la cruceta en una marca: posición, puntos de cada serie y filas
/// `(clase, valor, nombre)` del recuadro.
#[derive(Clone, Debug, PartialEq)]
struct Lectura {
    x: f64,
    cabecera: String,
    puntos: Vec<(&'static str, f64)>,
    filas: Vec<(&'static str, String, String)>,
}

fn lectura(series: &[Serie], unidad: Unidad, geo: &Geometria, idx: usize) -> Option<Lectura> {
    let t = *geo.tiempos.get(idx)?;
    let mut puntos = Vec::new();
    let mut filas = Vec::new();
    for s in series {
        let v = valor_en(&s.puntos, t);
        if let Some(v) = v {
            puntos.push((s.tono.clase(), geo.y(v)));
        }
        filas.push((
            s.tono.clase(),
            v.map_or_else(|| "sin datos".to_owned(), |v| formato_valor(unidad, v)),
            s.nombre.clone(),
        ));
    }
    Some(Lectura {
        x: geo.x(t),
        cabecera: formato_fecha_hora(t, geo.con_segundos()),
        puntos,
        filas,
    })
}

/// Resumen textual para lectores de pantalla.
fn resumen(titulo: &str, series: &[Serie], unidad: Unidad) -> String {
    let partes: Vec<String> = series
        .iter()
        .map(|s| match (s.ultimo(), s.maximo()) {
            (Some((_, u)), Some(m)) => format!(
                "{}: última {}, máxima {}",
                s.nombre,
                formato_valor(unidad, u),
                formato_valor(unidad, m)
            ),
            _ => format!("{}: sin datos", s.nombre),
        })
        .collect();
    format!("{titulo}. {}", partes.join("; "))
}

// ---------------------------------------------------------------------------
// Disponibilidad
// ---------------------------------------------------------------------------

/// Corte de la conexión: `hasta = None` si sigue caída.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Corte {
    pub desde: i64,
    pub hasta: Option<i64>,
}

/// Tramos caídos recortados a `[desde, hasta]` (un corte abierto llega hasta
/// `hasta`), sin tramos vacíos.
pub fn tramos_caidos(eventos: &[Corte], desde: i64, hasta: i64) -> Vec<(i64, i64)> {
    eventos
        .iter()
        .map(|c| (c.desde.max(desde), c.hasta.unwrap_or(hasta).min(hasta)))
        .filter(|(a, b)| b > a)
        .collect()
}

/// Cortes, segundos sin conexión y porcentaje disponible en un intervalo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResumenCortes {
    pub cortes: usize,
    pub segundos_caido: i64,
    pub disponibilidad: f64,
}

/// Resumen de los cortes en `[desde, hasta]`.
pub fn resumen_cortes(eventos: &[Corte], desde: i64, hasta: i64) -> ResumenCortes {
    let tramos = tramos_caidos(eventos, desde, hasta);
    let caido: i64 = tramos.iter().map(|(a, b)| b - a).sum();
    let total = (hasta - desde).max(1);
    ResumenCortes {
        cortes: tramos.len(),
        segundos_caido: caido,
        disponibilidad: 100.0 * (total - caido).max(0) as f64 / total as f64,
    }
}

/// Porcentaje de disponibilidad sin redondear hacia arriba: con cortes nunca
/// sale «100 %».
fn formato_disponibilidad(r: &ResumenCortes) -> String {
    if r.segundos_caido == 0 {
        "100 %".to_owned()
    } else {
        format!(
            "{} %",
            formato_decimal((r.disponibilidad * 100.0).floor() / 100.0, 2)
        )
    }
}

// ---------------------------------------------------------------------------
// Componentes
// ---------------------------------------------------------------------------

/// Anchura con la que se pinta en el servidor (y en la hidratación) antes de
/// medir el contenedor.
const ANCHO_INICIAL: f64 = 720.0;

/// Anchura en píxeles del contenedor `nodo`, medida en el navegador tras
/// hidratar y al cambiar el tamaño de la ventana. Los gráficos se dibujan a su
/// tamaño real para que los textos no se escalen.
fn usar_ancho(nodo: NodeRef<html::Div>) -> ReadSignal<f64> {
    let (ancho, set_ancho) = signal(ANCHO_INICIAL);
    Effect::new(move |_| {
        let Some(el) = nodo.get() else {
            return;
        };
        let medir = move || {
            let w = f64::from(el.client_width());
            if w > 0.0 {
                set_ancho.set(w);
            }
        };
        medir();
        let h = window_event_listener(ev::resize, move |_| medir());
        on_cleanup(move || h.remove());
    });
    ancho
}

/// X del puntero en coordenadas del `viewBox` de `svg` (que mide `ancho`).
fn x_del_puntero(svg: &NodeRef<svg::Svg>, client_x: i32, ancho: f64) -> Option<f64> {
    let el = svg.get_untracked()?;
    let r = el.get_bounding_client_rect();
    (r.width() > 0.0).then(|| (f64::from(client_x) - r.left()) * ancho / r.width())
}

/// Recuadro de lectura dentro del SVG junto a la cruceta en `x`, sin salirse
/// de `limites`: cabecera y filas `(clase, valor, nombre)`. El valor manda; el
/// nombre acompaña.
fn recuadro(
    x: f64,
    y: f64,
    limites: (f64, f64),
    cabecera: String,
    filas: Vec<(&'static str, String, String)>,
) -> impl IntoView {
    let ancho = filas
        .iter()
        .map(|(_, v, n)| {
            30.0 + v.chars().count() as f64 * 7.4 + n.chars().count() as f64 * 6.6 + 6.0
        })
        .fold(cabecera.chars().count() as f64 * 6.8, f64::max)
        + 20.0;
    let alto = 30.0 + filas.len() as f64 * 18.0;
    let (min, max) = limites;
    let tx = if x + 12.0 + ancho <= max {
        x + 12.0
    } else {
        (x - 12.0 - ancho).max(min)
    };
    let filas = filas
        .into_iter()
        .enumerate()
        .map(|(i, (clase, valor, nombre))| {
            let fy = 40.0 + i as f64 * 18.0;
            view! {
                <line class=format!("clave {clase}") x1="10" x2="22" y1=fy - 4.0 y2=fy - 4.0 />
                <text x="30" y=fy>
                    <tspan class="lectura-valor">{valor}</tspan>
                    <tspan class="lectura-nombre" dx="6">
                        {nombre}
                    </tspan>
                </text>
            }
        })
        .collect_view();
    view! {
        <g class="lectura" transform=format!("translate({tx:.1},{y:.1})")>
            <rect
                class="lectura-fondo"
                width=format!("{ancho:.0}")
                height=format!("{alto:.0}")
                rx="6"
            />
            <text class="lectura-cabecera" x="10" y="19">
                {cabecera}
            </text>
            {filas}
        </g>
    }
}

/// Gráfico de líneas temporal con una sola escala Y desde cero, rejilla, ejes
/// con unidades, cortes de línea en los huecos, etiqueta directa del último
/// valor, cruceta con lectura de todas las series (puntero, toque o flechas
/// del teclado) y tabla de datos plegada.
#[component]
pub fn SerieTemporal(
    /// Título visible (y del resumen accesible).
    #[prop(into)]
    titulo: String,
    #[prop(into)] series: Signal<Vec<Serie>>,
    unidad: Unidad,
    /// Alto total en píxeles (con el eje X).
    #[prop(default = 240)]
    alto: u32,
    /// Intervalo fijo del eje X (`desde`, `hasta`); sin él, el de los datos.
    #[prop(into, optional)]
    ventana: MaybeProp<(i64, i64)>,
) -> impl IntoView {
    let alto = f64::from(alto);
    let contenedor = NodeRef::<html::Div>::new();
    let svg_ref = NodeRef::<svg::Svg>::new();
    let details_ref = NodeRef::<html::Details>::new();
    let ancho = usar_ancho(contenedor);
    let geo =
        Memo::new(move |_| series.with(|s| geometria(s, unidad, ancho.get(), alto, ventana.get())));
    let (foco, set_foco) = signal(None::<usize>);
    let (tabla_abierta, set_tabla_abierta) = signal(false);

    let mover = move |client_x: i32| {
        let g = geo.get_untracked();
        if let Some(x) = x_del_puntero(&svg_ref, client_x, g.ancho) {
            let t = escala((g.izq, g.der), (g.t0 as f64, g.t1 as f64))(x);
            set_foco.set(indice_mas_cercano(&g.tiempos, t));
        }
    };
    let teclado = move |ev: ev::KeyboardEvent| {
        let n = geo.with_untracked(|g| g.tiempos.len());
        if n == 0 {
            return;
        }
        let actual = foco.get_untracked().unwrap_or(n - 1);
        let nuevo = match ev.key().as_str() {
            "ArrowLeft" => actual.saturating_sub(1),
            "ArrowRight" => (actual + 1).min(n - 1),
            "Home" => 0,
            "End" => n - 1,
            "Escape" => {
                set_foco.set(None);
                return;
            }
            _ => return,
        };
        ev.prevent_default();
        set_foco.set(Some(nuevo));
    };

    let titulo_resumen = titulo.clone();
    let etiqueta_aria = move || series.with(|s| resumen(&titulo_resumen, s, unidad));

    let leyenda = move || {
        series.with(|s| {
            (s.len() >= 2).then(|| {
                let items = s
                    .iter()
                    .map(|serie| {
                        view! {
                            <li>
                                <span
                                    class=format!("clave-linea {}", serie.tono.clase())
                                    aria-hidden="true"
                                ></span>
                                {serie.nombre.clone()}
                            </li>
                        }
                    })
                    .collect_view();
                view! { <ul class="leyenda">{items}</ul> }
            })
        })
    };

    let estatico = move || {
        geo.with(|g| {
            let rejilla = g
                .ticks_y
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let clase = if i == 0 { "eje-base" } else { "rejilla" };
                    view! {
                        <line class=clase x1=g.izq x2=g.der y1=t.pos y2=t.pos />
                        <text class="eje-texto" x=g.izq - 8.0 y=t.pos + 4.0 text-anchor=t.ancla>
                            {t.texto.clone()}
                        </text>
                    }
                })
                .collect_view();
            let unidad_eje = g.unidad_eje.map(|u| {
                view! {
                    <text class="eje-unidad" x="0" y="11">
                        {u}
                    </text>
                }
            });
            let ticks_x = g
                .ticks_x
                .iter()
                .map(|t| {
                    view! {
                        <text class="eje-texto" x=t.pos y=g.alto - 8.0 text-anchor=t.ancla>
                            {t.texto.clone()}
                        </text>
                    }
                })
                .collect_view();
            let areas = g
                .trazos
                .iter()
                .flat_map(|t| {
                    t.areas.iter().map(move |d| {
                        view! { <path class=format!("area {}", t.clase) d=d.clone() /> }
                    })
                })
                .collect_view();
            let lineas = g
                .trazos
                .iter()
                .flat_map(|t| {
                    t.lineas.iter().map(move |d| {
                        view! { <path class=format!("linea {}", t.clase) d=d.clone() /> }
                    })
                })
                .collect_view();
            let marcadores = g
                .trazos
                .iter()
                .flat_map(|t| {
                    t.sueltos.iter().chain(t.fin.iter()).map(move |&(x, y)| {
                        view! { <circle class=format!("marcador {}", t.clase) cx=x cy=y r="4" /> }
                    })
                })
                .collect_view();
            let etiquetas = g
                .trazos
                .iter()
                .filter_map(|t| t.etiqueta.clone())
                .map(|(x, y, texto)| {
                    view! {
                        <text class="etiqueta-directa" x=x y=y text-anchor="end">
                            {texto}
                        </text>
                    }
                })
                .collect_view();
            let vacio = g.tiempos.is_empty().then(|| {
                view! {
                    <text
                        class="grafica-vacia"
                        x=(g.izq + g.der) / 2.0
                        y=(g.arriba + g.abajo) / 2.0
                        text-anchor="middle"
                    >
                        "Sin datos en este intervalo"
                    </text>
                }
            });
            view! {
                {unidad_eje}
                {rejilla}
                {ticks_x}
                {areas}
                {lineas}
                {marcadores}
                {etiquetas}
                {vacio}
            }
        })
    };

    let interactivo = move || {
        let idx = foco.get()?;
        let g = geo.get();
        let l = series.with(|s| lectura(s, unidad, &g, idx))?;
        let puntos = l
            .puntos
            .iter()
            .map(|&(clase, y)| {
                view! { <circle class=format!("marcador {clase}") cx=l.x cy=y r="4" /> }
            })
            .collect_view();
        Some(view! {
            <line class="cruceta" x1=l.x x2=l.x y1=g.arriba y2=g.abajo />
            {puntos}
            {recuadro(l.x, g.arriba + 2.0, (0.0, g.ancho), l.cabecera, l.filas)}
        })
    };

    let tabla = move || {
        tabla_abierta.get().then(|| {
            let g = geo.get_untracked();
            let (columnas, filas) = series.with_untracked(|s| {
                let mut columnas = vec![Columna::new("Hora")];
                columnas.extend(
                    s.iter()
                        .map(|serie| Columna::numerica(serie.nombre.clone())),
                );
                let filas: Vec<Vec<Celda>> = g
                    .tiempos
                    .iter()
                    .map(|&t| {
                        let mut fila = vec![Celda::Numero(formato_fecha_hora(t, g.con_segundos()))];
                        fila.extend(s.iter().map(|serie| {
                            valor_en(&serie.puntos, t)
                                .map_or(Celda::Vacia, |v| Celda::Numero(formato_valor(unidad, v)))
                        }));
                        fila
                    })
                    .collect();
                (columnas, filas)
            });
            view! { <Tabla columnas filas /> }
        })
    };

    view! {
        <figure class="grafica">
            <figcaption class="grafica-cabecera">
                <span class="grafica-titulo">{titulo}</span>
                {leyenda}
            </figcaption>
            <div class="grafica-lienzo" node_ref=contenedor>
                <svg
                    node_ref=svg_ref
                    viewBox=move || format!("0 0 {:.0} {alto:.0}", ancho.get())
                    role="img"
                    aria-label=etiqueta_aria
                    tabindex="0"
                    on:pointermove=move |ev| mover(ev.client_x())
                    on:pointerdown=move |ev| mover(ev.client_x())
                    on:pointerleave=move |ev| {
                        // En pantallas táctiles la lectura se queda tras el
                        // toque; se quita al tocar fuera (pierde el foco).
                        if ev.pointer_type() != "touch" {
                            set_foco.set(None);
                        }
                    }
                    on:keydown=teclado
                    on:blur=move |_| set_foco.set(None)
                >
                    {estatico}
                    {interactivo}
                </svg>
            </div>
            <details
                class="grafica-datos"
                node_ref=details_ref
                on:toggle=move |_| {
                    if let Some(d) = details_ref.get_untracked() {
                        set_tabla_abierta.set(d.open());
                    }
                }
            >
                <summary>"Ver los datos en una tabla"</summary>
                {tabla}
            </details>
        </figure>
    }
    .into_any()
}

/// Geometría de una minigráfica en un lienzo de `ancho × alto` (sin ejes,
/// desde cero): trazados de línea y de área y último punto.
fn geometria_mini(valores: &[Option<f64>], ancho: f64, alto: f64) -> Trazo {
    let puntos: Vec<(i64, Option<f64>)> = valores
        .iter()
        .enumerate()
        .map(|(i, v)| (i as i64, *v))
        .collect();
    let max = valores
        .iter()
        .flatten()
        .copied()
        .filter(|v| v.is_finite())
        .fold(0.0, f64::max);
    let max = if max > 0.0 { max } else { 1.0 };
    let n = valores.len().saturating_sub(1).max(1) as f64;
    let sx = escala((0.0, n), (0.0, ancho));
    let sy = escala((0.0, max), (alto - 1.0, 3.0));
    let mut trazo = Trazo::vacio("");
    for tramo in segmentos(&puntos) {
        let px: Vec<(f64, f64)> = tramo.iter().map(|&(i, v)| (sx(i as f64), sy(v))).collect();
        if px.len() > 1 {
            trazo.areas.push(ruta_area(&px, alto));
            trazo.lineas.push(ruta_linea(&px));
        }
        trazo.fin = px.last().copied();
    }
    trazo
}

/// Minigráfica decorativa (sin ejes) de los últimos valores, desde cero. El
/// valor lo da el texto que la acompaña.
#[component]
pub fn Minigrafica(
    #[prop(into)] valores: Signal<Vec<Option<f64>>>,
    #[prop(optional)] tono: Tono,
) -> impl IntoView {
    const ANCHO: f64 = 120.0;
    const ALTO: f64 = 32.0;
    let clase = tono.clase();
    let dibujo = move || {
        let t = valores.with(|v| geometria_mini(v, ANCHO, ALTO));
        let areas = t
            .areas
            .into_iter()
            .map(|d| view! { <path class=format!("area {clase}") d=d /> })
            .collect_view();
        let lineas = t
            .lineas
            .into_iter()
            .map(|d| {
                view! {
                    <path class=format!("linea {clase}") d=d vector-effect="non-scaling-stroke" />
                }
            })
            .collect_view();
        // Trazo de longitud cero con extremo redondo: un punto que no se
        // deforma con `preserveAspectRatio="none"`.
        let fin = t.fin.map(|(x, y)| {
            view! {
                <path
                    class=format!("mini-fin {clase}")
                    d=format!("M{x:.1},{y:.1}l0,0")
                    vector-effect="non-scaling-stroke"
                />
            }
        });
        view! {
            {areas}
            {lineas}
            {fin}
        }
    };
    view! {
        <svg
            class="minigrafica"
            viewBox=format!("0 0 {ANCHO} {ALTO}")
            preserveAspectRatio="none"
            aria-hidden="true"
            focusable="false"
        >
            {dibujo}
        </svg>
    }
}

/// Franja de disponibilidad de internet en `[desde, hasta]`: resumen (cortes,
/// tiempo sin conexión, porcentaje), barra con los cortes, lectura al pasar
/// el puntero o tocar (o con las flechas, de corte en corte) y tabla de
/// cortes plegada.
#[component]
pub fn FranjaDisponibilidad(
    #[prop(into)] eventos: Signal<Vec<Corte>>,
    #[prop(into)] desde: Signal<i64>,
    #[prop(into)] hasta: Signal<i64>,
) -> impl IntoView {
    const ALTO: f64 = 52.0;
    const BARRA_Y: f64 = 4.0;
    const BARRA_ALTO: f64 = 20.0;
    // Distancia a la que el puntero «engancha» un corte estrecho.
    const ENGANCHE: f64 = 10.0;
    let contenedor = NodeRef::<html::Div>::new();
    let svg_ref = NodeRef::<svg::Svg>::new();
    let details_ref = NodeRef::<html::Details>::new();
    let ancho = usar_ancho(contenedor);
    let (foco, set_foco) = signal(None::<f64>);
    let (tabla_abierta, set_tabla_abierta) = signal(false);

    let tramos = Memo::new(move |_| eventos.with(|e| tramos_caidos(e, desde.get(), hasta.get())));
    let resumen = Memo::new(move |_| eventos.with(|e| resumen_cortes(e, desde.get(), hasta.get())));
    let sx = move || escala((desde.get() as f64, hasta.get() as f64), (0.0, ancho.get()));

    let cabecera = move || {
        let r = resumen.get();
        let (estado, texto) = match r.cortes {
            0 => (Estado::Ok, "Sin cortes".to_owned()),
            1 => (Estado::Error, "1 corte".to_owned()),
            n => (Estado::Error, format!("{n} cortes")),
        };
        let detalle = if r.cortes == 0 {
            format!("{} disponible", formato_disponibilidad(&r))
        } else {
            format!(
                "{} disponible · {} sin conexión",
                formato_disponibilidad(&r),
                formato_duracion(r.segundos_caido)
            )
        };
        view! {
            <Insignia estado texto />
            <span class="franja-detalle">{detalle}</span>
        }
    };

    let barra = move || {
        let s = sx();
        let w = ancho.get();
        let cortes = tramos
            .get()
            .into_iter()
            .map(|(a, b)| {
                let x = s(a as f64);
                let ancho_corte = (s(b as f64) - x).max(2.0);
                view! {
                    <rect
                        class="franja-corte"
                        x=x
                        y=BARRA_Y
                        width=ancho_corte
                        height=BARRA_ALTO
                    />
                }
            })
            .collect_view();
        let max_ticks = (w / SEPARACION_TICKS_X).max(2.0) as usize;
        let ticks = ticks_tiempo(desde.get(), hasta.get(), max_ticks)
            .into_iter()
            .map(|(t, texto)| {
                let x = s(t as f64);
                let ancla = ancla_dentro(x, &texto, w);
                view! {
                    <line
                        class="franja-tick"
                        x1=x
                        x2=x
                        y1=BARRA_Y + BARRA_ALTO
                        y2=BARRA_Y + BARRA_ALTO + 4.0
                    />
                    <text class="eje-texto" x=x y=ALTO - 6.0 text-anchor=ancla>
                        {texto}
                    </text>
                }
            })
            .collect_view();
        view! {
            <rect class="franja-ok" x="0" y=BARRA_Y width=w height=BARRA_ALTO rx="3" />
            {cortes}
            {ticks}
        }
    };

    // Lectura: el corte bajo el puntero (o a menos de `ENGANCHE` px), o el
    // estado «conectado» en ese instante.
    let interactivo = move || {
        let x = foco.get()?;
        let s = sx();
        let w = ancho.get();
        let (d, h) = (desde.get(), hasta.get());
        let t = escala((0.0, w), (d as f64, h as f64))(x) as i64;
        let corte = tramos.get().into_iter().find(|&(a, b)| {
            let xa = s(a as f64);
            let xb = s(b as f64).max(xa + 2.0);
            x >= xa - ENGANCHE && x <= xb + ENGANCHE
        });
        let (cabecera, filas, resalte) = match corte {
            Some((a, b)) => {
                let abierto =
                    b == h && eventos.with(|e| e.iter().any(|c| c.hasta.is_none_or(|fin| fin > h)));
                let fin = if abierto {
                    "ahora".to_owned()
                } else {
                    formato_hora(b, false)
                };
                let xa = s(a as f64);
                (
                    format!("{} → {fin}", formato_fecha_hora(a, false)),
                    vec![(
                        "franja-clave-corte",
                        formato_duracion(b - a),
                        "sin conexión".to_owned(),
                    )],
                    Some((xa, (s(b as f64) - xa).max(2.0))),
                )
            }
            None => (
                formato_fecha_hora(t.min(h), false),
                vec![("franja-clave-ok", "Conectado".to_owned(), String::new())],
                None,
            ),
        };
        let resalte = resalte.map(|(rx, rw)| {
            view! {
                <rect
                    class="franja-resalte"
                    x=rx - 2.0
                    y=BARRA_Y - 2.0
                    width=rw + 4.0
                    height=BARRA_ALTO + 4.0
                    rx="3"
                />
            }
        });
        Some(view! {
            {resalte}
            <line class="cruceta" x1=x x2=x y1=BARRA_Y - 2.0 y2=BARRA_Y + BARRA_ALTO + 2.0 />
            {recuadro(x, BARRA_Y + BARRA_ALTO + 6.0, (0.0, w), cabecera, filas)}
        })
    };

    let mover = move |client_x: i32| {
        let w = ancho.get_untracked();
        if let Some(x) = x_del_puntero(&svg_ref, client_x, w) {
            set_foco.set(Some(x.clamp(0.0, w)));
        }
    };
    // Flechas: de corte en corte.
    let teclado = move |ev: ev::KeyboardEvent| {
        let s = sx();
        let centros: Vec<f64> = tramos
            .get_untracked()
            .iter()
            .map(|&(a, b)| (s(a as f64) + s(b as f64)) / 2.0)
            .collect();
        if centros.is_empty() {
            return;
        }
        let actual = foco.get_untracked();
        let nuevo = match ev.key().as_str() {
            "ArrowRight" => centros
                .iter()
                .copied()
                .find(|c| actual.is_none_or(|x| *c > x + 0.5))
                .or(centros.last().copied()),
            "ArrowLeft" => centros
                .iter()
                .rev()
                .copied()
                .find(|c| actual.is_none_or(|x| *c < x - 0.5))
                .or(centros.first().copied()),
            "Escape" => None,
            _ => return,
        };
        ev.prevent_default();
        set_foco.set(nuevo);
    };

    let tabla = move || {
        tabla_abierta.get().then(|| {
            let columnas = vec![
                Columna::new("Inicio"),
                Columna::new("Fin"),
                Columna::numerica("Duración"),
            ];
            let (d, h) = (desde.get_untracked(), hasta.get_untracked());
            let filas: Vec<Vec<Celda>> = eventos.with_untracked(|e| {
                e.iter()
                    .filter(|c| c.hasta.unwrap_or(h) > d && c.desde < h)
                    .map(|c| {
                        vec![
                            Celda::Numero(formato_fecha_hora(c.desde, false)),
                            c.hasta.map_or(Celda::Texto("en curso".to_owned()), |t| {
                                Celda::Numero(formato_fecha_hora(t, false))
                            }),
                            Celda::Numero(formato_duracion(c.hasta.unwrap_or(h) - c.desde)),
                        ]
                    })
                    .collect()
            });
            view! { <Tabla columnas filas vacia="Sin cortes en este intervalo" /> }
        })
    };

    let etiqueta_aria = move || {
        let r = resumen.get();
        format!(
            "Disponibilidad de internet: {}, {} cortes, {} sin conexión",
            formato_disponibilidad(&r),
            r.cortes,
            formato_duracion(r.segundos_caido)
        )
    };

    view! {
        <figure class="franja">
            <figcaption class="franja-cabecera">{cabecera}</figcaption>
            <div class="franja-lienzo" node_ref=contenedor>
                <svg
                    node_ref=svg_ref
                    viewBox=move || format!("0 0 {:.0} {ALTO}", ancho.get())
                    role="img"
                    aria-label=etiqueta_aria
                    tabindex="0"
                    on:pointermove=move |ev| mover(ev.client_x())
                    on:pointerdown=move |ev| mover(ev.client_x())
                    on:pointerleave=move |ev| {
                        if ev.pointer_type() != "touch" {
                            set_foco.set(None);
                        }
                    }
                    on:keydown=teclado
                    on:blur=move |_| set_foco.set(None)
                >
                    {barra}
                    {interactivo}
                </svg>
            </div>
            <ul class="leyenda leyenda-franja">
                <li>
                    <span class="clave-bloque franja-clave-ok" aria-hidden="true"></span>
                    "Conectado"
                </li>
                <li>
                    <span class="clave-bloque franja-clave-corte" aria-hidden="true"></span>
                    "Sin conexión"
                </li>
            </ul>
            <details
                class="grafica-datos"
                node_ref=details_ref
                on:toggle=move |_| {
                    if let Some(d) = details_ref.get_untracked() {
                        set_tabla_abierta.set(d.open());
                    }
                }
            >
                <summary>"Ver los cortes en una tabla"</summary>
                {tabla}
            </details>
        </figure>
    }
    .into_any()
}

/// Tarjeta de indicador: título, valor destacado, subtítulo opcional, estado
/// opcional (icono junto al valor, que hace de etiqueta) y minigráfica
/// opcional.
#[component]
pub fn Indicador(
    #[prop(into)] titulo: String,
    #[prop(into)] valor: Signal<String>,
    #[prop(into, optional)] sub: MaybeProp<String>,
    #[prop(into, optional)] minigrafica: Option<Signal<Vec<Option<f64>>>>,
    #[prop(into, optional)] estado: MaybeProp<Estado>,
) -> impl IntoView {
    let icono = move || estado.get().map(|estado| view! { <IconoEstado estado /> });
    let clase = move || match estado.get() {
        Some(e) => format!("indicador {}", e.clase()),
        None => "indicador".to_owned(),
    };
    view! {
        <article class=clase>
            <h3 class="indicador-titulo">{titulo}</h3>
            <p class="indicador-valor">{icono} <span>{valor}</span></p>
            {move || sub.get().map(|s| view! { <p class="indicador-sub">{s}</p> })}
            {minigrafica.map(|valores| view! { <Minigrafica valores /> })}
        </article>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-08 00:00:00 UTC.
    const D2026_10_08: i64 = 1_791_417_600;

    #[test]
    fn escala_lineal() {
        let s = escala((0.0, 100.0), (200.0, 0.0));
        assert_eq!(s(0.0), 200.0);
        assert_eq!(s(100.0), 0.0);
        assert_eq!(s(25.0), 150.0);
        // Dominio degenerado: no divide por cero.
        let d = escala((5.0, 5.0), (0.0, 10.0));
        assert_eq!(d(5.0), 5.0);
    }

    #[test]
    fn ticks_bonitos_0_a_95() {
        assert_eq!(
            ticks_bonitos(0.0, 95.0, 5),
            vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0]
        );
    }

    #[test]
    fn ticks_bonitos_decimales_exactos_y_bordes() {
        assert_eq!(
            ticks_bonitos(0.0, 0.9, 5),
            vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]
        );
        assert_eq!(ticks_bonitos(3.0, 3.0, 5), vec![3.0]);
        assert!(ticks_bonitos(0.0, f64::NAN, 5).is_empty());
    }

    #[test]
    fn formato_bps_unidades() {
        assert_eq!(formato_bps(1_500.0), "1,5 kbit/s");
        assert_eq!(formato_bps(12_345_678.0), "12,3 Mbit/s");
        assert_eq!(formato_bps(0.0), "0 bit/s");
        assert_eq!(formato_bps(850.0), "850 bit/s");
        assert_eq!(formato_bps(230_400_000.0), "230 Mbit/s");
        assert_eq!(formato_bps(999_960.0), "1,0 Mbit/s");
        assert_eq!(formato_bps(1.2e9), "1,2 Gbit/s");
        assert_eq!(formato_bps(f64::NAN), "—");
    }

    #[test]
    fn formato_decimal_con_coma_y_sin_menos_cero() {
        assert_eq!(formato_decimal(0.4249, 2), "0,42");
        assert_eq!(formato_decimal(-0.001, 1), "0,0");
        assert_eq!(formato_corto(2.5), "2,5");
        assert_eq!(formato_corto(20.0), "20");
        assert_eq!(formato_valor(Unidad::Porcentaje, 61.26), "61,3 %");
        assert_eq!(formato_valor(Unidad::Entero, 13.6), "14");
    }

    #[test]
    fn eje_y_desde_cero_con_unidad() {
        let e = eje_y(Unidad::Bps, 47e6);
        assert_eq!(e.max, 50e6);
        assert_eq!(e.unidad, Some("Mbit/s"));
        assert_eq!(e.ticks.first().unwrap(), &(0.0, "0".to_owned()));
        assert_eq!(e.ticks.last().unwrap().1, "50");
        // Porcentaje: siempre 0–100.
        assert_eq!(eje_y(Unidad::Porcentaje, 12.0).max, 100.0);
        // Sin datos: eje válido, nunca degenerado.
        assert!(eje_y(Unidad::Decimal, f64::NAN).max > 0.0);
        // Cantidades: ticks enteros.
        let c = eje_y(Unidad::Entero, 7.0);
        assert!(
            c.ticks.iter().all(|(v, _)| v.fract() == 0.0),
            "{:?}",
            c.ticks
        );
    }

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

    #[test]
    fn huecos_cortan_la_linea() {
        // Paso de 10 s: un None y un salto de 60 s cortan; uno de 20 s no.
        let puntos = vec![
            (0, Some(1.0)),
            (10, Some(2.0)),
            (20, None),
            (30, Some(3.0)),
            (40, Some(4.0)),
            (60, Some(5.0)),
            (120, Some(6.0)),
            (130, Some(7.0)),
        ];
        assert_eq!(
            segmentos(&puntos),
            vec![
                vec![(0, 1.0), (10, 2.0)],
                vec![(30, 3.0), (40, 4.0), (60, 5.0)],
                vec![(120, 6.0), (130, 7.0)],
            ]
        );
        let serie = Serie::new("Bajada", Tono::Primario, puntos).con_area();
        let g = geometria(&[serie], Unidad::Bps, 600.0, 240.0, None);
        assert_eq!(g.trazos[0].lineas.len(), 3);
        assert_eq!(g.trazos[0].areas.len(), 3);
        assert!(g.trazos[0].lineas.iter().all(|d| d.starts_with('M')));
    }

    #[test]
    fn punto_aislado_es_marcador() {
        let puntos = vec![
            (0, Some(1.0)),
            (10, None),
            (20, Some(2.0)),
            (30, None),
            (40, Some(1.0)),
            (50, Some(1.0)),
        ];
        let serie = Serie::new("x", Tono::Primario, puntos);
        let g = geometria(&[serie], Unidad::Decimal, 400.0, 200.0, None);
        assert_eq!(g.trazos[0].sueltos.len(), 2);
        assert_eq!(g.trazos[0].lineas.len(), 1);
    }

    #[test]
    fn geometria_desde_cero_y_dentro_del_lienzo() {
        let puntos: Vec<(i64, Option<f64>)> = (0..60)
            .map(|i| (i * 10, Some(10e6 + i as f64 * 1e5)))
            .collect();
        let serie = Serie::new("Bajada", Tono::Primario, puntos);
        let g = geometria(&[serie], Unidad::Bps, 360.0, 200.0, None);
        // El primer tick (0) cae en la base del gráfico.
        assert_eq!(g.ticks_y[0].texto, "0");
        assert_eq!(g.ticks_y[0].pos, g.abajo);
        assert_eq!(g.unidad_eje, Some("Mbit/s"));
        assert!(g.izq > 0.0 && g.der < 360.0 && g.der > g.izq);
        // Ticks X sin amontonarse.
        assert!(g.ticks_x.len() >= 2);
        for w in g.ticks_x.windows(2) {
            assert!(
                w[1].pos - w[0].pos >= SEPARACION_TICKS_X - 1.0,
                "{:?}",
                g.ticks_x
            );
        }
        let (x, _, texto) = g.trazos[0].etiqueta.clone().unwrap();
        assert!(x <= g.der);
        assert_eq!(texto, "15,9 Mbit/s");
    }

    #[test]
    fn etiquetas_directas_que_chocan_se_quitan() {
        let a: Vec<(i64, Option<f64>)> = (0..10).map(|i| (i * 10, Some(5.0))).collect();
        let b: Vec<(i64, Option<f64>)> = (0..10).map(|i| (i * 10, Some(5.1))).collect();
        let series = [
            Serie::new("a", Tono::Primario, a),
            Serie::new("b", Tono::Secundario, b),
        ];
        let g = geometria(&series, Unidad::Decimal, 400.0, 200.0, None);
        assert!(g.trazos.iter().all(|t| t.etiqueta.is_none()));
    }

    #[test]
    fn lectura_de_todas_las_series_en_la_marca() {
        let a = Serie::new(
            "Bajada",
            Tono::Primario,
            vec![(0, Some(2e6)), (10, Some(3e6))],
        );
        let b = Serie::new("Subida", Tono::Secundario, vec![(0, Some(1e5)), (10, None)]);
        let series = [a, b];
        let g = geometria(&series, Unidad::Bps, 400.0, 200.0, None);
        let l = lectura(&series, Unidad::Bps, &g, 1).unwrap();
        assert_eq!(l.filas[0].1, "3,0 Mbit/s");
        assert_eq!(l.filas[1].1, "sin datos");
        assert_eq!(l.puntos.len(), 1);
        assert!(lectura(&series, Unidad::Bps, &g, 9).is_none());
    }

    #[test]
    fn indice_mas_cercano_por_busqueda_binaria() {
        let ts = [0, 10, 20, 30];
        assert_eq!(indice_mas_cercano(&ts, -5.0), Some(0));
        assert_eq!(indice_mas_cercano(&ts, 14.0), Some(1));
        assert_eq!(indice_mas_cercano(&ts, 16.0), Some(2));
        assert_eq!(indice_mas_cercano(&ts, 99.0), Some(3));
        assert_eq!(indice_mas_cercano(&[], 1.0), None);
    }

    #[test]
    fn resumen_de_cortes_recortado_al_intervalo() {
        let eventos = vec![
            // Empieza antes del intervalo: cuenta desde 1000.
            Corte {
                desde: 900,
                hasta: Some(1100),
            },
            Corte {
                desde: 1500,
                hasta: Some(1600),
            },
            // Sigue abierto: hasta el final.
            Corte {
                desde: 1900,
                hasta: None,
            },
            // Fuera del intervalo.
            Corte {
                desde: 100,
                hasta: Some(200),
            },
        ];
        let r = resumen_cortes(&eventos, 1000, 2000);
        assert_eq!(r.cortes, 3);
        assert_eq!(r.segundos_caido, 300);
        assert_eq!(r.disponibilidad, 70.0);
        assert_eq!(formato_disponibilidad(&r), "70,00 %");
        let casi = resumen_cortes(
            &[Corte {
                desde: 0,
                hasta: Some(1),
            }],
            0,
            100_000,
        );
        assert_eq!(formato_disponibilidad(&casi), "99,99 %");
    }

    #[test]
    fn minigrafica_con_hueco() {
        let t = geometria_mini(
            &[Some(1.0), Some(2.0), None, Some(1.0), Some(3.0)],
            120.0,
            32.0,
        );
        assert_eq!(t.lineas.len(), 2);
        assert_eq!(t.fin, Some((120.0, 3.0)));
    }
}
