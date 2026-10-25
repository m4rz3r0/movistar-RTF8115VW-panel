//! De dónde sale cada recomendación: las fuentes de `fuentes.toml` y su comprobación.
//!
//! Una recomendación (un `recomendado`, una opción `recomendada`, un paso de
//! lote o la opción por defecto de una elección) lleva `fuentes` (ids de
//! `[[fuente]]`) o `respaldo = "criterio"` si no hay una fuente real detrás.
//! Los títulos van en su idioma original y no se traducen.
//!
//! ```toml
//! [[fuente]]
//! id = "cert-wps"
//! titulo = "Wi-Fi Protected Setup (WPS) PIN brute force vulnerability"
//! organismo = "CERT/CC"
//! url = "https://kb.cert.org/vuls/id/723755"
//! consultada = "2026-10-22"
//! clase = "guia"
//! ```

use serde::Deserialize;
use std::collections::HashSet;

use crate::carga::CatalogoAjustes;

/// Una fuente citada por las recomendaciones.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fuente {
    pub id: String,
    /// Título en el idioma original del documento.
    pub titulo: String,
    pub organismo: String,
    /// Siempre `https://`.
    pub url: String,
    /// Día de la consulta, `AAAA-MM-DD`.
    pub consultada: String,
    pub clase: ClaseFuente,
    /// Otras fuentes con que se comprobó esta cuando no se pudo leer
    /// (la guía de la NSA, cuyo PDF dio 403).
    #[serde(default)]
    pub verificada_con: Vec<String>,
}

/// Qué tipo de documento es una fuente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaseFuente {
    /// Guía de un organismo (NSA, CISA, BSI, INCIBE…) o de una referencia
    /// independiente.
    Guia,
    /// Documentación del propio proveedor de un servicio.
    Proveedor,
    /// Hilo de un foro o de una comunidad de usuarios.
    Foro,
    /// Artículo de prensa o de un blog técnico.
    Prensa,
}

impl ClaseFuente {
    /// Todas, en el orden del enum.
    pub const TODAS: [ClaseFuente; 4] = [
        ClaseFuente::Guia,
        ClaseFuente::Proveedor,
        ClaseFuente::Foro,
        ClaseFuente::Prensa,
    ];

    /// Nombre en el TOML.
    pub fn slug(self) -> &'static str {
        match self {
            ClaseFuente::Guia => "guia",
            ClaseFuente::Proveedor => "proveedor",
            ClaseFuente::Foro => "foro",
            ClaseFuente::Prensa => "prensa",
        }
    }

    /// Clave de traducción (`fuentes-clase-guia`).
    pub fn clave(self) -> String {
        format!("fuentes-clase-{}", self.slug())
    }
}

/// En qué se apoya una recomendación, además de sus fuentes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Respaldo {
    /// Las guías coinciden.
    Consenso,
    /// Lo cuentan los foros (exige alguna fuente de clase `foro`).
    Foro,
    /// Criterio del panel: no hay una fuente real detrás.
    Criterio,
}

impl Respaldo {
    /// Todos, en el orden del enum.
    pub const TODOS: [Respaldo; 3] = [Respaldo::Consenso, Respaldo::Foro, Respaldo::Criterio];

    /// Nombre en el TOML.
    pub fn slug(self) -> &'static str {
        match self {
            Respaldo::Consenso => "consenso",
            Respaldo::Foro => "foro",
            Respaldo::Criterio => "criterio",
        }
    }

    /// Clave de traducción (`fuentes-respaldo-criterio`).
    pub fn clave(self) -> String {
        format!("fuentes-respaldo-{}", self.slug())
    }
}

/// Formato de `fuentes.toml`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FicheroFuentes {
    #[serde(default)]
    pub fuente: Vec<Fuente>,
}

/// `AAAA-MM-DD` con cifras (no comprueba el calendario).
fn es_fecha(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        })
}

/// Fallos de las fuentes y de sus referencias: ids únicos, `https://`,
/// fecha `AAAA-MM-DD`, referencias que existen, cada recomendación con
/// fuentes o como criterio del panel, y `respaldo = "foro"` con alguna
/// fuente de foro.
pub(crate) fn comprobar_fuentes(c: &CatalogoAjustes, fallos: &mut Vec<String>) {
    let mut ids = HashSet::new();
    for f in c.fuentes() {
        if !ids.insert(f.id.as_str()) {
            fallos.push(format!("fuente {}: id repetido", f.id));
        }
        if !f.url.starts_with("https://") {
            fallos.push(format!("fuente {}: la url debe empezar por https://", f.id));
        }
        if !es_fecha(&f.consultada) {
            fallos.push(format!(
                "fuente {}: `consultada` debe tener la forma AAAA-MM-DD",
                f.id
            ));
        }
        for v in &f.verificada_con {
            if c.fuente(v).is_none() {
                fallos.push(format!(
                    "fuente {}: `verificada_con` cita {v}, que no existe",
                    f.id
                ));
            }
        }
    }
    let mut revisar = |donde: String, fuentes: &[String], r: Option<Respaldo>, exige: bool| {
        respaldada(c, &donde, fuentes, r, exige, fallos);
    };
    for a in c.todos() {
        for x in &a.campos {
            let donde = format!("{}.{}", a.id, x.id);
            revisar(
                donde.clone(),
                &x.fuentes,
                x.respaldo,
                x.recomendado.is_some(),
            );
            for o in &x.opciones {
                revisar(
                    format!("{donde} (opción {})", o.clave),
                    &o.fuentes,
                    o.respaldo,
                    o.recomendada,
                );
            }
        }
    }
    for e in c.elementos() {
        revisar(format!("elemento {}", e.id), &e.fuentes, e.respaldo, false);
    }
    for p in c.pasos() {
        revisar(format!("paso {}", p.id), &p.fuentes, p.respaldo, true);
    }
    for e in c.elecciones() {
        let defecto = e.defecto().id.as_str();
        for o in &e.opciones {
            revisar(
                format!("elección {} (opción {})", e.id, o.id),
                &o.fuentes,
                o.respaldo,
                o.id == defecto,
            );
        }
    }
}

/// Fallos de una recomendación en `donde`: sus fuentes existen, con
/// `exige` lleva fuentes o es criterio del panel, y el respaldo de foro
/// cita algún foro.
fn respaldada(
    c: &CatalogoAjustes,
    donde: &str,
    fuentes: &[String],
    r: Option<Respaldo>,
    exige: bool,
    fallos: &mut Vec<String>,
) {
    for id in fuentes {
        if c.fuente(id).is_none() {
            fallos.push(format!("{donde}: la fuente {id} no existe"));
        }
    }
    if exige && fuentes.is_empty() && r != Some(Respaldo::Criterio) {
        fallos.push(format!(
            "{donde}: una recomendación lleva fuentes o respaldo = \"criterio\""
        ));
    }
    let hay_foro = fuentes
        .iter()
        .filter_map(|id| c.fuente(id))
        .any(|f| f.clase == ClaseFuente::Foro);
    if r == Some(Respaldo::Foro) && !hay_foro {
        fallos.push(format!(
            "{donde}: respaldo = \"foro\" exige alguna fuente de clase foro"
        ));
    }
}
