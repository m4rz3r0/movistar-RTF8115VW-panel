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
//! - `--nombre-ref NOMBRE` (con `--diff` y `--historial`, repetible): revisa
//!   también ese nombre de ref (lo usa el gancho `pre-push`).
//!
//! Usa las excepciones de `tools/publicacion-permitidos.txt` del directorio
//! revisado. Imprime un hallazgo por línea, `ruta:linea  regla  [detalle]`,
//! sin el valor encontrado. Sale con 0 si no hay hallazgos, con 1 si los hay
//! y con 2 si el uso es incorrecto, la lista de permitidos no es válida o git
//! falla.
//!
//! Nunca repite sus argumentos en la salida: una ruta, un rango o un nombre
//! de ref pueden contener un dato.

use comprobar_publicacion::{
    Hallazgo, ListasLocales, Permitidos, arbol, diff, historial, revisar_refs,
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::ExitCode,
};

const PERMITIDOS: &str = "tools/publicacion-permitidos.txt";

const USO: &str = "uso: comprobar-publicacion --arbol [DIR] [--solo RUTA…] [--sin-locales]
       comprobar-publicacion --diff A..B [REPO] [--nombre-ref NOMBRE…] [--sin-locales]
       comprobar-publicacion --historial [REPO] [--rev REV…] [--nombre-ref NOMBRE…] [--sin-locales]";

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

fn leer_permitidos(raiz: &Path) -> Result<Permitidos, ExitCode> {
    match std::fs::read_to_string(raiz.join(PERMITIDOS)) {
        Ok(texto) => Permitidos::leer(&texto).map_err(|e| {
            eprintln!("{PERMITIDOS}: {e}");
            ExitCode::from(2)
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("aviso: no hay {PERMITIDOS}; no se aplica ninguna excepción");
            Ok(Permitidos::default())
        }
        Err(e) => {
            eprintln!("{PERMITIDOS}: {e}");
            Err(ExitCode::from(2))
        }
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
    let antes = args.len();
    args.retain(|a| a != "--sin-locales");
    let sin_locales = args.len() != antes;
    let Some((modo, raiz)) = interpretar(&args) else {
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

    let permitidos = match leer_permitidos(&raiz) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let locales = if sin_locales {
        ListasLocales::vacia()
    } else {
        let (l, avisos) = ListasLocales::cargar(&raiz);
        for a in avisos {
            eprintln!("{a}");
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
