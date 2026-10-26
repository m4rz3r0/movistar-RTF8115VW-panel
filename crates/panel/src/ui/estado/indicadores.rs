//! Indicadores de la página de estado: datos y tarjetas.

use super::*;

/// Un indicador de la página: título (sin él no se pinta), valor,
/// subtítulo, estado y, si la tiene, minigráfica de la última hora con su
/// color. Lo pintan [`Indicadores`] (tarjetas) y
/// [`texto::IndicadoresTexto`] (pares nombre/valor).
#[derive(Clone, Copy)]
pub(super) struct DatoIndicador {
    pub(super) titulo: Signal<Option<String>>,
    pub(super) valor: Signal<String>,
    pub(super) sub: Signal<Option<String>>,
    estado: Signal<Option<Estado>>,
    mini: Option<(Signal<Vec<Option<f64>>>, Tono)>,
}

impl DatoIndicador {
    fn new(titulo: String, valor: Signal<String>, sub: Signal<Option<String>>) -> Self {
        DatoIndicador {
            titulo: Signal::stored(Some(titulo)),
            valor,
            sub,
            estado: Signal::stored(None),
            mini: None,
        }
    }

    fn con_mini(self, valores: Signal<Vec<Option<f64>>>, tono: Tono) -> Self {
        DatoIndicador {
            mini: Some((valores, tono)),
            ..self
        }
    }
}

/// Los indicadores de la página, de los datos del estado y de la última
/// hora.
pub(super) fn indicadores(
    vista: Memo<Option<EstadoVista>>,
    ultima_hora: Memo<Vec<SampleVista>>,
) -> Vec<DatoIndicador> {
    let campo = move |f: fn(&EstadoVista) -> String| {
        Signal::derive(move || vista.with(|v| v.as_ref().map(f).unwrap_or_default()))
    };
    let mini = move |f: fn(&SampleVista) -> Option<f64>| {
        Signal::derive(move || ultima_hora.with(|m| m.iter().map(f).collect::<Vec<_>>()))
    };

    // Sin conexión con el router no se sabe si hay internet: se dice lo
    // último que se vio y cuándo.
    let internet_estado = Signal::derive(move || {
        vista.with(|v| {
            v.as_ref().map(|v| match (v.router_ok, v.internet_up) {
                (false, _) => Estado::Neutro,
                (true, true) => Estado::Ok,
                (true, false) => Estado::Error,
            })
        })
    });
    let internet_valor = campo(|v| match (v.router_ok, v.internet_up) {
        (false, _) => t!("estado-desconocido"),
        (true, true) => t!("comun-conectado"),
        (true, false) => t!("comun-sin-conexion"),
    });
    let internet_sub = Signal::derive(move || {
        vista.with(|v| {
            let v = v.as_ref()?;
            if !v.router_ok {
                let antes = if v.internet_up {
                    t!("comun-conectado-min")
                } else {
                    t!("comun-sin-conexion-min")
                };
                return Some(t!(
                    "estado-ultimo-dato",
                    valor = antes,
                    hora = formato_hora(v.at?, false)
                ));
            }
            Some(match &v.ip_publica {
                Some(ip) => t!("estado-ip-publica", ip = ip.as_str()),
                None => t!("estado-sin-ip-publica"),
            })
        })
    });

    // Tiempo de conexión: desde el último corte visto; si no se vio ninguno
    // desde que arrancó el router, su tiempo encendido.
    let conexion = Memo::new(move |_| {
        vista.with(|v| {
            let v = v.as_ref()?;
            let (titulo, desde, estado) = match (v.internet_up, v.conexion_desde, v.caida_desde) {
                (false, _, desde) => (t!("estado-sin-conexion-desde"), desde, Some(Estado::Error)),
                (true, Some(d), _) => (t!("estado-tiempo-conexion"), Some(d), None),
                (true, None, _) => (t!("estado-router-encendido"), v.encendido_desde, None),
            };
            // Sin conexión con el router, hasta el último dato: no sigue
            // creciendo como si se estuviera midiendo.
            let valor =
                desde.map_or_else(|| "—".to_owned(), |d| formato_duracion(v.medido_hasta - d));
            let sub = desde.map(|d| {
                let desde = formato_fecha_hora(d, false);
                if v.router_ok {
                    t!("estado-desde", desde = desde)
                } else {
                    t!(
                        "estado-desde-hasta-ultimo",
                        desde = desde,
                        hora = formato_hora(v.medido_hasta, false)
                    )
                }
            });
            Some((titulo, valor, sub, estado))
        })
    });
    let titulo_conexion = Memo::new(move |_| conexion.with(|c| c.as_ref().map(|c| c.0.clone())));
    let indicador_conexion = DatoIndicador {
        titulo: titulo_conexion.into(),
        valor: Signal::derive(move || {
            conexion.with(|c| c.as_ref().map(|c| c.1.clone()).unwrap_or_default())
        }),
        sub: Signal::derive(move || conexion.with(|c| c.as_ref().and_then(|c| c.2.clone()))),
        estado: Signal::derive(move || conexion.with(|c| c.as_ref().and_then(|c| c.3))),
        mini: None,
    };

    // Tasas: la de la última muestra vigente (el servidor la descarta si es
    // vieja). Sin conexión con el router no se presentan como actuales.
    let tasa = move |f: fn(&EstadoVista) -> Option<f64>| {
        Signal::derive(move || {
            vista.with(|v| match v.as_ref() {
                Some(v) if v.router_ok => f(v).map_or_else(|| "—".to_owned(), formato_bps),
                _ => "—".to_owned(),
            })
        })
    };
    let tasa_sub = move |f: fn(&EstadoVista) -> Option<f64>, g: fn(&SampleVista) -> Option<f64>| {
        Signal::derive(move || {
            let caido = vista.with(|v| {
                v.as_ref().filter(|v| !v.router_ok).map(|v| {
                    let valor = f(v).map_or_else(|| t!("estado-sin-dato"), formato_bps);
                    match v.at {
                        Some(at) => t!(
                            "estado-ultimo-dato",
                            valor = valor,
                            hora = formato_hora(at, false)
                        ),
                        None => t!("estado-sin-datos-router"),
                    }
                })
            });
            caido.or_else(|| {
                ultima_hora.with(|m| {
                    m.iter()
                        .filter_map(g)
                        .reduce(f64::max)
                        .map(|x| t!("estado-max-1h", valor = formato_bps(x)))
                })
            })
        })
    };

    let carga_sub = campo(|v| {
        let (a, b, c) = v.load;
        format!(
            "1 · 5 · 15 min: {} · {} · {}",
            formato_decimal(f64::from(a), 2),
            formato_decimal(f64::from(b), 2),
            formato_decimal(f64::from(c), 2)
        )
    });
    let ram_sub = campo(|v| {
        t!(
            "estado-ram-de",
            total = formato_decimal(v.ram_total_kb as f64 / 1024.0, 0)
        )
    });
    let clientes_sub = campo(|v| {
        let cable = v.clientes.saturating_sub(v.clientes_wifi);
        t!("estado-clientes-sub", wifi = v.clientes_wifi, cable = cable)
    });

    let con_algo = |s: Signal<String>| Signal::derive(move || Some(s.get()));
    vec![
        DatoIndicador {
            estado: internet_estado,
            ..DatoIndicador::new(t!("estado-internet"), internet_valor, internet_sub)
        },
        indicador_conexion,
        DatoIndicador::new(
            t!("estado-bajada"),
            tasa(|v| v.bajada_bps),
            tasa_sub(|v| v.bajada_bps, |s| s.rx_bps),
        )
        .con_mini(mini(|s| s.rx_bps), Tono::Primario),
        DatoIndicador::new(
            t!("estado-subida"),
            tasa(|v| v.subida_bps),
            tasa_sub(|v| v.subida_bps, |s| s.tx_bps),
        )
        .con_mini(mini(|s| s.tx_bps), Tono::Secundario),
        DatoIndicador::new(
            t!("estado-carga"),
            campo(|v| formato_decimal(f64::from(v.load.0), 2)),
            con_algo(carga_sub),
        )
        .con_mini(mini(|s| Some(f64::from(s.load1))), Tono::default()),
        DatoIndicador::new(
            t!("estado-ram-usada"),
            campo(|v| format!("{} %", formato_decimal(f64::from(v.ram_pct), 1))),
            con_algo(ram_sub),
        )
        .con_mini(mini(|s| Some(f64::from(s.ram_used_pct))), Tono::default()),
        DatoIndicador::new(
            t!("estado-clientes"),
            campo(|v| v.clientes.to_string()),
            con_algo(clientes_sub),
        )
        .con_mini(mini(|s| s.clients.map(f64::from)), Tono::default()),
    ]
}

/// Fila de indicadores en tarjetas: con minigráfica y subtítulo (modo
/// «mixto») o, `compactos`, más pequeñas y sin minigráfica (modo «gráfica»).
#[component]
pub(super) fn Indicadores(
    datos: Vec<DatoIndicador>,
    #[prop(optional)] compactos: bool,
) -> impl IntoView {
    let tarjetas = datos
        .into_iter()
        .map(|d| {
            move || {
                d.titulo.get().map(|titulo| {
                    let DatoIndicador {
                        valor, estado, sub, ..
                    } = d;
                    match d.mini.filter(|_| !compactos) {
                        Some((minigrafica, tono)) => view! {
                            <Indicador titulo valor sub estado minigrafica tono />
                        }
                        .into_any(),
                        None => view! { <Indicador titulo valor sub estado /> }.into_any(),
                    }
                })
            }
        })
        .collect_view();
    let clase = if compactos {
        "rejilla-indicadores compactos"
    } else {
        "rejilla-indicadores"
    };
    view! {
        // Con su título (oculto): los de los indicadores son h3 (axe,
        // heading-order).
        <section class=clase aria-labelledby="indicadores-titulo">
            <h2 id="indicadores-titulo" class="solo-lector">{t!("estado-indicadores")}</h2>
            {tarjetas}
        </section>
    }
    .into_any()
}
