//! Genera las fixtures saneadas a partir de las respuestas capturadas.
//!
//! Uso: `sanear-fixtures <dir_bruto> <dir_fixtures> [--secrets <fichero>]`
//!
//! Copia las respuestas `cbGet*`, `cbList*` y `cbDhcpV4SvrPool*` que sean un
//! `<Response>` válido, sustituyendo los valores sensibles y neutralizando la
//! topología de la red (ver [`Neutralizador`]). Falla sin tocar
//! las fixtures si aparece una etiqueta que no está en `fixtures-tags.txt` o
//! un nombre de fichero con parámetros sensibles. Con `--secrets` añade (modo
//! 600) los valores reales retirados al fichero, conservando las líneas que ya
//! tuviera (p. ej. secretos conocidos por otra vía añadidos a mano).

use router_sim::sanitize::{Neutralizador, Sanitizer};
use std::{
    collections::BTreeSet, fs, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf,
    process::ExitCode,
};

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
    // ficheros que se procesan antes de encontrarlos. Todo se sanea en memoria
    // antes de tocar el directorio de salida. El neutralizador numera por
    // orden de aparición: los ficheros van ordenados por nombre.
    let mut s = Sanitizer::new();
    let mut n = Neutralizador::new();
    for (_, text) in &docs {
        s.learn(text);
    }
    let mut clean = Vec::new();
    for (name, text) in &docs {
        s.check_file_name(name.trim_end_matches(".xml"))
            .map_err(|e| format!("{name}: {e}"))?;
        let saneado = s.sanitize(text).map_err(|e| format!("{name}: {e}"))?;
        let neutro = n
            .neutralizar(name, &saneado)
            .map_err(|e| format!("{name}: {e}"))?;
        clean.push((name, neutro));
    }

    fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    for old in fs::read_dir(&out).map_err(|e| e.to_string())? {
        let old = old.map_err(|e| e.to_string())?.path();
        if old.extension().is_some_and(|e| e == "xml") {
            fs::remove_file(&old).map_err(|e| format!("{}: {e}", old.display()))?;
        }
    }
    for (name, text) in &clean {
        fs::write(out.join(name), text).map_err(|e| format!("{name}: {e}"))?;
    }
    eprintln!("{} fixtures escritas en {}", clean.len(), out.display());

    if let Some(path) = secrets {
        let mut all: BTreeSet<String> = match fs::read_to_string(&path) {
            Ok(old) => old.lines().map(str::to_owned).collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeSet::new(),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        all.extend(s.removed_values().map(str::to_owned));
        all.retain(|v| !v.is_empty());
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        for v in &all {
            writeln!(f, "{v}").map_err(|e| e.to_string())?;
        }
        eprintln!("{} valores reales en {}", all.len(), path.display());
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
