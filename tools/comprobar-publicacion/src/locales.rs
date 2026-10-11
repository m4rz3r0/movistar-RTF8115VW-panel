//! Listas locales con valores del dueño. Solo existen en su máquina y nunca
//! se versionan: el comprobador las lee, pero nunca muestra una entrada; un
//! hallazgo la nombra por su número (`lista:#N`, `prohibidos:#N`) o por su
//! clave (`config:CLAVE`).
//!
//! Fuentes:
//!
//! 1. `<raiz>/.superpowers/sdd/*/real-secrets.txt` (`lista-secretos`): cada
//!    línea no vacía es una entrada; `N` cuenta las líneas de todos los
//!    ficheros, en orden.
//! 2. `$RP_PUBLICACION_PROHIBIDOS`, o `$XDG_CONFIG_HOME/routerpanel/
//!    publicacion-prohibidos.txt`, o `~/.config/routerpanel/
//!    publicacion-prohibidos.txt` (`lista-prohibidos`): una entrada por línea;
//!    `#` empieza un comentario de línea entera; `N` es el número de línea.
//! 3. Los valores no vacíos de las líneas `CLAVE=valor` de
//!    `<raiz>/deploy/config.local` (`config-local`).
//!
//! Comparación: sin distinguir mayúsculas; una entrada de menos de 6
//! caracteres solo cuenta como palabra completa (letras y cifras; `_` separa);
//! una entrada con forma de MAC se busca en cualquier escritura; las demás se
//! buscan como subcadena (así un dominio también encuentra sus subdominios).
//! Se salta una entrada que sea un `valor` de los permitidos.

use crate::{Permitidos, Regla, permitidos::mac_normalizada, reglas::normalizar};
use regex::{Regex, RegexSet, RegexSetBuilder};
use std::{
    collections::HashSet,
    fmt, fs,
    path::{Path, PathBuf},
};

/// Una entrada de menos de estos caracteres solo cuenta como palabra completa.
const MINIMO_SUBCADENA: usize = 6;

struct Entrada {
    regla: Regla,
    detalle: String,
    /// El valor, para saltarlo si es un `valor` permitido.
    valor: String,
}

/// Listas con valores del dueño (ver el módulo). Su `Debug` no muestra
/// ninguna entrada.
#[derive(Default)]
pub struct ListasLocales {
    entradas: Vec<Entrada>,
    /// Un patrón por entrada, en el mismo orden.
    patrones: Option<RegexSet>,
}

impl fmt::Debug for ListasLocales {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ListasLocales {{ {} entradas }}", self.entradas.len())
    }
}

impl ListasLocales {
    /// Sin listas locales (CI, o máquina sin datos del dueño).
    pub fn vacia() -> Self {
        Self::default()
    }

    /// Lee las listas de `raiz` y del entorno (ver el módulo). El segundo
    /// valor son avisos para el usuario, sin valores ni rutas personales: una
    /// lista que falta o no se puede leer se omite y se avisa.
    pub fn cargar(raiz: &Path) -> (ListasLocales, Vec<String>) {
        let mut c = Constructor::default();
        let mut avisos = Vec::new();

        let secretos = ficheros_de_secretos(raiz);
        if secretos.is_empty() {
            avisos.push(
                "aviso: no hay listas de secretos (.superpowers/sdd/*/real-secrets.txt): \
                 se omite la regla lista-secretos"
                    .to_owned(),
            );
        }
        let mut n = 0;
        for (k, f) in secretos.iter().enumerate() {
            let Ok(texto) = fs::read_to_string(f) else {
                avisos.push(format!(
                    "aviso: no se pudo leer la lista de secretos n.º {}: se omiten sus entradas \
                     (regla lista-secretos)",
                    k + 1
                ));
                continue;
            };
            for linea in texto.lines() {
                n += 1;
                c.anadir(Regla::ListaSecretos, format!("lista:#{n}"), linea);
            }
        }

        match ruta_de_prohibidos() {
            None => avisos.push(
                "aviso: no se sabe dónde buscar la lista de prohibidos (ni \
                 RP_PUBLICACION_PROHIBIDOS, ni XDG_CONFIG_HOME, ni HOME): se omite la regla \
                 lista-prohibidos"
                    .to_owned(),
            ),
            Some(ruta) => match fs::read_to_string(&ruta) {
                Ok(texto) => {
                    for (i, linea) in texto.lines().enumerate() {
                        if !linea.trim_start().starts_with('#') {
                            c.anadir(
                                Regla::ListaProhibidos,
                                format!("prohibidos:#{}", i + 1),
                                linea,
                            );
                        }
                    }
                }
                Err(_) => avisos.push(
                    "aviso: falta la lista de prohibidos o no se puede leer: se omite la regla \
                     lista-prohibidos"
                        .to_owned(),
                ),
            },
        }

        match fs::read_to_string(raiz.join("deploy/config.local")) {
            Ok(texto) => {
                for linea in texto.lines() {
                    if let Some((clave, valor)) = asignacion(linea) {
                        c.anadir(Regla::ConfigLocal, format!("config:{clave}"), valor);
                    }
                }
            }
            Err(_) => avisos.push(
                "aviso: no hay deploy/config.local o no se puede leer: se omite la regla \
                 config-local"
                    .to_owned(),
            ),
        }

        (c.terminar(), avisos)
    }

    /// Coincidencias de las listas en una línea, como `(regla, detalle)`.
    /// Una por entrada, aunque aparezca varias veces.
    pub(crate) fn revisar_linea(&self, linea: &str, p: &Permitidos) -> Vec<(Regla, String)> {
        let Some(patrones) = &self.patrones else {
            return Vec::new();
        };
        let mut indices: Vec<usize> = patrones.matches(linea).into_iter().collect();
        let normal = normalizar(linea);
        if normal != linea {
            indices.extend(patrones.matches(&normal));
            indices.sort_unstable();
            indices.dedup();
        }
        indices
            .into_iter()
            .map(|i| &self.entradas[i])
            .filter(|e| !p.valor_permitido(&e.valor))
            .map(|e| (e.regla, e.detalle.clone()))
            .collect()
    }
}

#[derive(Default)]
struct Constructor {
    entradas: Vec<Entrada>,
    patrones: Vec<String>,
    vistas: HashSet<String>,
}

impl Constructor {
    /// Añade una entrada (recortada). Se omiten las vacías y las repetidas:
    /// las listas de secretos de varias fases suelen ser copias.
    fn anadir(&mut self, regla: Regla, detalle: String, valor: &str) {
        let valor = valor.trim();
        if valor.is_empty() || !self.vistas.insert(valor.to_lowercase()) {
            return;
        }
        self.patrones.push(patron(valor));
        self.entradas.push(Entrada {
            regla,
            detalle,
            valor: valor.to_owned(),
        });
    }

    fn terminar(self) -> ListasLocales {
        let patrones = (!self.patrones.is_empty()).then(|| {
            RegexSetBuilder::new(&self.patrones)
                .size_limit(1 << 28)
                .build()
                .expect("los patrones de las listas se escapan")
        });
        ListasLocales {
            entradas: self.entradas,
            patrones,
        }
    }
}

/// Expresión que busca `valor` según las reglas de comparación del módulo.
fn patron(valor: &str) -> String {
    if let Some(mac) = mac_normalizada(valor) {
        // Cada byte, con o sin el cero de delante, y cualquier separador (o
        // ninguno) entre bytes: vale para `:`, `-`, `_`, `.`, espacios, la
        // forma de Cisco, la de HP y la que no lleva separador.
        let bytes: Vec<String> = mac
            .as_bytes()
            .chunks(2)
            .map(|b| {
                let (alto, bajo) = (b[0] as char, b[1] as char);
                if alto == '0' {
                    format!("0?{bajo}")
                } else {
                    format!("{alto}{bajo}")
                }
            })
            .collect();
        return format!(r"(?i){}", bytes.join(r"[\s:._-]?"));
    }
    let escapado = regex::escape(valor);
    if valor.chars().count() < MINIMO_SUBCADENA {
        format!(r"(?i)(?:^|[^\p{{L}}\p{{N}}]){escapado}(?:$|[^\p{{L}}\p{{N}}])")
    } else {
        format!("(?i){escapado}")
    }
}

/// `CLAVE=valor` (con `export` opcional y comillas alrededor del valor), si
/// la línea lo es y el valor no está vacío.
fn asignacion(linea: &str) -> Option<(&str, &str)> {
    static CLAVE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").expect("válida"));
    let linea = linea.trim();
    let linea = linea.strip_prefix("export ").unwrap_or(linea).trim_start();
    let (clave, valor) = linea.split_once('=')?;
    if !CLAVE.is_match(clave) {
        return None;
    }
    let valor = valor.trim();
    let valor = ['"', '\'']
        .into_iter()
        .find_map(|q| valor.strip_prefix(q).and_then(|v| v.strip_suffix(q)))
        .unwrap_or(valor)
        .trim();
    (!valor.is_empty()).then_some((clave, valor))
}

/// `<raiz>/.superpowers/sdd/*/real-secrets.txt`, en orden.
fn ficheros_de_secretos(raiz: &Path) -> Vec<PathBuf> {
    let Ok(dirs) = fs::read_dir(raiz.join(".superpowers/sdd")) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = dirs
        .filter_map(Result::ok)
        .map(|d| d.path().join("real-secrets.txt"))
        .filter(|f| f.is_file())
        .collect();
    v.sort();
    v
}

/// La ruta de la lista de prohibidos según el entorno.
fn ruta_de_prohibidos() -> Option<PathBuf> {
    let var = |v: &str| std::env::var_os(v).filter(|s| !s.is_empty());
    if let Some(r) = var("RP_PUBLICACION_PROHIBIDOS") {
        return Some(PathBuf::from(r));
    }
    let config = var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| var("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(config.join("routerpanel/publicacion-prohibidos.txt"))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn listas(valores: &[&str]) -> ListasLocales {
        let mut c = Constructor::default();
        for (i, v) in valores.iter().enumerate() {
            c.anadir(Regla::ListaProhibidos, format!("prohibidos:#{}", i + 1), v);
        }
        c.terminar()
    }

    fn detalles(l: &ListasLocales, linea: &str) -> Vec<String> {
        l.revisar_linea(linea, &Permitidos::default())
            .into_iter()
            .map(|(_, d)| d)
            .collect()
    }

    #[test]
    fn asignaciones() {
        assert_eq!(asignacion("A=1"), Some(("A", "1")));
        assert_eq!(asignacion("export B = \"x y\""), None);
        assert_eq!(asignacion("export B=\"x y\""), Some(("B", "x y")));
        assert_eq!(asignacion("C=''"), None);
        assert_eq!(asignacion("# D=1"), None);
        assert_eq!(asignacion("D="), None);
    }

    #[test]
    fn comparaciones() {
        let l = listas(&["abcd", "Cosa.Example.Net", "02:00:00:0a:bb:cc"]);
        assert!(detalles(&l, "abcdef").is_empty());
        assert_eq!(detalles(&l, "(ABCD)"), ["prohibidos:#1"]);
        assert_eq!(detalles(&l, "a.cosa.example.net"), ["prohibidos:#2"]);
        assert_eq!(detalles(&l, "2-0-0-a-bb-cc"), ["prohibidos:#3"]);
        assert_eq!(detalles(&l, "020000%3A0abbcc"), ["prohibidos:#3"]);
        assert_eq!(
            detalles(&l, "02%3A00%3A00%3A0a%3Abb%3Acc"),
            ["prohibidos:#3"]
        );
        // Repetidas (sin distinguir mayúsculas) y vacías no cuentan.
        assert_eq!(listas(&["x1y2z3", "X1Y2Z3", "  "]).entradas.len(), 1);
    }

    #[test]
    fn debug_sin_valores() {
        let l = listas(&["valor-secreto"]);
        assert!(!format!("{l:?}").contains("valor"));
    }
}
