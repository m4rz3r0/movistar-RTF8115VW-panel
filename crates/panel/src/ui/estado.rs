//! Página de estado (`/`): avisos, indicadores con minigráfica, gráficos con
//! selector de intervalo (1 h / 24 h / 7 d), disponibilidad de internet y
//! tablas de clientes, concesiones DHCP y redes WiFi.
//!
//! Se refresca cada 10 s en el navegador (no mientras la pestaña está oculta)
//! sin rehacer la página: los componentes leen señales derivadas de los
//! recursos, que conservan el dato anterior mientras llega el nuevo, así que
//! no se mueve nada ni se pierde la lectura de un gráfico al refrescar.

mod datos;

pub use datos::*;

use crate::ui::{
    Estado, IconoEstado, Insignia,
    charts::{
        Corte, FranjaDisponibilidad, Indicador, Serie, SerieTemporal, Tono, Unidad, formato_bps,
        formato_decimal, formato_duracion, formato_fecha_hora, formato_hora,
    },
    table::{Celda, Columna, Tabla},
};
use leptos::prelude::*;

/// Cada cuánto se piden datos nuevos.
const REFRESCO: std::time::Duration = std::time::Duration::from_secs(10);

/// Resultado de las series junto con el intervalo pedido.
type Historico = (Rango, Result<Vec<SampleVista>, ServerFnError>);

impl Nivel {
    fn estado(self) -> Estado {
        match self {
            Nivel::Info => Estado::Neutro,
            Nivel::Aviso => Estado::Aviso,
            Nivel::Error => Estado::Error,
        }
    }
}

/// Texto del error de una server function para la interfaz.
fn texto_error(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(msg) => msg.clone(),
        _ => "No se puede contactar con el panel".to_owned(),
    }
}

/// Incrementa `tick` cada [`REFRESCO`] mientras la pestaña está visible, y
/// al volver a ella.
fn refresco_periodico(set_tick: WriteSignal<u64>) {
    Effect::new(move |_| {
        let visible = || !document().hidden();
        let avanzar = move || {
            if visible() {
                set_tick.update(|t| *t += 1);
            }
        };
        let intervalo = set_interval_with_handle(avanzar, REFRESCO).ok();
        let vuelta = window_event_listener_untyped("visibilitychange", move |_| avanzar());
        on_cleanup(move || {
            if let Some(h) = intervalo {
                h.clear();
            }
            vuelta.remove();
        });
    });
}

/// Página de estado.
#[component]
pub fn PaginaEstado() -> impl IntoView {
    let (tick, set_tick) = signal(0u64);
    let (rango, set_rango) = signal(Rango::Hora);
    let estado = Resource::new(move || tick.get(), |_| estado_actual());
    let historico = Resource::new(
        move || (rango.get(), tick.get()),
        |(r, _)| async move { (r, series(r).await) },
    );
    let cortes_r = Resource::new(move || (rango.get(), tick.get()), |(r, _)| cortes(r));
    // Las minigráficas son siempre de la última hora.
    let hora = Resource::new(move || tick.get(), |_| series(Rango::Hora));
    refresco_periodico(set_tick);

    view! {
        <Transition fallback=|| {
            view! {
                <header class="pagina-cabecera">
                    <h1>"Estado"</h1>
                </header>
                <p class="estado-espera">"Cargando…"</p>
            }
        }>
            <Contenido estado historico cortes_r hora rango set_rango />
        </Transition>
    }
    .into_any()
}

/// Contenido de la página. Se crea una vez (dentro de `Transition`, para que
/// el servidor espere a los datos) y se actualiza por señales.
#[component]
fn Contenido(
    estado: Resource<Result<EstadoVista, ServerFnError>>,
    historico: Resource<Historico>,
    cortes_r: Resource<Result<Vec<CorteVista>, ServerFnError>>,
    hora: Resource<Result<Vec<SampleVista>, ServerFnError>>,
    rango: ReadSignal<Rango>,
    set_rango: WriteSignal<Rango>,
) -> impl IntoView {
    // Último dato bueno de cada recurso: si una petición falla, se sigue
    // mostrando el anterior (con el error aparte).
    let vista = Memo::new(
        move |prev: Option<&Option<EstadoVista>>| match estado.get() {
            Some(Ok(v)) => Some(v),
            _ => prev.cloned().flatten(),
        },
    );
    let muestras = Memo::new(
        move |prev: Option<&Option<(Rango, Vec<SampleVista>)>>| match historico.get() {
            Some((r, Ok(m))) => Some((r, m)),
            _ => prev.cloned().flatten(),
        },
    );
    let cortes = Memo::new(move |prev: Option<&Vec<CorteVista>>| match cortes_r.get() {
        Some(Ok(c)) => c,
        _ => prev.cloned().unwrap_or_default(),
    });
    let ultima_hora = Memo::new(move |prev: Option<&Vec<SampleVista>>| match hora.get() {
        Some(Ok(m)) => m,
        _ => prev.cloned().unwrap_or_default(),
    });
    let error_panel = Memo::new(move |_| {
        [
            estado.get().and_then(Result::err),
            historico.get().and_then(|(_, r)| r.err()),
            cortes_r.get().and_then(Result::err),
            hora.get().and_then(Result::err),
        ]
        .into_iter()
        .flatten()
        .next()
        .map(|e| texto_error(&e))
    });

    let cabecera = cabecera(vista, error_panel);
    let resumen = move || {
        let v = vista.get()?;
        Some(if v.at.is_none() {
            sin_datos(&v).into_any()
        } else {
            view! {
                <Avisos vista />
                <Indicadores vista ultima_hora />
            }
            .into_any()
        })
    };
    let tablas = move || {
        vista
            .with(|v| v.as_ref().is_some_and(|v| v.at.is_some()))
            .then(|| view! { <Tablas vista /> })
    };

    view! {
        {cabecera}
        {resumen}
        <Graficas vista muestras cortes rango set_rango />
        {tablas}
    }
    .into_any()
}

/// Título, modelo y hora del dato; error del panel si lo hay.
fn cabecera(vista: Memo<Option<EstadoVista>>, error_panel: Memo<Option<String>>) -> impl IntoView {
    let sub = move || {
        vista.with(|v| {
            let v = v.as_ref()?;
            let at = v.at?;
            Some(format!(
                "{} · firmware {} · datos de las {}",
                v.modelo,
                v.firmware,
                formato_hora(at, true)
            ))
        })
    };
    let error = move || {
        error_panel.get().map(|e| {
            let sesion = e.contains("Sesión");
            view! {
                <p class="aviso estado-error" role="alert">
                    <IconoEstado estado=Estado::Error />
                    <span>
                        "No se pueden actualizar los datos: " {e}
                        {sesion
                            .then(|| {
                                view! {
                                    " "
                                    <a href="/login?next=%2F">"Volver a entrar"</a>
                                }
                            })}
                    </span>
                </p>
            }
        })
    };
    view! {
        <header class="pagina-cabecera">
            <div>
                <h1>"Estado"</h1>
                <p class="pagina-sub">{sub}</p>
            </div>
        </header>
        {error}
    }
}

/// Tarjeta cuando aún no hay ningún dato del router.
fn sin_datos(v: &EstadoVista) -> impl IntoView {
    match &v.error {
        Some(e) => view! {
            <section class="tarjeta estado-espera estado-error" role="status">
                <h2>
                    <IconoEstado estado=Estado::Error />
                    "Sin conexión con el router"
                </h2>
                <p>"El panel aún no ha podido leer el router: sin datos todavía."</p>
                <p class="estado-espera-detalle mono">{e.clone()}</p>
                <p class="pagina-sub">"Se reintenta solo; esta página se actualiza cada 10 s."</p>
            </section>
        }
        .into_any(),
        None => view! {
            <section class="tarjeta estado-espera" role="status">
                <h2>"Esperando el primer dato del router…"</h2>
                <p class="pagina-sub">"El primer sondeo tarda unos segundos."</p>
            </section>
        }
        .into_any(),
    }
}

/// Avisos derivados de los datos.
#[component]
fn Avisos(vista: Memo<Option<EstadoVista>>) -> impl IntoView {
    let avisos = move || vista.with(|v| v.as_ref().map(|v| v.avisos.clone()).unwrap_or_default());
    move || {
        let avisos = avisos();
        (!avisos.is_empty()).then(|| {
            let items = avisos
                .into_iter()
                .map(|a| {
                    let estado = a.nivel.estado();
                    view! {
                        <li class=format!("aviso {}", estado.clase())>
                            <IconoEstado estado />
                            <span>{a.texto}</span>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <ul class="avisos" aria-label="Avisos">
                    {items}
                </ul>
            }
        })
    }
}

/// Fila de indicadores.
#[component]
fn Indicadores(
    vista: Memo<Option<EstadoVista>>,
    ultima_hora: Memo<Vec<SampleVista>>,
) -> impl IntoView {
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
    let internet_valor = campo(|v| {
        match (v.router_ok, v.internet_up) {
            (false, _) => "Desconocido",
            (true, true) => "Conectado",
            (true, false) => "Sin conexión",
        }
        .to_owned()
    });
    let internet_sub = Signal::derive(move || {
        vista.with(|v| {
            let v = v.as_ref()?;
            if !v.router_ok {
                let antes = if v.internet_up {
                    "conectado"
                } else {
                    "sin conexión"
                };
                return Some(format!(
                    "Último dato: {antes} a las {}",
                    formato_hora(v.at?, false)
                ));
            }
            Some(match &v.ip_publica {
                Some(ip) => format!("IP pública {ip}"),
                None => "Sin IP pública".to_owned(),
            })
        })
    });

    // Tiempo de conexión: desde el último corte visto; si no se vio ninguno
    // desde que arrancó el router, su tiempo encendido.
    let conexion = Memo::new(move |_| {
        vista.with(|v| {
            let v = v.as_ref()?;
            let (titulo, desde, estado) = match (v.internet_up, v.conexion_desde, v.caida_desde) {
                (false, _, desde) => ("Sin conexión desde hace", desde, Some(Estado::Error)),
                (true, Some(d), _) => ("Tiempo de conexión", Some(d), None),
                (true, None, _) => ("Router encendido", v.encendido_desde, None),
            };
            // Sin conexión con el router, hasta el último dato: no sigue
            // creciendo como si se estuviera midiendo.
            let valor =
                desde.map_or_else(|| "—".to_owned(), |d| formato_duracion(v.medido_hasta - d));
            let sub = desde.map(|d| {
                let desde = format!("desde el {}", formato_fecha_hora(d, false));
                if v.router_ok {
                    desde
                } else {
                    format!(
                        "{desde} hasta el último dato ({})",
                        formato_hora(v.medido_hasta, false)
                    )
                }
            });
            Some((titulo, valor, sub, estado))
        })
    });
    let titulo_conexion = Memo::new(move |_| conexion.with(|c| c.as_ref().map(|c| c.0)));
    let indicador_conexion = move || {
        titulo_conexion.get().map(|titulo| {
            let valor = Signal::derive(move || {
                conexion.with(|c| c.as_ref().map(|c| c.1.clone()).unwrap_or_default())
            });
            let sub =
                Signal::derive(move || conexion.with(|c| c.as_ref().and_then(|c| c.2.clone())));
            let estado = Signal::derive(move || conexion.with(|c| c.as_ref().and_then(|c| c.3)));
            view! { <Indicador titulo valor sub estado /> }
        })
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
                    let valor = f(v).map_or_else(|| "sin dato".to_owned(), formato_bps);
                    match v.at {
                        Some(at) => {
                            format!("Último dato: {valor} a las {}", formato_hora(at, false))
                        }
                        None => "Sin datos del router".to_owned(),
                    }
                })
            });
            caido.or_else(|| {
                ultima_hora.with(|m| {
                    m.iter()
                        .filter_map(g)
                        .reduce(f64::max)
                        .map(|x| format!("máx. 1 h: {}", formato_bps(x)))
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
        format!(
            "de {} MB",
            formato_decimal(v.ram_total_kb as f64 / 1024.0, 0)
        )
    });
    let clientes_sub = campo(|v| {
        let cable = v.clientes.saturating_sub(v.clientes_wifi);
        format!("{} por WiFi · {cable} por cable", v.clientes_wifi)
    });

    view! {
        <section class="rejilla-indicadores" aria-label="Indicadores">
            <Indicador
                titulo="Internet"
                valor=internet_valor
                sub=internet_sub
                estado=internet_estado
            />
            {indicador_conexion}
            <Indicador
                titulo="Bajada"
                valor=tasa(|v| v.bajada_bps)
                sub=tasa_sub(|v| v.bajada_bps, |s| s.rx_bps)
                minigrafica=mini(|s| s.rx_bps)
                tono=Tono::Primario
            />
            <Indicador
                titulo="Subida"
                valor=tasa(|v| v.subida_bps)
                sub=tasa_sub(|v| v.subida_bps, |s| s.tx_bps)
                minigrafica=mini(|s| s.tx_bps)
                tono=Tono::Secundario
            />
            <Indicador
                titulo="Carga"
                valor=campo(|v| formato_decimal(f64::from(v.load.0), 2))
                sub=carga_sub
                minigrafica=mini(|s| Some(f64::from(s.load1)))
            />
            <Indicador
                titulo="RAM usada"
                valor=campo(|v| format!("{} %", formato_decimal(f64::from(v.ram_pct), 1)))
                sub=ram_sub
                minigrafica=mini(|s| Some(f64::from(s.ram_used_pct)))
            />
            <Indicador
                titulo="Clientes"
                valor=campo(|v| v.clientes.to_string())
                sub=clientes_sub
                minigrafica=mini(|s| s.clients.map(f64::from))
            />
        </section>
    }
    .into_any()
}

/// Selector de intervalo, gráficos y disponibilidad.
#[component]
fn Graficas(
    vista: Memo<Option<EstadoVista>>,
    muestras: Memo<Option<(Rango, Vec<SampleVista>)>>,
    cortes: Memo<Vec<CorteVista>>,
    rango: ReadSignal<Rango>,
    set_rango: WriteSignal<Rango>,
) -> impl IntoView {
    let ahora = move || vista.with(|v| v.as_ref().map(|v| v.ahora));
    // El intervalo de los datos que se están mostrando (mientras llegan los
    // del nuevo intervalo, se queda el anterior).
    let rango_datos = move || muestras.with(|m| m.as_ref().map_or(rango.get(), |m| m.0));
    let ventana = Signal::derive(move || {
        let fin = ahora()?;
        Some((fin - rango_datos().segundos(), fin))
    });
    let serie = move |f: fn(&SampleVista) -> Option<f64>| {
        move || {
            muestras.with(|m| {
                m.as_ref()
                    .map(|(_, m)| m.iter().map(|s| (s.ts, f(s))).collect::<Vec<_>>())
                    .unwrap_or_default()
            })
        }
    };
    let trafico = Signal::derive({
        let bajada = serie(|s| s.rx_bps);
        let subida = serie(|s| s.tx_bps);
        move || {
            vec![
                Serie::new("Bajada", Tono::Primario, bajada()).con_area(),
                Serie::new("Subida", Tono::Secundario, subida()).con_area(),
            ]
        }
    });
    let carga = Signal::derive({
        let f = serie(|s| Some(f64::from(s.load1)));
        move || vec![Serie::new("Carga (1 min)", Tono::Primario, f())]
    });
    let ram = Signal::derive({
        let f = serie(|s| Some(f64::from(s.ram_used_pct)));
        move || vec![Serie::new("RAM usada", Tono::Primario, f()).con_area()]
    });
    let clientes = Signal::derive({
        let f = serie(|s| s.clients.map(f64::from));
        move || vec![Serie::new("Clientes", Tono::Primario, f())]
    });
    let eventos = Signal::derive(move || {
        cortes.with(|c| {
            c.iter()
                .map(|c| Corte {
                    desde: c.desde,
                    hasta: c.hasta,
                })
                .collect::<Vec<_>>()
        })
    });
    let hasta = Signal::derive(move || ahora().unwrap_or_default());
    let desde = Signal::derive(move || hasta.get() - rango_datos().segundos());

    let botones = Rango::TODOS
        .into_iter()
        .map(|r| {
            view! {
                <button
                    type="button"
                    class="selector-boton"
                    aria-pressed=move || (rango.get() == r).to_string()
                    on:click=move |_| set_rango.set(r)
                >
                    {r.etiqueta()}
                </button>
            }
        })
        .collect_view();
    let titulo_franja = move || {
        format!(
            "Disponibilidad de internet · {}",
            rango_datos().descripcion()
        )
    };

    view! {
        <section class="seccion graficas" aria-labelledby="graficas-titulo">
            <div class="seccion-cabecera">
                <h2 id="graficas-titulo">"Histórico"</h2>
                <div class="selector" role="group" aria-label="Intervalo de los gráficos">
                    {botones}
                </div>
            </div>
            <SerieTemporal
                titulo="Tráfico de los dispositivos"
                nota="Lo que envían y reciben los equipos de la red por cable y WiFi. Incluye el tráfico entre equipos conectados a puertos distintos del router."
                series=trafico
                unidad=Unidad::Bps
                alto=280
                ventana=ventana
            />
            <section class="tarjeta franja-tarjeta">
                <h3 class="franja-titulo">{titulo_franja}</h3>
                <FranjaDisponibilidad eventos desde hasta />
            </section>
            <div class="rejilla-estado">
                <SerieTemporal
                    titulo="Carga del router"
                    series=carga
                    unidad=Unidad::Decimal
                    ventana=ventana
                />
                <SerieTemporal
                    titulo="RAM usada"
                    series=ram
                    unidad=Unidad::Porcentaje
                    ventana=ventana
                />
                <SerieTemporal
                    titulo="Clientes conectados"
                    series=clientes
                    unidad=Unidad::Entero
                    ventana=ventana
                />
            </div>
        </section>
    }
    .into_any()
}

/// Filas de la tabla de clientes LAN.
fn filas_hosts(v: &EstadoVista) -> Vec<Vec<Celda>> {
    v.hosts
        .iter()
        .map(|h| {
            vec![
                Celda::Mono(h.ip.clone()),
                Celda::Mono(h.mac.clone()),
                if h.nombre.is_empty() {
                    Celda::Vacia
                } else {
                    Celda::Texto(h.nombre.clone())
                },
                if h.activo {
                    Celda::Estado(Estado::Ok, "Conectado".into())
                } else {
                    Celda::Estado(Estado::Neutro, "Desconectado".into())
                },
                Celda::Texto(if h.fijo { "Sí" } else { "No" }.into()),
                h.interfaz.clone().map_or(Celda::Vacia, Celda::Texto),
            ]
        })
        .collect()
}

/// Tiempo restante a partir del cual una concesión se da por vencida (el
/// contador de 32 bits del router dio la vuelta).
const CADUCADA_DESDE: u64 = (1 << 32) - 365 * 86_400;
/// `u32::MAX` es una concesión infinita.
const SIN_CADUCIDAD: u64 = u32::MAX as u64;

/// Filas de la tabla de concesiones DHCP.
fn filas_leases(v: &EstadoVista) -> Vec<Vec<Celda>> {
    v.leases
        .iter()
        .map(|l| {
            let tipo = match (l.fijo, l.activa) {
                (true, true) => Celda::Estado(Estado::Neutro, "Fija".into()),
                (true, false) => Celda::Estado(Estado::Aviso, "Fija, desactivada".into()),
                (false, _) => Celda::Texto("Dinámica".into()),
            };
            vec![
                Celda::Mono(l.ip.clone()),
                Celda::Mono(l.mac.clone()),
                if l.nombre.is_empty() {
                    Celda::Vacia
                } else {
                    Celda::Texto(l.nombre.clone())
                },
                tipo,
                match l.caduca_s {
                    None => Celda::Vacia,
                    // El router da el tiempo restante como un entero de 32
                    // bits: una concesión vencida aparece como ~49 710 días.
                    Some(SIN_CADUCIDAD) => Celda::Texto("Sin caducidad".into()),
                    Some(s) if s >= CADUCADA_DESDE => Celda::Texto("Caducada".into()),
                    Some(s) => {
                        Celda::Numero(formato_duracion(i64::try_from(s).unwrap_or(i64::MAX)))
                    }
                },
            ]
        })
        .collect()
}

/// Filas de la tabla de redes WiFi (la clave nunca llega: solo si existe).
fn filas_wifi(v: &EstadoVista) -> Vec<Vec<Celda>> {
    v.wifi
        .iter()
        .map(|r| {
            let estado = if r.activa {
                Celda::Estado(Estado::Ok, "Activa".into())
            } else if r.radio_apagado {
                Celda::Estado(Estado::Neutro, "Radio apagado".into())
            } else {
                Celda::Estado(Estado::Neutro, "Desactivada".into())
            };
            let canal = if r.canal == "auto" {
                "canal automático".to_owned()
            } else {
                format!("canal {}", r.canal)
            };
            vec![
                Celda::Texto(r.ssid.clone()),
                Celda::Texto(format!("{} · {canal}", r.banda)),
                estado,
                Celda::Texto(r.seguridad.clone()),
                if r.con_clave {
                    Celda::Secreto
                } else {
                    Celda::Texto("Sin clave".into())
                },
                match (r.wps, r.activa) {
                    (true, true) => Celda::Estado(Estado::Aviso, "Activo".into()),
                    (true, false) => Celda::Texto("Activado (red apagada)".into()),
                    (false, _) => Celda::Texto("No".into()),
                },
                Celda::Numero(r.asociados.to_string()),
            ]
        })
        .collect()
}

/// Tablas de clientes, concesiones y redes WiFi.
#[component]
fn Tablas(vista: Memo<Option<EstadoVista>>) -> impl IntoView {
    let filas = move |f: fn(&EstadoVista) -> Vec<Vec<Celda>>| {
        Signal::derive(move || vista.with(|v| v.as_ref().map(f).unwrap_or_default()))
    };
    let lentos = move || {
        vista.with(|v| {
            let v = v.as_ref()?;
            Some(match v.lentos_at {
                Some(t) => format!(
                    "Datos de las {} · se leen cada minuto",
                    formato_hora(t, false)
                ),
                None => "Aún sin leer: se leen cada minuto".to_owned(),
            })
        })
    };
    let resumen_hosts = move || {
        vista.with(|v| {
            v.as_ref().map(|v| {
                let inactivos = v.hosts.len().saturating_sub(v.clientes as usize);
                format!("{} conectados · {inactivos} desconectados", v.clientes)
            })
        })
    };
    let con_wps = move || {
        vista.with(|v| {
            v.as_ref()
                .is_some_and(|v| v.wifi.iter().any(|r| r.wps && r.activa))
                .then(|| view! { <Insignia estado=Estado::Aviso texto="WPS activo" /> })
        })
    };

    view! {
        <section class="seccion tarjeta" aria-labelledby="clientes-titulo">
            <div class="seccion-cabecera">
                <div>
                    <h2 id="clientes-titulo">"Clientes LAN"</h2>
                    <p class="pagina-sub">{resumen_hosts} " · " {lentos}</p>
                </div>
            </div>
            <Tabla
                columnas=vec![
                    Columna::new("IP"),
                    Columna::new("MAC"),
                    Columna::new("Nombre"),
                    Columna::new("Estado"),
                    Columna::new("Lease fijo"),
                    Columna::new("Interfaz"),
                ]
                filas=filas(filas_hosts)
                vacia="El router no ha devuelto ningún equipo"
            />
        </section>
        <section class="seccion tarjeta" aria-labelledby="leases-titulo">
            <div class="seccion-cabecera">
                <div>
                    <h2 id="leases-titulo">"Concesiones DHCP"</h2>
                    <p class="pagina-sub">"Dinámicas y reservas fijas por MAC"</p>
                </div>
            </div>
            <Tabla
                columnas=vec![
                    Columna::new("IP"),
                    Columna::new("MAC"),
                    Columna::new("Nombre"),
                    Columna::new("Tipo"),
                    Columna::numerica("Caduca en"),
                ]
                filas=filas(filas_leases)
                vacia="Sin concesiones"
            />
        </section>
        <section class="seccion tarjeta" aria-labelledby="wifi-titulo">
            <div class="seccion-cabecera">
                <div>
                    <h2 id="wifi-titulo">"Redes WiFi"</h2>
                    <p class="pagina-sub">"Las claves no salen del servidor"</p>
                </div>
                {con_wps}
            </div>
            <Tabla
                columnas=vec![
                    Columna::new("SSID"),
                    Columna::new("Banda"),
                    Columna::new("Estado"),
                    Columna::new("Seguridad"),
                    Columna::new("Clave"),
                    Columna::new("WPS"),
                    Columna::numerica("Asociados"),
                ]
                filas=filas(filas_wifi)
                vacia="Sin redes WiFi"
            />
        </section>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caducidad_de_las_concesiones() {
        let lease = |caduca_s| LeaseVista {
            ip: "192.168.1.2".into(),
            mac: "02:00:00:00:00:01".into(),
            nombre: String::new(),
            fijo: false,
            activa: true,
            caduca_s,
        };
        let v = EstadoVista {
            leases: vec![
                lease(Some(600)),
                lease(Some(u64::from(u32::MAX))),
                lease(Some(u64::from(u32::MAX) - 1)),
                lease(None),
            ],
            ..vista_vacia()
        };
        let caducidad: Vec<Celda> = filas_leases(&v).into_iter().map(|f| f[4].clone()).collect();
        assert_eq!(
            caducidad,
            [
                Celda::Numero("10 min".into()),
                Celda::Texto("Sin caducidad".into()),
                Celda::Texto("Caducada".into()),
                Celda::Vacia,
            ]
        );
    }

    fn vista_vacia() -> EstadoVista {
        EstadoVista {
            ahora: 0,
            at: None,
            router_ok: false,
            error: None,
            modelo: String::new(),
            firmware: String::new(),
            internet_up: false,
            ip_publica: None,
            conexion_desde: None,
            caida_desde: None,
            encendido_desde: None,
            load: (0.0, 0.0, 0.0),
            ram_pct: 0.0,
            ram_total_kb: 0,
            clientes: 0,
            clientes_wifi: 0,
            hosts: Vec::new(),
            leases: Vec::new(),
            wifi: Vec::new(),
            lentos_at: None,
            bajada_bps: None,
            subida_bps: None,
            medido_hasta: 0,
            avisos: Vec::new(),
        }
    }
}
