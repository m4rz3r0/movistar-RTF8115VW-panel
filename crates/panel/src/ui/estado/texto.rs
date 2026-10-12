//! Modo de visualización «texto» de la página de estado: ningún gráfico.
//!
//! Los indicadores van como una lista de pares nombre/valor (`<dl>`) y el
//! histórico como una tabla con el último valor, el mínimo, la media y el
//! máximo de cada serie en el intervalo elegido (1 h, 24 h, 7 d), con las
//! unidades y el formato de números del idioma ([`formato_valor`]). La
//! disponibilidad de internet, en una frase y la tabla de cortes. Todo es
//! texto y tablas con título: se lee igual con lector de pantalla.

use super::{CorteVista, DatoIndicador, EstadoVista, Rango, SampleVista, selector_rango};
use crate::t;
use crate::ui::charts::{
    Corte, Unidad, columnas_cortes, filas_cortes, formato_decimal, formato_valor, resumen_cortes,
    texto_disponibilidad,
};
use crate::ui::table::{Celda, Columna, Tabla};
use leptos::prelude::*;

/// Último valor, mínimo, media y máximo de una serie.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ResumenSerie {
    ultimo: f64,
    minimo: f64,
    media: f64,
    maximo: f64,
}

/// Resumen de `valores` sin los huecos (`None`) ni valores no finitos; `None`
/// si no queda ninguno.
fn resumen_serie(valores: impl IntoIterator<Item = Option<f64>>) -> Option<ResumenSerie> {
    let mut n = 0u32;
    let mut suma = 0.0;
    let mut r: Option<ResumenSerie> = None;
    for v in valores.into_iter().flatten().filter(|v| v.is_finite()) {
        n += 1;
        suma += v;
        r = Some(match r {
            None => ResumenSerie {
                ultimo: v,
                minimo: v,
                media: v,
                maximo: v,
            },
            Some(r) => ResumenSerie {
                ultimo: v,
                minimo: r.minimo.min(v),
                maximo: r.maximo.max(v),
                ..r
            },
        });
    }
    r.map(|r| ResumenSerie {
        media: suma / f64::from(n),
        ..r
    })
}

/// Una serie del histórico: nombre, unidad y valor de cada muestra.
type SerieTexto = (String, Unidad, fn(&SampleVista) -> Option<f64>);

/// Series del histórico (las mismas que los gráficos del modo «mixto»).
fn series() -> [SerieTexto; 5] {
    [
        (t!("estado-bajada"), Unidad::Bps, |s| s.rx_bps),
        (t!("estado-subida"), Unidad::Bps, |s| s.tx_bps),
        (t!("estado-carga-1min"), Unidad::Decimal, |s| {
            Some(f64::from(s.load1))
        }),
        (t!("estado-ram-usada"), Unidad::Porcentaje, |s| {
            Some(f64::from(s.ram_used_pct))
        }),
        (t!("estado-clientes"), Unidad::Entero, |s| {
            s.clients.map(f64::from)
        }),
    ]
}

/// Filas de la tabla del histórico: una por serie; sin muestras, ninguna.
fn filas_historico(muestras: &[SampleVista]) -> Vec<Vec<Celda>> {
    if muestras.is_empty() {
        return Vec::new();
    }
    series()
        .into_iter()
        .map(|(nombre, unidad, f)| {
            let mut fila = vec![Celda::Texto(nombre)];
            match resumen_serie(muestras.iter().map(f)) {
                Some(r) => {
                    let valor = |x| Celda::Numero(formato_valor(unidad, x));
                    // La media de una cantidad entera (clientes) con un
                    // decimal: «3,4», no «3».
                    let media = match unidad {
                        Unidad::Entero => Celda::Numero(formato_decimal(r.media, 1)),
                        _ => valor(r.media),
                    };
                    fila.extend([valor(r.ultimo), valor(r.minimo), media, valor(r.maximo)]);
                }
                None => fila.extend([Celda::Vacia, Celda::Vacia, Celda::Vacia, Celda::Vacia]),
            }
            fila
        })
        .collect()
}

/// Indicadores como pares nombre/valor (con su subtítulo).
#[component]
pub(super) fn IndicadoresTexto(datos: Vec<DatoIndicador>) -> impl IntoView {
    let pares = datos
        .into_iter()
        .map(|d| {
            move || {
                d.titulo.get().map(|titulo| {
                    let sub = move || {
                        d.sub
                            .get()
                            .map(|s| view! { <span class="par-sub">{s}</span> })
                    };
                    view! {
                        <div class="par">
                            <dt>{titulo}</dt>
                            <dd>
                                <span class="par-valor">{d.valor}</span>
                                {sub}
                            </dd>
                        </div>
                    }
                })
            }
        })
        .collect_view();
    view! {
        <dl class="indicadores-texto" aria-label=t!("estado-indicadores")>
            {pares}
        </dl>
    }
}

/// Histórico en texto: selector de intervalo, tabla de último, mínimo, media
/// y máximo, y disponibilidad de internet.
#[component]
pub(super) fn HistoricoTexto(
    vista: Memo<Option<EstadoVista>>,
    muestras: Memo<Option<(Rango, Vec<SampleVista>)>>,
    cortes: Memo<Vec<CorteVista>>,
    rango: ReadSignal<Rango>,
    set_rango: WriteSignal<Rango>,
) -> impl IntoView {
    // El intervalo de los datos que se están mostrando (mientras llegan los
    // del nuevo, se queda el anterior), como en los gráficos.
    let rango_datos = move || muestras.with(|m| m.as_ref().map_or(rango.get(), |m| m.0));
    let hasta = move || vista.with(|v| v.as_ref().map(|v| v.ahora).unwrap_or_default());
    let desde = move || hasta() - rango_datos().segundos();
    // Con 24 h y 7 d el servidor da medias de 1 y de 15 minutos: el título
    // lo dice (el mínimo y el máximo son los de esas medias, no picos).
    let titulo = Signal::derive(move || {
        let r = rango_datos();
        let rango = r.descripcion();
        match r {
            Rango::Hora => t!("estado-texto-historico", rango = rango),
            Rango::Dia => t!("estado-texto-historico-1min", rango = rango),
            Rango::Semana => t!("estado-texto-historico-15min", rango = rango),
        }
    });
    let columnas = vec![
        Columna::new(t!("estado-texto-col-dato")),
        Columna::numerica(t!("estado-texto-col-ultimo")),
        Columna::numerica(t!("estado-texto-col-minimo")),
        Columna::numerica(t!("estado-texto-col-media")),
        Columna::numerica(t!("estado-texto-col-maximo")),
    ];
    let filas = Signal::derive(move || {
        muestras
            .with(|m| filas_historico(m.as_ref().map(|(_, m)| m.as_slice()).unwrap_or_default()))
    });

    let eventos = move || {
        cortes.with(|c| {
            c.iter()
                .map(|c| Corte {
                    desde: c.desde,
                    hasta: c.hasta,
                })
                .collect::<Vec<_>>()
        })
    };
    let titulo_franja = move || t!("estado-disponibilidad", rango = rango_datos().descripcion());
    let disponibilidad =
        move || texto_disponibilidad(&resumen_cortes(&eventos(), desde(), hasta()));
    let filas_cortes = Signal::derive(move || filas_cortes(&eventos(), desde(), hasta()));

    view! {
        <section class="seccion graficas historico-texto" aria-labelledby="graficas-titulo">
            <div class="seccion-cabecera">
                <h2 id="graficas-titulo">{t!("estado-historico")}</h2>
                {selector_rango(rango, set_rango)}
            </div>
            <div class="tarjeta">
                <Tabla
                    titulo
                    columnas
                    filas
                    vacia=t!("estado-texto-sin-muestras")
                    cabecera_fila=true
                />
            </div>
            <section class="tarjeta" aria-labelledby="disponibilidad-titulo">
                <h3 id="disponibilidad-titulo" class="franja-titulo">
                    {titulo_franja}
                </h3>
                <p>{disponibilidad}</p>
                <Tabla
                    columnas=columnas_cortes()
                    filas=filas_cortes
                    vacia=t!("comun-sin-cortes-intervalo")
                />
            </section>
        </section>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resumen_sin_huecos() {
        let r = resumen_serie([Some(2.0), None, Some(6.0), Some(f64::NAN), Some(1.0)]).unwrap();
        assert_eq!(
            r,
            ResumenSerie {
                ultimo: 1.0,
                minimo: 1.0,
                media: 3.0,
                maximo: 6.0,
            }
        );
        assert_eq!(resumen_serie([None, None]), None);
        assert_eq!(resumen_serie([]), None);
    }

    fn muestra(ts: i64, rx: Option<f64>) -> SampleVista {
        SampleVista {
            ts,
            rx_bps: rx,
            tx_bps: None,
            load1: 0.5,
            ram_used_pct: 40.0,
            clients: Some(3),
            wan_up: true,
        }
    }

    #[test]
    fn una_fila_por_serie_con_unidades() {
        assert!(filas_historico(&[]).is_empty());
        let filas = filas_historico(&[
            muestra(0, Some(1_000_000.0)),
            muestra(60, Some(3_000_000.0)),
        ]);
        assert_eq!(filas.len(), 5);
        assert_eq!(filas[0][0], Celda::Texto(t!("estado-bajada")));
        assert_eq!(filas[0][1], Celda::Numero(formato_valor(Unidad::Bps, 3e6)));
        assert_eq!(filas[0][2], Celda::Numero(formato_valor(Unidad::Bps, 1e6)));
        assert_eq!(filas[0][3], Celda::Numero(formato_valor(Unidad::Bps, 2e6)));
        // La subida no tiene ningún valor: «—» en las cuatro columnas.
        assert_eq!(
            filas[1][1..],
            [Celda::Vacia, Celda::Vacia, Celda::Vacia, Celda::Vacia]
        );
        assert_eq!(filas[3][1], Celda::Numero("40,0 %".into()));
        assert_eq!(filas[4][4], Celda::Numero("3".into()));
        // La media de los clientes, con un decimal.
        let mut otra = muestra(120, None);
        otra.clients = Some(4);
        let filas = filas_historico(&[muestra(0, None), muestra(60, None), otra]);
        assert_eq!(filas[4][3], Celda::Numero("3,3".into()));
        assert_eq!(filas[4][2], Celda::Numero("3".into()));
    }
}
