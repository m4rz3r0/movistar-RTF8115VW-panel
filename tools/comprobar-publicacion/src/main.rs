//! `comprobar-publicacion --arbol [DIR] [--solo RUTA…]`
//!
//! Revisa el árbol de `DIR` (por defecto, el directorio actual) con las reglas
//! genéricas y las excepciones de `DIR/tools/publicacion-permitidos.txt`.
//! Imprime un hallazgo por línea, `ruta:linea  regla  [detalle]`, sin el
//! valor encontrado. Sale con 0 si no hay hallazgos, con 1 si los hay y con 2
//! si el uso es incorrecto o la lista de permitidos no es válida.
//!
//! Con `--solo` revisa solo esas rutas (relativas a `DIR`) y no comprueba los
//! ficheros obligatorios; una ruta que no existe es un error de uso.
//!
//! Nunca repite sus argumentos en la salida: una ruta puede contener un dato.

use comprobar_publicacion::{ListasLocales, Permitidos, arbol};
use std::{ffi::OsString, path::PathBuf, process::ExitCode};

const PERMITIDOS: &str = "tools/publicacion-permitidos.txt";

const USO: &str = "uso: comprobar-publicacion --arbol [DIR] [--solo RUTA…]";

fn uso() -> ExitCode {
    eprintln!("{USO}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let mut it = args.iter().peekable();
    match it.next().and_then(|a| a.to_str()) {
        Some("--arbol") => {}
        Some("-h" | "--ayuda" | "--help") => {
            println!("{USO}");
            return ExitCode::SUCCESS;
        }
        _ => return uso(),
    }
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
            return uso();
        }
        solo.extend(it.map(PathBuf::from));
        if solo.is_empty() {
            return uso();
        }
    }
    // Los argumentos nunca se repiten en la salida: pueden contener un dato
    // (por ejemplo, un fichero borrado cuyo nombre lo lleva).
    if !raiz.is_dir() {
        eprintln!("error: el directorio indicado no existe o no es un directorio");
        return ExitCode::from(2);
    }
    for (k, s) in solo.iter().enumerate() {
        let camino = if s.is_absolute() {
            s.clone()
        } else {
            raiz.join(s)
        };
        if camino.symlink_metadata().is_err() {
            eprintln!("error: la ruta n.º {} de --solo no existe", k + 1);
            return ExitCode::from(2);
        }
    }

    let permitidos = match std::fs::read_to_string(raiz.join(PERMITIDOS)) {
        Ok(texto) => match Permitidos::leer(&texto) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{PERMITIDOS}: {e}");
                return ExitCode::from(2);
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("aviso: no hay {PERMITIDOS}; no se aplica ninguna excepción");
            Permitidos::default()
        }
        Err(e) => {
            eprintln!("{PERMITIDOS}: {e}");
            return ExitCode::from(2);
        }
    };

    let hallazgos = arbol(&raiz, &solo, &permitidos, &ListasLocales::vacia());
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
