//! Saneado de las respuestas XML capturadas del router real.
//!
//! Solo se tocan los valores de los elementos hoja; el resto del documento se
//! conserva byte a byte. Las sustituciones son deterministas y coherentes entre
//! ficheros si se procesan en el mismo orden con el mismo [`Sanitizer`].

use regex::{Captures, Regex};
use std::{
    collections::{BTreeSet, HashMap},
    net::Ipv4Addr,
    sync::LazyLock,
};

/// Valor que sustituye a los secretos.
pub const REDACTED: &str = "X-REDACTED-X";

/// Fragmentos de nombre (en minúsculas) que marcan un elemento como secreto.
const SENSITIVE: &[&str] = &[
    "key",
    "passw",
    "psk",
    "wep",
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
];

/// Nombres exactos con datos personales codificados (opciones DHCP en hexadecimal).
const SENSITIVE_EXACT: &[&str] = &["dhcpv4_opt", "opt_value"];

/// Sufijos de identificadores, índices e indicadores que no son secretos
/// aunque contengan un fragmento de [`SENSITIVE`].
const HARMLESS_SUFFIX: &[&str] = &["_id", "_0i", "_idx", "_type", "_hide", "_interval"];

/// Fragmentos que provocan falsos positivos (`mapping` contiene `pin`, etc.).
const HARMLESS_CONTAINS: &[&str] = &[
    "mapping",
    "snooping",
    "unnumbered",
    "revisionnumber",
    "bssid",
];

/// Nombres de rol del router (también aparecen en `<UserRole>`): se retiran de
/// `<UserName>` pero no cuentan como secretos.
const ROLE_NAMES: &[&str] = &["admin", "system", "user"];

/// Descripciones del sistema que no son datos personales.
const SYSTEM_DESC: &[&str] = &["LAN", "Default"];

static LEAF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<([A-Za-z_][\w.-]*)>([^<]*)</([A-Za-z_][\w.-]*)>").expect("regex válida")
});
static MAC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b[0-9a-f]{2}(?::[0-9a-f]{2}){5}\b").expect("regex válida"));
static IPV4: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{1,3}(?:\.\d{1,3}){3}\b").expect("regex válida"));

/// `true` si el valor del elemento `tag` es secreto y debe valer [`REDACTED`]
/// (salvo `ssid`, que se sustituye por `RED-n`).
pub fn is_sensitive(tag: &str) -> bool {
    let t = tag.to_ascii_lowercase();
    if SENSITIVE_EXACT.contains(&t.as_str()) {
        return true;
    }
    if HARMLESS_SUFFIX.iter().any(|s| t.ends_with(s))
        || HARMLESS_CONTAINS.iter().any(|s| t.contains(s))
    {
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

fn is_personal_desc(tag: &str, value: &str) -> bool {
    tag == "desc" && !SYSTEM_DESC.contains(&value) && !value.bytes().all(|b| b.is_ascii_digit())
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

/// Saneador con estado: recuerda las sustituciones para mantener la igualdad
/// entre ficheros y acumula los valores reales retirados.
#[derive(Default)]
pub struct Sanitizer {
    macs: HashMap<String, usize>,
    ips: HashMap<Ipv4Addr, usize>,
    ssids: HashMap<String, usize>,
    hosts: HashMap<String, usize>,
    descs: HashMap<String, usize>,
    /// Secretos largos que se buscan también dentro de otros valores.
    embedded: BTreeSet<String>,
    removed: BTreeSet<String>,
}

/// Longitud mínima de un secreto para buscarlo dentro de otros valores.
const EMBEDDED_MIN: usize = 8;

/// Elementos hoja `(etiqueta, valor sin espacios, rango del elemento)`.
fn leaves(xml: &str) -> impl Iterator<Item = (&str, &str, std::ops::Range<usize>)> {
    LEAF.captures_iter(xml).filter_map(|c| {
        let whole = c.get(0)?;
        let (tag, value, close) = (c.get(1)?.as_str(), c.get(2)?.as_str(), c.get(3)?.as_str());
        (tag == close && !value.trim().is_empty()).then(|| (tag, value.trim(), whole.range()))
    })
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
            if tag.eq_ignore_ascii_case("ssid") {
                index_of(&mut self.ssids, value.to_owned());
                self.remember(value);
            } else if is_sensitive(tag) {
                self.remember(value);
                if value.len() >= EMBEDDED_MIN {
                    self.embedded.insert(value.to_owned());
                }
            }
        }
    }

    /// Devuelve el documento con los valores sensibles sustituidos.
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
        if tag.eq_ignore_ascii_case("ssid") {
            self.remember(value);
            return Ok(format!(
                "RED-{}",
                index_of(&mut self.ssids, value.to_owned())
            ));
        }
        if is_sensitive(tag) {
            self.remember(value);
            return Ok(REDACTED.to_owned());
        }
        if tag == "hostname" {
            self.remember(value);
            return Ok(format!(
                "host-{}",
                index_of(&mut self.hosts, value.to_owned())
            ));
        }
        if is_personal_desc(tag, value) {
            self.remember(value);
            return Ok(format!(
                "desc-{}",
                index_of(&mut self.descs, value.to_owned())
            ));
        }
        let value = self.scrub_embedded(value);
        let value = if tag.ends_with("mask") {
            value
        } else {
            self.replace_macs(&value)?
        };
        self.replace_public_ips(&value)
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

    fn replace_macs(&mut self, value: &str) -> Result<String, String> {
        let mut err = None;
        let out = MAC.replace_all(value, |c: &Captures| {
            let mac = c[0].to_ascii_lowercase();
            if mac == "00:00:00:00:00:00" || mac == "ff:ff:ff:ff:ff:ff" {
                return c[0].to_owned();
            }
            self.remember(&c[0]);
            let n = index_of(&mut self.macs, mac);
            if n > 0xffff {
                err = Some("demasiadas MAC distintas".to_owned());
            }
            format!("02:00:00:00:{:02x}:{:02x}", n >> 8, n & 0xff)
        });
        err.map_or(Ok(out.into_owned()), Err)
    }

    fn replace_public_ips(&mut self, value: &str) -> Result<String, String> {
        let mut err = None;
        let out = IPV4.replace_all(value, |c: &Captures| {
            let Ok(ip) = c[0].parse::<Ipv4Addr>() else {
                return c[0].to_owned();
            };
            if !is_public_ipv4(ip) {
                return c[0].to_owned();
            }
            self.remember(&c[0]);
            let n = index_of(&mut self.ips, ip);
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
    fn sensitive_names() {
        for t in [
            "key_phrase",
            "acs_password",
            "GponSerialNumber",
            "wps_device_pin",
            "directory_number",
            "register_uri",
            "connectionRequestURL",
            "UserName",
            "dhcpv4_opt",
        ] {
            assert!(is_sensitive(t), "{t}");
        }
        for t in [
            "nat_port_mapping_id",
            "wifi_ssid_id",
            "ssid_hide",
            "regen_key_interval",
            "wep_key1_type",
            "bssid",
            "igmp_snooping",
            "RevisionNumber",
            "dhcpv4_opt60",
            "name",
        ] {
            assert!(!is_sensitive(t), "{t}");
        }
    }

    #[test]
    fn sanitizes_consistently() {
        let mut s = Sanitizer::new();
        let a = s
            .sanitize(
                "<R><ssid>Casa</ssid><key_phrase>secreta1</key_phrase><mac_addr>AA:BB:CC:00:11:22</mac_addr>\
                 <ip>198.51.100.4</ip><lan>192.168.1.5</lan><pre_shared_key></pre_shared_key><desc>LAN</desc></R>",
            )
            .unwrap();
        assert_eq!(
            a,
            "<R><ssid>RED-1</ssid><key_phrase>X-REDACTED-X</key_phrase><mac_addr>02:00:00:00:00:01</mac_addr>\
             <ip>203.0.113.1</ip><lan>192.168.1.5</lan><pre_shared_key></pre_shared_key><desc>LAN</desc></R>"
        );
        let b = s
            .sanitize("<R><bssid>aa:bb:cc:00:11:22</bssid><u>http://198.51.100.4:7547/</u><desc>nas</desc></R>")
            .unwrap();
        assert_eq!(
            b,
            "<R><bssid>02:00:00:00:00:01</bssid><u>http://203.0.113.1:7547/</u><desc>desc-1</desc></R>"
        );
        let removed: Vec<_> = s.removed_values().collect();
        assert!(removed.contains(&"secreta1") && removed.contains(&"198.51.100.4"));
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
}
