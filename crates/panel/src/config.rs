//! Configuración del servidor a partir de las variables de entorno `RP_*`.

use crate::redes::{self, RedIp};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

/// Fichero SQLite y clave del historial por defecto, según las costumbres del
/// sistema: en FreeBSD, `/var/db` y `/usr/local/etc`; en otro caso (Linux),
/// `/var/lib` y `/etc`.
pub fn rutas_por_defecto() -> (&'static str, &'static str) {
    if cfg!(target_os = "freebsd") {
        (
            "/var/db/routerpanel/panel.db",
            "/usr/local/etc/routerpanel/history.key",
        )
    } else {
        (
            "/var/lib/routerpanel/panel.db",
            "/etc/routerpanel/history.key",
        )
    }
}

/// Fichero SQLite según `RP_DB`. Es lo único que necesitan las órdenes de
/// administración (`panel user …`), que no deben exigir el resto de la
/// configuración del servidor.
pub fn db_path_from_env() -> PathBuf {
    std::env::var_os("RP_DB").map_or_else(|| rutas_por_defecto().0.into(), PathBuf::from)
}

/// Configuración del panel.
#[derive(Clone, Debug)]
pub struct Config {
    /// Dirección de escucha (`RP_LISTEN`); por defecto, solo el propio equipo.
    pub listen: SocketAddr,
    /// Fichero SQLite (`RP_DB`).
    pub db_path: PathBuf,
    /// URL base del router (`RP_ROUTER_URL`). Las tres variables del router
    /// solo sirven para importar las credenciales a la base de datos la
    /// primera vez (ver `instancia::arrancar_router`).
    pub router_url: Option<String>,
    /// Usuario del router (`RP_ROUTER_USER`).
    pub router_user: Option<String>,
    /// Fichero con la contraseña del router (`RP_ROUTER_SECRET`).
    pub router_secret_file: Option<PathBuf>,
    /// Fichero con la clave del historial (`RP_HISTORY_KEY`).
    pub history_key_file: PathBuf,
    /// Proxies de confianza para `X-Forwarded-For` (`RP_TRUSTED_PROXY`: IPs
    /// o redes separadas por comas).
    pub trusted_proxies: Vec<RedIp>,
    /// Orígenes públicos del panel, sin `/` final (`RP_ORIGIN`, obligatoria:
    /// uno o varios separados por comas).
    pub public_origins: Vec<String>,
    /// Redes que el asistente de instalación acepta además de la red local
    /// (`RP_INSTALL_NETS`, separadas por comas; por defecto, ninguna).
    pub install_nets: Vec<RedIp>,
    /// Pausa mínima entre peticiones al router: la de serie,
    /// [`hgu::client::PAUSA_MINIMA`] (150 ms). No sale de ninguna variable
    /// de entorno (una pausa menor podría saturar el router); solo las
    /// pruebas la acortan.
    pub router_min_gap: Duration,
    /// Lo más que se espera a la detección de la red del router al arrancar
    /// sin ninguna guardada o al guardar las credenciales: la de serie,
    /// [`crate::instancia::ESPERA_DETECCION`] (5 s). Tampoco sale de ninguna
    /// variable; solo las pruebas la acortan.
    pub espera_deteccion: Duration,
    /// Habilita todo lo que el panel aún no escribe en el router real (las
    /// escrituras sin comprobar del catálogo y las funciones desactivadas
    /// de [`crate::validacion_real`]). Solo para las pruebas contra
    /// router-sim (`RP_E2E_HABILITAR_TODO=1`): solo lo atiende un binario de
    /// depuración ([`Config::todo_habilitado`]).
    #[doc(hidden)]
    pub habilitar_todo: bool,
}

impl Config {
    /// Lee la configuración del entorno; las variables ausentes toman su valor
    /// por defecto, salvo `RP_ORIGIN`, que es obligatoria.
    pub fn from_env() -> Result<Config, String> {
        Config::from_vars(|name| std::env::var(name).ok())
    }

    /// Lee la configuración con `var` como fuente de las variables `RP_*`.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Config, String> {
        let get = |name: &str, default: &str| var(name).unwrap_or_else(|| default.to_owned());
        // En las variables sin valor por defecto, vacía cuenta como ausente.
        let opt = |name: &str| var(name).filter(|v| !v.is_empty());
        let redes = |name: &str, texto: &str| {
            redes::lista(texto).map_err(|e| format!("{name} no es válida: {e}"))
        };
        let public_origins = origenes(&opt("RP_ORIGIN").ok_or_else(|| {
            "falta RP_ORIGIN: el origen público del panel (p. ej. https://panel.example)".to_owned()
        })?)?;
        let (db, clave) = rutas_por_defecto();
        Ok(Config {
            listen: parse("RP_LISTEN", &get("RP_LISTEN", "127.0.0.1:3000"))?,
            db_path: get("RP_DB", db).into(),
            router_url: opt("RP_ROUTER_URL"),
            router_user: opt("RP_ROUTER_USER"),
            router_secret_file: opt("RP_ROUTER_SECRET").map(PathBuf::from),
            history_key_file: get("RP_HISTORY_KEY", clave).into(),
            trusted_proxies: redes("RP_TRUSTED_PROXY", &get("RP_TRUSTED_PROXY", "127.0.0.1"))?,
            public_origins,
            install_nets: match opt("RP_INSTALL_NETS") {
                Some(texto) => redes("RP_INSTALL_NETS", &texto)?,
                None => Vec::new(),
            },
            router_min_gap: hgu::client::PAUSA_MINIMA,
            espera_deteccion: crate::instancia::ESPERA_DETECCION,
            habilitar_todo: var(HABILITAR_TODO).as_deref() == Some("1"),
        })
    }

    /// ¿Está todo habilitado para las pruebas ([`Config::habilitar_todo`])?
    /// Nunca en un binario sin `debug_assertions` (release).
    pub fn todo_habilitado(&self) -> bool {
        todo_habilitado(self.habilitar_todo, cfg!(debug_assertions))
    }
}

/// Variable con la que el e2e lo habilita todo (ver
/// [`Config::habilitar_todo`]).
pub const HABILITAR_TODO: &str = "RP_E2E_HABILITAR_TODO";

/// [`Config::todo_habilitado`] con `depuracion` explícito.
fn todo_habilitado(pedido: bool, depuracion: bool) -> bool {
    pedido && depuracion
}

/// Orígenes de `RP_ORIGIN`, separados por comas y sin la `/` final (el
/// navegador nunca la manda en `Origin`). Cada uno es
/// `http(s)://equipo[:puerto]`, sin ruta, consulta ni usuario. El error dice
/// qué origen falla (contando desde 1) sin repetir su valor.
fn origenes(texto: &str) -> Result<Vec<String>, String> {
    texto
        .split(',')
        .enumerate()
        .map(|(i, o)| {
            let o = o.trim().trim_end_matches('/');
            validar_origen(o).map(|()| o.to_owned()).map_err(|e| {
                format!(
                    "RP_ORIGIN no es válida: el origen {} {e} (p. ej. https://panel.example)",
                    i + 1
                )
            })
        })
        .collect()
}

/// ¿Es `o` un origen `http(s)://equipo[:puerto]`? El error completa la
/// frase «el origen N …».
fn validar_origen(o: &str) -> Result<(), &'static str> {
    if o.is_empty() {
        return Err("está vacío");
    }
    let resto = o
        .strip_prefix("https://")
        .or_else(|| o.strip_prefix("http://"))
        .ok_or("debe empezar por http:// o https://")?;
    if resto.contains(['/', '?', '#', '@']) || resto.chars().any(char::is_whitespace) {
        return Err("no puede llevar ruta, consulta, usuario ni espacios");
    }
    // `[IPv6]` o un nombre o IPv4; luego, `:puerto` opcional.
    let (equipo, puerto) = match resto.strip_prefix('[') {
        Some(r) => {
            let (ip6, tras) = r.split_once(']').ok_or("tiene un equipo no válido")?;
            if ip6.parse::<std::net::Ipv6Addr>().is_err() {
                return Err("tiene un equipo no válido");
            }
            let puerto = match tras {
                "" => None,
                t => Some(t.strip_prefix(':').ok_or("tiene un puerto no válido")?),
            };
            (ip6, puerto)
        }
        None => match resto.split_once(':') {
            Some((e, p)) => (e, Some(p)),
            None => (resto, None),
        },
    };
    if equipo.is_empty() {
        return Err("no tiene equipo");
    }
    if let Some(p) = puerto
        && !(p.bytes().all(|b| b.is_ascii_digit()) && p.parse::<u16>().is_ok())
    {
        return Err("tiene un puerto no válido");
    }
    Ok(())
}

/// Valor de `name`; el error nombra la variable sin repetir el valor.
fn parse<T: FromStr>(name: &str, value: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    value
        .parse()
        .map_err(|e| format!("{name} no es válida: {e}"))
}

/// Lee la contraseña del router del fichero `path` (sin el salto de línea
/// final). El fichero no puede ser legible por el grupo ni por otros (600 o
/// 400), igual que la clave del historial. El error nunca incluye el
/// contenido del fichero.
pub fn read_router_secret(path: &Path) -> Result<String, String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mode = meta.permissions().mode();
            if mode & 0o077 != 0 {
                return Err(format!(
                    "el fichero de la contraseña del router {} tiene permisos {:o}; deben ser 600 o 400",
                    path.display(),
                    mode & 0o777
                ));
            }
        }
    }
    let text = std::fs::read_to_string(path).map_err(|e| {
        format!(
            "no se puede leer la contraseña del router en {} (RP_ROUTER_SECRET): {e}",
            path.display()
        )
    })?;
    let secret = text.trim_end_matches(['\n', '\r']);
    if secret.is_empty() {
        return Err(format!(
            "el fichero de la contraseña del router {} está vacío",
            path.display()
        ));
    }
    Ok(secret.to_owned())
}

#[cfg(test)]
mod tests;
