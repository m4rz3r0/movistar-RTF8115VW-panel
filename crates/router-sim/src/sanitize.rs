//! Saneado de las respuestas XML capturadas del router real.
//!
//! Solo se tocan los valores de los elementos hoja; el resto del documento se
//! conserva byte a byte. Las sustituciones son deterministas y coherentes entre
//! ficheros si se procesan en el mismo orden con el mismo [`Sanitizer`].
//!
//! Cada etiqueta debe figurar en la lista revisada `fixtures-tags.txt` (ver
//! [`tag_class`]); una etiqueta desconocida es un error, para que un campo nuevo
//! no se filtre sin que nadie decida qué hacer con él.

use regex::{Captures, Regex};
use std::{
    collections::{BTreeSet, HashMap},
    net::{Ipv4Addr, Ipv6Addr},
    sync::LazyLock,
};

/// Valor que sustituye a los secretos.
pub const REDACTED: &str = "X-REDACTED-X";

/// Lista revisada de etiquetas permitidas y su clase.
pub const TAGS_FILE: &str = include_str!("../fixtures-tags.txt");

/// Fragmentos de nombre (en minúsculas) que marcan un elemento como secreto.
const SENSITIVE: &[&str] = &[
    "key",
    "passw",
    "passphrase",
    "pwd",
    "psk",
    "wep",
    "secret",
    "token",
    "auth",
    "email",
    "username",
    "user_name",
    "acs",
    "conn_req",
    "connectionrequest",
    "gpon",
    "serial",
    "phone",
    "number",
    "pin",
    "ssid",
    "uri",
    "domain",
    "description",
];

/// Nombres exactos con datos personales codificados (opciones DHCP en hexadecimal).
const SENSITIVE_EXACT: &[&str] = &["dhcpv4_opt", "opt_value"];

/// Sufijos de identificadores, índices e indicadores: no son secretos aunque
/// contengan un fragmento de [`SENSITIVE`], **solo si el valor es un entero**.
const NUMERIC_SUFFIX: &[&str] = &["_id", "_0i", "_idx", "_type", "_hide", "_interval"];

/// Nombres exactos exentos, también solo con valor entero.
const NUMERIC_EXACT: &[&str] = &[
    "igmp_snooping",
    "ipv6_unnumbered",
    "revisionnumber",
    "authentication",
];

/// Nombres de rol del router (también aparecen en `<UserRole>`): se retiran de
/// `<UserName>` pero no cuentan como secretos.
const ROLE_NAMES: &[&str] = &["admin", "system", "user"];

/// Descripciones del sistema que no son datos personales.
const SYSTEM_DESC: &[&str] = &["LAN", "Default"];

/// Longitud mínima de un secreto para buscarlo dentro de otros valores.
const EMBEDDED_MIN: usize = 8;

static LEAF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<([A-Za-z_][\w.-]*)>([^<]*)</([A-Za-z_][\w.-]*)>").expect("regex válida")
});
static MAC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b[0-9a-f]{2}(?::[0-9a-f]{2}){5}\b|\b[0-9a-f]{2}(?:-[0-9a-f]{2}){5}\b|\b[0-9a-f]{12}\b",
    )
    .expect("regex válida")
});
static IPV4: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{1,3}(?:\.\d{1,3}){3}\b").expect("regex válida"));
static IPV6: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)[0-9a-f:]*:[0-9a-f:]*:[0-9a-f:]*").expect("regex válida"));
static INTEGER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-?\d+$").expect("regex válida"));

/// Qué hacer con el valor de una etiqueta permitida.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagClass {
    /// Se conserva (con seudónimos para MAC, IP públicas y secretos incrustados).
    Keep,
    /// `X-REDACTED-X`.
    Redact,
    /// `RED-n`.
    Ssid,
    /// `host-n`.
    Host,
    /// `desc-n` salvo descripciones del sistema.
    Desc,
}

/// Interpreta la lista de etiquetas (`<etiqueta> <clase>` por línea, `#` comenta).
pub fn parse_tag_list(text: &str) -> Result<HashMap<String, TagClass>, String> {
    let mut map = HashMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split_whitespace();
        let (Some(tag), Some(class), None) = (it.next(), it.next(), it.next()) else {
            return Err(format!("línea {}: se espera `<etiqueta> <clase>`", n + 1));
        };
        let class = match class {
            "keep" => TagClass::Keep,
            "redact" => TagClass::Redact,
            "ssid" => TagClass::Ssid,
            "host" => TagClass::Host,
            "desc" => TagClass::Desc,
            other => return Err(format!("línea {}: clase desconocida {other}", n + 1)),
        };
        if map.insert(tag.to_ascii_lowercase(), class).is_some() {
            return Err(format!("línea {}: {tag} repetida", n + 1));
        }
    }
    Ok(map)
}

static TAGS: LazyLock<HashMap<String, TagClass>> =
    LazyLock::new(|| parse_tag_list(TAGS_FILE).expect("fixtures-tags.txt válido"));

/// Clase de una etiqueta según `fixtures-tags.txt`; `None` si no está permitida.
pub fn tag_class(tag: &str) -> Option<TagClass> {
    TAGS.get(&tag.to_ascii_lowercase()).copied()
}

fn unknown_tag(tag: &str) -> String {
    format!("etiqueta no permitida <{tag}>: decide su clase en crates/router-sim/fixtures-tags.txt")
}

/// `true` si el valor del elemento `tag` es secreto por el nombre. Los
/// identificadores e indicadores solo quedan exentos si el valor es entero.
pub fn is_sensitive(tag: &str, value: &str) -> bool {
    let t = tag.to_ascii_lowercase();
    if SENSITIVE_EXACT.contains(&t.as_str()) {
        return true;
    }
    let numeric = value.is_empty() || INTEGER.is_match(value);
    if numeric
        && (NUMERIC_SUFFIX.iter().any(|s| t.ends_with(s)) || NUMERIC_EXACT.contains(&t.as_str()))
    {
        return false;
    }
    if t == "bssid" && macs_in(tag, value).len() == 1 {
        return false;
    }
    SENSITIVE.iter().any(|s| t.contains(s))
}

/// `true` si la IPv4 es pública (ni privada, ni especial, ni de documentación).
pub fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_documentation()
        || o[0] == 0
        || o[0] >= 224
        || (o[0] == 100 && (o[1] & 0xc0) == 64))
}

/// `true` si la IPv6 es unicast global y no de documentación (2001:db8::/32).
pub fn is_global_ipv6(ip: Ipv6Addr) -> bool {
    let s = ip.segments();
    (s[0] & 0xe000) == 0x2000 && !(s[0] == 0x2001 && s[1] == 0x0db8)
}

/// MAC (`aa:bb:..`, `aa-bb-..` o 12 hexadecimales) dentro de `value`,
/// normalizadas a 12 hexadecimales en minúsculas. La forma sin separadores
/// solo cuenta si tiene alguna letra o la etiqueta es de MAC (`mac`, `hwaddr`,
/// `bssid`), para no confundirla con contadores decimales.
pub fn macs_in(tag: &str, value: &str) -> Vec<String> {
    let t = tag.to_ascii_lowercase();
    let mac_tag = t.contains("mac") || t.contains("hwaddr") || t.contains("bssid");
    MAC.find_iter(value)
        .map(|m| m.as_str())
        .filter(|m| m.len() != 12 || mac_tag || m.bytes().any(|b| b.is_ascii_alphabetic()))
        .map(|m| m.replace([':', '-'], "").to_ascii_lowercase())
        .collect()
}

/// `true` si la MAC normalizada es un seudónimo o una MAC especial.
pub fn is_fake_mac(normalized: &str) -> bool {
    normalized.starts_with("02000000")
        || normalized == "000000000000"
        || normalized == "ffffffffffff"
}

/// IPv6 globales dentro de `value`.
pub fn global_ipv6_in(value: &str) -> Vec<Ipv6Addr> {
    IPV6.find_iter(value)
        .filter_map(|m| m.as_str().parse::<Ipv6Addr>().ok())
        .filter(|ip| is_global_ipv6(*ip))
        .collect()
}

/// Divide el nombre de una fixture `<call>__k1=v1_k2=v2` (sin `.xml`) en la
/// llamada y sus parámetros. Las claves pueden contener `_`, los valores no.
pub fn split_fixture_name(stem: &str) -> (&str, Vec<(String, String)>) {
    let Some((call, params)) = stem.split_once("__") else {
        return (stem, Vec::new());
    };
    let parts: Vec<&str> = params.split('=').collect();
    let mut out = Vec::new();
    let mut key = parts[0].to_owned();
    for (i, part) in parts.iter().enumerate().skip(1) {
        if i == parts.len() - 1 {
            out.push((key.clone(), (*part).to_owned()));
        } else {
            let (value, next) = part.split_once('_').unwrap_or((part, ""));
            out.push((key, value.to_owned()));
            key = next.to_owned();
        }
    }
    (call, out)
}

fn is_system_desc(value: &str) -> bool {
    SYSTEM_DESC.contains(&value) || value.bytes().all(|b| b.is_ascii_digit())
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Índice (desde 1) de `key` en `map`, asignando el siguiente si es nueva.
fn index_of<K: std::hash::Hash + Eq>(map: &mut HashMap<K, usize>, key: K) -> usize {
    let next = map.len() + 1;
    *map.entry(key).or_insert(next)
}

/// Elementos hoja `(etiqueta, valor sin espacios, rango del elemento)`.
fn leaves(xml: &str) -> impl Iterator<Item = (&str, &str, std::ops::Range<usize>)> {
    LEAF.captures_iter(xml).filter_map(|c| {
        let whole = c.get(0)?;
        let (tag, value, close) = (c.get(1)?.as_str(), c.get(2)?.as_str(), c.get(3)?.as_str());
        (tag == close).then(|| (tag, value.trim(), whole.range()))
    })
}

/// Saneador con estado: recuerda las sustituciones para mantener la igualdad
/// entre ficheros y acumula los valores reales retirados.
#[derive(Default)]
pub struct Sanitizer {
    macs: HashMap<String, usize>,
    ipv4: HashMap<Ipv4Addr, usize>,
    ipv6: HashMap<Ipv6Addr, usize>,
    ssids: HashMap<String, usize>,
    hosts: HashMap<String, usize>,
    descs: HashMap<String, usize>,
    /// Secretos largos que se buscan también dentro de otros valores.
    embedded: BTreeSet<String>,
    removed: BTreeSet<String>,
}

impl Sanitizer {
    /// Crea un saneador vacío.
    pub fn new() -> Self {
        Self::default()
    }

    /// Primera pasada: aprende los SSID y los secretos de un documento para
    /// poder retirarlos también cuando aparecen dentro de otros valores
    /// (p. ej. el SSID en `wps_status_string`). Se llama con todos los
    /// documentos antes de [`Sanitizer::sanitize`].
    pub fn learn(&mut self, xml: &str) {
        for (tag, value, _) in leaves(xml) {
            if value.is_empty() {
                continue;
            }
            match tag_class(tag) {
                Some(TagClass::Ssid) => {
                    index_of(&mut self.ssids, value.to_owned());
                    self.remember(value);
                }
                Some(TagClass::Redact) => self.learn_secret(value),
                Some(TagClass::Keep) if is_sensitive(tag, value) => self.learn_secret(value),
                _ => {}
            }
        }
    }

    fn learn_secret(&mut self, value: &str) {
        self.remember(value);
        if value.len() >= EMBEDDED_MIN {
            self.embedded.insert(value.to_owned());
        }
    }

    /// Devuelve el documento con los valores sensibles sustituidos. Falla si
    /// aparece una etiqueta que no está en `fixtures-tags.txt`.
    pub fn sanitize(&mut self, xml: &str) -> Result<String, String> {
        let mut out = String::with_capacity(xml.len());
        let mut last = 0;
        for (tag, value, range) in leaves(xml) {
            out.push_str(&xml[last..range.start]);
            last = range.end;
            let new = self.leaf(tag, value)?;
            out.push_str(&format!("<{tag}>{new}</{tag}>"));
        }
        out.push_str(&xml[last..]);
        Ok(out)
    }

    /// Comprueba el nombre de una fixture (sin `.xml`): las claves de sus
    /// parámetros deben estar permitidas y sus valores no deben necesitar
    /// saneado (los nombres no se reescriben).
    pub fn check_file_name(&mut self, stem: &str) -> Result<(), String> {
        for (k, v) in split_fixture_name(stem).1 {
            if self.leaf(&k, &v)? != v {
                return Err(format!("el parámetro {k} del nombre necesita saneado"));
            }
        }
        Ok(())
    }

    /// Valores reales retirados de al menos 4 caracteres, ordenados (sin los
    /// nombres de rol del router, que no son secretos).
    pub fn removed_values(&self) -> impl Iterator<Item = &str> {
        self.removed
            .iter()
            .map(String::as_str)
            .filter(|v| v.len() >= 4 && !ROLE_NAMES.contains(v))
    }

    fn remember(&mut self, value: &str) {
        self.removed.insert(value.to_owned());
        let plain = unescape(value);
        if plain != value {
            self.removed.insert(plain);
        }
    }

    fn leaf(&mut self, tag: &str, value: &str) -> Result<String, String> {
        let class = tag_class(tag).ok_or_else(|| unknown_tag(tag))?;
        if value.is_empty() {
            return Ok(String::new());
        }
        let pseudo =
            |s: &mut Self, prefix: &str, map: fn(&mut Self) -> &mut HashMap<String, usize>| {
                s.remember(value);
                format!("{prefix}-{}", index_of(map(s), value.to_owned()))
            };
        match class {
            TagClass::Ssid => return Ok(pseudo(self, "RED", |s| &mut s.ssids)),
            TagClass::Host => return Ok(pseudo(self, "host", |s| &mut s.hosts)),
            TagClass::Desc if !is_system_desc(value) => {
                return Ok(pseudo(self, "desc", |s| &mut s.descs));
            }
            TagClass::Redact => {
                self.remember(value);
                return Ok(REDACTED.to_owned());
            }
            TagClass::Keep if is_sensitive(tag, value) => {
                self.remember(value);
                return Ok(REDACTED.to_owned());
            }
            TagClass::Keep | TagClass::Desc => {}
        }
        let value = self.scrub_embedded(value);
        let value = if tag.to_ascii_lowercase().ends_with("mask") {
            value
        } else {
            self.replace_macs(tag, &value)?
        };
        let value = self.replace_global_ipv6(&value);
        self.replace_public_ipv4(&value)
    }

    /// Sustituye los SSID y secretos aprendidos que aparecen dentro del valor.
    fn scrub_embedded(&self, value: &str) -> String {
        let mut pats: Vec<(&str, String)> = self
            .ssids
            .iter()
            .filter(|(s, _)| s.len() >= 4)
            .map(|(s, n)| (s.as_str(), format!("RED-{n}")))
            .chain(
                self.embedded
                    .iter()
                    .map(|s| (s.as_str(), REDACTED.to_owned())),
            )
            .collect();
        pats.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then(a.0.cmp(b.0)));
        let mut out = value.to_owned();
        for (pat, with) in pats {
            if out.contains(pat) {
                out = out.replace(pat, &with);
            }
        }
        out
    }

    fn replace_macs(&mut self, tag: &str, value: &str) -> Result<String, String> {
        let real: BTreeSet<String> = macs_in(tag, value).into_iter().collect();
        let mut err = None;
        let out = MAC.replace_all(value, |c: &Captures| {
            let found = &c[0];
            let norm = found.replace([':', '-'], "").to_ascii_lowercase();
            // Los seudónimos (`02:00:00:…`) ya están saneados: no se renumeran.
            if !real.contains(&norm) || is_fake_mac(&norm) {
                return found.to_owned();
            }
            self.remember(found);
            let n = index_of(&mut self.macs, norm);
            if n > 0xffff {
                err = Some("demasiadas MAC distintas".to_owned());
            }
            let (hi, lo) = (n >> 8, n & 0xff);
            match found.as_bytes()[2] {
                b':' => format!("02:00:00:00:{hi:02x}:{lo:02x}"),
                b'-' => format!("02-00-00-00-{hi:02x}-{lo:02x}"),
                _ => format!("02000000{hi:02x}{lo:02x}"),
            }
        });
        err.map_or(Ok(out.into_owned()), Err)
    }

    fn replace_global_ipv6(&mut self, value: &str) -> String {
        IPV6.replace_all(value, |c: &Captures| match c[0].parse::<Ipv6Addr>() {
            Ok(ip) if is_global_ipv6(ip) => {
                self.remember(&c[0]);
                format!("2001:db8::{:x}", index_of(&mut self.ipv6, ip))
            }
            _ => c[0].to_owned(),
        })
        .into_owned()
    }

    fn replace_public_ipv4(&mut self, value: &str) -> Result<String, String> {
        let mut err = None;
        let out = IPV4.replace_all(value, |c: &Captures| {
            let Ok(ip) = c[0].parse::<Ipv4Addr>() else {
                return c[0].to_owned();
            };
            if !is_public_ipv4(ip) {
                return c[0].to_owned();
            }
            self.remember(&c[0]);
            let n = index_of(&mut self.ipv4, ip);
            if n > 254 {
                err = Some("demasiadas IP públicas distintas".to_owned());
            }
            format!("203.0.113.{n}")
        });
        err.map_or(Ok(out.into_owned()), Err)
    }
}

/// Primer equipo de la LAN renumerado (`192.168.1.60`).
const LAN_PRIMERO: u8 = 60;
/// Equipos distintos de la LAN que caben en `.60`–`.99`.
const LAN_MAX: usize = 40;
/// Pool DHCP neutro y su DNS (el propio router).
const POOL_INICIO: &str = "192.168.1.100";
const POOL_FIN: &str = "192.168.1.199";
const POOL_DNS: &str = "192.168.1.1";
/// Puerto neutro de los reenvíos.
const PUERTO_REENVIO: &str = "8080";
/// Hora fija del resumen (2026-01-01 00:00 UTC), para no fechar la captura.
const HORA_RESUMEN: &str = "1767225600";

/// Segunda etapa del saneado: retira la topología de la red del dueño.
///
/// - Los equipos de la LAN (`192.168.1.x` salvo `.0`, `.1` y `.255`) pasan a
///   `192.168.1.60`, `.61`… por orden de primera aparición; más de
///   [`LAN_MAX`] es un error.
/// - Las direcciones del operador (`10/8`, `172.16/12`, `100.64/10`) pasan a
///   `10.255.0.N` por orden de aparición. La puerta de enlace WAN de fábrica
///   (`192.168.144.1`) no cambia.
/// - El pool DHCP (`cbGetDhcpv4ServerPool*`) pasa a `.100`–`.199` con el router
///   como DNS; los reenvíos (`cbGetNatPortMapping*`), al puerto 8080; la hora
///   de `cbGetSummary`, a una fija.
///
/// Es determinista si las fixtures se procesan en el mismo orden (por nombre),
/// e idempotente: aplicarlo sobre fixtures ya neutralizadas no las cambia.
/// Sus errores nunca incluyen el valor original.
#[derive(Default)]
pub struct Neutralizador {
    equipos: HashMap<Ipv4Addr, usize>,
    operador: HashMap<Ipv4Addr, usize>,
}

impl Neutralizador {
    /// Crea un neutralizador vacío.
    pub fn new() -> Self {
        Self::default()
    }

    /// Devuelve la fixture `fixture` (su nombre, con o sin `.xml`) con la
    /// topología neutralizada. Solo toca los valores de los elementos hoja.
    pub fn neutralizar(&mut self, fixture: &str, xml: &str) -> Result<String, String> {
        let llamada = split_fixture_name(fixture.trim_end_matches(".xml")).0;
        let mut out = String::with_capacity(xml.len());
        let mut last = 0;
        for (tag, value, range) in leaves(xml) {
            out.push_str(&xml[last..range.start]);
            last = range.end;
            let new = match fijo(llamada, tag, value) {
                Some(v) => v.to_owned(),
                None => self.direcciones(value)?,
            };
            out.push_str(&format!("<{tag}>{new}</{tag}>"));
        }
        out.push_str(&xml[last..]);
        Ok(out)
    }

    fn direcciones(&mut self, value: &str) -> Result<String, String> {
        let mut err = None;
        let out = IPV4.replace_all(value, |c: &Captures| {
            let Ok(ip) = c[0].parse::<Ipv4Addr>() else {
                return c[0].to_owned();
            };
            let o = ip.octets();
            if [o[0], o[1], o[2]] == [192, 168, 1] && ![0, 1, 255].contains(&o[3]) {
                let n = index_of(&mut self.equipos, ip);
                if n > LAN_MAX {
                    err = Some(format!("más de {LAN_MAX} equipos distintos en la LAN"));
                    return c[0].to_owned();
                }
                // n ≤ 40, cabe en un octeto.
                return format!("192.168.1.{}", LAN_PRIMERO + (n - 1) as u8);
            }
            let operador = o[0] == 10
                || (o[0] == 172 && (o[1] & 0xf0) == 16)
                || (o[0] == 100 && (o[1] & 0xc0) == 64);
            if operador {
                let n = index_of(&mut self.operador, ip);
                if n > 254 {
                    err = Some("más de 254 direcciones distintas del operador".to_owned());
                    return c[0].to_owned();
                }
                return format!("10.255.0.{n}");
            }
            c[0].to_owned()
        });
        err.map_or(Ok(out.into_owned()), Err)
    }
}

/// Valor fijo de un campo que se sustituye entero según la llamada.
fn fijo(llamada: &str, tag: &str, value: &str) -> Option<&'static str> {
    let es_ip = || {
        value
            .parse::<Ipv4Addr>()
            .is_ok_and(|ip| !ip.is_unspecified())
    };
    match llamada {
        "cbGetDhcpv4ServerPool" => match tag {
            "ipv4_addr_min" if es_ip() => Some(POOL_INICIO),
            "ipv4_addr_max" if es_ip() => Some(POOL_FIN),
            t if t.starts_with("ipv4_dns_addr") && es_ip() => Some(POOL_DNS),
            _ => None,
        },
        "cbGetNatPortMapping" => {
            let puerto = ["ext", "int"].iter().any(|lado| {
                tag.strip_prefix(lado)
                    .and_then(|t| t.strip_prefix("_l4_port_"))
                    .is_some_and(|t| t == "min_value" || t == "max_value")
            });
            (puerto && INTEGER.is_match(value)).then_some(PUERTO_REENVIO)
        }
        "cbGetSummary" if tag == "SysTime" && INTEGER.is_match(value) => Some(HORA_RESUMEN),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
