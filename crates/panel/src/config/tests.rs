//! Pruebas de la configuración a partir de las variables `RP_*`.

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

/// `RP_E2E_HABILITAR_TODO=1` lo pide; solo vale en depuración.
#[test]
fn habilitar_todo_solo_en_depuracion() {
    let base = [("RP_ORIGIN", "https://panel.example")];
    assert!(!from(&base).unwrap().habilitar_todo);
    let con = from(&[base[0], (HABILITAR_TODO, "1")]).unwrap();
    assert!(con.habilitar_todo);
    assert!(
        !from(&[base[0], (HABILITAR_TODO, "si")])
            .unwrap()
            .habilitar_todo
    );
    assert!(todo_habilitado(true, true));
    assert!(!todo_habilitado(true, false));
    assert!(!todo_habilitado(false, true));
}
