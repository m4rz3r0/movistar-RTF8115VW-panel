//! Comprobación de lo que se publica: busca datos personales, de la red del
//! dueño o de proveedores en el árbol que va al repositorio público.
//!
//! Nunca guarda ni muestra el valor encontrado: cada [`Hallazgo`] lleva la
//! ruta, la línea, la [`Regla`] y un detalle que es un tipo («ipv4 privada»,
//! «oui real») o una referencia a una lista (`lista:#N`).
//!
//! Las reglas genéricas (`reglas`) no necesitan datos locales y valen para
//! cualquier copia del repositorio; las excepciones revisadas a mano están en
//! `tools/publicacion-permitidos.txt` ([`Permitidos`]). Las listas con valores
//! del dueño ([`ListasLocales`]) solo existen en su máquina.

mod arbol;
mod git;
mod locales;
mod permitidos;
mod reglas;

use std::fmt;

pub use arbol::arbol;
pub use git::{diff, fichero_en_commit, historial, revisar_refs};
pub use locales::ListasLocales;
pub use permitidos::Permitidos;

/// Regla que produce un hallazgo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Regla {
    Ipv4,
    Ipv6,
    Mac,
    Dominio,
    Correo,
    Clave,
    Identificador,
    Fixture,
    RutaProhibida,
    Binario,
    FaltaFichero,
    ListaSecretos,
    ListaProhibidos,
    ConfigLocal,
}

impl Regla {
    /// Todas las reglas, en el orden de la enumeración.
    pub const TODAS: [Regla; 14] = [
        Regla::Ipv4,
        Regla::Ipv6,
        Regla::Mac,
        Regla::Dominio,
        Regla::Correo,
        Regla::Clave,
        Regla::Identificador,
        Regla::Fixture,
        Regla::RutaProhibida,
        Regla::Binario,
        Regla::FaltaFichero,
        Regla::ListaSecretos,
        Regla::ListaProhibidos,
        Regla::ConfigLocal,
    ];

    /// Nombre de la regla en la salida y en `permitidos`.
    pub fn as_str(self) -> &'static str {
        match self {
            Regla::Ipv4 => "ipv4",
            Regla::Ipv6 => "ipv6",
            Regla::Mac => "mac",
            Regla::Dominio => "dominio",
            Regla::Correo => "correo",
            Regla::Clave => "clave",
            Regla::Identificador => "identificador",
            Regla::Fixture => "fixture",
            Regla::RutaProhibida => "ruta-prohibida",
            Regla::Binario => "binario",
            Regla::FaltaFichero => "falta-fichero",
            Regla::ListaSecretos => "lista-secretos",
            Regla::ListaProhibidos => "lista-prohibidos",
            Regla::ConfigLocal => "config-local",
        }
    }

    /// La regla con ese nombre.
    pub fn desde_nombre(nombre: &str) -> Option<Regla> {
        Regla::TODAS.into_iter().find(|r| r.as_str() == nombre)
    }
}

impl fmt::Display for Regla {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Un hallazgo. `linea` es 0 cuando se refiere al fichero entero o a su
/// nombre. `detalle` nunca contiene el valor encontrado.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hallazgo {
    pub ruta: String,
    pub linea: usize,
    pub regla: Regla,
    pub detalle: String,
}

impl fmt::Display for Hallazgo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}  {}  [{}]",
            self.ruta, self.linea, self.regla, self.detalle
        )
    }
}

/// Qué reglas se aplican a un fichero de texto según su ruta.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Clase {
    Normal,
    /// `Cargo.lock` y `e2e/package-lock.json`: sumas y URL de dependencias;
    /// solo correo y listas locales.
    Bloqueo,
    /// Respuestas capturadas del router (`crates/hgu/fixtures*/**`).
    Fixture,
}

fn clase(ruta: &str) -> Clase {
    if ruta == "Cargo.lock" || ruta == "e2e/package-lock.json" {
        Clase::Bloqueo
    } else if ruta
        .strip_prefix("crates/hgu/fixtures")
        .is_some_and(|r| r.contains('/'))
    {
        Clase::Fixture
    } else {
        Clase::Normal
    }
}

/// Revisa el nombre de un fichero (cada componente de `ruta`): las reglas
/// genéricas, las listas locales y, en las fixtures, los parámetros del
/// nombre. Los hallazgos tienen línea 0 y el detalle empieza por `nombre:`.
///
/// **Si devuelve algo, la ruta contiene un dato**: quien imprima hallazgos de
/// ese fichero no debe mostrar `ruta` (ver [`ocultar_ruta`]).
pub fn revisar_nombre(ruta: &str, p: &Permitidos, l: &ListasLocales) -> Vec<Hallazgo> {
    let mut detalles: Vec<(Regla, String)> = Vec::new();
    for componente in ruta.split('/') {
        detalles.extend(
            reglas::genericas(componente, p)
                .into_iter()
                .map(|(r, d)| (r, d.to_owned())),
        );
        detalles.extend(l.revisar_linea(componente, p));
    }
    // Una entrada de las listas con `/` solo puede coincidir con la ruta
    // entera.
    if ruta.contains('/') {
        detalles.extend(l.revisar_linea(ruta, p));
    }
    if clase(ruta) == Clase::Fixture {
        detalles.extend(
            reglas::regla_nombre_fixture(ruta)
                .into_iter()
                .map(|d| (Regla::Fixture, d.to_owned())),
        );
    }
    detalles.sort();
    detalles.dedup();
    detalles
        .into_iter()
        .filter(|(regla, _)| !p.ruta_permitida(ruta, *regla))
        .map(|(regla, detalle)| Hallazgo {
            ruta: ruta.to_owned(),
            linea: 0,
            regla,
            detalle: format!("nombre: {detalle}"),
        })
        .collect()
}

/// Sustituye la ruta de los hallazgos por una referencia sin datos
/// (`[fichero n.º K del árbol]`, con `K` desde 1) cuando el nombre del fichero
/// contiene un dato.
pub fn ocultar_ruta(hallazgos: &mut [Hallazgo], numero: usize) {
    for h in hallazgos {
        h.ruta = format!("[fichero n.º {numero} del árbol]");
    }
}

/// Revisa el contenido de un fichero de texto (no su nombre: ver
/// [`revisar_nombre`]). `ruta` es relativa a la raíz del árbol, con `/`, y
/// decide qué reglas se aplican y qué excepciones de `p` valen.
pub fn revisar_texto(ruta: &str, texto: &str, p: &Permitidos, l: &ListasLocales) -> Vec<Hallazgo> {
    let clase = clase(ruta);
    let mut out = Vec::new();
    let mut anotar = |linea: usize, regla: Regla, detalle: String| {
        if !p.ruta_permitida(ruta, regla) {
            out.push(Hallazgo {
                ruta: ruta.to_owned(),
                linea,
                regla,
                detalle,
            });
        }
    };
    for (i, linea) in texto.lines().enumerate() {
        let n = i + 1;
        if clase == Clase::Bloqueo {
            for d in reglas::regla_correo(linea, p) {
                anotar(n, Regla::Correo, d.to_owned());
            }
        } else {
            for (regla, d) in reglas::genericas(linea, p) {
                anotar(n, regla, d.to_owned());
            }
            if clase == Clase::Fixture {
                for d in reglas::regla_fixture(linea, p) {
                    anotar(n, Regla::Fixture, d.to_owned());
                }
            }
        }
        for (regla, d) in l.revisar_linea(linea, p) {
            anotar(n, regla, d);
        }
    }
    out
}

/// Revisa un fichero binario: debe figurar en `p` con su sha256 y, si es una
/// imagen, no llevar metadatos de texto ni EXIF.
pub fn revisar_binario(ruta: &str, bytes: &[u8], p: &Permitidos) -> Vec<Hallazgo> {
    reglas::regla_binario(ruta, bytes, p)
        .into_iter()
        .filter(|_| !p.ruta_permitida(ruta, Regla::Binario))
        .map(|detalle| Hallazgo {
            ruta: ruta.to_owned(),
            linea: 0,
            regla: Regla::Binario,
            detalle: detalle.to_owned(),
        })
        .collect()
}
