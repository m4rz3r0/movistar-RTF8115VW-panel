//! Server functions de «Ajustes del panel» (`/panel`): credenciales del
//! router, parámetros de la instancia, confirmación del firmware y usuarios
//! del panel.
//!
//! Todas exigen sesión. Las delicadas (cambiar las credenciales del router,
//! confirmar el firmware y todo lo de usuarios) exigen además haber vuelto a
//! escribir la contraseña del panel en los últimos 5 minutos
//! ([`reautenticar`]); sin eso responden 401 con
//! [`crate::auth::ERROR_REAUTENTICACION`]. La contraseña del router nunca
//! sale del servidor.

use ::i18n::Idioma;
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

pub use crate::instalar::InfoRouter;

/// Máximo de servidores DNS preferidos.
pub const MAX_DNS: usize = 4;
/// Zona horaria por defecto.
pub const ZONA_POR_DEFECTO: &str = "UTC";

/// Router configurado (sin la contraseña).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouterPanel {
    pub url: String,
    pub usuario: String,
}

/// Equipo que conoce el router, para elegir los de confianza.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipoLan {
    pub mac: String,
    pub nombre: String,
}

/// Todo lo que muestra la página. Nunca lleva la contraseña del router.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AjustesPanel {
    pub router: RouterPanel,
    pub solo_lectura: bool,
    /// Firmware de la última lectura del router.
    pub firmware: Option<String>,
    pub idioma: Idioma,
    pub zona_horaria: String,
    pub dns_preferidos: Vec<String>,
    pub dispositivos_confianza: Vec<String>,
    pub mantenimiento: Option<String>,
    pub servidor_propio: Option<String>,
    pub usuarios: Vec<String>,
    /// Equipos de la última lectura del router.
    pub equipos: Vec<EquipoLan>,
    /// Usuario de la sesión.
    pub yo: String,
}

/// Parámetros de la instancia que se guardan: solo los presentes (`Some`);
/// cada sección de la página manda los suyos. En `mantenimiento` y
/// `servidor_propio`, vacío quita el valor.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CamposInstancia {
    #[serde(default)]
    pub idioma: Option<Idioma>,
    #[serde(default)]
    pub zona_horaria: Option<String>,
    #[serde(default)]
    pub dns_preferidos: Option<Vec<String>>,
    #[serde(default)]
    pub dispositivos_confianza: Option<Vec<String>>,
    #[serde(default)]
    pub mantenimiento: Option<String>,
    #[serde(default)]
    pub servidor_propio: Option<String>,
}

/// Por qué no valen los parámetros de la instancia.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorInstancia {
    Dns(String),
    DemasiadosDns,
    Mac(String),
    Mantenimiento,
    ZonaHoraria,
    ServidorPropio,
}

/// Valida y normaliza los parámetros presentes: DNS como direcciones
/// IPv4/IPv6 (como mucho [`MAX_DNS`]), MACs en minúsculas
/// `aa:bb:cc:dd:ee:ff`, mantenimiento `HH:MM-HH:MM`, zona horaria `UTC` o
/// `UTC±HH:MM` y servidor propio IP o nombre. Quita espacios y repetidos;
/// vacío es «sin valor» (en la zona, `UTC`).
pub fn normalizar(c: CamposInstancia) -> Result<CamposInstancia, ErrorInstancia> {
    Ok(CamposInstancia {
        idioma: c.idioma,
        zona_horaria: c.zona_horaria.map(|z| normalizar_zona(&z)).transpose()?,
        dns_preferidos: c.dns_preferidos.map(|d| normalizar_dns(&d)).transpose()?,
        dispositivos_confianza: c
            .dispositivos_confianza
            .map(|m| normalizar_macs(&m))
            .transpose()?,
        mantenimiento: c
            .mantenimiento
            .map(|m| {
                let m = m.trim();
                if m.is_empty() || franja_valida(m) {
                    Ok(m.to_owned())
                } else {
                    Err(ErrorInstancia::Mantenimiento)
                }
            })
            .transpose()?,
        servidor_propio: c
            .servidor_propio
            .map(|v| {
                let v = v.trim();
                if v.is_empty() || v.parse::<IpAddr>().is_ok() || nombre_valido(v) {
                    Ok(v.to_owned())
                } else {
                    Err(ErrorInstancia::ServidorPropio)
                }
            })
            .transpose()?,
    })
}

fn normalizar_dns(l: &[String]) -> Result<Vec<String>, ErrorInstancia> {
    let mut dns: Vec<String> = Vec::new();
    for d in l.iter().map(|d| d.trim()).filter(|d| !d.is_empty()) {
        let ip: IpAddr = d.parse().map_err(|_| ErrorInstancia::Dns(d.to_owned()))?;
        let ip = ip.to_string();
        if !dns.contains(&ip) {
            dns.push(ip);
        }
    }
    if dns.len() > MAX_DNS {
        return Err(ErrorInstancia::DemasiadosDns);
    }
    Ok(dns)
}

fn normalizar_macs(l: &[String]) -> Result<Vec<String>, ErrorInstancia> {
    let mut macs: Vec<String> = Vec::new();
    for m in l.iter().map(|m| m.trim()).filter(|m| !m.is_empty()) {
        let mac = normalizar_mac(m).ok_or_else(|| ErrorInstancia::Mac(m.to_owned()))?;
        if !macs.contains(&mac) {
            macs.push(mac);
        }
    }
    Ok(macs)
}

fn normalizar_zona(z: &str) -> Result<String, ErrorInstancia> {
    let z = z.trim();
    if z.is_empty() {
        Ok(ZONA_POR_DEFECTO.to_owned())
    } else if zona_valida(z) {
        Ok(z.to_owned())
    } else {
        Err(ErrorInstancia::ZonaHoraria)
    }
}

/// MAC con `:`, `-`, `.` o sin separadores, en minúsculas y con `:`.
pub fn normalizar_mac(m: &str) -> Option<String> {
    let hex: String = m
        .chars()
        .filter(|c| !matches!(c, ':' | '-' | '.'))
        .collect();
    if hex.len() != 12 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let hex = hex.to_ascii_lowercase();
    Some(
        hex.as_bytes()
            .chunks(2)
            .map(|p| std::str::from_utf8(p).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(":"),
    )
}

/// `HH:MM` con dos cifras cada parte, `HH` < 24 y `MM` < 60. Devuelve los
/// minutos desde medianoche.
fn hora(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    if h.len() != 2 || m.len() != 2 || !(h.bytes().chain(m.bytes())).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// Franja `HH:MM-HH:MM` (puede cruzar la medianoche; no vacía).
fn franja_valida(s: &str) -> bool {
    match s.split_once('-') {
        Some((a, b)) => matches!((hora(a), hora(b)), (Some(a), Some(b)) if a != b),
        None => false,
    }
}

/// `UTC` o `UTC±HH:MM` entre `UTC-12:00` y `UTC+14:00`.
pub fn zona_valida(s: &str) -> bool {
    if s == "UTC" {
        return true;
    }
    let Some(resto) = s.strip_prefix("UTC+").or_else(|| s.strip_prefix("UTC-")) else {
        return false;
    };
    let negativo = s.as_bytes()[3] == b'-';
    match hora(resto) {
        Some(min) => min <= if negativo { 12 * 60 } else { 14 * 60 },
        None => false,
    }
}

/// Zonas horarias que ofrece la página: los desfases en uso, de `UTC-12:00`
/// a `UTC+14:00` (sin cambio de horario: es un desfase fijo).
pub fn zonas() -> Vec<String> {
    const DESFASES: &[i32] = &[
        -720, -660, -600, -570, -540, -480, -420, -360, -300, -240, -210, -180, -120, -60, 0, 60,
        120, 180, 210, 240, 270, 300, 330, 345, 360, 390, 420, 480, 525, 540, 570, 600, 630, 660,
        720, 765, 780, 840,
    ];
    DESFASES
        .iter()
        .map(|&d| {
            if d == 0 {
                ZONA_POR_DEFECTO.to_owned()
            } else {
                let signo = if d < 0 { '-' } else { '+' };
                format!("UTC{signo}{:02}:{:02}", d.abs() / 60, d.abs() % 60)
            }
        })
        .collect()
}

/// Nombre de equipo: etiquetas de letras, cifras y guiones (sin guion al
/// principio ni al final) separadas por puntos, hasta 253 caracteres.
fn nombre_valido(s: &str) -> bool {
    s.len() <= 253
        && s.split('.').all(|e| {
            !e.is_empty()
                && e.len() <= 63
                && !e.starts_with('-')
                && !e.ends_with('-')
                && e.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

/// Todo lo de la página (sin la contraseña del router).
#[server(endpoint = "leer_panel", input = Json)]
pub async fn leer_panel() -> Result<AjustesPanel, ServerFnError> {
    servidor::leer().await
}

/// Prueba unas credenciales del router sin guardarlas.
#[server(endpoint = "panel_probar_router", input = Json)]
pub async fn probar_router(
    url: String,
    usuario: String,
    contrasena: String,
) -> Result<InfoRouter, ServerFnError> {
    let (state, _) = servidor::preparar().await?;
    crate::instalar::servidor::probar(&state, &url, &usuario, &contrasena)
        .await
        .map(|(info, _)| info)
}

/// Prueba las credenciales del router y, si entran, las guarda y el panel
/// pasa a usarlas. No con un cambio en curso. Exige reautenticación.
#[server(endpoint = "panel_guardar_router", input = Json)]
pub async fn guardar_router(
    url: String,
    usuario: String,
    contrasena: String,
) -> Result<InfoRouter, ServerFnError> {
    let (state, _) = servidor::preparar_delicada().await?;
    crate::instalar::servidor::guardar(&state, &url, &usuario, &contrasena).await
}

/// Guarda los parámetros de la instancia (validados).
#[server(endpoint = "guardar_instancia", input = Json)]
pub async fn guardar_instancia(campos: CamposInstancia) -> Result<CamposInstancia, ServerFnError> {
    servidor::guardar_instancia(campos).await
}

/// Confirma el firmware del router aunque no esté probado: quita el modo
/// solo lectura. Exige reautenticación.
#[server(endpoint = "confirmar_firmware", input = Json)]
pub async fn confirmar_firmware() -> Result<(), ServerFnError> {
    servidor::confirmar_firmware().await
}

/// Da de alta un usuario del panel. Exige reautenticación.
#[server(endpoint = "crear_usuario", input = Json)]
pub async fn crear_usuario(usuario: String, contrasena: String) -> Result<(), ServerFnError> {
    servidor::crear_usuario(usuario, contrasena).await
}

/// Borra un usuario del panel (nunca el último ni el propio). Exige
/// reautenticación.
#[server(endpoint = "borrar_usuario", input = Json)]
pub async fn borrar_usuario(usuario: String) -> Result<(), ServerFnError> {
    servidor::borrar_usuario(usuario).await
}

/// Cambia la contraseña de un usuario del panel y cierra sus sesiones; si es
/// el propio, la respuesta trae una sesión nueva. Exige reautenticación.
#[server(endpoint = "cambiar_contrasena", input = Json)]
pub async fn cambiar_contrasena(usuario: String, contrasena: String) -> Result<(), ServerFnError> {
    servidor::cambiar_contrasena(usuario, contrasena).await
}

/// Vuelve a pedir la contraseña del panel (mismo freno que el login) y abre
/// la ventana de 5 minutos de las operaciones delicadas.
#[server(endpoint = "reautenticar", input = Json)]
pub async fn reautenticar(contrasena: String) -> Result<(), ServerFnError> {
    servidor::preparar().await?;
    crate::auth::reautenticar_sesion(&contrasena).await
}

#[cfg(feature = "ssr")]
mod servidor {
    use super::*;
    use crate::{
        auth::{self, UserError},
        instalar::servidor::{fallo, interno},
        instancia::{self, claves},
        state::AppState,
        t,
    };
    use axum::http::{HeaderValue, StatusCode, header};
    use leptos_axum::ResponseOptions;
    use serde_json::Value;

    /// Respuesta `no-store` y estado del panel.
    fn estado() -> Result<AppState, ServerFnError> {
        if let Some(resp) = use_context::<ResponseOptions>() {
            resp.insert_header(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        }
        use_context::<AppState>().ok_or_else(|| interno("falta el estado del panel"))
    }

    /// Estado del panel y usuario de la sesión (401 sin ella).
    pub(super) async fn preparar() -> Result<(AppState, auth::User), ServerFnError> {
        let state = estado()?;
        Ok((state, auth::require_user().await?))
    }

    /// Como [`preparar`], con la ventana de reautenticación abierta.
    pub(super) async fn preparar_delicada() -> Result<(AppState, auth::User), ServerFnError> {
        let state = estado()?;
        Ok((state, auth::require_reauth().await?))
    }

    fn texto(db: Option<Value>) -> Option<String> {
        match db {
            Some(Value::String(s)) if !s.is_empty() => Some(s),
            _ => None,
        }
    }

    fn lista(db: Option<Value>) -> Vec<String> {
        db.and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    pub(super) async fn leer() -> Result<AjustesPanel, ServerFnError> {
        let (state, yo) = preparar().await?;
        let yo = yo.username;
        let db = &state.db;
        let router = instancia::leer_credenciales(db, &state.history_key)
            .await
            .map_err(interno)?
            .map(|c| RouterPanel {
                url: c.url,
                usuario: c.usuario,
            })
            .unwrap_or_default();
        let usuarios: Vec<String> = sqlx::query_scalar("SELECT username FROM users ORDER BY id")
            .fetch_all(db)
            .await
            .map_err(interno)?;
        let foto = state.poller.latest();
        let firmware = foto
            .as_ref()
            .map(|s| s.summary.firmware.clone())
            .filter(|f| !f.is_empty());
        let mut equipos: Vec<EquipoLan> = Vec::new();
        for h in foto.iter().flat_map(|s| &s.hosts) {
            if let Some(mac) = normalizar_mac(&h.mac)
                && !equipos.iter().any(|e| e.mac == mac)
            {
                equipos.push(EquipoLan {
                    mac,
                    nombre: h.hostname.clone(),
                });
            }
        }
        let idioma = instancia::leer(db, claves::IDIOMA)
            .await
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        Ok(AjustesPanel {
            router,
            solo_lectura: instancia::solo_lectura(db).await,
            firmware,
            idioma,
            zona_horaria: texto(instancia::leer(db, claves::ZONA_HORARIA).await)
                .unwrap_or_else(|| ZONA_POR_DEFECTO.to_owned()),
            dns_preferidos: lista(instancia::leer(db, claves::DNS_PREFERIDOS).await),
            dispositivos_confianza: lista(
                instancia::leer(db, claves::DISPOSITIVOS_CONFIANZA).await,
            ),
            mantenimiento: texto(instancia::leer(db, claves::MANTENIMIENTO).await),
            servidor_propio: texto(instancia::leer(db, claves::SERVIDOR_PROPIO).await),
            usuarios,
            equipos,
            yo,
        })
    }

    pub(super) async fn guardar_instancia(
        campos: CamposInstancia,
    ) -> Result<CamposInstancia, ServerFnError> {
        let (state, _) = preparar().await?;
        let c = normalizar(campos).map_err(|e| {
            let msg = match e {
                ErrorInstancia::Dns(d) => t!("panel-error-dns", valor = d),
                ErrorInstancia::DemasiadosDns => t!("panel-error-demasiados-dns", n = MAX_DNS),
                ErrorInstancia::Mac(m) => t!("panel-error-mac", valor = m),
                ErrorInstancia::Mantenimiento => t!("panel-error-mantenimiento"),
                ErrorInstancia::ZonaHoraria => t!("panel-error-zona"),
                ErrorInstancia::ServidorPropio => t!("panel-error-servidor"),
            };
            fallo(StatusCode::BAD_REQUEST, msg)
        })?;
        // Solo lo que llegó: lo que no, queda como estaba.
        let texto = |v: &str| {
            if v.is_empty() {
                Value::Null
            } else {
                Value::String(v.to_owned())
            }
        };
        let mut valores: Vec<(&str, Value)> = Vec::new();
        if let Some(i) = c.idioma {
            valores.push((claves::IDIOMA, serde_json::to_value(i).map_err(interno)?));
        }
        if let Some(z) = &c.zona_horaria {
            valores.push((claves::ZONA_HORARIA, Value::String(z.clone())));
        }
        if let Some(d) = &c.dns_preferidos {
            valores.push((claves::DNS_PREFERIDOS, serde_json::json!(d)));
        }
        if let Some(m) = &c.dispositivos_confianza {
            valores.push((claves::DISPOSITIVOS_CONFIANZA, serde_json::json!(m)));
        }
        if let Some(m) = &c.mantenimiento {
            valores.push((claves::MANTENIMIENTO, texto(m)));
        }
        if let Some(v) = &c.servidor_propio {
            valores.push((claves::SERVIDOR_PROPIO, texto(v)));
        }
        instancia::escribir_varios(&state.db, &valores)
            .await
            .map_err(interno)?;
        if let Some(i) = c.idioma {
            state.fijar_idioma_por_defecto(i);
        }
        tracing::info!("parámetros de la instancia guardados");
        Ok(c)
    }

    pub(super) async fn confirmar_firmware() -> Result<(), ServerFnError> {
        let (state, _) = preparar_delicada().await?;
        // La versión que tiene ahora el router: solo esa queda confirmada.
        let firmware = hgu::read::summary(&state.hgu)
            .await
            .map_err(|e| {
                fallo(
                    StatusCode::BAD_GATEWAY,
                    t!("instalar-error-router", error = e.to_string()),
                )
            })?
            .firmware;
        if firmware.is_empty() {
            return Err(fallo(
                StatusCode::BAD_GATEWAY,
                t!("instalar-error-router", error = "SWVersion"),
            ));
        }
        instancia::escribir_varios(
            &state.db,
            &[
                (claves::FIRMWARE_CONFIRMADO, Value::String(firmware.clone())),
                (claves::SOLO_LECTURA, Value::Bool(false)),
            ],
        )
        .await
        .map_err(interno)?;
        tracing::info!("firmware del router {firmware} confirmado: fuera el modo solo lectura");
        Ok(())
    }

    fn error_usuario(e: UserError) -> ServerFnError {
        match e {
            UserError::InvalidUsername => {
                fallo(StatusCode::BAD_REQUEST, t!("instalar-error-usuario"))
            }
            UserError::PasswordTooShort => fallo(
                StatusCode::BAD_REQUEST,
                t!(
                    "instalar-error-contrasena-corta",
                    n = auth::MIN_PASSWORD_CHARS
                ),
            ),
            UserError::PasswordTooLong => fallo(
                StatusCode::BAD_REQUEST,
                t!(
                    "instalar-error-contrasena-larga",
                    n = auth::MAX_PASSWORD_BYTES
                ),
            ),
            UserError::Exists => fallo(StatusCode::CONFLICT, t!("panel-error-usuario-existe")),
            UserError::NotFound => {
                fallo(StatusCode::NOT_FOUND, t!("panel-error-usuario-no-existe"))
            }
            UserError::LastUser => fallo(StatusCode::BAD_REQUEST, t!("panel-error-ultimo-usuario")),
            UserError::OwnUser => fallo(StatusCode::BAD_REQUEST, t!("panel-error-usuario-propio")),
            e @ UserError::Db(_) => interno(e),
        }
    }

    pub(super) async fn crear_usuario(
        usuario: String,
        contrasena: String,
    ) -> Result<(), ServerFnError> {
        let (state, _) = preparar_delicada().await?;
        auth::create_user(&state.db, &usuario, &contrasena)
            .await
            .map_err(error_usuario)?;
        tracing::info!("usuario del panel creado: {usuario}");
        Ok(())
    }

    pub(super) async fn borrar_usuario(usuario: String) -> Result<(), ServerFnError> {
        let (state, yo) = preparar_delicada().await?;
        auth::delete_user(&state.db, &usuario, yo.id)
            .await
            .map_err(error_usuario)?;
        tracing::info!("usuario del panel borrado: {usuario} (por {})", yo.username);
        Ok(())
    }

    pub(super) async fn cambiar_contrasena(
        usuario: String,
        contrasena: String,
    ) -> Result<(), ServerFnError> {
        let (state, yo) = preparar_delicada().await?;
        if usuario == yo.username {
            // Ya autenticado: sesión nueva sin pasar por el freno del login
            // (un usuario frenado por otros no se queda fuera).
            let id = auth::change_own_password(&state.db, &yo, &contrasena)
                .await
                .map_err(error_usuario)?;
            auth::poner_cookie_de_sesion(&id);
        } else {
            auth::set_password(&state.db, &usuario, &contrasena)
                .await
                .map_err(error_usuario)?;
        }
        tracing::info!(
            "contraseña del usuario del panel {usuario} cambiada; sus sesiones, cerradas"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn campos() -> CamposInstancia {
        CamposInstancia::default()
    }

    #[test]
    fn macs_normalizadas() {
        for m in [
            "AA:BB:CC:00:11:22",
            "aa-bb-cc-00-11-22",
            "aabb.cc00.1122",
            "AABBCC001122",
        ] {
            assert_eq!(
                normalizar_mac(m).as_deref(),
                Some("aa:bb:cc:00:11:22"),
                "{m}"
            );
        }
        for m in [
            "",
            "aa:bb:cc:00:11",
            "aa:bb:cc:00:11:2g",
            "aa:bb:cc:00:11:22:33",
        ] {
            assert_eq!(normalizar_mac(m), None, "{m}");
        }
    }

    #[test]
    fn zonas_horarias() {
        for z in [
            "UTC",
            "UTC+01:00",
            "UTC-03:30",
            "UTC+05:45",
            "UTC+14:00",
            "UTC-12:00",
        ] {
            assert!(zona_valida(z), "{z}");
        }
        for z in [
            "",
            "utc",
            "Europe/Madrid",
            "UTC+1",
            "UTC+14:30",
            "UTC-12:30",
            "UTC+01:60",
            "UTC 01:00",
        ] {
            assert!(!zona_valida(z), "{z}");
        }
        let todas = zonas();
        assert!(todas.iter().all(|z| zona_valida(z)), "{todas:?}");
        assert!(todas.contains(&"UTC+01:00".to_owned()));
        assert!(todas.contains(&"UTC".to_owned()));
    }

    #[test]
    fn franjas_de_mantenimiento() {
        for f in ["03:00-05:30", "23:00-01:00", "00:00-23:59"] {
            assert!(franja_valida(f), "{f}");
        }
        for f in [
            "3:00-5:00",
            "03:00-03:00",
            "24:00-01:00",
            "03:00",
            "03:00-05:60",
            "aa:bb-cc:dd",
        ] {
            assert!(!franja_valida(f), "{f}");
        }
    }

    #[test]
    fn normaliza_y_valida() {
        let c = CamposInstancia {
            zona_horaria: Some(" ".into()),
            dns_preferidos: Some(vec![
                " 9.9.9.9 ".into(),
                "".into(),
                "9.9.9.9".into(),
                "2620:FE::FE".into(),
            ]),
            servidor_propio: Some(" nas.home.arpa ".into()),
            mantenimiento: Some(" ".into()),
            ..campos()
        };
        let n = normalizar(c).unwrap();
        assert_eq!(
            n.dns_preferidos.as_deref(),
            Some(&["9.9.9.9".to_owned(), "2620:fe::fe".to_owned()][..])
        );
        assert_eq!(n.zona_horaria.as_deref(), Some("UTC"));
        assert_eq!(n.servidor_propio.as_deref(), Some("nas.home.arpa"));
        assert_eq!(n.mantenimiento.as_deref(), Some(""), "vacío: quitar");
        // Lo que no llega sigue sin llegar.
        assert_eq!(n.idioma, None);
        assert_eq!(n.dispositivos_confianza, None);
        assert_eq!(normalizar(campos()), Ok(campos()));

        let c = CamposInstancia {
            dns_preferidos: Some(
                [
                    "1.1.1.1",
                    "1.0.0.1",
                    "9.9.9.9",
                    "149.112.112.112",
                    "194.242.2.2",
                ]
                .map(String::from)
                .to_vec(),
            ),
            ..campos()
        };
        assert_eq!(normalizar(c), Err(ErrorInstancia::DemasiadosDns));
        for s in ["-nas.home.arpa", "nas..home.arpa", "http://nas", "nas lan"] {
            let c = CamposInstancia {
                servidor_propio: Some(s.into()),
                ..campos()
            };
            assert_eq!(normalizar(c), Err(ErrorInstancia::ServidorPropio), "{s}");
        }
        let c = CamposInstancia {
            servidor_propio: Some("192.168.1.74".into()),
            ..campos()
        };
        assert!(normalizar(c).is_ok());
    }
}
