//! Asistente de primera instalación: token de un solo uso, alta del
//! administrador, credenciales del router y lectura inicial («foto de
//! partida»).
//!
//! Sin usuarios en la base de datos, el panel solo sirve `/instalar` (ver
//! `auth::require_page_session`) y sus server functions exigen el token que
//! se genera al arrancar ([`token_inicial`]), y solo desde la red local. El
//! token sirve para crear un único administrador: al crearlo se borra y el
//! resto de pasos piden su sesión. La instalación termina con la lectura
//! inicial; desde entonces (o tras reiniciar el proceso con usuarios) el
//! asistente responde 404.

use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
pub use servidor::{
    Instalacion, en_curso, gw_de_proc_net_route, gw_de_route_get, hay_usuarios,
    lecturas_de_partida, ruta_token, sugerible, token_inicial,
};

/// Lo que se sabe del router al probar sus credenciales.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InfoRouter {
    pub modelo: String,
    pub firmware: String,
    /// El firmware es uno de los probados con el panel.
    pub probado: bool,
}

/// Tipo de la conexión a internet (como `hgu::models::TipoConexion`, que
/// solo existe en el servidor).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoRed {
    Pppoe,
    Dhcp,
    Estatica,
    Otro,
}

/// Red del router detectada, para mostrarla: lo de
/// `hgu::deteccion::RedDetectada` (que solo existe en el servidor) que ve la
/// interfaz, con los mismos nombres de campo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumenRed {
    /// Nombre de la conexión a internet (p. ej. `ip2`); `None` si no se pudo
    /// decidir.
    pub wan: Option<String>,
    pub tipo: Option<TipoRed>,
    /// Subred LAN, `"a.b.c.0/24"`.
    pub lan: Option<String>,
    pub voip: bool,
    pub iptv: bool,
}

#[cfg(feature = "ssr")]
impl From<&hgu::deteccion::RedDetectada> for ResumenRed {
    fn from(red: &hgu::deteccion::RedDetectada) -> Self {
        use hgu::models::TipoConexion;
        ResumenRed {
            wan: red.wan.clone(),
            tipo: red.tipo.map(|t| match t {
                TipoConexion::Pppoe => TipoRed::Pppoe,
                TipoConexion::Dhcp => TipoRed::Dhcp,
                TipoConexion::Estatica => TipoRed::Estatica,
                TipoConexion::Otro => TipoRed::Otro,
            }),
            lan: red.lan.clone(),
            voip: red.voip,
            iptv: red.iptv,
        }
    }
}

/// Resultado de la lectura inicial.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumenFoto {
    pub funciones_leidas: u32,
    /// Funciones cuya lectura falló.
    pub fallidas: Vec<String>,
    /// Red del router detectada; `None` si no se pudo detectar.
    pub red: Option<ResumenRed>,
}

/// En qué punto está la instalación, para retomar el asistente.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EstadoInstalacion {
    pub admin_creado: bool,
    pub router_configurado: bool,
}

/// Paso 1: crea el administrador con el token de instalación (de un solo
/// uso) y abre su sesión.
#[server(endpoint = "crear_admin", input = Json)]
pub async fn crear_admin(
    // Sin el campo, vacío: 403 como un token malo.
    #[server(default)] token: String,
    usuario: String,
    contrasena: String,
) -> Result<(), ServerFnError> {
    servidor::crear_admin(token, usuario, contrasena).await
}

/// Paso 2: dirección del router que se sugiere: `http://` y la puerta de
/// enlace por defecto de este equipo si está en una red privada que no es
/// 172.16/12 (la de Docker y otros contenedores, donde la puerta de
/// enlace no es el router); si no, `None`.
#[server(endpoint = "url_sugerida", input = Json)]
pub async fn url_sugerida(
    #[server(default)] token: String,
) -> Result<Option<String>, ServerFnError> {
    servidor::autorizar(&token).await?;
    Ok(servidor::url_sugerida().await)
}

/// Paso 2: prueba las credenciales del router (sin guardarlas) y dice el
/// modelo y el firmware.
#[server(endpoint = "probar_router", input = Json)]
pub async fn probar_router(
    #[server(default)] token: String,
    url: String,
    usuario: String,
    contrasena: String,
) -> Result<InfoRouter, ServerFnError> {
    let state = servidor::autorizar(&token).await?;
    servidor::probar(&state, &url, &usuario, &contrasena)
        .await
        .map(|(info, _)| info)
}

/// Paso 2: guarda las credenciales del router (si entran) y lo empieza a
/// sondear. Con un firmware no probado, el panel queda en solo lectura.
#[server(endpoint = "guardar_router", input = Json)]
pub async fn guardar_router(
    #[server(default)] token: String,
    url: String,
    usuario: String,
    contrasena: String,
) -> Result<InfoRouter, ServerFnError> {
    let state = servidor::autorizar(&token).await?;
    servidor::guardar(&state, &url, &usuario, &contrasena).await
}

/// Paso 3: lee todas las funciones de lectura del catálogo y guarda la foto
/// de partida. Si lee alguna, la instalación termina.
#[server(endpoint = "lectura_inicial", input = Json)]
pub async fn lectura_inicial(
    #[server(default)] token: String,
) -> Result<ResumenFoto, ServerFnError> {
    let state = servidor::autorizar(&token).await?;
    servidor::lectura_inicial(&state).await
}

/// Punto de la instalación (sin token: no revela nada más).
#[server(endpoint = "fase_instalacion", input = Json)]
pub async fn fase_instalacion() -> Result<EstadoInstalacion, ServerFnError> {
    servidor::fase().await
}

#[cfg(feature = "ssr")]
pub(crate) mod servidor {
    use super::{EstadoInstalacion, InfoRouter, ResumenFoto, ResumenRed};
    use crate::{
        auth,
        config::Config,
        fotos::{self, TipoFoto},
        instancia::{self, CredencialesRouter},
        state::AppState,
        t,
    };
    use axum::{
        extract::ConnectInfo,
        http::{HeaderMap, HeaderValue, StatusCode, header},
    };
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use catalogo::{Catalogo, Funcion, Origen, Tipo};
    use leptos::prelude::*;
    use leptos_axum::ResponseOptions;
    use sha2::{Digest, Sha256};
    use std::{
        io::Write,
        net::{IpAddr, Ipv4Addr, SocketAddr},
        path::{Path, PathBuf},
        sync::atomic::{AtomicBool, Ordering},
    };
    use tokio::sync::Mutex;

    /// Funciones de lectura que no van en la foto: no leen el router sino la
    /// sesión del propio panel. Es por función; `fotos::fuera_de_la_foto`
    /// deja fuera, por ajuste, las listas que comparten función con otra.
    const FUERA_DE_LA_FOTO: &[&str] = &["cbCheckSession"];

    /// Estado de la instalación en memoria (vive en `AppState`).
    #[derive(Default)]
    pub struct Instalacion {
        fase: Mutex<Fase>,
        /// Ya se ha visto algún usuario: no vuelve a `false` (nunca se borra
        /// el último), así que no hace falta preguntar más a la base de datos.
        hay_usuarios: AtomicBool,
    }

    #[derive(Default)]
    struct Fase {
        /// Token para crear el administrador (hasta crearlo).
        token: Option<String>,
        /// Desde el arranque sin usuarios hasta la lectura inicial.
        en_curso: bool,
    }

    /// Fichero del token: `instalar.token` junto a la base de datos.
    pub fn ruta_token(cfg: &Config) -> PathBuf {
        cfg.db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("instalar.token")
    }

    /// ¿Hay algún usuario del panel?
    pub async fn hay_usuarios(state: &AppState) -> Result<bool, sqlx::Error> {
        let inst = &state.instalacion;
        if inst.hay_usuarios.load(Ordering::SeqCst) {
            return Ok(true);
        }
        let hay: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users)")
            .fetch_one(&state.db)
            .await?;
        if hay {
            inst.hay_usuarios.store(true, Ordering::SeqCst);
        }
        Ok(hay)
    }

    /// ¿Sigue en curso la instalación (asistente sin terminar)?
    pub async fn en_curso(state: &AppState) -> bool {
        state.instalacion.fase.lock().await.en_curso
    }

    /// Al arrancar: sin usuarios, genera el token de instalación, lo escribe
    /// en el registro («token de instalación: …») y en [`ruta_token`] (0600)
    /// y lo devuelve. Con usuarios, borra el fichero si quedó de antes y
    /// devuelve `None`.
    pub async fn token_inicial(state: &AppState) -> Option<String> {
        let fichero = ruta_token(&state.cfg);
        match hay_usuarios(state).await {
            Ok(false) => {}
            Ok(true) => {
                borrar_fichero(&fichero);
                return None;
            }
            Err(e) => {
                tracing::error!("no se puede saber si hay usuarios: {e}");
                return None;
            }
        }
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("generador aleatorio del sistema");
        let token = URL_SAFE_NO_PAD.encode(bytes);
        if let Err(e) = escribir_fichero(&fichero, &token) {
            tracing::warn!(
                "no se puede escribir el token de instalación en {}: {e}",
                fichero.display()
            );
        }
        tracing::info!(
            "panel sin usuarios: abre /instalar; token de instalación: {token} (también en {})",
            fichero.display()
        );
        let mut fase = state.instalacion.fase.lock().await;
        fase.token = Some(token.clone());
        fase.en_curso = true;
        Some(token)
    }

    fn escribir_fichero(fichero: &Path, token: &str) -> std::io::Result<()> {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(fichero)?;
        // `mode` solo cuenta al crearlo: uno que ya existía queda en 0600.
        f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        writeln!(f, "{token}")
    }

    fn borrar_fichero(fichero: &Path) {
        match std::fs::remove_file(fichero) {
            Ok(()) => tracing::info!("borrado el token de instalación {}", fichero.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => tracing::warn!("no se puede borrar {}: {e}", fichero.display()),
        }
    }

    /// Compara en tiempo constante (los resúmenes tienen siempre la misma
    /// longitud y se recorren enteros).
    fn mismo_token(dado: &str, esperado: &str) -> bool {
        let (a, b) = (
            Sha256::digest(dado.as_bytes()),
            Sha256::digest(esperado.as_bytes()),
        );
        let distintos = a
            .iter()
            .zip(b.iter())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y));
        !dado.is_empty() && std::hint::black_box(distintos) == 0
    }

    /// Red local: privada, de enlace local o el propio equipo.
    fn ip_local(ip: IpAddr) -> bool {
        match ip.to_canonical() {
            IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
            IpAddr::V6(v6) => {
                let s = v6.segments()[0];
                v6.is_loopback() || (s & 0xfe00) == 0xfc00 || (s & 0xffc0) == 0xfe80
            }
        }
    }

    pub(crate) fn fallo(status: StatusCode, msg: impl Into<String>) -> ServerFnError {
        if let Some(resp) = use_context::<ResponseOptions>() {
            resp.set_status(status);
        }
        ServerFnError::new(msg.into())
    }

    fn no_existe() -> ServerFnError {
        fallo(StatusCode::NOT_FOUND, t!("instalar-error-terminada"))
    }

    fn sin_permiso() -> ServerFnError {
        fallo(StatusCode::FORBIDDEN, t!("instalar-error-token"))
    }

    pub(crate) fn interno(e: impl std::fmt::Display) -> ServerFnError {
        tracing::error!("error interno en la instalación o en Ajustes del panel: {e}");
        fallo(StatusCode::INTERNAL_SERVER_ERROR, t!("comun-error-interno"))
    }

    /// Estado del panel, con la respuesta marcada `no-store` (llevan
    /// credenciales en la petición).
    fn preparar_respuesta() -> Result<AppState, ServerFnError> {
        if let Some(resp) = use_context::<ResponseOptions>() {
            resp.insert_header(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        }
        use_context::<AppState>().ok_or_else(|| interno("falta el estado del panel"))
    }

    /// IP del cliente; error 403 si no es de la red local ni de las redes
    /// añadidas con `RP_INSTALL_NETS`.
    async fn ip_de_la_red_local(state: &AppState) -> Result<IpAddr, ServerFnError> {
        let ConnectInfo(peer) = leptos_axum::extract::<ConnectInfo<SocketAddr>>().await?;
        let headers: HeaderMap = leptos_axum::extract().await?;
        let ip = auth::client_ip(peer.ip(), &headers, &state.cfg.trusted_proxies);
        if ip_local(ip) || state.cfg.install_nets.iter().any(|red| red.contiene(ip)) {
            Ok(ip)
        } else {
            tracing::warn!("asistente de instalación pedido desde fuera de la red local: {ip}");
            Err(fallo(StatusCode::FORBIDDEN, t!("instalar-error-red")))
        }
    }

    /// Permiso para los pasos tras el administrador: sin usuarios, el token;
    /// con usuarios, la instalación en curso (si no, 404) y sesión (401).
    pub(super) async fn autorizar(token: &str) -> Result<AppState, ServerFnError> {
        let state = preparar_respuesta()?;
        ip_de_la_red_local(&state).await?;
        if !hay_usuarios(&state).await.map_err(interno)? {
            let fase = state.instalacion.fase.lock().await;
            return match fase.token.as_deref() {
                Some(t) if mismo_token(token, t) => Ok(state.clone()),
                _ => Err(sin_permiso()),
            };
        }
        if !en_curso(&state).await {
            return Err(no_existe());
        }
        auth::require_user().await?;
        Ok(state)
    }

    pub(super) async fn crear_admin(
        token: String,
        usuario: String,
        contrasena: String,
    ) -> Result<(), ServerFnError> {
        let state = preparar_respuesta()?;
        let ip = ip_de_la_red_local(&state).await?;
        {
            // Con el candado tomado: dos peticiones con el mismo token no
            // crean dos administradores.
            let mut fase = state.instalacion.fase.lock().await;
            if hay_usuarios(&state).await.map_err(interno)? {
                return Err(no_existe());
            }
            match fase.token.as_deref() {
                Some(t) if mismo_token(&token, t) => {}
                _ => return Err(sin_permiso()),
            }
            auth::create_user(&state.db, &usuario, &contrasena)
                .await
                .map_err(|e| match e {
                    auth::UserError::InvalidUsername => {
                        fallo(StatusCode::BAD_REQUEST, t!("instalar-error-usuario"))
                    }
                    auth::UserError::PasswordTooShort => fallo(
                        StatusCode::BAD_REQUEST,
                        t!(
                            "instalar-error-contrasena-corta",
                            n = auth::MIN_PASSWORD_CHARS
                        ),
                    ),
                    auth::UserError::PasswordTooLong => fallo(
                        StatusCode::BAD_REQUEST,
                        t!(
                            "instalar-error-contrasena-larga",
                            n = auth::MAX_PASSWORD_BYTES
                        ),
                    ),
                    e => interno(e),
                })?;
            fase.token = None;
            state.instalacion.hay_usuarios.store(true, Ordering::SeqCst);
            borrar_fichero(&ruta_token(&state.cfg));
            tracing::info!("instalación: creado el administrador {usuario}");
        }
        let id = auth::attempt_login(&state, ip, &usuario, &contrasena)
            .await
            .map_err(|e| interno(format!("{e:?}")))?;
        auth::poner_cookie_de_sesion(&id);
        Ok(())
    }

    /// URL base del router: `http(s)://anfitrión[:puerto]`, sin ruta ni
    /// consulta, sin la barra final.
    pub(crate) fn validar_url(url: &str) -> Option<String> {
        let url = url.trim().trim_end_matches('/');
        let resto = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))?;
        let valido = !resto.is_empty()
            && resto
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ".-:[]".contains(c));
        valido.then(|| url.to_owned())
    }

    /// Puerta de enlace por defecto de `/proc/net/route` (Linux): la línea
    /// con destino y máscara `00000000` y el bit de pasarela (`RTF_GATEWAY`)
    /// en las banderas; la dirección va en hexadecimal little-endian.
    pub fn gw_de_proc_net_route(texto: &str) -> Option<Ipv4Addr> {
        const RTF_UP: u32 = 0x1;
        const RTF_GATEWAY: u32 = 0x2;
        texto.lines().skip(1).find_map(|l| {
            let c: Vec<&str> = l.split_whitespace().collect();
            let (destino, gw, banderas, mascara) = (c.get(1)?, c.get(2)?, c.get(3)?, c.get(7)?);
            let banderas = u32::from_str_radix(banderas, 16).ok()?;
            let gw = u32::from_str_radix(gw, 16).ok()?;
            (*destino == "00000000"
                && *mascara == "00000000"
                && banderas & (RTF_UP | RTF_GATEWAY) == RTF_UP | RTF_GATEWAY
                && gw != 0)
                .then(|| Ipv4Addr::from(gw.to_le_bytes()))
        })
    }

    /// Puerta de enlace por defecto de la salida de `route -n get default`
    /// (FreeBSD): la línea `gateway: a.b.c.d`.
    pub fn gw_de_route_get(texto: &str) -> Option<Ipv4Addr> {
        texto.lines().find_map(|l| {
            l.trim()
                .strip_prefix("gateway:")
                .and_then(|g| g.trim().parse().ok())
        })
    }

    /// ¿Se puede sugerir `gw` como dirección del router? Solo en una red
    /// privada que no sea 172.16/12.
    pub fn sugerible(gw: Ipv4Addr) -> bool {
        let [a, b, ..] = gw.octets();
        gw.is_private() && !(a == 172 && (16..32).contains(&b))
    }

    /// Puerta de enlace por defecto de este equipo, si se sabe leer en este
    /// sistema.
    async fn puerta_de_enlace() -> Option<Ipv4Addr> {
        tokio::task::spawn_blocking(|| {
            if cfg!(target_os = "linux") {
                std::fs::read_to_string("/proc/net/route")
                    .ok()
                    .and_then(|t| gw_de_proc_net_route(&t))
            } else if cfg!(target_os = "freebsd") {
                std::process::Command::new("route")
                    .args(["-n", "get", "default"])
                    .output()
                    .ok()
                    .and_then(|o| gw_de_route_get(&String::from_utf8_lossy(&o.stdout)))
            } else {
                None
            }
        })
        .await
        .ok()
        .flatten()
    }

    pub(super) async fn url_sugerida() -> Option<String> {
        puerta_de_enlace()
            .await
            .filter(|gw| sugerible(*gw))
            .map(|gw| format!("http://{gw}"))
    }

    /// Entra en el router con esas credenciales, lee `cbGetSummary` y sale.
    pub(crate) async fn probar(
        state: &AppState,
        url: &str,
        usuario: &str,
        contrasena: &str,
    ) -> Result<(InfoRouter, CredencialesRouter), ServerFnError> {
        let url = validar_url(url)
            .ok_or_else(|| fallo(StatusCode::BAD_REQUEST, t!("instalar-error-url")))?;
        if usuario.is_empty() || contrasena.is_empty() {
            return Err(fallo(StatusCode::BAD_REQUEST, t!("instalar-error-faltan")));
        }
        let c = CredencialesRouter {
            url,
            usuario: usuario.to_owned(),
            contrasena: contrasena.to_owned(),
        };
        let cliente = hgu::Client::new(c.client_config(&state.cfg));
        let resumen = hgu::read::summary(&cliente).await;
        cliente.logout().await;
        let s = resumen.map_err(|e| {
            fallo(
                StatusCode::BAD_GATEWAY,
                t!("instalar-error-router", error = e.to_string()),
            )
        })?;
        let info = InfoRouter {
            probado: instancia::firmware_probado(&s.firmware),
            modelo: s.model,
            firmware: s.firmware,
        };
        Ok((info, c))
    }

    /// Prueba las credenciales del router y, si entran, las guarda, las pasa
    /// al cliente del panel y fija el modo según el firmware. La usan el
    /// asistente y «Configuración del router → Conexión con el router». Con
    /// un cambio aplicándose o vigilándose, 409 sin tocar nada: el vigilante
    /// seguiría con otro router; mientras dura, el gestor no empieza ningún
    /// cambio.
    pub(crate) async fn guardar(
        state: &AppState,
        url: &str,
        usuario: &str,
        contrasena: &str,
    ) -> Result<InfoRouter, ServerFnError> {
        let _sin_cambios = state
            .cambios()
            .map(|m| m.excluir_cambios())
            .transpose()
            .map_err(|_| fallo(StatusCode::CONFLICT, t!("panel-error-cambio-en-curso")))?;
        let (info, c) = probar(state, url, usuario, contrasena).await?;
        instancia::guardar_credenciales(&state.db, &state.history_key, &c)
            .await
            .map_err(interno)?;
        state.hgu.reconfigurar(c.client_config(&state.cfg)).await;
        // Otro router puede tener otra conexión a internet.
        instancia::detectar_sin_esperar_mucho(state, state.cfg.espera_deteccion).await;
        state.poller.despertar();
        instancia::fijar_modo_firmware(&state.db, &info.firmware)
            .await
            .map_err(interno)?;
        tracing::info!(
            "credenciales del router guardadas: {} ({}, firmware {}{})",
            c.url,
            info.modelo,
            info.firmware,
            if info.probado {
                ""
            } else {
                ", no probado: solo lectura"
            }
        );
        Ok(info)
    }

    /// ¿Es `n` un parámetro que elige una instancia (`*_id`, `*_0i`)? Devuelve
    /// el nombre sin el sufijo.
    fn base_de_id(n: &str) -> Option<&str> {
        n.strip_suffix("_id").or_else(|| n.strip_suffix("_0i"))
    }

    /// ¿Hay que dar un valor a `p` para leer `f`? No en `all` (se manda
    /// `ALL`) ni en los filtros (no son ids). Un id que elige la instancia
    /// de la propia lista tampoco: sin él, el router devuelve todas (así
    /// responden `cbGetWanIPIntf` o `cbGetDnsRelayServer`). Sí en el id de
    /// otro objeto del que cuelga lo que se lee (el conjunto DHCP de sus
    /// reservas, la interfaz de una dirección): es todo id que no elige la
    /// instancia de la lista cuando la función es una lista, admite `all` o
    /// lleva ids de dos objetos. Sin nada de eso, su único id elige su propia
    /// instancia (`cbGetWifiWds`, `cbGetVoiceLine`).
    fn obligatorio(f: &Funcion, p: &str) -> bool {
        let Some(base) = base_de_id(p) else {
            return false;
        };
        let bases: std::collections::BTreeSet<&str> = f
            .parametros
            .iter()
            .filter_map(|q| base_de_id(&q.nombre))
            .collect();
        match &f.lista {
            Some(l) => base_de_id(&l.id_param) != Some(base),
            None => f.parametro("all").is_some() || bases.len() > 1,
        }
    }

    /// Lecturas de la foto de partida: las funciones de lectura de la UI
    /// avanzada, no bloqueadas y sin parámetros obligatorios, con `all=ALL`
    /// si lo admiten. `(función, parámetros)`.
    pub fn lecturas_de_partida(cat: &Catalogo) -> Vec<(&str, Vec<(&'static str, &'static str)>)> {
        cat.todas()
            .iter()
            .filter(|f| {
                f.tipo == Tipo::Lectura
                    && f.bloqueada.is_none()
                    && f.origen == Origen::Avanzada
                    && f.name.starts_with("cb")
                    && !FUERA_DE_LA_FOTO.contains(&f.name.as_str())
                    && !f.parametros.iter().any(|p| obligatorio(f, &p.nombre))
            })
            .map(|f| {
                let params = if f.parametro("all").is_some() {
                    vec![("all", "ALL")]
                } else {
                    Vec::new()
                };
                (f.name.as_str(), params)
            })
            .collect()
    }

    pub(super) async fn lectura_inicial(state: &AppState) -> Result<ResumenFoto, ServerFnError> {
        if !state.hgu.configurado() {
            return Err(fallo(StatusCode::CONFLICT, t!("instalar-error-sin-router")));
        }
        let fotos::FotoLeida {
            contenido: foto,
            fallidas,
            mut firmware,
        } = {
            // Sin el sondeo en medio: la lectura es una sola foto.
            let _pausa = state.poller.pause().await;
            fotos::leer_partida(state).await
        };
        // La red ya se detectó al guardar las credenciales; si aquello no
        // llegó a tiempo (o falló), se intenta ahora.
        let red = match state.red() {
            Some(r) => Some(r),
            None => instancia::detectar_y_guardar(state)
                .await
                .inspect_err(|e| tracing::warn!("lectura inicial: {e}"))
                .ok(),
        };
        let resumen = ResumenFoto {
            funciones_leidas: u32::try_from(foto.len()).unwrap_or(u32::MAX),
            fallidas,
            red: red.as_ref().map(ResumenRed::from),
        };
        if foto.is_empty() {
            // Nada que guardar: se puede repetir.
            return Ok(resumen);
        }
        // El modo según el firmware que se acaba de leer: el router pudo
        // configurarse sin pasar por `guardar` (importado del entorno). Si el
        // resumen no llegó, se reintenta una vez; si tampoco, no se adivina:
        // lo decide el sondeo en cuanto lo lea (misma regla).
        if firmware.is_empty() {
            match hgu::read::summary(&state.hgu).await {
                Ok(s) => firmware = s.firmware,
                Err(e) => tracing::warn!(
                    "lectura inicial: sin resumen del router ({e}); el modo solo lectura lo \
                     fijará el sondeo"
                ),
            }
        }
        if !firmware.is_empty() {
            instancia::fijar_modo_firmware(&state.db, &firmware)
                .await
                .map_err(interno)?;
        }
        let id = fotos::guardar(&state.db, &state.history_key, TipoFoto::Partida, &foto)
            .await
            .map_err(interno)?;
        state.instalacion.fase.lock().await.en_curso = false;
        tracing::info!(
            "instalación terminada: foto de partida {id} con {} funciones ({} fallidas)",
            resumen.funciones_leidas,
            resumen.fallidas.len()
        );
        Ok(resumen)
    }

    pub(super) async fn fase() -> Result<EstadoInstalacion, ServerFnError> {
        let state = preparar_respuesta()?;
        ip_de_la_red_local(&state).await?;
        let admin_creado = hay_usuarios(&state).await.map_err(interno)?;
        if admin_creado {
            if !en_curso(&state).await {
                return Err(no_existe());
            }
            auth::require_user().await?;
        }
        Ok(EstadoInstalacion {
            admin_creado,
            router_configurado: state.hgu.configurado(),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn token_en_tiempo_constante() {
            assert!(mismo_token("abc", "abc"));
            assert!(!mismo_token("abd", "abc"));
            assert!(!mismo_token("ab", "abc"));
            assert!(!mismo_token("", ""), "vacío nunca vale");
        }

        #[test]
        fn solo_red_local() {
            for ip in [
                "127.0.0.1",
                "192.168.1.90",
                "10.255.0.10",
                "172.16.0.1",
                "169.254.1.1",
                "::1",
                "fd00::1",
                "fe80::1",
                "::ffff:192.168.1.99",
            ] {
                assert!(ip_local(ip.parse().unwrap()), "{ip}");
            }
            for ip in ["203.0.113.9", "9.9.9.9", "2001:db8::1", "::ffff:9.9.9.9"] {
                assert!(!ip_local(ip.parse().unwrap()), "{ip}");
            }
        }

        #[test]
        fn url_del_router() {
            assert_eq!(
                validar_url(" http://192.168.1.1/ ").as_deref(),
                Some("http://192.168.1.1")
            );
            assert_eq!(
                validar_url("https://router.home.arpa:8443").as_deref(),
                Some("https://router.home.arpa:8443")
            );
            for mala in [
                "",
                "192.168.1.1",
                "ftp://x",
                "http://",
                "http://a/b",
                "http://a?b",
                "http://u@a",
                "http://a b",
            ] {
                assert_eq!(validar_url(mala), None, "{mala}");
            }
        }

        #[test]
        fn lecturas_sin_parametros_obligatorios() {
            let cat = Catalogo::load().unwrap();
            let l = lecturas_de_partida(&cat);
            let nombres: Vec<&str> = l.iter().map(|(n, _)| *n).collect();
            assert!(l.len() >= 40, "{nombres:?}");
            // Las listas con `all`, con `all=ALL`; las demás, sin nada.
            assert!(l.contains(&("cbGetDdns", vec![("all", "ALL")])));
            assert!(l.contains(&("cbGetSummary", vec![])));
            assert!(l.contains(&("cbListWifiAccPt", vec![])));
            // Fuera: las que piden otro objeto, las de la sesión y las de la
            // UI básica.
            assert!(l.contains(&("cbGetWifiWds", vec![])));
            for fuera in [
                "cbDhcpV4SvrPoolGetStatic",
                "cbDhcpV4SvrPoolGetClt",
                "cbGetLanIPv4Addr",
                "cbCheckSession",
                "cbCheckLogout",
            ] {
                assert!(!nombres.contains(&fuera), "{fuera}");
            }
            assert!(nombres.iter().all(|n| n.starts_with("cb")));
        }
    }
}
