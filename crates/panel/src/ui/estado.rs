//! Página de estado (`/`): avisos, indicadores con minigráfica, gráficos con
//! selector de intervalo (1 h / 24 h / 7 d), disponibilidad de internet y
//! tablas de clientes, concesiones DHCP y redes WiFi.
//!
//! Respeta el modo de visualización del usuario ([`ModoVista`]): «mixto»
//! (lo de siempre), «gráfica» (los gráficos primero y a todo el ancho, los
//! indicadores compactos) y «texto» (sin gráficos: [`texto`]). Se cambia en
//! la cabecera ([`SelectorVista`]) sin recargar.
//!
//! Se refresca cada 10 s en el navegador (no mientras la pestaña está oculta)
//! sin rehacer la página: los componentes leen señales derivadas de los
//! recursos, que conservan el dato anterior mientras llega el nuevo, así que
//! no se mueve nada ni se pierde la lectura de un gráfico al refrescar.

mod datos;
mod graficas;
mod indicadores;
mod tablas;
mod texto;

pub use datos::*;

use graficas::Graficas;
use indicadores::{Indicadores, indicadores};
use tablas::Tablas;

use crate::t;
use crate::ui::{
    Estado, IconoEstado, IconosOcultos,
    charts::{
        Corte, FranjaDisponibilidad, Indicador, Serie, SerieTemporal, Tono, Unidad, formato_bps,
        formato_decimal, formato_duracion, formato_fecha_hora, formato_hora,
    },
    comun::{EstadoVisual, Insignia},
    table::{Celda, Columna, Tabla},
    vista::{SelectorVista, use_vista},
};
use crate::vista::ModoVista;
use leptos::prelude::*;
use leptos::reactive::computed::ScopedFuture;

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
        ServerFnError::ServerError(msg) => super::mensaje_del_servidor(msg),
        _ => t!("comun-error-conexion"),
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

    // El contenido se crea cuando ya han llegado los cuatro recursos, y no
    // antes: Leptos avisa a `Transition` de que un recurso está listo un
    // instante antes de marcar como sucios los `Memo` que lo leen, así que un
    // `Memo` creado antes podía pintarse en el servidor todavía «sin datos»
    // mientras los datos serializados para el navegador sí los llevaban (error
    // de hidratación intermitente). La espera no sigue a los recursos: al
    // refrescar, el contenido no se rehace (se actualiza por señales).
    let contenido = move || {
        Suspend::new(ScopedFuture::new_untracked(async move {
            estado.ready().await;
            historico.ready().await;
            cortes_r.ready().await;
            hora.ready().await;
            view! { <Contenido estado historico cortes_r hora rango set_rango /> }
        }))
    };

    view! {
        <Transition fallback=|| {
            view! {
                <header class="pagina-cabecera">
                    <h1>{t!("estado-titulo")}</h1>
                </header>
                <p class="estado-espera">{t!("comun-cargando")}</p>
            }
        }>{contenido}</Transition>
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
        .map(|e| (texto_error(&e), super::es_error_de_sesion(&e)))
    });

    // En el modo «texto», ningún gráfico: tampoco los iconos de estado.
    let modo = use_vista();
    provide_context(IconosOcultos(Signal::derive(move || {
        modo.get() == ModoVista::Texto
    })));
    let datos = StoredValue::new(indicadores(vista, ultima_hora));
    let hay_dato = Memo::new(move |_| vista.with(|v| v.as_ref().is_some_and(|v| v.at.is_some())));

    let cabecera = cabecera(vista, error_panel);
    // Avisos e indicadores (o la tarjeta de «sin datos»). En «gráfica», los
    // indicadores van después de los gráficos.
    let resumen = move |modo: ModoVista| {
        move || {
            let v = vista.get()?;
            Some(if v.at.is_none() {
                sin_datos(&v).into_any()
            } else {
                let datos = datos.get_value();
                match modo {
                    ModoVista::Mixto => view! {
                        <Avisos vista />
                        <Indicadores datos />
                    }
                    .into_any(),
                    ModoVista::Grafica => view! { <Avisos vista /> }.into_any(),
                    ModoVista::Texto => view! {
                        <Avisos vista />
                        <texto::IndicadoresTexto datos />
                    }
                    .into_any(),
                }
            })
        }
    };
    let tablas = move || {
        vista
            .with(|v| v.as_ref().is_some_and(|v| v.at.is_some()))
            .then(|| view! { <Tablas vista /> })
    };
    // Solo se rehace al cambiar de modo; los datos llegan por señales.
    let cuerpo = move || match modo.get() {
        ModoVista::Mixto => view! {
            {resumen(ModoVista::Mixto)}
            <Graficas vista muestras cortes rango set_rango />
            {tablas}
        }
        .into_any(),
        ModoVista::Grafica => {
            let compactos = move || {
                hay_dato.get().then(|| {
                    view! { <Indicadores datos=datos.get_value() compactos=true /> }
                })
            };
            view! {
                {resumen(ModoVista::Grafica)}
                <Graficas vista muestras cortes rango set_rango una_columna=true />
                {compactos}
                {tablas}
            }
            .into_any()
        }
        ModoVista::Texto => view! {
            {resumen(ModoVista::Texto)}
            <texto::HistoricoTexto vista muestras cortes rango set_rango />
            {tablas}
        }
        .into_any(),
    };

    view! {
        {cabecera}
        {cuerpo}
    }
    .into_any()
}

/// Título, modelo y hora del dato; error del panel si lo hay.
fn cabecera(
    vista: Memo<Option<EstadoVista>>,
    error_panel: Memo<Option<(String, bool)>>,
) -> impl IntoView {
    let sub = move || {
        vista.with(|v| {
            let v = v.as_ref()?;
            let at = v.at?;
            Some(t!(
                "estado-sub",
                modelo = v.modelo.as_str(),
                firmware = v.firmware.as_str(),
                hora = formato_hora(at, true)
            ))
        })
    };
    let error = move || {
        error_panel.get().map(|(e, sesion)| {
            view! {
                <p class="aviso estado-error" role="alert">
                    <IconoEstado estado=Estado::Error />
                    <span>
                        {t!("estado-error-actualizar", error = e)}
                        {sesion
                            .then(|| {
                                view! {
                                    " "
                                    <a href="/login?next=%2F">{t!("estado-volver-a-entrar")}</a>
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
                <h1>{t!("estado-titulo")}</h1>
                <p class="pagina-sub">{sub}</p>
            </div>
            <SelectorVista />
        </header>
        {error}
    }
}

/// Tarjeta cuando aún no hay ningún dato del router.
fn sin_datos(v: &EstadoVista) -> impl IntoView {
    if v.sin_configurar {
        return view! {
            <section class="tarjeta estado-espera" role="status">
                <h2>{t!("estado-sin-configurar")}</h2>
                <p class="pagina-sub">{t!("estado-sin-configurar-sub")}</p>
            </section>
        }
        .into_any();
    }
    match &v.error {
        Some(e) => view! {
            <section class="tarjeta estado-espera estado-error" role="status">
                <h2>
                    <IconoEstado estado=Estado::Error />
                    {t!("estado-sin-conexion-router")}
                </h2>
                <p>{t!("estado-sin-datos-aun")}</p>
                <p class="estado-espera-detalle mono">{e.clone()}</p>
                <p class="pagina-sub">{t!("estado-reintenta")}</p>
            </section>
        }
        .into_any(),
        None => view! {
            <section class="tarjeta estado-espera" role="status">
                <h2>{t!("estado-esperando")}</h2>
                <p class="pagina-sub">{t!("estado-esperando-sub")}</p>
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
                <ul class="avisos" aria-label=t!("estado-avisos")>
                    {items}
                </ul>
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tablas::filas_leases;

    #[test]
    fn caducidad_de_las_concesiones() {
        let lease = |caduca_s| LeaseVista {
            ip: "192.168.1.62".into(),
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
            sin_configurar: false,
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
