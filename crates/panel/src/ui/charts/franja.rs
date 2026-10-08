//! Disponibilidad: cortes, resumen y componente `FranjaDisponibilidad`.

use super::{
    escalas::{escala, formato_decimal},
    geometria::{SEPARACION_TICKS_X, ancla_dentro},
    serie::{recuadro, usar_ancho, x_del_puntero},
    tiempo::{formato_duracion, formato_fecha_hora, formato_hora, ticks_tiempo},
};
use crate::ui::{
    Estado, Insignia,
    table::{Celda, Columna, Tabla},
};
use leptos::{ev, html, prelude::*, svg};

// ---------------------------------------------------------------------------
// Disponibilidad
// ---------------------------------------------------------------------------

/// Corte de la conexión: `hasta = None` si sigue caída.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Corte {
    pub desde: i64,
    pub hasta: Option<i64>,
}

/// Parte de un corte que cae dentro del intervalo mostrado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tramo {
    pub desde: i64,
    pub hasta: i64,
    /// El corte de origen sigue abierto (`Corte::hasta = None`).
    pub en_curso: bool,
}

/// Tramos caídos recortados a `[desde, hasta]` (un corte abierto llega hasta
/// `hasta`), sin tramos vacíos; cada uno recuerda si su corte sigue abierto.
pub fn tramos_caidos(eventos: &[Corte], desde: i64, hasta: i64) -> Vec<Tramo> {
    eventos
        .iter()
        .map(|c| Tramo {
            desde: c.desde.max(desde),
            hasta: c.hasta.unwrap_or(hasta).min(hasta),
            en_curso: c.hasta.is_none(),
        })
        .filter(|t| t.hasta > t.desde)
        .collect()
}

/// Cabecera de la lectura de un tramo: `"08/10 13:00 → 13:26"`, o
/// `"… → ahora"` si su corte sigue abierto.
fn cabecera_tramo(t: &Tramo) -> String {
    let fin = if t.en_curso {
        "ahora".to_owned()
    } else {
        formato_hora(t.hasta, false)
    };
    format!("{} → {fin}", formato_fecha_hora(t.desde, false))
}

/// Resumen para lectores de pantalla.
fn texto_aria(r: &ResumenCortes) -> String {
    let cortes = match r.cortes {
        1 => "1 corte".to_owned(),
        n => format!("{n} cortes"),
    };
    format!(
        "Disponibilidad de internet: {}, {cortes}, {} sin conexión",
        formato_disponibilidad(r),
        formato_duracion(r.segundos_caido)
    )
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
    let caido: i64 = tramos.iter().map(|t| t.hasta - t.desde).sum();
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
            .map(|t| {
                let x = s(t.desde as f64);
                let ancho_corte = (s(t.hasta as f64) - x).max(2.0);
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
        let corte = tramos.get().into_iter().find(|t| {
            let xa = s(t.desde as f64);
            let xb = s(t.hasta as f64).max(xa + 2.0);
            x >= xa - ENGANCHE && x <= xb + ENGANCHE
        });
        let (cabecera, filas, resalte) = match corte {
            Some(tramo) => {
                let xa = s(tramo.desde as f64);
                (
                    cabecera_tramo(&tramo),
                    vec![(
                        "franja-clave-corte",
                        formato_duracion(tramo.hasta - tramo.desde),
                        "sin conexión".to_owned(),
                    )],
                    Some((xa, (s(tramo.hasta as f64) - xa).max(2.0))),
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
            .map(|t| (s(t.desde as f64) + s(t.hasta as f64)) / 2.0)
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

    let etiqueta_aria = move || texto_aria(&resumen.get());

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn solo_el_corte_abierto_dice_ahora() {
        let h = 1_791_417_600 + 14 * 3600;
        let eventos = vec![
            // Cerrado, pero termina después del final del intervalo.
            Corte {
                desde: h - 600,
                hasta: Some(h + 600),
            },
            // Abierto, empezado antes.
            Corte {
                desde: h - 3000,
                hasta: None,
            },
            Corte {
                desde: h - 7200,
                hasta: Some(h - 7000),
            },
        ];
        let tramos = tramos_caidos(&eventos, h - 3600, h);
        assert_eq!(tramos.len(), 2);
        assert_eq!(
            tramos[0],
            Tramo {
                desde: h - 600,
                hasta: h,
                en_curso: false
            }
        );
        assert!(tramos[1].en_curso);
        assert!(
            !cabecera_tramo(&tramos[0]).ends_with("ahora"),
            "{:?}",
            cabecera_tramo(&tramos[0])
        );
        assert!(cabecera_tramo(&tramos[1]).ends_with("→ ahora"));
    }

    #[test]
    fn resumen_accesible_con_singular_y_plural() {
        let uno = resumen_cortes(
            &[Corte {
                desde: 10,
                hasta: Some(70),
            }],
            0,
            1000,
        );
        assert!(
            texto_aria(&uno).contains(", 1 corte,"),
            "{}",
            texto_aria(&uno)
        );
        let dos = resumen_cortes(
            &[
                Corte {
                    desde: 10,
                    hasta: Some(70),
                },
                Corte {
                    desde: 100,
                    hasta: Some(160),
                },
            ],
            0,
            1000,
        );
        assert!(texto_aria(&dos).contains(", 2 cortes,"));
        let cero = resumen_cortes(&[], 0, 1000);
        assert!(texto_aria(&cero).contains(", 0 cortes,"));
    }
}
