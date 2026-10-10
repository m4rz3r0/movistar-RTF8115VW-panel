//! Configuración del servidor a partir de las variables de entorno `RP_*`.

use std::{
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
};

/// Fichero SQLite por defecto.
const DEFAULT_DB: &str = "/var/db/routerpanel/panel.db";

/// Fichero SQLite según `RP_DB`. Es lo único que necesitan las órdenes de
/// administración (`panel user …`), que no deben exigir el resto de la
/// configuración del servidor.
pub fn db_path_from_env() -> PathBuf {
    std::env::var_os("RP_DB").map_or_else(|| DEFAULT_DB.into(), PathBuf::from)
}

/// Configuración del panel.
#[derive(Clone, Debug)]
pub struct Config {
    /// Dirección de escucha (`RP_LISTEN`).
    pub listen: SocketAddr,
    /// Fichero SQLite (`RP_DB`).
    pub db_path: PathBuf,
    /// URL base del router (`RP_ROUTER_URL`).
    pub router_url: String,
    /// Usuario del router (`RP_ROUTER_USER`).
    pub router_user: String,
    /// Fichero con la contraseña del router (`RP_ROUTER_SECRET`).
    pub router_secret_file: PathBuf,
    /// Fichero con la clave del historial (`RP_HISTORY_KEY`).
    pub history_key_file: PathBuf,
    /// Proxy de confianza para `X-Forwarded-For` (`RP_TRUSTED_PROXY`).
    pub trusted_proxy: IpAddr,
    /// Origen público del panel (`RP_ORIGIN`).
    pub public_origin: String,
}

impl Config {
    /// Lee la configuración del entorno; las variables ausentes toman su valor
    /// por defecto.
    pub fn from_env() -> Result<Config, String> {
        Config::from_vars(|name| std::env::var(name).ok())
    }

    /// Lee la configuración con `var` como fuente de las variables `RP_*`.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Config, String> {
        let get = |name: &str, default: &str| var(name).unwrap_or_else(|| default.to_owned());
        Ok(Config {
            listen: parse("RP_LISTEN", &get("RP_LISTEN", "0.0.0.0:3000"))?,
            db_path: get("RP_DB", DEFAULT_DB).into(),
            router_url: get("RP_ROUTER_URL", "http://192.168.1.1"),
            router_user: get("RP_ROUTER_USER", "1234"),
            router_secret_file: get(
                "RP_ROUTER_SECRET",
                "/usr/local/etc/routerpanel/router.secret",
            )
            .into(),
            history_key_file: get("RP_HISTORY_KEY", "/usr/local/etc/routerpanel/history.key")
                .into(),
            trusted_proxy: parse(
                "RP_TRUSTED_PROXY",
                &get("RP_TRUSTED_PROXY", "192.0.2.11"),
            )?,
            public_origin: get("RP_ORIGIN", "https://panel.home.arpa"),
        })
    }
}

fn parse<T: FromStr>(name: &str, value: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    value
        .parse()
        .map_err(|e| format!("{name}={value} no es válida: {e}"))
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

    #[test]
    fn valores_por_defecto() {
        let c = from(&[]).unwrap();
        assert_eq!(c.listen, "0.0.0.0:3000".parse().unwrap());
        assert_eq!(c.db_path, PathBuf::from("/var/db/routerpanel/panel.db"));
        assert_eq!(c.router_url, "http://192.168.1.1");
        assert_eq!(c.router_user, "1234");
        assert_eq!(
            c.router_secret_file,
            PathBuf::from("/usr/local/etc/routerpanel/router.secret")
        );
        assert_eq!(
            c.history_key_file,
            PathBuf::from("/usr/local/etc/routerpanel/history.key")
        );
        assert_eq!(c.trusted_proxy, "192.0.2.11".parse::<IpAddr>().unwrap());
        assert_eq!(c.public_origin, "https://panel.home.arpa");
    }

    #[test]
    fn variables_sustituyen_los_valores() {
        let c = from(&[
            ("RP_LISTEN", "127.0.0.1:4000"),
            ("RP_DB", "/tmp/p.db"),
            ("RP_ROUTER_URL", "http://127.0.0.1:8081"),
            ("RP_ROUTER_USER", "sim"),
            ("RP_ROUTER_SECRET", "/tmp/r.secret"),
            ("RP_HISTORY_KEY", "/tmp/h.key"),
            ("RP_TRUSTED_PROXY", "127.0.0.1"),
            ("RP_ORIGIN", "http://localhost:3000"),
        ])
        .unwrap();
        assert_eq!(c.listen, "127.0.0.1:4000".parse().unwrap());
        assert_eq!(c.db_path, PathBuf::from("/tmp/p.db"));
        assert_eq!(c.router_url, "http://127.0.0.1:8081");
        assert_eq!(c.router_user, "sim");
        assert_eq!(c.router_secret_file, PathBuf::from("/tmp/r.secret"));
        assert_eq!(c.history_key_file, PathBuf::from("/tmp/h.key"));
        assert_eq!(c.trusted_proxy, "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(c.public_origin, "http://localhost:3000");
    }

    #[test]
    fn direccion_no_valida_es_error() {
        let e = from(&[("RP_LISTEN", "nada")]).unwrap_err();
        assert!(e.contains("RP_LISTEN"), "{e}");
        let e = from(&[("RP_TRUSTED_PROXY", "x")]).unwrap_err();
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
