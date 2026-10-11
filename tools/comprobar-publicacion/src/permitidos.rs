//! Excepciones revisadas a mano (`tools/publicacion-permitidos.txt`).
//!
//! Una por línea, `#` empieza un comentario de línea entera, y el motivo tras
//! `—` es obligatorio:
//!
//! - `valor <v> — <motivo>`: un valor concreto (sin distinguir mayúsculas; si
//!   tiene forma de MAC, en cualquier escritura);
//! - `red <cidr> — <motivo>`: una red IPv4 o IPv6;
//! - `dominio <d> — <motivo>`: el dominio y sus subdominios;
//! - `ruta <glob> <regla> — <motivo>`: la regla no se aplica a esas rutas
//!   (`*` no cruza `/`, `**` sí);
//! - `binario <ruta> <sha256> — <motivo>`: un binario revisado.

use crate::Regla;
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
};

/// Reglas que una entrada `ruta` no puede desactivar: las redes de seguridad
/// con datos del dueño y las rutas que nunca deben publicarse.
const NO_DESACTIVABLES: [Regla; 4] = [
    Regla::ListaSecretos,
    Regla::ListaProhibidos,
    Regla::ConfigLocal,
    Regla::RutaProhibida,
];

#[derive(Clone, Debug, Default)]
pub struct Permitidos {
    valores: HashSet<String>,
    macs: HashSet<String>,
    redes: Vec<(IpAddr, u8)>,
    dominios: Vec<String>,
    rutas: Vec<(String, Regla)>,
    binarios: HashMap<String, String>,
}

impl Permitidos {
    /// Interpreta la lista. El error indica la línea, sin repetir su contenido.
    pub fn leer(texto: &str) -> Result<Permitidos, String> {
        let mut p = Permitidos::default();
        for (i, linea) in texto.lines().enumerate() {
            let n = i + 1;
            let linea = linea.trim();
            if linea.is_empty() || linea.starts_with('#') {
                continue;
            }
            let Some((regla, motivo)) = linea.split_once('—') else {
                return Err(format!("línea {n}: falta el motivo tras «—»"));
            };
            if motivo.trim().is_empty() {
                return Err(format!("línea {n}: el motivo está vacío"));
            }
            let campos: Vec<&str> = regla.split_whitespace().collect();
            match campos.as_slice() {
                ["valor", v @ ..] if !v.is_empty() => {
                    let v = v.join(" ");
                    if let Some(mac) = mac_normalizada(&v) {
                        p.macs.insert(mac);
                    }
                    p.valores.insert(v.to_lowercase());
                }
                ["red", cidr] => {
                    let red = leer_red(cidr).ok_or(format!("línea {n}: red no válida"))?;
                    p.redes.push(red);
                }
                ["dominio", d] => p.dominios.push(d.trim_matches('.').to_lowercase()),
                ["ruta", glob, regla] => {
                    let regla = Regla::desde_nombre(regla)
                        .ok_or(format!("línea {n}: regla desconocida"))?;
                    if NO_DESACTIVABLES.contains(&regla) {
                        return Err(format!(
                            "línea {n}: la regla {regla} no se puede desactivar por ruta"
                        ));
                    }
                    p.rutas.push(((*glob).to_owned(), regla));
                }
                ["binario", ruta, sha] => {
                    if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
                        return Err(format!("línea {n}: sha256 no válido"));
                    }
                    p.binarios
                        .insert((*ruta).to_owned(), sha.to_ascii_lowercase());
                }
                _ => {
                    return Err(format!(
                        "línea {n}: se espera `valor`, `red`, `dominio`, `ruta` o `binario` con sus campos"
                    ));
                }
            }
        }
        Ok(p)
    }

    /// `true` si el valor está permitido tal cual (o como MAC normalizada).
    pub fn valor_permitido(&self, v: &str) -> bool {
        self.valores.contains(&v.to_lowercase())
            || mac_normalizada(v).is_some_and(|m| self.macs.contains(&m))
    }

    /// `true` si la MAC (12 hexadecimales en minúsculas) está permitida.
    pub fn mac_permitida(&self, normalizada: &str) -> bool {
        self.macs.contains(normalizada)
    }

    /// `true` si la IP cae en una `red` permitida o es un `valor` permitido.
    pub fn ip_permitida(&self, ip: IpAddr) -> bool {
        self.valores.contains(&ip.to_string())
            || self
                .redes
                .iter()
                .any(|&(red, prefijo)| en_red(ip, red, prefijo))
    }

    /// `true` si el dominio, o uno de sus padres, está permitido.
    pub fn dominio_permitido(&self, d: &str) -> bool {
        let d = d.trim_end_matches('.').to_lowercase();
        self.valores.contains(&d)
            || self
                .dominios
                .iter()
                .any(|p| d == *p || d.ends_with(&format!(".{p}")))
    }

    /// `true` si la regla no se aplica a esa ruta.
    pub fn ruta_permitida(&self, ruta: &str, regla: Regla) -> bool {
        self.rutas
            .iter()
            .any(|(glob, r)| *r == regla && glob_coincide(glob, ruta))
    }

    /// sha256 revisado del binario en esa ruta.
    pub fn sha_binario(&self, ruta: &str) -> Option<&str> {
        self.binarios.get(ruta).map(String::as_str)
    }
}

/// MAC en cualquier escritura (seis grupos de una o dos cifras separados por
/// `:`, `-`, `_` o espacios; tres grupos de cuatro separados por `.`; dos de
/// seis separados por `-`; o 12 cifras seguidas) como 12 hexadecimales en minúsculas.
pub(crate) fn mac_normalizada(v: &str) -> Option<String> {
    let grupos: Vec<&str> = v.split([':', '-', '_', ' ', '.']).collect();
    let hex = |g: &str| !g.is_empty() && g.bytes().all(|b| b.is_ascii_hexdigit());
    let plano = match grupos.len() {
        6 if grupos.iter().all(|g| hex(g) && g.len() <= 2) => {
            grupos.iter().map(|g| format!("{g:0>2}")).collect()
        }
        3 if grupos.iter().all(|g| hex(g) && g.len() == 4) => grupos.concat(),
        2 if grupos.iter().all(|g| hex(g) && g.len() == 6) => grupos.concat(),
        1 if hex(v) && v.len() == 12 => v.to_owned(),
        _ => return None,
    };
    Some(plano.to_ascii_lowercase())
}

fn leer_red(cidr: &str) -> Option<(IpAddr, u8)> {
    let (ip, prefijo) = cidr.split_once('/')?;
    let ip: IpAddr = ip.parse().ok()?;
    let prefijo: u8 = prefijo.parse().ok()?;
    let max = if ip.is_ipv4() { 32 } else { 128 };
    (prefijo <= max).then_some((ip, prefijo))
}

fn en_red(ip: IpAddr, red: IpAddr, prefijo: u8) -> bool {
    fn bits(ip: IpAddr) -> (u128, u8) {
        match ip {
            IpAddr::V4(v) => (u128::from(u32::from(v)), 32),
            IpAddr::V6(v) => (u128::from(v), 128),
        }
    }
    let ((a, la), (b, lb)) = (bits(ip), bits(red));
    if la != lb {
        return false;
    }
    if prefijo == 0 {
        return true;
    }
    let desplazamiento = u32::from(la - prefijo);
    (a >> desplazamiento) == (b >> desplazamiento)
}

/// Glob sencillo sobre rutas con `/`: `*` y `?` no cruzan `/`; `**` cruza
/// cualquier número de directorios.
pub(crate) fn glob_coincide(glob: &str, ruta: &str) -> bool {
    fn coincide(g: &[u8], r: &[u8]) -> bool {
        match g {
            [] => r.is_empty(),
            [b'*', b'*', resto @ ..] => match resto.strip_prefix(b"/") {
                // `**/`: cero o más directorios.
                Some(tras) => {
                    coincide(tras, r)
                        || (0..r.len()).any(|i| r[i] == b'/' && coincide(tras, &r[i + 1..]))
                }
                None => (0..=r.len()).any(|i| coincide(resto, &r[i..])),
            },
            [b'*', resto @ ..] => {
                for i in 0..=r.len() {
                    if coincide(resto, &r[i..]) {
                        return true;
                    }
                    if i < r.len() && r[i] == b'/' {
                        break;
                    }
                }
                false
            }
            [b'?', resto @ ..] => r.first().is_some_and(|&c| c != b'/') && coincide(resto, &r[1..]),
            [c, resto @ ..] => r.first() == Some(c) && coincide(resto, &r[1..]),
        }
    }
    coincide(glob.as_bytes(), ruta.as_bytes())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob_coincide("docs/**", "docs/a/b.md"));
        assert!(glob_coincide("docs/**", "docs/a.md"));
        assert!(glob_coincide("**/x.rs", "x.rs"));
        assert!(glob_coincide("**/x.rs", "a/b/x.rs"));
        assert!(glob_coincide("src/*.rs", "src/a.rs"));
        assert!(!glob_coincide("src/*.rs", "src/a/b.rs"));
        assert!(glob_coincide(
            "tools/publicacion-permitidos.txt",
            "tools/publicacion-permitidos.txt"
        ));
        assert!(!glob_coincide("tools/x", "tools/xy"));
    }

    #[test]
    fn redes() {
        let p = Permitidos::leer("red 203.0.113.0/25 — m\nred fd00::/8 — m").unwrap();
        assert!(p.ip_permitida("203.0.113.100".parse().unwrap()));
        assert!(!p.ip_permitida("203.0.113.200".parse().unwrap()));
        assert!(p.ip_permitida("fd12::1".parse().unwrap()));
    }

    #[test]
    fn macs_en_cualquier_forma() {
        let p = Permitidos::leer("valor aa:bb:cc:00:11:22 — m").unwrap();
        assert!(p.mac_permitida("aabbcc001122"));
        assert!(p.valor_permitido("AA-BB-CC-00-11-22"));
        assert!(p.valor_permitido("aa_bb_cc_0_11_22"));
        assert_eq!(
            mac_normalizada("aabb.cc00.1122").as_deref(),
            Some("aabbcc001122")
        );
        assert_eq!(mac_normalizada("1:2:3"), None);
    }
}
