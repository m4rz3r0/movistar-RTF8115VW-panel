//! Ficheros de la web (`/pkg/*`) y caché del navegador.
//!
//! cargo-leptos compila el wasm, el JS y el CSS con un hash de su contenido
//! en el nombre (`hash-files` en `Cargo.toml`) y deja los hashes en
//! `<site>/hash.txt`, dentro de la web que copia el despliegue. Con
//! `--precompress` (la compilación de `deploy/freebsd-jail/actualizar.sh`)
//! deja además las versiones `.br` y `.gz`, que sirve el manejador de
//! ficheros de Leptos según `Accept-Encoding`, sin gastar CPU por petición.
//!
//! - Un fichero de `/pkg` con hash en el nombre no cambia nunca: caché de un
//!   año, `immutable`.
//! - Uno sin hash (una compilación sin `hash.txt`), `no-cache`: se revalida.
//! - Las páginas HTML, `no-store`: llevan datos del router y de la sesión.

use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
};
use leptos::prelude::LeptosOptions;
use std::path::Path;

/// Nombre del fichero de hashes dentro de la web (ver `hash-file-name` en
/// `Cargo.toml`, relativo a `target/<perfil>`).
const FICHERO_HASH: &str = "hash.txt";

const INMUTABLE: &str = "public, max-age=31536000, immutable";
const REVALIDAR: &str = "no-cache";
const SIN_GUARDAR: &str = "no-store";

/// Usa los nombres con hash si la web trae `hash.txt` (Leptos lo busca por
/// la ruta absoluta que se fija aquí). Sin él, los nombres sin hash.
pub fn configurar_hash(opciones: &mut LeptosOptions) {
    let ruta = Path::new(opciones.site_root.as_ref()).join(FICHERO_HASH);
    let ruta = std::path::absolute(&ruta).unwrap_or(ruta);
    opciones.hash_files = ruta.is_file();
    opciones.hash_file = ruta.to_string_lossy().as_ref().into();
}

/// ¿Lleva `nombre` un hash de cargo-leptos (`panel.<hash>.<ext>`, con la
/// extensión de compresión si la hay)?
fn con_hash(nombre: &str) -> bool {
    let nombre = nombre
        .strip_suffix(".br")
        .or_else(|| nombre.strip_suffix(".gz"))
        .unwrap_or(nombre);
    let partes: Vec<&str> = nombre.split('.').collect();
    partes.len() == 3
        && partes[1].len() >= 16
        && partes[1]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Middleware: `Cache-Control` de los ficheros de `/pkg` y de las páginas
/// (si la respuesta no trae ya el suyo).
pub async fn cabeceras(req: Request, next: Next) -> Response {
    let pkg = req
        .uri()
        .path()
        .strip_prefix("/pkg/")
        .map(|n| con_hash(n.rsplit('/').next().unwrap_or(n)));
    let mut resp = next.run(req).await;
    let html = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|t| t.starts_with("text/html"));
    // Un 304 repite las cabeceras de caché del 200 (RFC 9110 §15.4.5).
    let servido = resp.status().is_success() || resp.status() == StatusCode::NOT_MODIFIED;
    let valor = match pkg {
        Some(_) if !servido => None,
        Some(true) => Some(INMUTABLE),
        Some(false) => Some(REVALIDAR),
        None if html => Some(SIN_GUARDAR),
        None => None,
    };
    if let Some(v) = valor {
        resp.headers_mut()
            .entry(header::CACHE_CONTROL)
            .or_insert(HeaderValue::from_static(v));
    }
    // El mismo fichero va comprimido o no según `Accept-Encoding`: que una
    // caché intermedia no sirva una versión a quien no la entiende.
    if pkg.is_some() && servido && !resp.headers().contains_key(header::VARY) {
        resp.headers_mut()
            .insert(header::VARY, HeaderValue::from_static("accept-encoding"));
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconoce_los_nombres_con_hash() {
        assert!(con_hash("panel.Ab3_x-9QwErTyUiOpAsDfG.wasm"));
        assert!(con_hash("panel.Ab3_x-9QwErTyUiOpAsDfG.wasm.br"));
        assert!(con_hash("panel.Ab3_x-9QwErTyUiOpAsDfG.css.gz"));
        assert!(!con_hash("panel.wasm"));
        assert!(!con_hash("panel.wasm.br"));
        assert!(!con_hash("panel_bg.wasm.d.ts"));
    }
}
