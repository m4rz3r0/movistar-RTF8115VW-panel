//! Componente `SerieTemporal` y piezas compartidas con la franja (medida del
//! contenedor, posición del puntero, recuadro de lectura).

use super::{
    escalas::{Unidad, escala, formato_valor},
    geometria::{Serie, geometria, indice_mas_cercano, lectura, resumen, valor_en},
    tiempo::formato_fecha_hora,
};
use crate::t;
use crate::ui::table::{Celda, Columna, Tabla};
use leptos::{ev, html, prelude::*, svg};

// ---------------------------------------------------------------------------
// Componentes
// ---------------------------------------------------------------------------

/// Anchura con la que se pinta en el servidor (y en la hidratación) antes de
/// medir el contenedor.
const ANCHO_INICIAL: f64 = 720.0;

/// Anchura en píxeles del contenedor `nodo`, medida en el navegador tras
/// hidratar y al cambiar el tamaño de la ventana. Los gráficos se dibujan a su
/// tamaño real para que los textos no se escalen.
pub(super) fn usar_ancho(nodo: NodeRef<html::Div>) -> ReadSignal<f64> {
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
pub(super) fn x_del_puntero(svg: &NodeRef<svg::Svg>, client_x: i32, ancho: f64) -> Option<f64> {
    let el = svg.get_untracked()?;
    let r = el.get_bounding_client_rect();
    (r.width() > 0.0).then(|| (f64::from(client_x) - r.left()) * ancho / r.width())
}

/// Recuadro de lectura dentro del SVG junto a la cruceta en `x`, sin salirse
/// de `limites`: cabecera y filas `(clase, valor, nombre)`. El valor manda; el
/// nombre acompaña.
pub(super) fn recuadro(
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
    /// Aclaración breve bajo el título (qué se mide).
    #[prop(into, optional)]
    nota: Option<String>,
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
                        {t!("comun-sin-datos-intervalo")}
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
                let mut columnas = vec![Columna::new(t!("comun-col-hora"))];
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
                {nota.map(|n| view! { <p class="grafica-nota">{n}</p> })}
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
                <summary>{t!("comun-ver-datos-tabla")}</summary>
                {tabla}
            </details>
        </figure>
    }
    .into_any()
}
