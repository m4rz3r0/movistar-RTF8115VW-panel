//! Datos de la página de estado: tipos que viajan al navegador, server
//! functions (`estado_actual`, `series`, `cortes`) y, en el servidor, su
//! construcción a partir de la instantánea del sondeo.
//!
//! Los tipos de aquí no dependen de `hgu` (que solo existe en el servidor) y
//! nunca llevan secretos: de las redes WiFi solo se envía si tienen clave, no
//! la clave.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Intervalo de las gráficas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rango {
    /// Última hora.
    #[default]
    Hora,
    /// Últimas 24 h.
    Dia,
    /// Últimos 7 d.
    Semana,
}

impl Rango {
    pub const TODOS: [Rango; 3] = [Rango::Hora, Rango::Dia, Rango::Semana];

    /// Duración en segundos.
    pub fn segundos(self) -> i64 {
        match self {
            Rango::Hora => 3600,
            Rango::Dia => 24 * 3600,
            Rango::Semana => 7 * 24 * 3600,
        }
    }

    /// Texto del selector.
    pub fn etiqueta(self) -> &'static str {
        match self {
            Rango::Hora => "1 h",
            Rango::Dia => "24 h",
            Rango::Semana => "7 d",
        }
    }

    /// Texto largo («última hora», …).
    pub fn descripcion(self) -> &'static str {
        match self {
            Rango::Hora => "última hora",
            Rango::Dia => "últimas 24 h",
            Rango::Semana => "últimos 7 días",
        }
    }
}

/// Gravedad de un aviso.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Nivel {
    Info,
    Aviso,
    Error,
}

/// Aviso informativo derivado de los datos.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aviso {
    pub nivel: Nivel,
    pub texto: String,
}

/// Equipo de la LAN.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostVista {
    pub ip: String,
    pub mac: String,
    pub nombre: String,
    pub activo: bool,
    /// Tiene reserva DHCP fija (por MAC).
    pub fijo: bool,
    /// «WiFi 5 GHz · SSID» si está asociado a una red del router; «Cable» si
    /// está activo y no lo está; `None` si está inactivo.
    pub interfaz: Option<String>,
}

/// Concesión DHCP o reserva fija.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseVista {
    pub ip: String,
    pub mac: String,
    pub nombre: String,
    /// Reserva fija.
    pub fijo: bool,
    /// Una reserva fija puede estar desactivada.
    pub activa: bool,
    /// Segundos que le quedaban a la concesión en la lectura.
    pub caduca_s: Option<u64>,
}

/// Red WiFi (SSID) sin su clave.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedWifiVista {
    pub ssid: String,
    /// «2,4 GHz» o «5 GHz».
    pub banda: String,
    /// Canal del radio («auto» o el número).
    pub canal: String,
    /// La red y su radio están activados.
    pub activa: bool,
    /// El radio está apagado (la red no emite aunque esté activada).
    pub radio_apagado: bool,
    /// Autenticación y cifrado legibles.
    pub seguridad: String,
    /// Tiene clave (nunca se envía cuál).
    pub con_clave: bool,
    pub wps: bool,
    /// Estaciones asociadas.
    pub asociados: u32,
}

/// Estado actual del router para la página.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EstadoVista {
    /// Hora del servidor al responder.
    pub ahora: i64,
    /// Hora del último dato correcto; `None` si aún no hay ninguno.
    pub at: Option<i64>,
    /// El sondeo tiene datos recientes y sin error.
    pub router_ok: bool,
    /// Error del último intento de lectura.
    pub error: Option<String>,
    pub modelo: String,
    pub firmware: String,
    /// Conexión PPP principal (`ip2`) levantada.
    pub internet_up: bool,
    pub ip_publica: Option<String>,
    /// Desde cuándo hay conexión, si el panel vio el último corte cerrarse
    /// después del arranque del router.
    pub conexion_desde: Option<i64>,
    /// Desde cuándo está caída la conexión (corte abierto).
    pub caida_desde: Option<i64>,
    /// Arranque del router (hora del dato menos su tiempo encendido).
    pub encendido_desde: Option<i64>,
    /// Carga media a 1, 5 y 15 min.
    pub load: (f32, f32, f32),
    pub ram_pct: f32,
    pub ram_total_kb: u64,
    /// Equipos LAN activos.
    pub clientes: u32,
    /// De ellos, asociados a la WiFi del router.
    pub clientes_wifi: u32,
    pub hosts: Vec<HostVista>,
    pub leases: Vec<LeaseVista>,
    pub wifi: Vec<RedWifiVista>,
    /// Hora de los datos de clientes y WiFi.
    pub lentos_at: Option<i64>,
    pub avisos: Vec<Aviso>,
}

/// Muestra de las series.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleVista {
    pub ts: i64,
    pub rx_bps: Option<f64>,
    pub tx_bps: Option<f64>,
    pub load1: f32,
    pub ram_used_pct: f32,
    pub clients: Option<u32>,
    pub wan_up: bool,
}

/// Corte de la conexión: `hasta = None` si sigue caída.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorteVista {
    pub desde: i64,
    pub hasta: Option<i64>,
}

/// Estado actual del router (última instantánea del sondeo).
#[server(endpoint = "estado_actual")]
pub async fn estado_actual() -> Result<EstadoVista, ServerFnError> {
    crate::auth::require_user().await?;
    let state = servidor::app_state()?;
    let p = &state.poller;
    let ultimo = crate::series::ultimo_corte(&state.db)
        .await
        .map_err(servidor::interno)?;
    Ok(servidor::vista(
        p.latest(),
        p.error(),
        p.healthy(),
        ultimo,
        &state.catalogo,
        crate::series::now(),
    ))
}

/// Muestras del `rango` (hasta ahora).
#[server(endpoint = "series")]
pub async fn series(rango: Rango) -> Result<Vec<SampleVista>, ServerFnError> {
    crate::auth::require_user().await?;
    let state = servidor::app_state()?;
    let muestras = crate::series::query(&state.db, servidor::rango(rango))
        .await
        .map_err(servidor::interno)?;
    Ok(muestras
        .into_iter()
        .map(|s| SampleVista {
            ts: s.ts,
            rx_bps: s.rx_bps,
            tx_bps: s.tx_bps,
            load1: s.load1,
            ram_used_pct: s.ram_used_pct,
            clients: s.clients,
            wan_up: s.wan_up,
        })
        .collect())
}

/// Cortes de la conexión que tocan el `rango`.
#[server(endpoint = "cortes")]
pub async fn cortes(rango: Rango) -> Result<Vec<CorteVista>, ServerFnError> {
    crate::auth::require_user().await?;
    let state = servidor::app_state()?;
    let cortes = crate::series::cortes(&state.db, servidor::rango(rango))
        .await
        .map_err(servidor::interno)?;
    Ok(cortes
        .into_iter()
        .map(|c| CorteVista {
            desde: c.desde,
            hasta: c.hasta,
        })
        .collect())
}

#[cfg(feature = "ssr")]
pub(crate) mod servidor {
    use super::*;
    use crate::{
        poller::Snapshot,
        series::WanEvento,
        state::AppState,
        ui::charts::{formato_fecha_hora, formato_hora},
    };
    use catalogo::Catalogo;
    use hgu::models::WifiStatus;
    use std::{collections::HashSet, fmt, net::Ipv4Addr};

    /// Conexión PPP principal.
    const WAN: &str = "ip2";

    pub fn app_state() -> Result<AppState, ServerFnError> {
        use_context::<AppState>().ok_or_else(|| ServerFnError::new("falta el estado del panel"))
    }

    /// Registra el error con un id y devuelve un mensaje genérico con ese id.
    pub fn interno(e: impl fmt::Display) -> ServerFnError {
        let mut id = [0u8; 4];
        getrandom::fill(&mut id).expect("generador aleatorio del sistema");
        let id: String = id.iter().map(|b| format!("{b:02x}")).collect();
        tracing::error!(id, "error interno en la página de estado: {e}");
        if let Some(resp) = use_context::<leptos_axum::ResponseOptions>() {
            resp.set_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
        ServerFnError::new(format!("Error interno del panel (id {id})"))
    }

    pub fn rango(r: Rango) -> crate::series::Rango {
        match r {
            Rango::Hora => crate::series::Rango::Hora,
            Rango::Dia => crate::series::Rango::Dia,
            Rango::Semana => crate::series::Rango::Semana,
        }
    }

    /// «las 14:05» si `ts` es de las últimas 20 h; «el 08/10 14:05» si no.
    fn cuando(ts: i64, ahora: i64) -> String {
        if ahora - ts < 20 * 3600 {
            format!("las {}", formato_hora(ts, false))
        } else {
            format!("el {}", formato_fecha_hora(ts, false))
        }
    }

    /// Etiqueta legible de un parámetro enumerado de `cbSetWifiAccPt` según el
    /// catálogo, o el código si no está.
    fn opcion(cat: &Catalogo, param: &str, codigo: &str) -> String {
        cat.funcion("cbSetWifiAccPt")
            .and_then(|f| f.parametro(param))
            .and_then(|p| p.opciones.iter().find(|(v, _)| v == codigo))
            .map_or_else(|| format!("código {codigo}"), |(_, e)| e.clone())
    }

    /// Seguridad legible (`"3/4"` ⇒ «WPA2-PSK · AES») y si la red tiene clave
    /// (autenticación con clave o WEP).
    fn seguridad(cat: &Catalogo, codigos: &str) -> (String, bool) {
        let (auth, enc) = codigos.split_once('/').unwrap_or((codigos, ""));
        let wep = matches!(enc, "1" | "2");
        if auth == "0" && !wep {
            return ("Abierta".to_owned(), false);
        }
        let mut texto = opcion(cat, "authentication", auth);
        if !enc.is_empty() && enc != "0" {
            texto = format!("{texto} · {}", opcion(cat, "encryption", enc));
        }
        (texto, true)
    }

    /// «2.4GHz» ⇒ «2,4 GHz».
    fn banda(b: &str) -> String {
        b.replace("GHz", " GHz").replace('.', ",")
    }

    fn misma_mac(a: &str, b: &str) -> bool {
        a.eq_ignore_ascii_case(b)
    }

    /// Clave de orden por IP (las que no son IPv4, al final).
    fn orden_ip(ip: &str) -> (u32, String) {
        ip.parse::<Ipv4Addr>()
            .map_or((u32::MAX, ip.to_owned()), |a| (u32::from(a), String::new()))
    }

    /// «WiFi 5 GHz · SSID» de la estación `mac`, si está asociada.
    fn interfaz_wifi(wifi: &WifiStatus, mac: &str) -> Option<String> {
        let st = wifi.stations.iter().find(|s| misma_mac(&s.mac, mac))?;
        let ap = wifi.aps.iter().find(|a| a.id == st.ap_id);
        let radio = ap.and_then(|ap| wifi.radios.iter().find(|r| r.id == ap.radio_id));
        Some(match (radio, ap) {
            (Some(r), Some(ap)) => format!("WiFi {} · {}", banda(&r.band), ap.ssid),
            (None, Some(ap)) => format!("WiFi · {}", ap.ssid),
            _ => "WiFi".to_owned(),
        })
    }

    /// Construye la vista a partir de la instantánea del sondeo (`snap`), el
    /// error del último ciclo, si el sondeo está sano y el último corte de la
    /// WAN registrado.
    pub fn vista(
        snap: Option<Snapshot>,
        error: Option<String>,
        sano: bool,
        ultimo_corte: Option<WanEvento>,
        cat: &Catalogo,
        ahora: i64,
    ) -> EstadoVista {
        let mut avisos = Vec::new();
        let Some(s) = snap else {
            if let Some(e) = &error {
                avisos.push(Aviso {
                    nivel: Nivel::Error,
                    texto: format!("Sin conexión con el router: {e}"),
                });
            }
            return EstadoVista {
                ahora,
                at: None,
                router_ok: false,
                error,
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
                avisos,
            };
        };

        if !sano {
            let detalle = error
                .as_deref()
                .map(|e| format!(": {e}"))
                .unwrap_or_default();
            avisos.push(Aviso {
                nivel: Nivel::Error,
                texto: format!(
                    "Sin conexión con el router desde {}{detalle}. Los datos mostrados son de esa hora.",
                    cuando(s.at, ahora)
                ),
            });
        }
        if s.slow_error.is_some() {
            let desde = s
                .slow_at
                .map(|t| format!(" (último dato: {})", cuando(t, ahora)))
                .unwrap_or_default();
            avisos.push(Aviso {
                nivel: Nivel::Aviso,
                texto: format!("Los datos de clientes y WiFi pueden estar desactualizados{desde}."),
            });
        }

        let wan = s.wan.iter().find(|w| w.name == WAN);
        let internet_up = wan.is_some_and(|w| w.up);
        let encendido_desde = s.at - i64::try_from(s.summary.uptime_s).unwrap_or(i64::MAX);
        let (conexion_desde, caida_desde) = match ultimo_corte {
            Some(c) if !internet_up => (None, c.hasta.is_none().then_some(c.desde)),
            Some(WanEvento {
                hasta: Some(hasta), ..
            }) if internet_up && hasta >= encendido_desde => (Some(hasta), None),
            _ => (None, None),
        };

        let sm = &s.summary;
        let ram_pct = if sm.ram_total_kb == 0 {
            0.0
        } else {
            sm.ram_total_kb.saturating_sub(sm.ram_free_kb) as f32 / sm.ram_total_kb as f32 * 100.0
        };

        // Nombre del equipo: el de la lista de hosts o, si no lo trae, el de
        // su concesión.
        let nombre_de = |mac: &str| {
            let hosts = s.hosts.iter().filter(|h| misma_mac(&h.mac, mac));
            let leases = s.leases.iter().filter(|l| misma_mac(&l.mac, mac));
            hosts
                .map(|h| &h.hostname)
                .chain(leases.map(|l| &l.hostname))
                .find(|n| !n.is_empty())
                .cloned()
                .unwrap_or_default()
        };
        let fija = |mac: &str| s.statics.iter().any(|st| misma_mac(&st.mac, mac));
        let mut hosts: Vec<HostVista> = s
            .hosts
            .iter()
            .map(|h| HostVista {
                ip: h.ip.clone(),
                mac: h.mac.clone(),
                nombre: nombre_de(&h.mac),
                activo: h.active,
                fijo: fija(&h.mac),
                interfaz: h
                    .active
                    .then(|| interfaz_wifi(&s.wifi, &h.mac).unwrap_or_else(|| "Cable".into())),
            })
            .collect();
        hosts.sort_by_key(|h| (!h.activo, orden_ip(&h.ip)));
        let clientes = hosts.iter().filter(|h| h.activo).count();
        let clientes_wifi = hosts
            .iter()
            .filter(|h| h.activo && h.interfaz.as_deref().is_some_and(|i| i.starts_with("WiFi")))
            .count();

        let mut leases: Vec<LeaseVista> = s
            .statics
            .iter()
            .map(|st| LeaseVista {
                ip: st.ip.clone(),
                mac: st.mac.clone(),
                nombre: nombre_de(&st.mac),
                fijo: true,
                activa: st.enabled,
                caduca_s: s
                    .leases
                    .iter()
                    .find(|l| misma_mac(&l.mac, &st.mac))
                    .and_then(|l| l.expires_s),
            })
            .collect();
        let mut vistas: HashSet<String> = leases.iter().map(|l| l.mac.to_lowercase()).collect();
        for l in &s.leases {
            if vistas.insert(l.mac.to_lowercase()) {
                leases.push(LeaseVista {
                    ip: l.ip.clone(),
                    mac: l.mac.clone(),
                    nombre: nombre_de(&l.mac),
                    fijo: false,
                    activa: true,
                    caduca_s: l.expires_s,
                });
            }
        }
        leases.sort_by_key(|l| orden_ip(&l.ip));

        let wifi: Vec<RedWifiVista> = s
            .wifi
            .aps
            .iter()
            .map(|ap| {
                let radio = s.wifi.radios.iter().find(|r| r.id == ap.radio_id);
                let radio_apagado = radio.is_some_and(|r| !r.enabled);
                let (seguridad, con_clave) = seguridad(cat, &ap.security);
                RedWifiVista {
                    ssid: ap.ssid.clone(),
                    banda: radio.map(|r| banda(&r.band)).unwrap_or_default(),
                    canal: radio.map(|r| r.channel.clone()).unwrap_or_default(),
                    activa: ap.enabled && !radio_apagado,
                    radio_apagado,
                    seguridad,
                    con_clave,
                    wps: ap.wps,
                    asociados: u32::try_from(
                        s.wifi
                            .stations
                            .iter()
                            .filter(|st| st.ap_id == ap.id)
                            .count(),
                    )
                    .unwrap_or(u32::MAX),
                }
            })
            .collect();

        let con_wps: Vec<&str> = wifi
            .iter()
            .filter(|r| r.activa && r.wps)
            .map(|r| r.ssid.as_str())
            .collect();
        if !con_wps.is_empty() {
            avisos.push(Aviso {
                nivel: Nivel::Aviso,
                texto: format!(
                    "WPS activo en {}: desactívalo si no lo usas (el PIN de WPS se puede adivinar por fuerza bruta).",
                    con_wps.join(", ")
                ),
            });
        }
        if s.tr69_activo == Some(true) {
            avisos.push(Aviso {
                nivel: Nivel::Info,
                texto: "Telefónica puede gestionar el router en remoto (TR-069).".to_owned(),
            });
        }

        let n = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
        EstadoVista {
            ahora,
            at: Some(s.at),
            router_ok: sano,
            error,
            modelo: sm.model.clone(),
            firmware: sm.firmware.clone(),
            internet_up,
            ip_publica: wan.and_then(|w| w.ipv4).map(|ip| ip.to_string()),
            conexion_desde,
            caida_desde,
            encendido_desde: Some(encendido_desde),
            load: (sm.load1, sm.load5, sm.load15),
            ram_pct,
            ram_total_kb: sm.ram_total_kb,
            clientes: n(clientes),
            clientes_wifi: n(clientes_wifi),
            hosts,
            leases,
            wifi,
            lentos_at: s.slow_at,
            avisos,
        }
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{servidor::vista, *};
    use crate::{poller::Snapshot, series::WanEvento};
    use catalogo::Catalogo;
    use hgu::models::{
        DhcpLease, LanHost, StaticLease, Summary, WanConn, WifiAp, WifiAssoc, WifiRadio, WifiStatus,
    };

    const AHORA: i64 = 1_791_417_600;

    fn host(ip: &str, mac: &str, activo: bool) -> LanHost {
        LanHost {
            ip: ip.into(),
            mac: mac.into(),
            hostname: String::new(),
            iface: "1:0".into(),
            active: activo,
        }
    }

    fn snapshot() -> Snapshot {
        Snapshot {
            at: AHORA - 5,
            summary: Summary {
                model: "RTF8115VW".into(),
                firmware: "fw".into(),
                hw: "REV5".into(),
                uptime_s: 3 * 86_400,
                load1: 0.5,
                load5: 0.4,
                load15: 0.3,
                ram_total_kb: 1000,
                ram_free_kb: 250,
                sys_time: 0,
            },
            wan: vec![WanConn {
                name: "ip2".into(),
                up: true,
                enabled: true,
                wan_type: "0".into(),
                ipv4: Some("203.0.113.6".parse().unwrap()),
            }],
            counters: Vec::new(),
            hosts: vec![
                host("192.168.1.100", "02:00:00:00:00:0B", true),
                host("192.0.2.10", "02:00:00:00:00:0a", true),
                host("192.168.1.9", "02:00:00:00:00:0c", false),
            ],
            leases: vec![
                DhcpLease {
                    ip: "192.168.1.100".into(),
                    mac: "02:00:00:00:00:0b".into(),
                    hostname: String::new(),
                    expires_s: Some(600),
                },
                DhcpLease {
                    ip: "192.0.2.10".into(),
                    mac: "02:00:00:00:00:0a".into(),
                    hostname: "servidor".into(),
                    expires_s: Some(60),
                },
            ],
            statics: vec![StaticLease {
                id: "0".into(),
                mac: "02:00:00:00:00:0A".into(),
                ip: "192.0.2.10".into(),
                enabled: true,
            }],
            wifi: WifiStatus {
                radios: vec![
                    WifiRadio {
                        id: "0".into(),
                        band: "2.4GHz".into(),
                        enabled: true,
                        channel: "auto".into(),
                    },
                    WifiRadio {
                        id: "1".into(),
                        band: "5GHz".into(),
                        enabled: false,
                        channel: "100".into(),
                    },
                ],
                aps: vec![
                    WifiAp {
                        id: "0".into(),
                        radio_id: "0".into(),
                        ssid: "casa".into(),
                        enabled: true,
                        security: "3/4".into(),
                        wps: true,
                    },
                    WifiAp {
                        id: "1".into(),
                        radio_id: "1".into(),
                        ssid: "casa-5".into(),
                        enabled: true,
                        security: "0/0".into(),
                        wps: true,
                    },
                ],
                stations: vec![WifiAssoc {
                    ap_id: "0".into(),
                    mac: "02:00:00:00:00:0b".into(),
                    rssi: Some(-50),
                }],
            },
            tr69_activo: Some(false),
            slow_at: Some(AHORA - 30),
            error: None,
            slow_error: None,
        }
    }

    fn cat() -> Catalogo {
        Catalogo::load().unwrap()
    }

    #[test]
    fn clientes_leases_y_wifi() {
        let v = vista(Some(snapshot()), None, true, None, &cat(), AHORA);
        assert!(v.router_ok);
        assert_eq!((v.clientes, v.clientes_wifi), (2, 1));
        assert_eq!(v.ram_pct, 75.0);
        // Activos primero, por IP numérica.
        let ips: Vec<&str> = v.hosts.iter().map(|h| h.ip.as_str()).collect();
        assert_eq!(ips, ["192.0.2.10", "192.168.1.100", "192.168.1.9"]);
        assert!(v.hosts[0].fijo, "la MAC se compara sin mayúsculas");
        assert_eq!(v.hosts[0].nombre, "servidor");
        assert_eq!(v.hosts[0].interfaz.as_deref(), Some("Cable"));
        assert_eq!(v.hosts[1].interfaz.as_deref(), Some("WiFi 2,4 GHz · casa"));
        assert_eq!(v.hosts[2].interfaz, None);
        // La reserva fija y su concesión son una sola fila.
        assert_eq!(v.leases.len(), 2);
        let fija = &v.leases[0];
        assert!(fija.fijo && fija.activa);
        // Sin nombre en la lista de hosts, el de la concesión.
        assert_eq!(
            (fija.nombre.as_str(), fija.caduca_s),
            ("servidor", Some(60))
        );
        assert!(!v.leases[1].fijo);
        // Redes: seguridad legible, sin clave si es abierta, radio apagado.
        assert_eq!(v.wifi[0].seguridad, "WPA2-PSK · AES");
        assert!(v.wifi[0].con_clave && v.wifi[0].activa);
        assert_eq!(v.wifi[0].asociados, 1);
        assert_eq!(v.wifi[1].seguridad, "Abierta");
        assert!(!v.wifi[1].con_clave);
        assert!(v.wifi[1].radio_apagado && !v.wifi[1].activa);
        assert_eq!(v.wifi[1].banda, "5 GHz");
    }

    #[test]
    fn avisos_de_wps_tr69_y_datos_lentos() {
        let mut s = snapshot();
        s.tr69_activo = Some(true);
        s.slow_error = Some("error de red: x".into());
        let v = vista(Some(s), None, true, None, &cat(), AHORA);
        let textos: Vec<&str> = v.avisos.iter().map(|a| a.texto.as_str()).collect();
        assert_eq!(textos.len(), 3, "{textos:?}");
        assert!(textos[0].contains("desactualizados"), "{textos:?}");
        assert!(textos[0].contains("las "), "{textos:?}");
        // Solo la red activa con WPS (la de 5 GHz tiene el radio apagado).
        assert!(textos[1].starts_with("WPS activo en casa:"), "{textos:?}");
        assert!(!textos[1].contains("casa-5"), "{textos:?}");
        assert_eq!(
            textos[2],
            "Telefónica puede gestionar el router en remoto (TR-069)."
        );
        assert_eq!(v.avisos[2].nivel, Nivel::Info);
    }

    #[test]
    fn router_caido_con_datos_avisa_con_la_hora_del_ultimo() {
        let v = vista(
            Some(snapshot()),
            Some("error de red: tiempo agotado".into()),
            false,
            None,
            &cat(),
            AHORA,
        );
        assert!(!v.router_ok);
        assert_eq!(v.at, Some(AHORA - 5));
        let a = &v.avisos[0];
        assert_eq!(a.nivel, Nivel::Error);
        let hora = crate::ui::charts::formato_hora(AHORA - 5, false);
        assert!(
            a.texto
                .starts_with(&format!("Sin conexión con el router desde las {hora}")),
            "{}",
            a.texto
        );
        assert!(a.texto.contains("tiempo agotado"), "{}", a.texto);
    }

    #[test]
    fn sin_datos() {
        let v = vista(None, None, false, None, &cat(), AHORA);
        assert_eq!((v.at, v.error.clone()), (None, None));
        assert!(v.avisos.is_empty());
        let v = vista(
            None,
            Some("error de red: x".into()),
            false,
            None,
            &cat(),
            AHORA,
        );
        assert_eq!(
            v.avisos[0].texto,
            "Sin conexión con el router: error de red: x"
        );
    }

    #[test]
    fn tiempo_de_conexion() {
        let arranque = AHORA - 5 - 3 * 86_400;
        let con = |corte: Option<WanEvento>, up: bool| {
            let mut s = snapshot();
            s.wan[0].up = up;
            let v = vista(Some(s), None, true, corte, &cat(), AHORA);
            (v.conexion_desde, v.caida_desde, v.encendido_desde)
        };
        // Sin cortes registrados: solo el arranque del router.
        assert_eq!(con(None, true), (None, None, Some(arranque)));
        // Corte cerrado después del arranque: conectado desde que acabó.
        let cerrado = WanEvento {
            desde: AHORA - 600,
            hasta: Some(AHORA - 300),
        };
        assert_eq!(
            con(Some(cerrado.clone()), true),
            (Some(AHORA - 300), None, Some(arranque))
        );
        // Corte anterior al arranque: no dice nada de la conexión actual.
        let viejo = WanEvento {
            desde: arranque - 600,
            hasta: Some(arranque - 300),
        };
        assert_eq!(con(Some(viejo), true), (None, None, Some(arranque)));
        // Caída: desde el corte abierto.
        let abierto = WanEvento {
            desde: AHORA - 120,
            hasta: None,
        };
        assert_eq!(
            con(Some(abierto), false),
            (None, Some(AHORA - 120), Some(arranque))
        );
        // Caída sin corte abierto registrado (aún): no se sabe desde cuándo.
        assert_eq!(con(Some(cerrado), false), (None, None, Some(arranque)));
    }
}
