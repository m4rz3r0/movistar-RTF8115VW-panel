//! El catálogo de ajustes compilado: cuadra con el de funciones, tiene todos
//! sus textos en los dos idiomas (y ninguno de sobra) y trae los ajustes que
//! usan las páginas por tema.

use ajustes::{
    Ajuste, Campo, CatalogoAjustes, Componente, Nivel, claves_de_elementos, claves_requeridas,
    comprobar, prefijo_i18n,
};
use catalogo::{Catalogo, Tipo, mask::Mascara};
use i18n::Idioma;
use regex::Regex;
use std::{collections::BTreeSet, fs, path::PathBuf};

mod comun_textos;
use comun_textos::{marcas_de_recomendacion, recomendacion_bien_formada};

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

/// Encender o apagar una red o una radio dice «Encendida» y «Apagada», no
/// «Sí» y «No»: sus textos de valor los pide el catálogo.
#[test]
fn encendida_y_apagada_con_sus_textos() {
    let c = cargar();
    for (ajuste, campo) in [
        ("wifi.red_24", "activa"),
        ("wifi.red_5", "activa"),
        ("wifi.invitados", "activa"),
        ("wifi.radio_24", "encendida"),
        ("wifi.radio_5", "encendida"),
    ] {
        let a = c.ajuste(ajuste).unwrap();
        assert!(a.campo(campo).unwrap().textos_valor, "{ajuste}.{campo}");
        let claves = claves_requeridas(a);
        let p = prefijo_i18n(a);
        for v in ["true", "false"] {
            assert!(
                claves.contains(&format!("{p}-{campo}-{v}")),
                "{ajuste}: {claves:?}"
            );
        }
    }
}

/// Los papeles de los elementos y los botones de las acciones tienen su
/// texto en los dos idiomas.
#[test]
fn claves_de_elementos_en_los_dos_idiomas() {
    let claves = claves_de_elementos(&cargar());
    assert!(
        claves.contains(&"ajustes-accion-quitar_wps".to_owned()),
        "{claves:?}"
    );
    assert!(
        claves.contains(&"ajuste-accion-quitar_wps-titulo".to_owned()),
        "{claves:?}"
    );
    for idioma in Idioma::TODOS {
        let faltan: Vec<&String> = claves.iter().filter(|k| !i18n::existe(idioma, k)).collect();
        assert!(faltan.is_empty(), "faltan en {idioma:?}: {faltan:#?}");
    }
}

/// Contrato que usan las páginas por tema: los ids de la tabla del brief,
/// con su nivel, si son lista y su componente (`_avanzado`: la parte
/// avanzada de una red o una radio).
#[test]
fn ajustes_minimos_presentes() {
    use Nivel::{Avanzado, Basico, Oculto};
    let c = cargar();
    let filas: [(&str, Nivel, bool, Option<Componente>); 33] = [
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
        ("sistema.registro", Avanzado, false, None),
        ("red_local.dns_casa", Avanzado, false, None),
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
    // Decisión del usuario: se recomienda apagada, con las dos opciones a la
    // vista (qué se gana y qué se pierde con cada una).
    let opciones: Vec<(&str, &str, bool)> = activa
        .opciones
        .iter()
        .map(|o| (o.valor.as_str(), o.clave.as_str(), o.recomendada))
        .collect();
    assert_eq!(
        opciones,
        [("1", "encendida", false), ("0", "apagada", true)]
    );
    // El relé DNS va sin recomendación hasta que la fase 3 compruebe qué
    // cambia al encenderlo en este router.
    let dns = c.ajuste("internet.dns").unwrap();
    assert_eq!(dns.campo("rele").unwrap().recomendado, None);
}

/// La contraseña de la línea de voz: el router la da vacía al leerla, así
/// que el catálogo de funciones la marca como tapada y solo se escribe
/// pidiéndola (un campo `secreto` en cada ajuste que reenvía la línea). Sin
/// valor anterior de verdad, la reversión y el deshacer la omiten (lo
/// comprueba el gestor). Antes de la Tarea 11 no se editaba en ningún sitio.
#[test]
fn contrasena_de_la_voz_solo_pedida() {
    let c = cargar();
    let f = catalogo::Catalogo::load().unwrap();
    let p = f
        .funcion("cbSetVoiceLine")
        .and_then(|f| f.parametro("auth_password"))
        .unwrap();
    assert!(p.tapado, "auth_password tiene que ir marcado como tapado");
    let editables: Vec<String> = c
        .todos()
        .iter()
        .flat_map(|a| a.campos.iter().map(move |x| (a, x)))
        .filter(|(_, x)| {
            x.escribible()
                .is_some_and(|e| e.funcion == "cbSetVoiceLine" && e.parametro == "auth_password")
        })
        .inspect(|(a, x)| assert_eq!(x.tipo, ajustes::TipoCampo::Secreto, "{}.{}", a.id, x.id))
        .map(|(a, x)| format!("{}.{}", a.id, x.id))
        .collect();
    assert_eq!(
        editables,
        [
            "telefono.linea.contrasena",
            "telefono.linea_oculto.contrasena"
        ]
    );
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

/// Formas literales de palabras corrientes que coinciden con un nombre de la
/// API y aparecen en los textos sin delatarla. Se comparan **tal cual**,
/// distinguiendo mayúsculas: solo se perdona un hallazgo si el trozo que casó
/// es exactamente una de estas formas (así «WMM» pasa, pero «wmm» o
/// «UserName» se detectan).
/// - «account», «Account», «all», «All», «password», «Password»: palabras
///   inglesas de uso diario («your WiFi password», «all devices»);
/// - «isolation»: término corriente al explicar el aislamiento de la WiFi;
/// - «beamforming»: el nombre de la técnica, que el texto explica;
/// - «WMM»: el nombre del estándar de la WiFi, en mayúsculas como se escribe.
const FORMAS_CORRIENTES: [&str; 9] = [
    "account",
    "Account",
    "all",
    "All",
    "beamforming",
    "isolation",
    "password",
    "Password",
    "WMM",
];

/// Nombres de la API que usa el catálogo de ajustes: parámetros (de
/// escritura, de lectura y fijos) y elementos leídos.
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
    nombres
}

/// Patrones que delatan la API: funciones `cb…`, páginas y órdenes del
/// router (`cwmp.asp`, `te_wifi.wl`, `scbidirectionalflt.cmd`…) y los
/// nombres de parámetro del catálogo, como palabra entera y sin distinguir
/// mayúsculas («SSID» delata `ssid`).
fn patrones_de_la_api(c: &CatalogoAjustes) -> Vec<Regex> {
    let mut patrones = vec![
        Regex::new(r"\bcb[A-Z]\w+").unwrap(),
        Regex::new(r"\b\w+\.(asp|cmd|cgi|wl)\b").unwrap(),
    ];
    patrones.extend(
        nombres_de_la_api(c)
            .iter()
            .map(|n| Regex::new(&format!(r"(?i)\b{}\b", regex::escape(n))).unwrap()),
    );
    patrones
}

/// Trozos de `texto` que casan con algún patrón, salvo los que son
/// exactamente una de las [`FORMAS_CORRIENTES`].
fn hallazgos(patrones: &[Regex], texto: &str) -> Vec<String> {
    patrones
        .iter()
        .flat_map(|p| p.find_iter(texto))
        .map(|m| m.as_str())
        .filter(|m| !FORMAS_CORRIENTES.contains(m))
        .map(str::to_owned)
        .collect()
}

/// El detector reconoce páginas, funciones y parámetros (escritos en
/// minúsculas, en mayúsculas o mezcladas) y deja pasar solo las formas
/// exactas de [`FORMAS_CORRIENTES`].
#[test]
fn detector_de_nombres_de_la_api() {
    let patrones = patrones_de_la_api(&cargar());
    for delator in [
        "se cambia en cwmp.asp",
        "la página voiceadvance.asp",
        "scbidirectionalflt.cmd?action=add",
        "con te_wifi.wl",
        "llama a cbSetTr69Config",
        "el campo hostname",
        "el valor de ssid",
        "pon enableCWMP a 0",
        "ssid_hide vale 1",
        "cambia el SSID de la red",
        "the router Hostname",
        "ENABLECWMP",
        "el campo UserName",
        "pon wmm a 1",
    ] {
        assert!(
            !hallazgos(&patrones, delator).is_empty(),
            "no detecta «{delator}»"
        );
    }
    for llano in [
        "Change your WiFi password for all devices.",
        "Turn on isolation and beamforming; All your Account details stay safe.",
        "Activa WMM en la red de casa, por ejemplo hora.example.",
    ] {
        assert!(
            hallazgos(&patrones, llano).is_empty(),
            "falso positivo en «{llano}»: {:?}",
            hallazgos(&patrones, llano)
        );
    }
}

/// El formato no se cumple con marcas vacías: tras cada una hay texto.
#[test]
fn formato_de_recomendacion_exige_texto_tras_cada_marca() {
    let es = marcas_de_recomendacion(Idioma::Es);
    let en = marcas_de_recomendacion(Idioma::En);
    assert!(recomendacion_bien_formada(
        "Recomendado si usas el fijo. A favor: llamas. En contra: ninguno importante.",
        es
    ));
    assert!(recomendacion_bien_formada(
        "Recommended if you use it. Pros: calls. Cons: none.",
        en
    ));
    for (vacia, marcas) in [
        ("Recomendado si. A favor: En contra:", es),
        (
            "Recomendado si usas el fijo. A favor: . En contra: ninguno.",
            es,
        ),
        (
            "Recomendado si usas el fijo. A favor: llamas. En contra: .",
            es,
        ),
        ("Recommended if . Pros: calls. Cons: none.", en),
    ] {
        assert!(
            !recomendacion_bien_formada(vacia, marcas),
            "acepta marcas vacías: «{vacia}»"
        );
    }
}

/// Cada recomendación dice para quién es, qué se gana y qué se pierde, en
/// todos los idiomas: «Recomendado si …. A favor: …. En contra: ….».
#[test]
fn recomendaciones_con_pros_y_contras() {
    let c = cargar();
    let mut fallos = Vec::new();
    let mut vistas = 0;
    for a in c.todos() {
        for x in &a.campos {
            if x.recomendado.is_none() && !x.opciones.iter().any(|o| o.recomendada) {
                continue;
            }
            vistas += 1;
            let clave = format!("{}-{}-recomendado", prefijo_i18n(a), x.id);
            for idioma in Idioma::TODOS {
                if !i18n::existe(idioma, &clave) {
                    fallos.push(format!("{idioma:?} {clave}: no existe"));
                    continue;
                }
                let texto = i18n::tr(idioma, &clave, None);
                if !recomendacion_bien_formada(&texto, marcas_de_recomendacion(idioma)) {
                    fallos.push(format!("{idioma:?} {clave}: «{texto}»"));
                }
            }
        }
    }
    assert!(vistas > 0, "ningún campo con recomendación");
    assert!(
        fallos.is_empty(),
        "recomendaciones sin «para quién / a favor / en contra»: {fallos:#?}"
    );
}

/// Ningún texto de los ajustes (ni de las recomendaciones ni de los lotes) nombra una
/// función, una página o un parámetro de la API: eso solo lo enseñan los
/// detalles técnicos plegados.
#[test]
fn textos_de_ajustes_sin_api() {
    let c = cargar();
    let nombres = nombres_de_la_api(&c);
    assert!(nombres.contains("ssid_hide") && nombres.contains("enableCWMP"));
    let patrones = patrones_de_la_api(&c);
    let mut fallos = Vec::new();
    for idioma in Idioma::TODOS {
        let rutas = ficheros_ftl(idioma.codigo(), |s| {
            s.starts_with("ajustes") || s == "recomendaciones" || s == "mejoras"
        });
        assert!(!rutas.is_empty(), "sin ficheros de ajustes en {idioma:?}");
        for ruta in rutas {
            for (clave, valor) in valores_del_fichero(&ruta) {
                for m in hallazgos(&patrones, &valor) {
                    fallos.push(format!(
                        "{} {clave}: «{m}»",
                        ruta.file_name().unwrap().to_string_lossy()
                    ));
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
/// opciones; solo difieren la recomendación del aislamiento (sí en
/// invitados, donde además es de seguridad), la de encender la red
/// (apagada en invitados) y si está separada de la red de casa (solo en
/// invitados, y solo se lee). Las dos radios, igual, salvo canales, anchos y modos (propios
/// de cada banda) y la señal dirigida, que solo está en 5 GHz.
#[test]
fn redes_y_radios_wifi_iguales_salvo_lo_previsto() {
    let c = cargar();
    for sufijo in ["", "_avanzado"] {
        let base = c.ajuste(&format!("wifi.red_24{sufijo}")).unwrap();
        for otra in ["red_5", "invitados"] {
            let a = c.ajuste(&format!("wifi.{otra}{sufijo}")).unwrap();
            assert_eq!(forma(a), forma(base), "{}", a.id);
            let quita = |x: &Campo| {
                let mut x = sin(x, matches!(x.id.as_str(), "aislamiento" | "activa"), false);
                if x.id == "aislamiento" {
                    x.seguridad = false;
                }
                x
            };
            let campos: Vec<Campo> = a
                .campos
                .iter()
                .filter(|x| !(otra == "invitados" && x.id == "separada"))
                .map(quita)
                .collect();
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
