//! El catálogo de ajustes compilado: cuadra con el de funciones, tiene todos
//! sus textos en los dos idiomas (y ninguno de sobra) y trae los ajustes que
//! usan las páginas por tema.

use ajustes::{Ajuste, Campo, CatalogoAjustes, Componente, Nivel, claves_requeridas, comprobar};
use catalogo::{Catalogo, Tipo, mask::Mascara};
use i18n::Idioma;
use regex::Regex;
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

/// Ficheros `.ftl` de `idioma` cuyo nombre (sin `.ftl`) cumple `cumple`.
fn ficheros_ftl(idioma: &str, cumple: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../i18n/locales")).join(idioma);
    let mut rutas: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "ftl")
                && p.file_stem().is_some_and(|s| cumple(&s.to_string_lossy()))
        })
        .collect();
    rutas.sort();
    rutas
}

/// Ficheros `ajustes-<tema>.ftl` de `idioma`.
fn ficheros_de_ajustes(idioma: &str) -> Vec<PathBuf> {
    ficheros_ftl(idioma, |s| s.starts_with("ajustes-"))
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
    let filas: [(&str, Nivel, bool, Option<Componente>); 31] = [
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
        ("seguridad.gestion_remota", Basico, false, None),
        ("seguridad.gestion_remota_avanzado", Avanzado, false, None),
        ("seguridad.gestion_remota_oculto", Oculto, false, None),
        (
            "seguridad.contrasena_router",
            Basico,
            false,
            Some(Componente::ContrasenaRouter),
        ),
        ("seguridad.usuarios_router", Avanzado, true, None),
        ("telefono.linea", Avanzado, false, None),
        ("telefono.linea_oculto", Oculto, false, None),
        ("telefono.sip", Avanzado, false, None),
        ("telefono.sip_oculto", Oculto, false, None),
        ("telefono.fisico", Avanzado, false, None),
        ("sistema.hora", Avanzado, false, None),
        ("sistema.diagnostico", Oculto, false, None),
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
    // La contraseña del router la escribe su componente (Tarea 12), fuera
    // del gestor: el ajuste solo se registra con su lectura.
    let pwd = c.ajuste("seguridad.contrasena_router").unwrap();
    assert!(pwd.campos.is_empty());
    assert_eq!(pwd.leer.funcion, "cbGetUserList");
    // Los usuarios del router solo se leen.
    let usuarios = c.ajuste("seguridad.usuarios_router").unwrap();
    assert!(!usuarios.campos.is_empty());
    assert!(
        usuarios
            .campos
            .iter()
            .all(|x| x.escribe.is_none() && x.solo_lectura.is_some()),
        "{:#?}",
        usuarios.campos
    );
    // La gestión remota se activa en la parte básica, con su lectura.
    let tr69 = c.ajuste("seguridad.gestion_remota").unwrap();
    let activa = tr69.campo("activa").unwrap();
    assert_eq!(
        activa.escribe.as_ref().map(|e| e.parametro.as_str()),
        Some("enableCWMP")
    );
    assert_eq!(tr69.leer.funcion, "cbGetTr69Config");
}

/// Cada función de escritura que no está bloqueada en el catálogo de
/// funciones la escribe algún campo o está en `excluidas.toml` con su motivo.
/// Las acciones (reinicio, guardado…) son de los prestablecidos (fase 4).
#[test]
fn cobertura_de_escrituras() {
    let c = cargar();
    let f = Catalogo::load().unwrap();
    let cubiertas: BTreeSet<&str> = c
        .todos()
        .iter()
        .flat_map(|a| &a.campos)
        .filter_map(|x| x.escribe.as_ref())
        .map(|e| e.funcion.as_str())
        .chain(c.excluidas().iter().map(|e| e.funcion.as_str()))
        .collect();
    let faltan: Vec<&str> = f
        .todas()
        .iter()
        .filter(|x| x.tipo == Tipo::Escritura && x.bloqueada.is_none())
        .map(|x| x.name.as_str())
        .filter(|n| !cubiertas.contains(n))
        .collect();
    assert!(
        faltan.is_empty(),
        "escrituras sin ajuste ni excluida: {faltan:#?}"
    );
}

/// Valores de los mensajes de un `.ftl`: la línea `clave = …` y sus
/// continuaciones sangradas, unidas (sin comentarios ni claves).
fn valores_del_fichero(ruta: &PathBuf) -> Vec<(String, String)> {
    let texto = fs::read_to_string(ruta).unwrap_or_else(|e| panic!("{}: {e}", ruta.display()));
    let mut valores: Vec<(String, String)> = Vec::new();
    for l in texto.lines() {
        if l.starts_with('#') || l.trim().is_empty() {
            continue;
        }
        if l.starts_with([' ', '\t']) {
            if let Some((_, v)) = valores.last_mut() {
                v.push(' ');
                v.push_str(l.trim());
            }
        } else if let Some((k, v)) = l.split_once('=') {
            valores.push((k.trim().to_owned(), v.trim().to_owned()));
        }
    }
    valores
}

/// Nombres de la API que usa el catálogo de ajustes: funciones, parámetros
/// (de escritura, de lectura y fijos) y elementos leídos. Solo los que
/// parecen identificadores (llevan `_`, una mayúscula o una cifra): una
/// palabra corriente como `region` no delata la API.
fn nombres_de_la_api(c: &CatalogoAjustes) -> BTreeSet<String> {
    let mut nombres = BTreeSet::new();
    for a in c.todos() {
        nombres.extend(a.leer.params.iter().map(|(k, _)| k.clone()));
        nombres.extend(a.fijos.keys().cloned());
        for x in &a.campos {
            nombres.insert(x.lee.clone());
            nombres.extend(x.escribe.iter().map(|e| e.parametro.clone()));
            for ad in &x.ademas {
                nombres.insert(ad.parametro.clone());
                nombres.insert(ad.lee.clone());
            }
        }
    }
    nombres.retain(|n| {
        n.contains('_')
            || n.chars()
                .any(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
    });
    nombres
}

/// Ningún texto de los ajustes (ni de las recomendaciones) nombra una
/// función de la API ni un parámetro del catálogo: eso solo lo enseñan los
/// detalles técnicos plegados.
#[test]
fn textos_de_ajustes_sin_api() {
    let c = cargar();
    let mut patrones = vec![
        Regex::new(r"\bcb[A-Z]\w+").unwrap(),
        Regex::new(r"\bte_\w+\.(asp|cmd|cgi|wl)\b").unwrap(),
    ];
    let nombres = nombres_de_la_api(&c);
    assert!(nombres.contains("ssid_hide") && nombres.contains("enableCWMP"));
    patrones.extend(
        nombres
            .iter()
            .map(|n| Regex::new(&format!(r"\b{}\b", regex::escape(n))).unwrap()),
    );
    let mut fallos = Vec::new();
    for idioma in Idioma::TODOS {
        let rutas = ficheros_ftl(idioma.codigo(), |s| {
            s.starts_with("ajustes") || s == "recomendaciones"
        });
        assert!(!rutas.is_empty(), "sin ficheros de ajustes en {idioma:?}");
        for ruta in rutas {
            for (clave, valor) in valores_del_fichero(&ruta) {
                for p in &patrones {
                    if let Some(m) = p.find(&valor) {
                        fallos.push(format!(
                            "{} {clave}: «{}»",
                            ruta.file_name().unwrap().to_string_lossy(),
                            m.as_str()
                        ));
                    }
                }
            }
        }
    }
    assert!(
        fallos.is_empty(),
        "nombres de la API en los textos: {fallos:#?}"
    );
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
