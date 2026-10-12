//! El catálogo de ajustes compilado: cuadra con el de funciones, tiene todos
//! sus textos en los dos idiomas (y ninguno de sobra) y trae los ajustes que
//! usan las páginas por tema.

use ajustes::{Ajuste, Campo, CatalogoAjustes, Componente, Nivel, claves_requeridas, comprobar};
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
    for idioma in Idioma::TODOS {
        // Todo lo que pide el catálogo existe en el idioma.
        let faltan: Vec<&String> = requeridas
            .iter()
            .filter(|k| !i18n::existe(idioma, k))
            .collect();
        assert!(faltan.is_empty(), "faltan en {idioma:?}: {faltan:#?}");
        // Y sus ficheros de ajustes no traen claves que nadie pide
        // (incluidos los avisos y efectos propios de cada tema).
        let rutas = ficheros_de_ajustes(idioma.codigo());
        assert!(
            !rutas.is_empty(),
            "sin ficheros ajustes-*.ftl en {idioma:?}"
        );
        for ruta in rutas {
            let sobran: Vec<String> = claves_del_fichero(&ruta)
                .into_iter()
                .filter(|k| !requeridas.contains(k))
                .collect();
            assert!(sobran.is_empty(), "{}: sobran {sobran:#?}", ruta.display());
        }
    }
}

/// Contrato que usan las páginas por tema: los ids de la tabla del brief,
/// con su nivel, si son lista y su componente (`_avanzado`: la parte
/// avanzada de una red o una radio).
#[test]
fn ajustes_minimos_presentes() {
    use Nivel::{Avanzado, Basico, Oculto};
    let c = cargar();
    let filas: [(&str, Nivel, bool, Option<Componente>); 19] = [
        ("wifi.red_24", Basico, false, None),
        ("wifi.red_24_avanzado", Avanzado, false, None),
        ("wifi.red_5", Basico, false, None),
        ("wifi.red_5_avanzado", Avanzado, false, None),
        ("wifi.invitados", Basico, false, None),
        ("wifi.invitados_avanzado", Avanzado, false, None),
        ("wifi.radio_24", Basico, false, None),
        ("wifi.radio_24_avanzado", Avanzado, false, None),
        ("wifi.radio_5", Basico, false, None),
        ("wifi.radio_5_avanzado", Avanzado, false, None),
        ("red_local.dhcp", Avanzado, false, None),
        (
            "red_local.reservas",
            Basico,
            true,
            Some(Componente::Reservas),
        ),
        (
            "red_local.dispositivos",
            Basico,
            false,
            Some(Componente::Dispositivos),
        ),
        ("red_local.ipv6", Avanzado, false, None),
        ("puertos.reglas", Basico, true, Some(Componente::Puertos)),
        ("puertos.upnp", Basico, false, None),
        ("puertos.alg", Oculto, false, None),
        ("internet.dns", Avanzado, false, None),
        ("internet.ddns", Oculto, true, None),
    ];
    for (id, nivel, lista, componente) in filas {
        let a = c.ajuste(id).unwrap_or_else(|| panic!("falta {id}"));
        assert_eq!(
            (a.nivel, a.lista, a.componente),
            (nivel, lista, componente),
            "{id}"
        );
    }
    // `dispositivos` solo se lee.
    assert!(
        c.ajuste("red_local.dispositivos")
            .unwrap()
            .campos
            .is_empty()
    );
    let upnp = c.ajuste("puertos.upnp").unwrap();
    assert_eq!(upnp.campos[0].recomendado.as_deref(), Some("false"));
}

/// Campo sin lo que difiere a propósito entre instancias del mismo tipo.
fn sin(c: &Campo, recomendado: bool, opciones: bool) -> Campo {
    let mut c = c.clone();
    if recomendado {
        c.recomendado = None;
    }
    if opciones {
        c.opciones.clear();
    }
    c
}

/// Lo que comparten dos ajustes gemelos: lectura, claves de los fijos,
/// nivel, efecto, lista y componente.
fn forma(
    a: &Ajuste,
) -> (
    String,
    Vec<&String>,
    Nivel,
    Option<String>,
    bool,
    Option<Componente>,
) {
    (
        a.leer.funcion.clone(),
        a.fijos.keys().collect(),
        a.nivel,
        a.efecto.clone(),
        a.lista,
        a.componente,
    )
}

/// Las tres redes WiFi tienen los mismos campos, parámetros, validaciones y
/// opciones; solo difiere la recomendación del aislamiento (sí en
/// invitados). Las dos radios, igual, salvo canales, anchos y modos (propios
/// de cada banda) y la señal dirigida, que solo está en 5 GHz.
#[test]
fn redes_y_radios_wifi_iguales_salvo_lo_previsto() {
    let c = cargar();
    for sufijo in ["", "_avanzado"] {
        let base = c.ajuste(&format!("wifi.red_24{sufijo}")).unwrap();
        for otra in ["red_5", "invitados"] {
            let a = c.ajuste(&format!("wifi.{otra}{sufijo}")).unwrap();
            assert_eq!(forma(a), forma(base), "{}", a.id);
            let quita = |x: &Campo| sin(x, x.id == "aislamiento", false);
            let campos: Vec<Campo> = a.campos.iter().map(quita).collect();
            let esperados: Vec<Campo> = base.campos.iter().map(quita).collect();
            assert_eq!(campos, esperados, "{}", a.id);
        }
    }
    for sufijo in ["", "_avanzado"] {
        let r24 = c.ajuste(&format!("wifi.radio_24{sufijo}")).unwrap();
        let r5 = c.ajuste(&format!("wifi.radio_5{sufijo}")).unwrap();
        assert_eq!(forma(r24), forma(r5), "{}", r5.id);
        let propias = ["canal", "ancho", "modo"];
        let quita = |x: &Campo| sin(x, false, propias.contains(&x.id.as_str()));
        let comunes: Vec<Campo> = r5
            .campos
            .iter()
            .filter(|x| x.id != "beamforming")
            .map(quita)
            .collect();
        let en_24: Vec<Campo> = r24.campos.iter().map(quita).collect();
        assert_eq!(en_24, comunes, "{}", r24.id);
    }
}
