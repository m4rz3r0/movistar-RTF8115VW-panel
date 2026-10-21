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
        })
    }
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
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn from(pairs: &[(&str, &str)]) -> Result<Config, String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::from_vars(|name| map.get(name).cloned())
    }

    /// Lo mínimo para arrancar: el origen público.
    const ORIGEN: (&str, &str) = ("RP_ORIGIN", "http://localhost:3000");

    fn red(s: &str) -> RedIp {
        s.parse().unwrap()
    }

    #[test]
    fn valores_por_defecto() {
        let c = from(&[ORIGEN]).unwrap();
        let (db, clave) = rutas_por_defecto();
        assert_eq!(c.listen, "127.0.0.1:3000".parse().unwrap());
        assert_eq!(c.db_path, PathBuf::from(db));
        assert_eq!(c.router_url, None);
        assert_eq!(c.router_user, None);
        assert_eq!(c.router_secret_file, None);
        assert_eq!(c.history_key_file, PathBuf::from(clave));
        assert_eq!(c.trusted_proxies, [red("127.0.0.1")]);
        assert_eq!(c.public_origins, ["http://localhost:3000"]);
        assert!(c.install_nets.is_empty());
        assert_eq!(c.router_min_gap, Duration::from_millis(150));
        assert_eq!(c.espera_deteccion, Duration::from_secs(5));
    }

    #[test]
    fn escucha_por_defecto_local() {
        let c = from(&[ORIGEN]).unwrap();
        assert!(c.listen.ip().is_loopback(), "{}", c.listen);
        assert_eq!(c.listen.port(), 3000);
    }

    #[test]
    fn rutas_por_defecto_segun_so() {
        let esperadas = if cfg!(target_os = "freebsd") {
            (
                "/var/db/routerpanel/panel.db",
                "/usr/local/etc/routerpanel/history.key",
            )
        } else {
            (
                "/var/lib/routerpanel/panel.db",
                "/etc/routerpanel/history.key",
            )
        };
        assert_eq!(rutas_por_defecto(), esperadas);
    }

    #[test]
    fn proxies_en_lista_y_cidr() {
        let c = from(&[
            ORIGEN,
            ("RP_TRUSTED_PROXY", "127.0.0.1, 198.51.100.0/24,::1"),
        ])
        .unwrap();
        assert_eq!(
            c.trusted_proxies,
            [red("127.0.0.1"), red("198.51.100.0/24"), red("::1")]
        );
        for malo in ["", " , ", "127.0.0.1,", "198.51.100.0/40"] {
            let e = from(&[ORIGEN, ("RP_TRUSTED_PROXY", malo)]).unwrap_err();
            assert!(e.contains("RP_TRUSTED_PROXY"), "{malo}: {e}");
        }
    }

    #[test]
    fn redes_de_instalacion() {
        let c = from(&[ORIGEN, ("RP_INSTALL_NETS", "203.0.113.0/24, fd00::/8")]).unwrap();
        assert_eq!(c.install_nets, [red("203.0.113.0/24"), red("fd00::/8")]);
        // Vacía cuenta como ausente.
        let c = from(&[ORIGEN, ("RP_INSTALL_NETS", "")]).unwrap();
        assert!(c.install_nets.is_empty());
        let e = from(&[ORIGEN, ("RP_INSTALL_NETS", "203.0.113.0/24,,")]).unwrap_err();
        assert!(e.contains("RP_INSTALL_NETS"), "{e}");
    }

    #[test]
    fn origenes_en_lista() {
        let c = from(&[(
            "RP_ORIGIN",
            "https://panel.example, http://panel.home.arpa:3000/",
        )])
        .unwrap();
        // Sin la `/` final, que el navegador nunca manda en `Origin`.
        assert_eq!(
            c.public_origins,
            ["https://panel.example", "http://panel.home.arpa:3000"]
        );
        for malo in [
            " , ",
            "https://panel.example,",
            ",https://panel.example",
            "/",
        ] {
            let e = from(&[("RP_ORIGIN", malo)]).unwrap_err();
            assert!(e.contains("RP_ORIGIN"), "{malo:?}: {e}");
        }
    }

    #[test]
    fn origen_con_forma_de_origen() {
        for bueno in [
            "https://panel.example",
            "http://panel.home.arpa:3000",
            "http://127.0.0.1:3000",
            "http://[::1]:3000",
            "https://[2001:db8::1]",
        ] {
            let c = from(&[("RP_ORIGIN", bueno)]).unwrap_or_else(|e| panic!("{bueno}: {e}"));
            assert_eq!(c.public_origins, [bueno]);
        }
        for malo in [
            "otro.example",
            "ftp://otro.example",
            "HTTPS://otro.example",
            "https://",
            "https://:3000",
            "https://otro.example/ruta",
            "https://otro.example?x=1",
            "https://otro.example#x",
            "https://usuario:clave@[::1]",
            "https://otro.example:",
            "https://otro.example:puerto",
            "https://otro.example:70000",
            "https://panel example",
            "http://[::1",
            "http://[nada]:3000",
            "http://[::1]3000",
        ] {
            let e = from(&[("RP_ORIGIN", malo)]).expect_err(malo);
            assert!(e.contains("RP_ORIGIN"), "{malo:?}: {e}");
            // El error no repite el valor (salvo lo que coincide con el ejemplo).
            if !"https://panel.example".contains(malo) {
                assert!(!e.contains(malo), "{malo:?}: {e}");
            }
        }
        // Con varios, dice cuál falla.
        let e = from(&[("RP_ORIGIN", "https://otro.example, otro.example")]).unwrap_err();
        assert!(e.contains("origen 2"), "{e}");
    }

    #[test]
    fn config_sin_origen_falla() {
        let e = from(&[]).unwrap_err();
        assert!(e.contains("RP_ORIGIN"), "{e}");
        let e = from(&[("RP_ORIGIN", "")]).unwrap_err();
        assert!(e.contains("RP_ORIGIN"), "{e}");
    }

    #[test]
    fn variables_vacias_del_router_cuentan_como_ausentes() {
        let c = from(&[ORIGEN, ("RP_ROUTER_URL", ""), ("RP_ROUTER_USER", "")]).unwrap();
        assert_eq!((c.router_url, c.router_user), (None, None));
    }

    /// Una instalación anterior exporta cada variable con un solo valor: un
    /// origen, un proxy (IP suelta) y rutas explícitas, que mandan sobre los
    /// valores por defecto. Arranca igual que antes.
    #[test]
    fn entorno_de_un_solo_valor_sigue_valiendo() {
        let c = from(&[
            ("RP_LISTEN", "0.0.0.0:3000"),
            ("RP_DB", "/srv/panel/panel.db"),
            ("RP_ROUTER_URL", "http://192.168.1.1"),
            ("RP_ROUTER_USER", "usuario"),
            ("RP_ROUTER_SECRET", "/srv/panel/router.secret"),
            ("RP_HISTORY_KEY", "/srv/panel/history.key"),
            ("RP_TRUSTED_PROXY", "192.0.2.100"),
            ("RP_ORIGIN", "https://panel.example"),
            ("RP_LOG", "info"),
        ])
        .unwrap();
        assert_eq!(c.listen, "0.0.0.0:3000".parse().unwrap());
        assert_eq!(c.db_path, PathBuf::from("/srv/panel/panel.db"));
        assert_eq!(c.router_url.as_deref(), Some("http://192.168.1.1"));
        assert_eq!(c.router_user.as_deref(), Some("usuario"));
        assert_eq!(
            c.router_secret_file,
            Some(PathBuf::from("/srv/panel/router.secret"))
        );
        assert_eq!(c.history_key_file, PathBuf::from("/srv/panel/history.key"));
        assert_eq!(c.trusted_proxies, [red("192.0.2.100")]);
        assert!(c.trusted_proxies[0].contiene("192.0.2.100".parse().unwrap()));
        assert!(!c.trusted_proxies[0].contiene("192.0.2.101".parse().unwrap()));
        assert_eq!(c.public_origins, ["https://panel.example"]);
        assert!(c.install_nets.is_empty());
    }

    #[test]
    fn direccion_no_valida_es_error() {
        let e = from(&[ORIGEN, ("RP_LISTEN", "nada")]).unwrap_err();
        assert!(e.contains("RP_LISTEN"), "{e}");
        let e = from(&[ORIGEN, ("RP_TRUSTED_PROXY", "x")]).unwrap_err();
        assert!(e.contains("RP_TRUSTED_PROXY"), "{e}");
    }

    /// Escribe `text` en `path` con permisos `mode`.
    fn escribir(path: &Path, text: &str, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, text).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    #[test]
    fn secreto_sin_salto_de_linea() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("router.secret");
        escribir(&path, "clave con espacios \n", 0o600);
        assert_eq!(read_router_secret(&path).unwrap(), "clave con espacios ");
        escribir(&path, "clave\n", 0o400);
        assert_eq!(read_router_secret(&path).unwrap(), "clave");
    }

    #[test]
    fn secreto_legible_por_otros_es_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("router.secret");
        for mode in [0o644, 0o640, 0o604] {
            escribir(&path, "clave-que-no-debe-salir\n", mode);
            let e = read_router_secret(&path).unwrap_err();
            assert!(e.contains("permisos"), "{e}");
            assert!(e.contains(&format!("{mode:o}")), "{e}");
            assert!(!e.contains("clave-que-no-debe-salir"), "{e}");
        }
    }

    #[test]
    fn secreto_ausente_es_error_claro() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no-existe");
        let e = read_router_secret(&path).unwrap_err();
        assert!(e.contains("contraseña del router"), "{e}");
        assert!(e.contains("no-existe"), "{e}");
    }
}
