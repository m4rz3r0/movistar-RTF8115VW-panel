//! Modo `--arbol`: revisa los ficheros que se publicarían.

use crate::{Hallazgo, ListasLocales, Permitidos, Regla, reglas, revisar_binario, revisar_texto};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Ficheros que debe tener todo árbol publicado.
const OBLIGATORIOS: [&str; 5] = [
    "LICENSE",
    "AVISO-LEGAL.md",
    "README.md",
    "README.en.md",
    "deploy/config.example",
];

/// Extensiones que siempre se tratan como binarios revisados a mano.
const EXTENSIONES_BINARIAS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "webp", "pdf", "woff2"];

/// Revisa el árbol de `raiz`: si es la raíz de un repositorio git, los
/// ficheros que lista `git ls-files` (los versionados y los nuevos no
/// ignorados, que se publicarían al añadirlos); si no, todos los ficheros bajo
/// `raiz` salvo los directorios `.git`.
///
/// Con `solo` (rutas relativas a `raiz`, o absolutas dentro de ella) revisa
/// únicamente esos ficheros o directorios y no comprueba los obligatorios.
///
/// Si el nombre de un fichero contiene un dato, sus hallazgos no muestran la
/// ruta sino su número en la lista ordenada de ficheros del árbol.
pub fn arbol(raiz: &Path, solo: &[PathBuf], p: &Permitidos, l: &ListasLocales) -> Vec<Hallazgo> {
    let Some(ficheros) = listar(raiz) else {
        return vec![Hallazgo {
            ruta: ".".to_owned(),
            linea: 0,
            regla: Regla::FaltaFichero,
            detalle: "no se pudo listar el árbol".to_owned(),
        }];
    };
    let solo: Vec<String> = solo.iter().map(|s| normalizar(raiz, s)).collect();
    let incluido = |ruta: &str| {
        solo.is_empty()
            || solo
                .iter()
                .any(|s| s.is_empty() || ruta == s || ruta.starts_with(&format!("{s}/")))
    };
    let mut out = Vec::new();
    for (k, ruta) in ficheros.iter().enumerate() {
        if !incluido(ruta) {
            continue;
        }
        let nombre = revisar_nombre(ruta, p);
        let nombre_con_dato = !nombre.is_empty();
        let mut h: Vec<Hallazgo> = prohibida(ruta)
            .filter(|_| !p.ruta_permitida(ruta, Regla::RutaProhibida))
            .map(|detalle| Hallazgo {
                ruta: ruta.clone(),
                linea: 0,
                regla: Regla::RutaProhibida,
                detalle: detalle.to_owned(),
            })
            .into_iter()
            .chain(nombre)
            .collect();
        h.extend(revisar_fichero(&raiz.join(ruta), ruta, p, l));
        if nombre_con_dato {
            for x in &mut h {
                x.ruta = format!("[fichero n.º {} del árbol]", k + 1);
            }
        }
        out.extend(h);
    }
    if solo.is_empty() {
        for f in OBLIGATORIOS {
            if !ficheros.iter().any(|r| r == f) && !p.ruta_permitida(f, Regla::FaltaFichero) {
                out.push(Hallazgo {
                    ruta: f.to_owned(),
                    linea: 0,
                    regla: Regla::FaltaFichero,
                    detalle: "obligatorio".to_owned(),
                });
            }
        }
    }
    out.sort_by(|a, b| (&a.ruta, a.linea).cmp(&(&b.ruta, b.linea)));
    out
}

/// Ficheros del árbol, relativos a `raiz`, con `/`, ordenados por bytes.
fn listar(raiz: &Path) -> Option<Vec<String>> {
    let mut ficheros = if es_raiz_git(raiz) {
        let salida = Command::new("git")
            .arg("-C")
            .arg(raiz)
            .args([
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
            ])
            .output()
            .ok()?;
        if !salida.status.success() {
            return None;
        }
        String::from_utf8(salida.stdout)
            .ok()?
            .split('\0')
            .filter(|r| !r.is_empty())
            // Los borrados del disco sin confirmar no se pueden leer.
            .filter(|r| raiz.join(r).symlink_metadata().is_ok())
            .map(str::to_owned)
            .collect()
    } else {
        let mut v = Vec::new();
        recorrer(raiz, "", &mut v).ok()?;
        v
    };
    ficheros.sort();
    ficheros.dedup();
    Some(ficheros)
}

fn es_raiz_git(raiz: &Path) -> bool {
    let Ok(salida) = Command::new("git")
        .arg("-C")
        .arg(raiz)
        .args(["rev-parse", "--show-toplevel"])
        .output()
    else {
        return false;
    };
    let cima = String::from_utf8_lossy(&salida.stdout);
    salida.status.success()
        && fs::canonicalize(cima.trim_end_matches('\n')).ok() == fs::canonicalize(raiz).ok()
}

fn recorrer(dir: &Path, prefijo: &str, v: &mut Vec<String>) -> std::io::Result<()> {
    for entrada in fs::read_dir(dir)? {
        let entrada = entrada?;
        let nombre = entrada.file_name().to_string_lossy().into_owned();
        let ruta = format!("{prefijo}{nombre}");
        let tipo = entrada.file_type()?;
        if tipo.is_dir() {
            if nombre != ".git" {
                recorrer(&entrada.path(), &format!("{ruta}/"), v)?;
            }
        } else {
            v.push(ruta);
        }
    }
    Ok(())
}

/// `solo` relativo a `raiz`, sin `./` ni `/` final (`""` es todo el árbol).
fn normalizar(raiz: &Path, s: &Path) -> String {
    let relativa = if s.is_absolute() {
        let (Ok(s), Ok(r)) = (fs::canonicalize(s), fs::canonicalize(raiz)) else {
            return s.to_string_lossy().into_owned();
        };
        s.strip_prefix(&r)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| s.clone())
    } else {
        s.to_path_buf()
    };
    let mut t = relativa.to_string_lossy().replace('\\', "/");
    while let Some(resto) = t.strip_prefix("./") {
        t = resto.to_owned();
    }
    let t = t.trim_end_matches('/');
    if t == "." {
        String::new()
    } else {
        t.to_owned()
    }
}

/// Patrón de ruta prohibida que cumple `ruta`, si alguno.
fn prohibida(ruta: &str) -> Option<&'static str> {
    let partes: Vec<&str> = ruta.split('/').collect();
    let nombre = partes.last().copied().unwrap_or("");
    let directorios = &partes[..partes.len().saturating_sub(1)];
    let contiene = |a: &str, b: &str| directorios.windows(2).any(|w| w == [a, b]);
    if directorios.contains(&".superpowers") {
        Some(".superpowers/")
    } else if contiene("docs", "superpowers") {
        Some("docs/superpowers/")
    } else if contiene("e2e", "capturas") {
        Some("e2e/capturas/")
    } else if contiene("e2e", "test-results") {
        Some("e2e/test-results/")
    } else if directorios.contains(&"target") {
        Some("target/")
    } else if [".db", ".db-shm", ".db-wal"]
        .iter()
        .any(|e| nombre.ends_with(e))
    {
        Some("*.db")
    } else if nombre.ends_with(".key") {
        Some("*.key")
    } else if nombre.ends_with(".secret") {
        Some("*.secret")
    } else if nombre == "instalar.token" {
        Some("instalar.token")
    } else if ruta == "deploy/config.local" {
        Some("deploy/config.local")
    } else if nombre.starts_with("real-secrets") && nombre.ends_with(".txt") {
        Some("real-secrets*.txt")
    } else {
        None
    }
}

/// Reglas genéricas sobre cada componente de la ruta (un nombre de fichero
/// también se publica). Por componentes, para que `a/b/c` no parezca una
/// cadena base64.
fn revisar_nombre(ruta: &str, p: &Permitidos) -> Vec<Hallazgo> {
    ruta.split('/')
        .flat_map(|componente| reglas::genericas(componente, p))
        .filter(|(regla, _)| !p.ruta_permitida(ruta, *regla))
        .map(|(regla, detalle)| Hallazgo {
            ruta: ruta.to_owned(),
            linea: 0,
            regla,
            detalle: format!("nombre: {detalle}"),
        })
        .collect()
}

fn revisar_fichero(camino: &Path, ruta: &str, p: &Permitidos, l: &ListasLocales) -> Vec<Hallazgo> {
    let ilegible = || {
        vec![Hallazgo {
            ruta: ruta.to_owned(),
            linea: 0,
            regla: Regla::Binario,
            detalle: "ilegible".to_owned(),
        }]
    };
    let Ok(meta) = camino.symlink_metadata() else {
        return ilegible();
    };
    if meta.file_type().is_symlink() {
        // Se publica el destino del enlace, no el contenido.
        let Ok(destino) = fs::read_link(camino) else {
            return ilegible();
        };
        return revisar_texto(ruta, &destino.to_string_lossy(), p, l);
    }
    let Ok(bytes) = fs::read(camino) else {
        return ilegible();
    };
    let extension = ruta
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    if EXTENSIONES_BINARIAS.contains(&extension.as_str()) || bytes.contains(&0) {
        return revisar_binario(ruta, &bytes, p);
    }
    match String::from_utf8(bytes) {
        Ok(texto) => revisar_texto(ruta, &texto, p, l),
        Err(e) => revisar_binario(ruta, e.as_bytes(), p),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn rutas_prohibidas() {
        assert_eq!(prohibida(".superpowers/sdd/x.md"), Some(".superpowers/"));
        assert_eq!(
            prohibida("docs/superpowers/a.md"),
            Some("docs/superpowers/")
        );
        assert_eq!(prohibida("e2e/capturas/a.png"), Some("e2e/capturas/"));
        assert_eq!(prohibida("crates/x/target/debug/y"), Some("target/"));
        assert_eq!(prohibida("datos/panel.db-wal"), Some("*.db"));
        assert_eq!(
            prohibida("deploy/config.local"),
            Some("deploy/config.local")
        );
        assert_eq!(prohibida("a/real-secrets-2.txt"), Some("real-secrets*.txt"));
        assert_eq!(prohibida("deploy/config.example"), None);
        assert_eq!(prohibida("docs/superpowers.md"), None);
        assert_eq!(prohibida("src/target.rs"), None);
    }

    #[test]
    fn solo_normalizado() {
        let r = Path::new(".");
        assert_eq!(normalizar(r, Path::new("./tools/")), "tools");
        assert_eq!(normalizar(r, Path::new(".")), "");
    }
}
