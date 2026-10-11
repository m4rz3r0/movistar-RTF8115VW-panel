//! Modo `--arbol`: revisa los ficheros que se publicarían.

use crate::{
    Hallazgo, ListasLocales, Permitidos, Regla, ocultar_ruta, revisar_binario, revisar_nombre,
    revisar_texto,
};
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
        // Cualquier hallazgo sobre el nombre (de cualquier regla) oculta la ruta.
        let nombre = revisar_nombre(ruta, p, l);
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
            ocultar_ruta(&mut h, k + 1);
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
pub(crate) fn prohibida(ruta: &str) -> Option<&'static str> {
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
    revisar_contenido(ruta, &bytes, p, l)
}

/// `true` si la extensión de `ruta` es de las que siempre son binarios.
pub(crate) fn binario_por_extension(ruta: &str) -> bool {
    let extension = ruta
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    EXTENSIONES_BINARIAS.contains(&extension.as_str())
}

/// Revisa el contenido de un fichero (o de un blob de git) según su tipo:
/// los binarios conocidos por la extensión, como binarios; el texto UTF-8,
/// como texto; y lo demás, como binario **y** como el texto que se pueda leer.
pub(crate) fn revisar_contenido(
    ruta: &str,
    bytes: &[u8],
    p: &Permitidos,
    l: &ListasLocales,
) -> Vec<Hallazgo> {
    if binario_por_extension(ruta) {
        return revisar_binario(ruta, bytes, p);
    }
    if !bytes.contains(&0)
        && let Ok(texto) = std::str::from_utf8(bytes)
    {
        return revisar_texto(ruta, texto, p, l);
    }
    // Ni texto UTF-8 ni binario conocido: se exige la revisión del binario y
    // además se revisa el texto que se pueda leer en él.
    let mut h = revisar_binario(ruta, bytes, p);
    h.extend(revisar_texto(ruta, &texto_legible(bytes), p, l));
    h
}

/// Texto de un fichero que no es UTF-8: UTF-16 si lleva BOM; si no, los
/// bytes sin NUL (UTF-16 sin BOM queda en ASCII) leídos con sustitución.
fn texto_legible(bytes: &[u8]) -> String {
    let utf16 = |resto: &[u8], le: bool| {
        let unidades: Vec<u16> = resto
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| {
                if le {
                    u16::from_le_bytes(c)
                } else {
                    u16::from_be_bytes(c)
                }
            })
            .collect();
        String::from_utf16_lossy(&unidades)
    };
    match bytes {
        [0xff, 0xfe, resto @ ..] => utf16(resto, true),
        [0xfe, 0xff, resto @ ..] => utf16(resto, false),
        _ => {
            let sin_nul: Vec<u8> = bytes.iter().copied().filter(|&b| b != 0).collect();
            String::from_utf8_lossy(&sin_nul).into_owned()
        }
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
