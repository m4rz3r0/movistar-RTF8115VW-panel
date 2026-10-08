//! Generador del catálogo de funciones.
//!
//! - `gen-catalogo borrador <dir_bruto> <salida.toml> [--forzar]`: recorre las
//!   páginas (`*.asp`) y scripts (`js__*.js`) capturados del router y escribe un
//!   **borrador** de catálogo: llamadas `cb*.xml` con sus parámetros (literales
//!   del `$.post` o campos del formulario serializado), URL de la UI básica
//!   (`*.cmd|.cgi|.wl`), páginas ocultas y peligrosas, emparejado
//!   lectura/escritura y listas. Nunca escribe en `crates/catalogo/funciones.toml`
//!   (la fuente de verdad, revisada a mano) y no pisa un fichero existente sin
//!   `--forzar`. Las descripciones quedan como `PENDIENTE` para revisarlas.
//! - `gen-catalogo comprobar <dir_bruto>`: lista lo que hay en los datos brutos y
//!   falta en el catálogo revisado.
//! - `gen-catalogo --api-md [salida.md]`: genera `docs/api.md` del catálogo
//!   revisado (y de los nombres de elementos de las fixtures saneadas).
//!
//! Los datos brutos contienen secretos: el generador solo usa nombres de
//! funciones, parámetros, campos y opciones de formularios, nunca valores de
//! las respuestas.

mod api_md;
mod extraer;

use std::{path::Path, process::ExitCode};

fn raiz() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn uso() -> ExitCode {
    eprintln!(
        "uso:\n  gen-catalogo borrador <dir_bruto> <salida.toml> [--forzar]\n  \
         gen-catalogo comprobar <dir_bruto>\n  gen-catalogo --api-md [salida.md]"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let r = match args.as_slice() {
        ["borrador", dir, salida] => borrador(Path::new(dir), Path::new(salida), false),
        ["borrador", dir, salida, "--forzar"] => borrador(Path::new(dir), Path::new(salida), true),
        ["comprobar", dir] => comprobar(Path::new(dir)),
        ["--api-md"] => api_md::escribir(&raiz().join("docs/api.md")),
        ["--api-md", salida] => api_md::escribir(Path::new(salida)),
        _ => return uso(),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gen-catalogo: {e}");
            ExitCode::FAILURE
        }
    }
}

fn borrador(dir: &Path, salida: &Path, forzar: bool) -> Result<(), String> {
    let fuente = raiz().join("crates/catalogo/funciones.toml");
    if let (Ok(a), Ok(b)) = (salida.canonicalize(), fuente.canonicalize())
        && a == b
    {
        return Err(
            "funciones.toml está revisado a mano: escribe el borrador en otro sitio".into(),
        );
    }
    if salida.exists() && !forzar {
        return Err(format!("{} ya existe (usa --forzar)", salida.display()));
    }
    let funciones = extraer::extraer(dir)?;
    let texto = extraer::a_toml(&funciones)?;
    std::fs::write(salida, texto).map_err(|e| format!("{}: {e}", salida.display()))?;
    eprintln!("{} funciones → {}", funciones.len(), salida.display());
    Ok(())
}

/// Nombres presentes en los datos brutos que faltan en el catálogo revisado.
fn faltan(dir: &Path) -> Result<Vec<String>, String> {
    let cat = catalogo::Catalogo::load().map_err(|e| e.to_string())?;
    Ok(extraer::extraer(dir)?
        .into_iter()
        .map(|f| f.name)
        .filter(|n| cat.funcion(n).is_none())
        .collect())
}

fn comprobar(dir: &Path) -> Result<(), String> {
    let f = faltan(dir)?;
    for n in &f {
        println!("{n}");
    }
    if f.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} funciones de los datos brutos no están en el catálogo",
            f.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Directorio de los datos brutos del análisis (`RTF_RAW_DIR` o el de la
    /// tarea); se borran tras generar el catálogo, así que la prueba se omite
    /// si no están.
    fn dir_bruto() -> Option<std::path::PathBuf> {
        let d = std::env::var_os("RTF_RAW_DIR")
            .map(Into::into)
            .unwrap_or_else(|| raiz().join(".superpowers/sdd/2026-10-08-nucleo/raw"));
        d.is_dir().then_some(d)
    }

    #[test]
    fn areas_cubren_todas_las_funciones_de_calls_tsv() {
        let Some(d) = dir_bruto() else {
            eprintln!("sin datos brutos: prueba omitida");
            return;
        };
        let f = faltan(&d).unwrap();
        assert!(f.is_empty(), "faltan en el catálogo: {f:?}");
    }

    #[test]
    fn no_pisa_el_catalogo_revisado() {
        let e = borrador(
            Path::new("/nada"),
            &raiz().join("crates/catalogo/funciones.toml"),
            true,
        )
        .unwrap_err();
        assert!(e.contains("revisado a mano"), "{e}");
    }
}
