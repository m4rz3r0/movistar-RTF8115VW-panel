//! Escalas, ticks redondos, formato de números (coma decimal) y unidades.

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
