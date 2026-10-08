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

/// `true` si la IPv6 es global (2000::/3) y no de documentación (2001:db8::/32).
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
            if !real.contains(&norm) || norm == "000000000000" || norm == "ffffffffffff" {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_list_is_valid_and_case_insensitive() {
        assert_eq!(tag_class("ssid"), Some(TagClass::Ssid));
        assert_eq!(tag_class("HostName"), Some(TagClass::Host));
        assert_eq!(tag_class("pri_svr_secret"), Some(TagClass::Redact));
        assert_eq!(tag_class("no_existe"), None);
        assert!(parse_tag_list("a keep\nb rara").is_err());
        assert!(parse_tag_list("a keep\nA redact").is_err());
    }

    #[test]
    fn sensitive_names() {
        for (t, v) in [
            ("key_phrase", "x"),
            ("acs_password", "x"),
            ("GponSerialNumber", "x"),
            ("wps_device_pin", "55556666"),
            ("directory_number", "600000000"),
            ("register_uri", "600000000"),
            ("connectionRequestURL", "x"),
            ("UserName", "x"),
            ("dhcpv4_opt", "0c 01"),
            ("pri_svr_secret", "x"),
            ("wifi_passphrase", "x"),
            ("admin_pwd", "x"),
            ("api_token", "x"),
            ("contact_email", "x"),
            ("Description", "x"),
            ("user_agent_domain", "x"),
            ("wifi_ssid_id", "Casa"),
            ("wep_key1_type", "abc"),
        ] {
            assert!(is_sensitive(t, v), "{t}");
        }
        for (t, v) in [
            ("nat_port_mapping_id", "3"),
            ("wifi_ssid_id", "1"),
            ("ssid_hide", "0"),
            ("regen_key_interval", "3600"),
            ("wep_key1_type", "-1"),
            ("bssid", "aa:bb:cc:00:11:22"),
            ("igmp_snooping", "1"),
            ("RevisionNumber", "7"),
            ("authentication", "4"),
            ("dhcpv4_opt60", "0"),
            ("name", "ip2"),
        ] {
            assert!(!is_sensitive(t, v), "{t}");
        }
    }

    #[test]
    fn sanitizes_consistently() {
        let mut s = Sanitizer::new();
        let a = s
            .sanitize(
                "<R><ssid>Casa</ssid><key_phrase>secreta1</key_phrase><mac_addr>AA:BB:CC:00:11:22</mac_addr>\
                 <ipaddr>198.51.100.4</ipaddr><gw_addr>192.168.1.5</gw_addr><pre_shared_key></pre_shared_key><desc>LAN</desc></R>",
            )
            .unwrap();
        assert_eq!(
            a,
            "<R><ssid>RED-1</ssid><key_phrase>X-REDACTED-X</key_phrase><mac_addr>02:00:00:00:00:01</mac_addr>\
             <ipaddr>203.0.113.1</ipaddr><gw_addr>192.168.1.5</gw_addr><pre_shared_key></pre_shared_key><desc>LAN</desc></R>"
        );
        let b = s
            .sanitize(
                "<R><bssid>aa:bb:cc:00:11:22</bssid><acs_URL>http://198.51.100.4:7547/</acs_URL>\
                 <ntp_svr_1>http://198.51.100.4:7547/</ntp_svr_1><desc>nas</desc><HostName>pc</HostName></R>",
            )
            .unwrap();
        assert_eq!(
            b,
            "<R><bssid>02:00:00:00:00:01</bssid><acs_URL>X-REDACTED-X</acs_URL>\
             <ntp_svr_1>http://203.0.113.1:7547/</ntp_svr_1><desc>desc-1</desc><HostName>host-1</HostName></R>"
        );
        let removed: Vec<_> = s.removed_values().collect();
        assert!(removed.contains(&"secreta1") && removed.contains(&"198.51.100.4"));
    }

    #[test]
    fn mac_forms_and_global_ipv6() {
        let mut s = Sanitizer::new();
        let out = s
            .sanitize(
                "<R><hwaddr>AA-BB-CC-00-11-22</hwaddr><mac_addr>aabbcc001122</mac_addr>\
                 <rx_byte>987654321098</rx_byte><ipv6_addr>2001:db8:1:2::10</ipv6_addr>\
                 <ss_ipv6_prefix>fd00::</ss_ipv6_prefix><dst_addr>2001:db8:1:2::10/64</dst_addr></R>",
            )
            .unwrap();
        assert_eq!(
            out,
            "<R><hwaddr>02-00-00-00-00-01</hwaddr><mac_addr>020000000001</mac_addr>\
             <rx_byte>987654321098</rx_byte><ipv6_addr>2001:db8::1</ipv6_addr>\
             <ss_ipv6_prefix>fd00::</ss_ipv6_prefix><dst_addr>2001:db8::1/64</dst_addr></R>"
        );
    }

    #[test]
    fn unknown_tags_and_sensitive_names_fail() {
        let mut s = Sanitizer::new();
        let e = s
            .sanitize("<R><nuevo_campo></nuevo_campo></R>")
            .unwrap_err();
        assert!(e.contains("nuevo_campo"), "{e}");
        assert!(s.check_file_name("cbGetX__dmz=1_all=ALL").is_ok());
        assert!(s.check_file_name("cbGetX__raro=1").is_err());
        assert!(s.check_file_name("cbGetX__ssid=Casa").is_err());
    }

    #[test]
    fn learned_values_are_scrubbed_inside_other_values() {
        let mut s = Sanitizer::new();
        let status =
            "<R><wps_status_string>WPS en Casa-Pepe y operadora1.com</wps_status_string></R>";
        let ap = "<R><ssid>Casa-Pepe</ssid><UserName>operadora1</UserName></R>";
        s.learn(status);
        s.learn(ap);
        assert_eq!(
            s.sanitize(status).unwrap(),
            "<R><wps_status_string>WPS en RED-1 y X-REDACTED-X.com</wps_status_string></R>"
        );
        assert_eq!(
            s.sanitize(ap).unwrap(),
            "<R><ssid>RED-1</ssid><UserName>X-REDACTED-X</UserName></R>"
        );
        let removed: Vec<_> = s.removed_values().collect();
        assert_eq!(removed, ["Casa-Pepe", "operadora1"]);
    }

    #[test]
    fn fixture_names_split_on_underscore_after_value() {
        assert_eq!(
            split_fixture_name("cbX__rtm_cfg_dhcpv4_svr_pool_id=0_all=ALL"),
            (
                "cbX",
                vec![
                    ("rtm_cfg_dhcpv4_svr_pool_id".to_owned(), "0".to_owned()),
                    ("all".to_owned(), "ALL".to_owned())
                ]
            )
        );
        assert_eq!(split_fixture_name("cbX"), ("cbX", vec![]));
    }
}
