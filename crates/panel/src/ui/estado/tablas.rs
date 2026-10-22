//! Tablas de clientes LAN, concesiones DHCP y WiFi.

use super::*;

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
                    Celda::Estado(Estado::Ok, t!("comun-conectado"))
                } else {
                    Celda::Estado(Estado::Neutro, t!("estado-desconectado"))
                },
                Celda::Texto(if h.fijo {
                    t!("comun-si")
                } else {
                    t!("comun-no")
                }),
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
pub(super) fn filas_leases(v: &EstadoVista) -> Vec<Vec<Celda>> {
    v.leases
        .iter()
        .map(|l| {
            let tipo = match (l.fijo, l.activa) {
                (true, true) => Celda::Estado(Estado::Neutro, t!("estado-lease-fija")),
                (true, false) => Celda::Estado(Estado::Aviso, t!("estado-lease-fija-desactivada")),
                (false, _) => Celda::Texto(t!("estado-lease-dinamica")),
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
                    Some(SIN_CADUCIDAD) => Celda::Texto(t!("estado-sin-caducidad")),
                    Some(s) if s >= CADUCADA_DESDE => Celda::Texto(t!("estado-caducada")),
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
                Celda::Estado(Estado::Ok, t!("estado-wifi-activa"))
            } else if r.radio_apagado {
                Celda::Estado(Estado::Neutro, t!("estado-wifi-radio-apagado"))
            } else {
                Celda::Estado(Estado::Neutro, t!("estado-wifi-desactivada"))
            };
            let canal = if r.canal == "auto" {
                t!("estado-canal-auto")
            } else {
                t!("estado-canal", canal = r.canal.as_str())
            };
            // El icono de emisión dice el estado antes que el texto: verde
            // si emite, gris si está apagada.
            let emision = if r.activa {
                EstadoVisual::Emite
            } else {
                EstadoVisual::Apagada
            };
            vec![
                Celda::Red(emision, r.ssid.clone()),
                Celda::Texto(format!("{} · {canal}", r.banda)),
                estado,
                Celda::Texto(r.seguridad.clone()),
                if r.con_clave {
                    Celda::Secreto
                } else {
                    Celda::Texto(t!("estado-sin-clave"))
                },
                match (r.wps, r.activa) {
                    (true, true) => Celda::Estado(Estado::Aviso, t!("estado-wps-activo")),
                    (true, false) => Celda::Texto(t!("estado-wps-red-apagada")),
                    (false, _) => Celda::Texto(t!("comun-no")),
                },
                Celda::Numero(r.asociados.to_string()),
            ]
        })
        .collect()
}

/// Tablas de clientes, concesiones y redes WiFi.
#[component]
pub(super) fn Tablas(vista: Memo<Option<EstadoVista>>) -> impl IntoView {
    let filas = move |f: fn(&EstadoVista) -> Vec<Vec<Celda>>| {
        Signal::derive(move || vista.with(|v| v.as_ref().map(f).unwrap_or_default()))
    };
    let lentos = move || {
        vista.with(|v| {
            let v = v.as_ref()?;
            Some(match v.lentos_at {
                Some(t) => t!("estado-lentos", hora = formato_hora(t, false)),
                None => t!("estado-lentos-sin-leer"),
            })
        })
    };
    let resumen_hosts = move || {
        vista.with(|v| {
            v.as_ref().map(|v| {
                let inactivos = v.hosts.len().saturating_sub(v.clientes as usize);
                t!(
                    "estado-resumen-hosts",
                    conectados = v.clientes,
                    desconectados = inactivos
                )
            })
        })
    };
    let con_wps = move || {
        vista.with(|v| {
            v.as_ref()
                .is_some_and(|v| v.wifi.iter().any(|r| r.wps && r.activa))
                .then(|| view! { <Insignia estado=EstadoVisual::Atencion texto=t!("estado-wps-activo-insignia") /> })
        })
    };

    view! {
        <section class="seccion tarjeta" aria-labelledby="clientes-titulo">
            <div class="seccion-cabecera">
                <div>
                    <h2 id="clientes-titulo">{t!("estado-clientes-lan")}</h2>
                    <p class="pagina-sub">{resumen_hosts} " · " {lentos}</p>
                </div>
            </div>
            <Tabla
                columnas=vec![
                    Columna::new("IP"),
                    Columna::new("MAC"),
                    Columna::new(t!("estado-col-nombre")),
                    Columna::new(t!("estado-col-estado")),
                    Columna::new(t!("estado-col-lease-fijo")),
                    Columna::new(t!("estado-col-interfaz")),
                ]
                filas=filas(filas_hosts)
                vacia=t!("estado-sin-equipos")
            />
        </section>
        <section class="seccion tarjeta" aria-labelledby="leases-titulo">
            <div class="seccion-cabecera">
                <div>
                    <h2 id="leases-titulo">{t!("estado-concesiones")}</h2>
                    <p class="pagina-sub">{t!("estado-concesiones-sub")}</p>
                </div>
            </div>
            <Tabla
                columnas=vec![
                    Columna::new("IP"),
                    Columna::new("MAC"),
                    Columna::new(t!("estado-col-nombre")),
                    Columna::new(t!("estado-col-tipo")),
                    Columna::numerica(t!("estado-col-caduca")),
                ]
                filas=filas(filas_leases)
                vacia=t!("estado-sin-concesiones")
            />
        </section>
        <section class="seccion tarjeta" aria-labelledby="wifi-titulo">
            <div class="seccion-cabecera">
                <div>
                    <h2 id="wifi-titulo">{t!("estado-redes-wifi")}</h2>
                    <p class="pagina-sub">{t!("estado-redes-wifi-sub")}</p>
                </div>
                {con_wps}
            </div>
            <Tabla
                columnas=vec![
                    Columna::new("SSID"),
                    Columna::new(t!("estado-col-banda")),
                    Columna::new(t!("estado-col-estado")),
                    Columna::new(t!("estado-col-seguridad")),
                    Columna::new(t!("estado-col-clave")),
                    Columna::new("WPS"),
                    Columna::numerica(t!("estado-col-asociados")),
                ]
                filas=filas(filas_wifi)
                vacia=t!("estado-sin-redes")
            />
        </section>
    }
    .into_any()
}
