//! `comprobar-publicacion`: revisa lo que se va a publicar.
//!
//! - `--arbol [DIR] [--solo RUTA…]`: el árbol de `DIR` (por defecto, el
//!   directorio actual). Con `--solo` revisa solo esas rutas (relativas a
//!   `DIR`) y no comprueba los ficheros obligatorios; una ruta que no existe
//!   es un error de uso.
//! - `--diff A..B [REPO]`: lo que añaden los commits del rango.
//! - `--historial [REPO] [--rev REV…]`: todo lo alcanzable desde las refs (o
//!   desde las revisiones `--rev`).
//!
//! Opciones comunes:
//! - `--sin-locales`: no lee las listas locales del dueño (CI);
//! - `--listas DIR` (o `$RP_PUBLICACION_LISTAS`): dónde están `.superpowers/`
//!   y `deploy/config.local`. Por defecto, la raíz del repositorio principal
//!   (el padre de `git rev-parse --git-common-dir`), que también vale desde un
//!   worktree enlazado; fuera de un repositorio, el directorio revisado;
//! - `--exigir-listas` (el gancho `pre-push`): si falta una lista obligatoria
//!   ([`ListasLocales::faltan`]), es un error y sale con 2;
//! - `--nombre-ref NOMBRE` (con `--diff` y `--historial`, repetible): revisa
//!   también ese nombre de ref (lo usa el gancho `pre-push`).
//!
//! Usa las excepciones de `tools/publicacion-permitidos.txt`: con `--arbol`,
//! la del directorio revisado; con `--diff`, la del final del rango; con
//! `--historial`, la de la primera `--rev` (o de `HEAD`). Así una excepción
//! sin confirmar no vale para lo que se sube. Imprime un hallazgo por línea, `ruta:linea  regla  [detalle]`,
//! sin el valor encontrado. Sale con 0 si no hay hallazgos, con 1 si los hay
//! y con 2 si el uso es incorrecto, la lista de permitidos no es válida o git
//! falla.
//!
//! Nunca repite sus argumentos en la salida: una ruta, un rango o un nombre
//! de ref pueden contener un dato.

use comprobar_publicacion::{
    Hallazgo, ListasLocales, Permitidos, arbol, diff, fichero_en_commit, historial, revisar_refs,
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const PERMITIDOS: &str = "tools/publicacion-permitidos.txt";

const USO: &str = "uso: comprobar-publicacion --arbol [DIR] [--solo RUTA…]
       comprobar-publicacion --diff A..B [REPO] [--nombre-ref NOMBRE…]
       comprobar-publicacion --historial [REPO] [--rev REV…] [--nombre-ref NOMBRE…]
opciones comunes: [--sin-locales | --listas DIR] [--exigir-listas]";

fn uso() -> ExitCode {
    eprintln!("{USO}");
    ExitCode::from(2)
}

fn error(texto: &str) -> ExitCode {
    eprintln!("error: {texto}");
    ExitCode::from(2)
}

enum Modo {
    Arbol {
        solo: Vec<PathBuf>,
    },
    Diff {
        rango: String,
        refs: Vec<String>,
    },
    Historial {
        revs: Vec<String>,
        refs: Vec<String>,
    },
}

/// Interpreta los argumentos (sin `--sin-locales`); `None` es un error de uso.
fn interpretar(args: &[OsString]) -> Option<(Modo, PathBuf)> {
    let (modo, resto) = args.split_first()?;
    let mut it = resto.iter().peekable();
    match modo.to_str()? {
        "--arbol" => {
            let mut raiz = PathBuf::from(".");
            if let Some(a) = it.peek()
                && !a.to_string_lossy().starts_with("--")
            {
                raiz = PathBuf::from(*a);
                it.next();
            }
            let mut solo = Vec::new();
            if let Some(a) = it.next() {
                if a != "--solo" {
                    return None;
                }
                solo.extend(it.map(PathBuf::from));
                if solo.is_empty() {
                    return None;
                }
            }
            Some((Modo::Arbol { solo }, raiz))
        }
        m @ ("--diff" | "--historial") => {
            let mut posicionales = Vec::new();
            let mut revs = Vec::new();
            let mut refs = Vec::new();
            while let Some(a) = it.next() {
                match a.to_str() {
                    Some("--rev") if m == "--historial" => {
                        revs.push(it.next()?.to_str()?.to_owned());
                    }
                    Some("--nombre-ref") => refs.push(it.next()?.to_string_lossy().into_owned()),
                    _ if a.to_string_lossy().starts_with('-') => return None,
                    _ => posicionales.push(a),
                }
            }
            let mut posicionales = posicionales.into_iter();
            let modo = if m == "--diff" {
                let rango = posicionales.next()?.to_str()?.to_owned();
                Modo::Diff { rango, refs }
            } else {
                Modo::Historial { revs, refs }
            };
            let raiz = posicionales
                .next()
                .map_or_else(|| ".".into(), PathBuf::from);
            if posicionales.next().is_some() {
                return None;
            }
            Some((modo, raiz))
        }
        _ => None,
    }
}

/// La lista de permitidos del directorio (`rev` = `None`) o del commit `rev`.
fn leer_permitidos(raiz: &Path, rev: Option<&str>) -> Result<Permitidos, ExitCode> {
    let texto = match rev {
        None => match std::fs::read(raiz.join(PERMITIDOS)) {
            Ok(t) => Some(t),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                eprintln!("{PERMITIDOS}: {e}");
                return Err(ExitCode::from(2));
            }
        },
        Some(rev) => fichero_en_commit(raiz, rev, PERMITIDOS).map_err(|e| error(&e))?,
    };
    let Some(texto) = texto else {
        eprintln!("aviso: no hay {PERMITIDOS}; no se aplica ninguna excepción");
        return Ok(Permitidos::default());
    };
    Permitidos::leer(&String::from_utf8_lossy(&texto)).map_err(|e| {
        eprintln!("{PERMITIDOS}: {e}");
        ExitCode::from(2)
    })
}

/// Dónde buscar las listas locales: `--listas`, `$RP_PUBLICACION_LISTAS`, la
/// raíz del repositorio principal de `raiz` o `raiz`.
fn raiz_de_listas(raiz: &Path, opcion: Option<PathBuf>) -> PathBuf {
    if let Some(d) = opcion {
        return d;
    }
    if let Some(d) = std::env::var_os("RP_PUBLICACION_LISTAS").filter(|v| !v.is_empty()) {
        return PathBuf::from(d);
    }
    let comun = Command::new("git")
        .arg("-C")
        .arg(raiz)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim_end_matches('\n')));
    match comun.as_deref().and_then(Path::parent) {
        Some(principal) => principal.to_path_buf(),
        None => raiz.to_path_buf(),
    }
}

fn main() -> ExitCode {
    let mut args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if matches!(
        args.first().and_then(|a| a.to_str()),
        Some("-h" | "--ayuda" | "--help")
    ) {
        println!("{USO}");
        return ExitCode::SUCCESS;
    }
    // Opciones comunes, en cualquier posición.
    let mut sin_locales = false;
    let mut exigir_listas = false;
    let mut listas = None;
    let mut resto = Vec::new();
    let mut it = args.drain(..);
    while let Some(a) = it.next() {
        match a.to_str() {
            Some("--sin-locales") => sin_locales = true,
            Some("--exigir-listas") => exigir_listas = true,
            Some("--listas") => match it.next() {
                Some(d) => listas = Some(PathBuf::from(d)),
                None => return uso(),
            },
            _ => resto.push(a),
        }
    }
    drop(it);
    if sin_locales && (exigir_listas || listas.is_some()) {
        return uso();
    }
    let Some((modo, raiz)) = interpretar(&resto) else {
        return uso();
    };

    // Los argumentos nunca se repiten en la salida: pueden contener un dato
    // (por ejemplo, un fichero borrado cuyo nombre lo lleva).
    if !raiz.is_dir() {
        return error("el directorio indicado no existe o no es un directorio");
    }
    if let Modo::Arbol { solo } = &modo {
        for (k, s) in solo.iter().enumerate() {
            let camino = if s.is_absolute() {
                s.clone()
            } else {
                raiz.join(s)
            };
            if camino.symlink_metadata().is_err() {
                return error(&format!("la ruta n.º {} de --solo no existe", k + 1));
            }
        }
    }

    // Lo que se sube lleva su propia lista de permitidos: la del commit.
    let rev_permitidos = match &modo {
        Modo::Arbol { .. } => None,
        Modo::Diff { rango, .. } => Some(
            rango
                .rsplit_once("..")
                .map_or(rango.as_str(), |(_, fin)| fin.trim_start_matches('.'))
                .to_owned(),
        )
        .map(|r| if r.is_empty() { "HEAD".to_owned() } else { r }),
        Modo::Historial { revs, .. } => {
            Some(revs.first().cloned().unwrap_or_else(|| "HEAD".to_owned()))
        }
    };
    let permitidos = match leer_permitidos(&raiz, rev_permitidos.as_deref()) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let locales = if sin_locales {
        ListasLocales::vacia()
    } else {
        let (l, avisos) = ListasLocales::cargar(&raiz_de_listas(&raiz, listas));
        for a in avisos {
            eprintln!("{a}");
        }
        if exigir_listas && !l.faltan().is_empty() {
            let reglas: Vec<&str> = l.faltan().iter().map(|r| r.as_str()).collect();
            return error(&format!(
                "faltan listas locales obligatorias ({}); no se revisa nada",
                reglas.join(", ")
            ));
        }
        l
    };

    let con_refs = |mut h: Vec<Hallazgo>, refs: &[String]| {
        h.extend(revisar_refs(refs, &permitidos, &locales));
        h
    };
    let resultado = match &modo {
        Modo::Arbol { solo } => Ok(arbol(&raiz, solo, &permitidos, &locales)),
        Modo::Diff { rango, refs } => {
            diff(&raiz, rango, &permitidos, &locales).map(|h| con_refs(h, refs))
        }
        Modo::Historial { revs, refs } => {
            historial(&raiz, revs, &permitidos, &locales).map(|h| con_refs(h, refs))
        }
    };
    let hallazgos = match resultado {
        Ok(h) => h,
        Err(e) => return error(&e),
    };
    for h in &hallazgos {
        println!("{h}");
    }
    if hallazgos.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("{} hallazgos", hallazgos.len());
        ExitCode::from(1)
    }
}
