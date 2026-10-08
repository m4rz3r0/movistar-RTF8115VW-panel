//! Muestra de componentes con datos sintéticos (`/_muestra`), para revisar el
//! sistema visual. Temporal: desaparece cuando las páginas reales los usen.
//! Los datos son deterministas para que el servidor y el navegador generen
//! lo mismo al hidratar.

use crate::ui::{
    Estado, Insignia,
    charts::{Corte, FranjaDisponibilidad, Indicador, Serie, SerieTemporal, Tono, Unidad},
    table::{Celda, Columna, Tabla},
};
use leptos::prelude::*;

/// 2026-10-08 16:00 hora peninsular.
const FIN: i64 = 1_791_417_600 + 14 * 3600;

/// Ruido determinista en [0, 1).
fn ruido(i: i64, semilla: u64) -> f64 {
    let mut x = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ semilla;
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

/// Tráfico de la última hora cada 10 s, con un tramo sin datos y otro sin
/// muestras.
fn trafico() -> Vec<Serie> {
    let mut bajada = Vec::new();
    let mut subida = Vec::new();
    for i in 0..360_i64 {
        if (250..262).contains(&i) {
            continue; // el sondeo no tomó muestras
        }
        let t = FIN - 3600 + i * 10;
        let sin_dato = (140..150).contains(&i);
        let base = 18e6 + 9e6 * ((i as f64) / 40.0).sin();
        let pico = if (300..320).contains(&i) { 38e6 } else { 0.0 };
        let rx = base + pico + 6e6 * ruido(i, 1);
        let tx = 1.6e6 + 0.9e6 * ((i as f64) / 25.0).cos() + 1.2e6 * ruido(i, 2);
        bajada.push((t, (!sin_dato).then_some(rx)));
        subida.push((t, (!sin_dato).then_some(tx)));
    }
    vec![
        Serie::new("Bajada", Tono::Primario, bajada).con_area(),
        Serie::new("Subida", Tono::Secundario, subida).con_area(),
    ]
}

/// Serie de 24 h por minuto.
fn dia(f: impl Fn(i64) -> f64) -> Vec<(i64, Option<f64>)> {
    (0..1440_i64)
        .map(|i| (FIN - 86_400 + i * 60, Some(f(i))))
        .collect()
}

fn ultimos(puntos: &[(i64, Option<f64>)], n: usize) -> Vec<Option<f64>> {
    let desde = puntos.len().saturating_sub(n);
    puntos[desde..].iter().map(|p| p.1).collect()
}

/// Página de muestra.
#[component]
pub fn Muestra() -> impl IntoView {
    let series_trafico = trafico();
    let carga = dia(|i| 0.25 + 0.2 * ((i as f64) / 180.0).sin().abs() + 0.05 * ruido(i / 5, 3));
    let ram = dia(|i| 58.0 + 6.0 * ((i as f64) / 300.0).sin() + 2.0 * ruido(i, 4));
    let clientes = dia(|i| {
        let h = (i / 60 + 16) % 24;
        let base = if (8..23).contains(&h) { 12.0 } else { 6.0 };
        (base + 4.0 * ruido(i / 20, 5)).round()
    });
    let bajada_mini = ultimos(&series_trafico[0].puntos, 60);
    let cortes = vec![
        Corte {
            desde: FIN - 20 * 3600,
            hasta: Some(FIN - 20 * 3600 + 7 * 60),
        },
        Corte {
            desde: FIN - 9 * 3600 - 1800,
            hasta: Some(FIN - 9 * 3600 - 1800 + 95),
        },
        Corte {
            desde: FIN - 3 * 3600,
            hasta: Some(FIN - 3 * 3600 + 26 * 60),
        },
    ];
    let filas_clientes = vec![
        vec![
            Celda::Mono("192.0.2.10".into()),
            Celda::Mono("02:00:00:00:ee:01".into()),
            Celda::Texto("servidor".into()),
            Celda::Estado(Estado::Ok, "Fijo".into()),
            Celda::Texto("Ethernet".into()),
        ],
        vec![
            Celda::Mono("192.168.1.76".into()),
            Celda::Mono("02:00:00:00:ee:02".into()),
            Celda::Texto("cámara-entrada".into()),
            Celda::Estado(Estado::Ok, "Fijo".into()),
            Celda::Texto("Ethernet".into()),
        ],
        vec![
            Celda::Mono("192.168.1.78".into()),
            Celda::Mono("02:00:00:00:ee:03".into()),
            Celda::Vacia,
            Celda::Estado(Estado::Neutro, "Dinámico".into()),
            Celda::Texto("WiFi 5 GHz".into()),
        ],
    ];

    // Cada bloque se borra de tipo para no desbordar al compilador.
    let cabecera = view! {
        <header class="pagina-cabecera">
            <div>
                <h1>"Muestra de componentes"</h1>
                <p class="pagina-sub">"Datos sintéticos · página temporal de revisión"</p>
            </div>
            <div class="insignias">
                <Insignia estado=Estado::Ok texto="Conectado" />
                <Insignia estado=Estado::Aviso texto="WPS activo" />
                <Insignia estado=Estado::Error texto="Sin conexión" />
                <Insignia estado=Estado::Neutro texto="TR-069" />
            </div>
        </header>
    }
    .into_any();
    let indicadores = view! {
        <section class="rejilla-tarjetas" aria-label="Indicadores">
            <Indicador
                titulo="Internet"
                valor="Conectado"
                sub="IP pública 203.0.113.45"
                estado=Estado::Ok
            />
            <Indicador titulo="Tiempo de conexión" valor="3 d 4 h" sub="desde el 05/10 11:52" />
            <Indicador
                titulo="Bajada"
                valor="27,4 Mbit/s"
                sub="última hora"
                minigrafica=bajada_mini
            />
            <Indicador
                titulo="Carga"
                valor="0,42"
                sub="1 · 5 · 15 min: 0,42 · 0,38 · 0,30"
                minigrafica=ultimos(&carga, 60)
            />
            <Indicador titulo="RAM usada" valor="61,3 %" minigrafica=ultimos(&ram, 60) />
            <Indicador titulo="Clientes" valor="14" sub="9 WiFi · 5 cable" minigrafica=ultimos(&clientes, 60) />
        </section>
    }
    .into_any();
    let trafico_wan = view! {
        <section class="seccion">
            <SerieTemporal titulo="Tráfico WAN" series=series_trafico unidad=Unidad::Bps alto=280 />
        </section>
    }
    .into_any();
    let graficas = view! {
        <section class="rejilla-graficas">
            <SerieTemporal
                titulo="Carga del router"
                series=vec![Serie::new("Carga (1 min)", Tono::Primario, carga)]
                unidad=Unidad::Decimal
            />
            <SerieTemporal
                titulo="RAM usada"
                series=vec![Serie::new("RAM", Tono::Primario, ram).con_area()]
                unidad=Unidad::Porcentaje
            />
            <SerieTemporal
                titulo="Clientes conectados"
                series=vec![Serie::new("Clientes", Tono::Primario, clientes)]
                unidad=Unidad::Entero
            />
            <SerieTemporal
                titulo="Sin datos"
                series={Vec::<Serie>::new()}
                unidad=Unidad::Bps
                ventana=(FIN - 3600, FIN)
            />
        </section>
    }
    .into_any();
    let disponibilidad = view! {
        <section class="seccion tarjeta">
            <h2>"Disponibilidad de internet · 24 h"</h2>
            <FranjaDisponibilidad eventos=cortes desde=FIN - 86_400 hasta=FIN />
        </section>
    }
    .into_any();
    let clientes_lan = view! {
        <section class="seccion tarjeta">
            <h2>"Clientes LAN"</h2>
            <Tabla
                columnas=vec![
                    Columna::new("IP"),
                    Columna::new("MAC"),
                    Columna::new("Nombre"),
                    Columna::new("Lease"),
                    Columna::new("Interfaz"),
                ]
                filas=filas_clientes
            />
        </section>
    }
    .into_any();
    let redes_wifi = view! {
        <section class="seccion tarjeta">
            <h2>"Redes WiFi"</h2>
            <Tabla
                columnas=vec![
                    Columna::new("SSID"),
                    Columna::new("Banda"),
                    Columna::new("Seguridad"),
                    Columna::new("Clave"),
                    Columna::numerica("Asociados"),
                ]
                filas=vec![
                    vec![
                        Celda::Texto("RED-1".into()),
                        Celda::Texto("2,4 GHz".into()),
                        Celda::Texto("WPA2-PSK".into()),
                        Celda::Secreto,
                        Celda::Numero("4".into()),
                    ],
                    vec![
                        Celda::Texto("RED-2".into()),
                        Celda::Texto("5 GHz".into()),
                        Celda::Texto("WPA2-PSK".into()),
                        Celda::Secreto,
                        Celda::Numero("5".into()),
                    ],
                ]
            />
        </section>
    }
    .into_any();
    view! {
        {cabecera}
        {indicadores}
        {trafico_wan}
        {graficas}
        {disponibilidad}
        {clientes_lan}
        {redes_wifi}
    }
}
