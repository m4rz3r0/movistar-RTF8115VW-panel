//! Genera las fixtures saneadas a partir de las respuestas capturadas.
//!
//! Uso: `sanear-fixtures <dir_bruto> <dir_fixtures> [--secrets <fichero>]`
//!
//! Copia las respuestas `cbGet*`, `cbList*` y `cbDhcpV4SvrPool*` que sean un
//! `<Response>` válido, sustituyendo los valores sensibles. Con `--secrets`
//! escribe (modo 600) los valores reales retirados, uno por línea.

use router_sim::sanitize::Sanitizer;
use std::{fs, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf, process::ExitCode};

const PREFIXES: &[&str] = &["cbGet", "cbList", "cbDhcpV4SvrPool"];

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let usage = "uso: sanear-fixtures <dir_bruto> <dir_fixtures> [--secrets <fichero>]";
    let raw = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().ok_or(usage)?);
    let secrets = match (args.next().as_deref(), args.next()) {
        (None, _) => None,
        (Some("--secrets"), Some(f)) => Some(PathBuf::from(f)),
        _ => return Err(usage.into()),
    };

    let mut names: Vec<String> = fs::read_dir(&raw)
        .map_err(|e| format!("{}: {e}", raw.display()))?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".xml") && PREFIXES.iter().any(|p| n.starts_with(p)))
        .collect();
    names.sort();

    fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    for old in fs::read_dir(&out).map_err(|e| e.to_string())? {
        let old = old.map_err(|e| e.to_string())?.path();
        if old.extension().is_some_and(|e| e == "xml") {
            fs::remove_file(&old).map_err(|e| format!("{}: {e}", old.display()))?;
        }
    }

    let mut docs = Vec::new();
    for name in &names {
        let text = fs::read_to_string(raw.join(name)).map_err(|e| format!("{name}: {e}"))?;
        let call = name
            .trim_end_matches(".xml")
            .split("__")
            .next()
            .unwrap_or("");
        if hgu::xml::parse_response(call, &text).is_err() {
            eprintln!("se omite {name}: no es una respuesta XML de {call}");
            continue;
        }
        docs.push((name, text));
    }

    // Dos pasadas: los SSID y secretos aprendidos se retiran también de los
    // ficheros que se procesan antes de encontrarlos.
    let mut s = Sanitizer::new();
    for (_, text) in &docs {
        s.learn(text);
    }
    let mut written = 0;
    for (name, text) in &docs {
        let clean = s.sanitize(text).map_err(|e| format!("{name}: {e}"))?;
        fs::write(out.join(name), clean).map_err(|e| format!("{name}: {e}"))?;
        written += 1;
    }
    eprintln!("{written} fixtures escritas en {}", out.display());

    if let Some(path) = secrets {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let mut n = 0;
        for v in s.removed_values() {
            writeln!(f, "{v}").map_err(|e| e.to_string())?;
            n += 1;
        }
        eprintln!("{n} valores reales en {}", path.display());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
