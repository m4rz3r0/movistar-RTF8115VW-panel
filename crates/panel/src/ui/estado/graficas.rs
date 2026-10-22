//! Gráficas del histórico y selector del intervalo.

use super::*;

/// Botones del intervalo del histórico (1 h, 24 h, 7 d).
pub(super) fn selector_rango(
    rango: ReadSignal<Rango>,
    set_rango: WriteSignal<Rango>,
) -> impl IntoView {
    let botones = Rango::TODOS
        .into_iter()
        .map(|r| {
            view! {
                <button
                    type="button"
                    class="b-sec"
                    aria-pressed=move || (rango.get() == r).to_string()
                    on:click=move |_| set_rango.set(r)
                >
                    {r.etiqueta()}
                </button>
            }
        })
        .collect_view();
    view! {
        <div class="selector" role="group" aria-label=t!("estado-intervalo")>
            {botones}
        </div>
    }
}

/// Selector de intervalo, gráficos y disponibilidad. Con `una_columna`
/// (modo «gráfica»), cada gráfico a todo el ancho.
#[component]
pub(super) fn Graficas(
    vista: Memo<Option<EstadoVista>>,
    muestras: Memo<Option<(Rango, Vec<SampleVista>)>>,
    cortes: Memo<Vec<CorteVista>>,
    rango: ReadSignal<Rango>,
    set_rango: WriteSignal<Rango>,
    #[prop(optional)] una_columna: bool,
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
                Serie::new(t!("estado-bajada"), Tono::Primario, bajada()).con_area(),
                Serie::new(t!("estado-subida"), Tono::Secundario, subida()).con_area(),
            ]
        }
    });
    let carga = Signal::derive({
        let f = serie(|s| Some(f64::from(s.load1)));
        move || vec![Serie::new(t!("estado-carga-1min"), Tono::Primario, f())]
    });
    let ram = Signal::derive({
        let f = serie(|s| Some(f64::from(s.ram_used_pct)));
        move || vec![Serie::new(t!("estado-ram-usada"), Tono::Primario, f()).con_area()]
    });
    let clientes = Signal::derive({
        let f = serie(|s| s.clients.map(f64::from));
        move || vec![Serie::new(t!("estado-clientes"), Tono::Primario, f())]
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

    let rejilla = if una_columna {
        "rejilla-estado una-columna"
    } else {
        "rejilla-estado"
    };
    let titulo_franja = move || t!("estado-disponibilidad", rango = rango_datos().descripcion());

    view! {
        <section class="seccion graficas" aria-labelledby="graficas-titulo">
            <div class="seccion-cabecera">
                <h2 id="graficas-titulo">{t!("estado-historico")}</h2>
                {selector_rango(rango, set_rango)}
            </div>
            <SerieTemporal
                titulo=t!("estado-trafico")
                nota=t!("estado-trafico-nota")
                series=trafico
                unidad=Unidad::Bps
                alto=280
                ventana=ventana
            />
            <section class="tarjeta franja-tarjeta">
                <h3 class="franja-titulo">{titulo_franja}</h3>
                <FranjaDisponibilidad eventos desde hasta />
            </section>
            <div class=rejilla>
                <SerieTemporal
                    titulo=t!("estado-carga-router")
                    series=carga
                    unidad=Unidad::Decimal
                    ventana=ventana
                />
                <SerieTemporal
                    titulo=t!("estado-ram-usada")
                    series=ram
                    unidad=Unidad::Porcentaje
                    ventana=ventana
                />
                <SerieTemporal
                    titulo=t!("estado-clientes-conectados")
                    series=clientes
                    unidad=Unidad::Entero
                    ventana=ventana
                />
            </div>
        </section>
    }
    .into_any()
}
