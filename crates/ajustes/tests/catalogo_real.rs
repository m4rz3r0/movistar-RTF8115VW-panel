//! El catálogo de ajustes compilado: cuadra con el de funciones, tiene todos
//! sus textos en los dos idiomas (y ninguno de sobra) y trae los ajustes que
//! usan las páginas por tema.

use ajustes::{CatalogoAjustes, claves_requeridas, comprobar};
use catalogo::{Catalogo, mask::Mascara};
use i18n::Idioma;
use std::{collections::BTreeSet, fs, path::PathBuf};

fn cargar() -> CatalogoAjustes {
    CatalogoAjustes::load().expect("ajustes/*.toml legibles")
}

#[test]
fn catalogo_real_sin_fallos() {
    let fallos = comprobar(
        &cargar(),
        &Catalogo::load().unwrap(),
        &Mascara::load().unwrap(),
    );
    assert!(fallos.is_empty(), "{fallos:#?}");
}

/// Ficheros `ajustes-<tema>.ftl` de `idioma`.
fn ficheros_de_ajustes(idioma: &str) -> Vec<PathBuf> {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../i18n/locales")).join(idioma);
    let mut rutas: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "ftl")
                && p.file_stem()
                    .is_some_and(|s| s.to_string_lossy().starts_with("ajustes-"))
        })
        .collect();
    rutas.sort();
    rutas
}

/// Claves de los mensajes de un `.ftl` (líneas `clave = …` sin sangría).
fn claves_del_fichero(ruta: &PathBuf) -> Vec<String> {
    fs::read_to_string(ruta)
        .unwrap_or_else(|e| panic!("{}: {e}", ruta.display()))
        .lines()
        .filter(|l| !l.starts_with([' ', '\t', '#']) && !l.trim().is_empty())
        .filter_map(|l| l.split_once('=').map(|(k, _)| k.trim().to_owned()))
        .collect()
}

#[test]
fn claves_i18n_completas_en_los_dos_sentidos() {
    let c = cargar();
    let requeridas: BTreeSet<String> = c.todos().iter().flat_map(claves_requeridas).collect();
    // Todo lo que pide el catálogo existe en los dos idiomas.
    for idioma in Idioma::TODOS {
        let faltan: Vec<&String> = requeridas
            .iter()
            .filter(|k| !i18n::existe(idioma, k))
            .collect();
        assert!(faltan.is_empty(), "faltan en {idioma:?}: {faltan:#?}");
    }
    // Y los ficheros de ajustes no traen claves que nadie pide (incluidos
    // los avisos y efectos propios de cada tema).
    for idioma in ["es", "en"] {
        let rutas = ficheros_de_ajustes(idioma);
        assert!(!rutas.is_empty(), "sin ficheros ajustes-*.ftl en {idioma}");
        for ruta in rutas {
            let sobran: Vec<String> = claves_del_fichero(&ruta)
                .into_iter()
                .filter(|k| !requeridas.contains(k))
                .collect();
            assert!(sobran.is_empty(), "{}: sobran {sobran:#?}", ruta.display());
        }
    }
}

#[test]
fn ajustes_minimos_presentes() {
    let c = cargar();
    let faltan: Vec<&str> = [
        "wifi.red_24",
        "wifi.red_5",
        "wifi.invitados",
        "wifi.radio_24",
        "wifi.radio_5",
        "red_local.dhcp",
        "red_local.reservas",
        "red_local.dispositivos",
        "red_local.ipv6",
        "puertos.reglas",
        "puertos.upnp",
        "puertos.alg",
        "internet.dns",
        "internet.ddns",
    ]
    .into_iter()
    .filter(|id| c.ajuste(id).is_none())
    .collect();
    assert!(faltan.is_empty(), "faltan {faltan:?}");
}
