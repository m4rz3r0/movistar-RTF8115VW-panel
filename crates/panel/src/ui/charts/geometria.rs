//! Huecos, trazados y geometría del gráfico temporal (sin navegador).

use super::{
    escalas::{Unidad, eje_y, escala, formato_valor},
    tiempo::{formato_fecha_hora, ticks_tiempo},
};
use crate::t;

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
pub(super) fn ruta_linea(px: &[(f64, f64)]) -> String {
    let mut d = String::with_capacity(px.len() * 14);
    for (i, (x, y)) in px.iter().enumerate() {
        d.push(if i == 0 { 'M' } else { 'L' });
        d.push_str(&format!("{x:.1},{y:.1}"));
    }
    d
}

/// Trazado SVG del área bajo una polilínea, cerrada sobre `y0`.
pub(super) fn ruta_area(px: &[(f64, f64)], y0: f64) -> String {
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
pub(super) fn valor_en(puntos: &[(i64, Option<f64>)], t: i64) -> Option<f64> {
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
    pub(super) fn clase(self) -> &'static str {
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

    pub(super) fn ultimo(&self) -> Option<(i64, f64)> {
        self.puntos
            .iter()
            .rev()
            .find_map(|&(t, v)| v.filter(|v| v.is_finite()).map(|v| (t, v)))
    }

    pub(super) fn maximo(&self) -> Option<f64> {
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
pub(super) const ANCHO_CARACTER: f64 = 7.0;
/// Anchura mínima (px) de un tramo para rellenar su área.
const ANCHO_MIN_AREA: f64 = 4.0;
/// Separación mínima entre ticks de tiempo.
pub(super) const SEPARACION_TICKS_X: f64 = 84.0;

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
    pub(super) fn vacio(clase: &'static str) -> Self {
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
    pub(super) fn x(&self, t: i64) -> f64 {
        escala((self.t0 as f64, self.t1 as f64), (self.izq, self.der))(t as f64)
    }

    pub(super) fn y(&self, v: f64) -> f64 {
        escala((0.0, self.y_max), (self.abajo, self.arriba))(v)
    }

    pub(super) fn con_segundos(&self) -> bool {
        self.paso.is_some_and(|p| p < 60)
    }
}

/// Ancla del texto de un tick centrado en `pos` para que no se salga de
/// `[0, ancho]`.
pub(super) fn ancla_dentro(pos: f64, texto: &str, ancho: f64) -> &'static str {
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
    // Con ventana, solo cuentan (y se dibujan) los puntos de dentro: nada se
    // pinta fuera del área del gráfico ni la cruceta engancha marcas de fuera.
    let recortadas: Vec<Serie>;
    let series = match ventana {
        Some((a, b)) => {
            recortadas = series
                .iter()
                .map(|s| Serie {
                    puntos: s
                        .puntos
                        .iter()
                        .filter(|p| (a..=b).contains(&p.0))
                        .copied()
                        .collect(),
                    ..s.clone()
                })
                .collect();
            &recortadas[..]
        }
        None => series,
    };
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
                // Un tramo muy estrecho (unas pocas muestras entre cortes) con
                // área parecería una barra a toda altura: solo la línea.
                let ancho_tramo = px[px.len() - 1].0 - px[0].0;
                if s.area && ancho_tramo >= ANCHO_MIN_AREA {
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
pub(super) struct Lectura {
    pub(super) x: f64,
    pub(super) cabecera: String,
    pub(super) puntos: Vec<(&'static str, f64)>,
    pub(super) filas: Vec<(&'static str, String, String)>,
}

pub(super) fn lectura(
    series: &[Serie],
    unidad: Unidad,
    geo: &Geometria,
    idx: usize,
) -> Option<Lectura> {
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
            v.map_or_else(|| t!("comun-sin-datos-min"), |v| formato_valor(unidad, v)),
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
pub(super) fn resumen(titulo: &str, series: &[Serie], unidad: Unidad) -> String {
    let partes: Vec<String> = series
        .iter()
        .map(|s| match (s.ultimo(), s.maximo()) {
            (Some((_, u)), Some(m)) => t!(
                "comun-serie-resumen",
                serie = s.nombre.as_str(),
                ultima = formato_valor(unidad, u),
                maxima = formato_valor(unidad, m)
            ),
            _ => t!("comun-serie-sin-datos", serie = s.nombre.as_str()),
        })
        .collect();
    format!("{titulo}. {}", partes.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn la_ventana_recorta_puntos_y_marcas() {
        let puntos: Vec<(i64, Option<f64>)> = (0..=10).map(|i| (i * 10, Some(i as f64))).collect();
        let serie = Serie::new("x", Tono::Primario, puntos);
        let g = geometria(
            std::slice::from_ref(&serie),
            Unidad::Decimal,
            400.0,
            200.0,
            Some((30, 60)),
        );
        // Solo se puede enganchar dentro de la ventana.
        assert_eq!(g.tiempos, [30, 40, 50, 60]);
        let fuera = indice_mas_cercano(&g.tiempos, 100.0).map(|i| g.tiempos[i]);
        assert_eq!(fuera, Some(60));
        // Nada se dibuja fuera del área del gráfico.
        let xs: Vec<f64> = g.trazos[0]
            .lineas
            .iter()
            .flat_map(|d| d.split(['M', 'L']).filter(|p| !p.is_empty()))
            .map(|p| p.split(',').next().unwrap().parse::<f64>().unwrap())
            .collect();
        assert!(!xs.is_empty());
        assert!(xs.iter().all(|x| (g.izq..=g.der).contains(x)), "{xs:?}");
        // El eje Y y la etiqueta salen de los datos de la ventana.
        assert_eq!(g.y_max, 6.0);
        assert_eq!(g.trazos[0].etiqueta.as_ref().unwrap().2, "6,00");
        let l = lectura(&[serie], Unidad::Decimal, &g, 3).unwrap();
        assert_eq!(l.filas[0].1, "6,00");
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
    fn sin_area_en_tramos_estrechos() {
        // Un tramo de 2 muestras tras un corte, en una hora de 10 s: ~2 px
        // de ancho. Su área sería una «barra» engañosa; solo va la línea.
        let mut puntos: Vec<(i64, Option<f64>)> = (0..300).map(|i| (i * 10, Some(5e6))).collect();
        puntos.extend([
            (3000, None),
            (3010, Some(5e6)),
            (3020, Some(5e6)),
            (3030, None),
        ]);
        puntos.extend((304..360).map(|i| (i * 10, Some(5e6))));
        let serie = Serie::new("Bajada", Tono::Primario, puntos).con_area();
        let g = geometria(&[serie], Unidad::Bps, 600.0, 200.0, None);
        let t = &g.trazos[0];
        assert_eq!(t.lineas.len(), 3);
        assert_eq!(t.areas.len(), 2);
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
}
