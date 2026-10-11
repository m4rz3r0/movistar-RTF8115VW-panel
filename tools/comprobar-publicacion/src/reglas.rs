//! Reglas genéricas: no necesitan datos del dueño y valen en CI.
//!
//! Cada regla de texto es una función `regla_x(linea, permitidos)` que
//! devuelve los detalles de lo que encuentra en la línea (un tipo, como «ipv4
//! privada»), **nunca** el valor. Ante la duda una regla da hallazgo: un falso
//! positivo se resuelve con una entrada revisada en `permitidos`; un falso
//! negativo publica un dato.

use crate::{
    Regla,
    permitidos::{Permitidos, mac_normalizada},
};
use regex::Regex;
use router_sim::sanitize::{self, TagClass};
use sha2::{Digest, Sha256};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::LazyLock,
};

fn re(patron: &str) -> Regex {
    Regex::new(patron).expect("expresión regular válida")
}

type ReglaDeLinea = fn(&str, &Permitidos) -> Vec<&'static str>;

/// Reglas de texto que se aplican a cualquier fichero salvo los de bloqueo.
const GENERICAS: [(Regla, ReglaDeLinea); 7] = [
    (Regla::Ipv4, regla_ipv4),
    (Regla::Ipv6, regla_ipv6),
    (Regla::Mac, regla_mac),
    (Regla::Dominio, regla_dominio),
    (Regla::Correo, regla_correo),
    (Regla::Clave, regla_clave),
    (Regla::Identificador, regla_identificador),
];

/// Todas las reglas genéricas de texto sobre una línea.
pub(crate) fn genericas(linea: &str, p: &Permitidos) -> Vec<(Regla, &'static str)> {
    GENERICAS
        .iter()
        .flat_map(|&(regla, f)| f(linea, p).into_iter().map(move |d| (regla, d)))
        .collect()
}

/// La línea con `_` cambiado por espacio, para que `\b` separe también en
/// `mac_<valor>` o `ip_<valor>` (`_` es un carácter de palabra).
///
/// Entre dos cifras se conserva: `1_791_417_600` es un literal numérico, no un
/// teléfono.
fn separar_guiones_bajos(linea: &str) -> String {
    let b = linea.as_bytes();
    linea
        .char_indices()
        .map(|(i, c)| {
            let entre_cifras =
                i > 0 && b[i - 1].is_ascii_digit() && b.get(i + 1).is_some_and(u8::is_ascii_digit);
            if c == '_' && !entre_cifras { ' ' } else { c }
        })
        .collect()
}

// --- IPv4 -------------------------------------------------------------------

static IPV4: LazyLock<Regex> = LazyLock::new(|| re(r"\d{1,3}(?:\.\d{1,3}){3}"));

/// IPv4 fuera de los rangos de documentación, de las especiales y de
/// `permitidos`. Se reconoce también dentro de URL, con `:puerto`, con ceros a
/// la izquierda y pegada a letras, pero no como parte de un número con más
/// componentes ni como versión (`v1.2.3.4`, `version = "1.2.3.4"`, `==`).
pub(crate) fn regla_ipv4(linea: &str, p: &Permitidos) -> Vec<&'static str> {
    let linea = separar_guiones_bajos(linea);
    let b = linea.as_bytes();
    IPV4.find_iter(&linea)
        .filter_map(|m| {
            let (antes, despues) = (&b[..m.start()], &b[m.end()..]);
            let digito = |c: Option<&u8>| c.is_some_and(u8::is_ascii_digit);
            // Parte de un número más largo o con más componentes.
            if digito(antes.last())
                || (antes.last() == Some(&b'.') && digito(antes.iter().rev().nth(1)))
                || digito(despues.first())
                || (despues.first() == Some(&b'.') && digito(despues.get(1)))
            {
                return None;
            }
            if es_version(&linea[..m.start()]) {
                return None;
            }
            let ip = octetos(m.as_str())?;
            clasificar_ipv4(ip, p)
        })
        .collect()
}

/// `true` si lo que precede a un número de cuatro componentes indica que es
/// una versión: una `v` suelta, un operador de versión (`==`, `>=`, `~=`…) o
/// una asignación a una clave que se llama versión.
fn es_version(antes: &str) -> bool {
    let a = antes.as_bytes();
    if let Some((&v, resto)) = a.split_last()
        && (v == b'v' || v == b'V')
    {
        return !resto.last().is_some_and(u8::is_ascii_alphanumeric);
    }
    let t = antes.trim_end_matches([' ', '\t', '"', '\'', '\\']);
    let Some(sin_igual) = t.strip_suffix('=') else {
        return false;
    };
    if sin_igual.ends_with(['=', '>', '<', '~', '!', '^']) {
        return true;
    }
    let clave: String = sin_igual
        .trim_end_matches([' ', '\t', '"', '\'', '\\'])
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.'))
        .collect::<String>()
        .to_ascii_lowercase();
    // `clave` está al revés.
    clave.starts_with("noisrev") || clave == "v"
}

fn octetos(s: &str) -> Option<Ipv4Addr> {
    let mut o = [0u8; 4];
    for (i, parte) in s.split('.').enumerate() {
        o[i] = parte.parse::<u16>().ok()?.try_into().ok()?;
    }
    Some(Ipv4Addr::from(o))
}

fn clasificar_ipv4(ip: Ipv4Addr, p: &Permitidos) -> Option<&'static str> {
    let o = ip.octets();
    let permitida = ip.is_documentation()
        || ip.is_unspecified()
        || o[0] == 127
        || o[0] == 255
        || (224..=239).contains(&o[0])
        || ip == Ipv4Addr::new(192, 168, 1, 1)
        || p.ip_permitida(IpAddr::V4(ip));
    if permitida {
        None
    } else if ip.is_private() {
        Some("ipv4 privada")
    } else if o[0] == 100 && (o[1] & 0xc0) == 64 {
        Some("ipv4 cgnat")
    } else if sanitize::is_public_ipv4(ip) {
        Some("ipv4 pública")
    } else {
        None
    }
}

// --- IPv6 -------------------------------------------------------------------

static IPV6: LazyLock<Regex> = LazyLock::new(|| re(r"[0-9A-Fa-f:]*:[0-9A-Fa-f:]*:[0-9A-Fa-f:]*"));

/// IPv6 globales fuera de `2001:db8::/32` y de `permitidos`. Si el tramo de
/// hexadecimales y `:` no es una dirección (p. ej. `addr:` delante), se
/// prueban sus sufijos.
pub(crate) fn regla_ipv6(linea: &str, p: &Permitidos) -> Vec<&'static str> {
    IPV6.find_iter(linea)
        .filter_map(|m| primera_ipv6(m.as_str()))
        .filter(|&ip| sanitize::is_global_ipv6(ip) && !p.ip_permitida(IpAddr::V6(ip)))
        .map(|_| "ipv6 global")
        .collect()
}

fn primera_ipv6(tramo: &str) -> Option<Ipv6Addr> {
    let b = tramo.as_bytes();
    (0..b.len())
        .filter(|&i| i == 0 || b[i - 1] == b':' || tramo[i..].starts_with("::"))
        .find_map(|i| {
            let t = &tramo[i..];
            t.parse().ok().or_else(|| {
                t.strip_suffix(':')
                    .filter(|s| !s.ends_with(':'))
                    .and_then(|s| s.parse().ok())
            })
        })
}

// --- MAC --------------------------------------------------------------------

static MAC: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)[0-9a-f]{2}(?::[0-9a-f]{2}){5}|[0-9a-f]{2}(?:-[0-9a-f]{2}){5}|[0-9a-f]{4}\.[0-9a-f]{4}\.[0-9a-f]{4}|[0-9a-f]{12}",
    )
});

/// MAC con OUI real (`:`, `-`, `.` o sin separador; mayúsculas o
/// minúsculas). Valen las especiales, las administradas localmente, el rango
/// de documentación `00:00:5e:00:53:xx` y las de `permitidos`. La forma sin
/// separador solo cuenta si tiene alguna letra (un contador de 12 cifras no es
/// una MAC).
pub(crate) fn regla_mac(linea: &str, p: &Permitidos) -> Vec<&'static str> {
    let linea = separar_guiones_bajos(linea);
    let b = linea.as_bytes();
    MAC.find_iter(&linea)
        .filter(|m| {
            let alnum = |c: Option<&u8>| c.is_some_and(u8::is_ascii_alphanumeric);
            !alnum(b[..m.start()].last()) && !alnum(b.get(m.end()))
        })
        .filter_map(|m| mac_normalizada(m.as_str()))
        .filter(|n| !n.bytes().all(|c| c.is_ascii_digit()))
        .filter(|n| !mac_permitida(n, p))
        .map(|_| "oui real")
        .collect()
}

fn mac_permitida(n: &str, p: &Permitidos) -> bool {
    let primero = u8::from_str_radix(&n[..2], 16).unwrap_or(0);
    sanitize::is_fake_mac(n)
        || primero & 0x02 != 0
        || n.starts_with("00005e0053")
        || p.mac_permitida(n)
}

// --- Dominios y correos -----------------------------------------------------

static DOMINIO: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+(?:com|net|org|es|eu|io|dev|app|me|info|xyz|lan|local|home|arpa|cloud|tech|site)\b",
    )
});

/// Nombres de dominio fuera de los reservados para documentación y de
/// `permitidos`.
///
/// Aquí `_` sigue siendo parte de la palabra: `campo.es_valido` es código, no
/// un dominio, y un nombre de máquina no lleva `_`.
pub(crate) fn regla_dominio(linea: &str, p: &Permitidos) -> Vec<&'static str> {
    DOMINIO
        .find_iter(linea)
        .filter(|m| !dominio_permitido(&m.as_str().to_ascii_lowercase(), p))
        .map(|_| "dominio")
        .collect()
}

fn dominio_permitido(d: &str, p: &Permitidos) -> bool {
    let bajo = |padre: &str| d == padre || d.ends_with(&format!(".{padre}"));
    bajo("home.arpa")
        || ["example.com", "example.net", "example.org"]
            .into_iter()
            .any(bajo)
        || p.dominio_permitido(d)
        || p.valor_permitido(d)
}

static CORREO: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)[a-z0-9._%+-]+@(?:[a-z0-9-]+\.)+[a-z]{2,}"));

/// Direcciones de correo, salvo la de los créditos de commits y las
/// anónimas de GitHub.
pub(crate) fn regla_correo(linea: &str, p: &Permitidos) -> Vec<&'static str> {
    CORREO
        .find_iter(linea)
        .map(|m| m.as_str().to_ascii_lowercase())
        .filter(|c| {
            c != "noreply@anthropic.com"
                && !c.ends_with("@users.noreply.github.com")
                && !p.valor_permitido(c)
        })
        .map(|_| "correo")
        .collect()
}

// --- Claves y huellas -------------------------------------------------------

static HUELLA_SSH: LazyLock<Regex> = LazyLock::new(|| re(r"SHA256:[A-Za-z0-9+/]{43}"));
static CLAVE_SSH: LazyLock<Regex> = LazyLock::new(|| re(r"ssh-(?:ed25519|rsa) AAAA"));
static PEM: LazyLock<Regex> = LazyLock::new(|| re(r"-{5}BEGIN"));
static CADENA: LazyLock<Regex> = LazyLock::new(|| re(r"[A-Za-z0-9+/_-]{32,}"));

/// Huellas y claves SSH, bloques PEM y cadenas largas con aspecto de clave:
/// 32 o más hexadecimales, o 32 o más caracteres base64 (o base64url) con
/// mayúsculas, minúsculas y cifras. Más de un `_` o `-` cada diez caracteres
/// indica un identificador (`cb_x__y_z`), no datos aleatorios.
pub(crate) fn regla_clave(linea: &str, p: &Permitidos) -> Vec<&'static str> {
    let mut out = Vec::new();
    for (re, detalle) in [
        (&*HUELLA_SSH, "huella ssh"),
        (&*CLAVE_SSH, "clave ssh"),
        (&*PEM, "bloque pem"),
    ] {
        out.extend(re.find_iter(linea).map(|_| detalle));
    }
    if !out.is_empty() {
        return out;
    }
    for m in CADENA.find_iter(linea) {
        let t = m.as_str();
        if p.valor_permitido(t) {
            continue;
        }
        if es_cadena_hex(t) {
            out.push("cadena hex larga");
        } else if es_cadena_base64(t) {
            out.push("cadena base64 larga");
        }
    }
    out
}

fn separadores(t: &str) -> usize {
    t.bytes().filter(|c| matches!(c, b'_' | b'-')).count()
}

fn es_cadena_hex(t: &str) -> bool {
    t.len() - separadores(t) >= 32
        && t.bytes()
            .all(|c| c.is_ascii_hexdigit() || matches!(c, b'_' | b'-'))
}

/// Una cadena base64 o base64url aleatoria. Si mezcla los dos alfabetos
/// (`/` o `+` con `_` o `-`) no es un único valor sino una ruta o un
/// identificador, y se revisa cada tramo entre `/`.
fn es_cadena_base64(t: &str) -> bool {
    let sep = separadores(t);
    if t.contains(['/', '+']) && sep > 0 {
        return t
            .split('/')
            .any(|tramo| tramo.len() >= 32 && es_cadena_base64(tramo));
    }
    sep * 10 <= t.len()
        && t.bytes().any(|c| c.is_ascii_digit())
        && t.bytes().any(|c| c.is_ascii_uppercase())
        && t.bytes().any(|c| c.is_ascii_lowercase())
}

// --- Identificadores personales ---------------------------------------------

static GPON: LazyLock<Regex> = LazyLock::new(|| re(r"\b[A-Z]{4}[0-9A-F]{8}\b"));
static TELEFONO: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?:\+34|\b)[6789]\d{8}\b|\b[6789]\d{2} \d{3} \d{3}\b|\b[6789]\d{2} \d{2} \d{2} \d{2}\b")
});
static DNI: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\b\d{8}[A-HJ-NP-TV-Z]\b"));
static NIE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\b[XYZ]\d{7}[A-HJ-NP-TV-Z]\b"));

/// Número de serie GPON, teléfonos españoles, DNI y NIE.
pub(crate) fn regla_identificador(linea: &str, p: &Permitidos) -> Vec<&'static str> {
    let linea = separar_guiones_bajos(linea);
    let mut out = Vec::new();
    for (re, detalle) in [
        (&*GPON, "serie gpon"),
        (&*TELEFONO, "teléfono"),
        (&*DNI, "dni"),
        (&*NIE, "nie"),
    ] {
        out.extend(
            re.find_iter(&linea)
                .filter(|m| !p.valor_permitido(m.as_str()))
                .map(|_| detalle),
        );
    }
    out
}

// --- Fixtures del router ----------------------------------------------------

static HOJA: LazyLock<Regex> =
    LazyLock::new(|| re(r"<([A-Za-z_][\w.-]*)>([^<]*)</([A-Za-z_][\w.-]*)>"));

/// Elementos hoja de una fixture que no están saneados según su clase en
/// `crates/router-sim/fixtures-tags.txt` (la misma comprobación que la prueba
/// `fixtures_have_no_secrets`), o cuya etiqueta no tiene clase.
pub(crate) fn regla_fixture(linea: &str, _p: &Permitidos) -> Vec<&'static str> {
    HOJA.captures_iter(linea)
        .filter(|c| c[1] == c[3])
        .filter_map(|c| valor_sin_sanear(&c[1], c[2].trim()))
        .collect()
}

/// Los parámetros del nombre de una fixture (`cbX__clave=valor.xml`).
pub(crate) fn regla_nombre_fixture(ruta: &str) -> Vec<&'static str> {
    let nombre = ruta.rsplit('/').next().unwrap_or(ruta);
    let Some(tallo) = nombre.strip_suffix(".xml") else {
        return Vec::new();
    };
    sanitize::split_fixture_name(tallo)
        .1
        .iter()
        .filter_map(|(k, v)| valor_sin_sanear(k, v))
        .collect()
}

fn valor_sin_sanear(etiqueta: &str, valor: &str) -> Option<&'static str> {
    let Some(clase) = sanitize::tag_class(etiqueta) else {
        return Some("etiqueta sin clase");
    };
    if valor.is_empty() {
        return None;
    }
    let seudonimo = |prefijo: &str| {
        valor
            .strip_prefix(prefijo)
            .is_some_and(|n| n.parse::<u32>().is_ok())
    };
    let (saneado, detalle) = match clase {
        TagClass::Redact => (valor == sanitize::REDACTED, "redact sin sanear"),
        TagClass::Ssid => (seudonimo("RED-"), "ssid sin sanear"),
        TagClass::Host => (seudonimo("host-"), "host sin sanear"),
        TagClass::Desc => (
            seudonimo("desc-")
                || ["LAN", "Default"].contains(&valor)
                || valor.parse::<u64>().is_ok(),
            "desc sin sanear",
        ),
        TagClass::Keep => (
            valor == sanitize::REDACTED || !sanitize::is_sensitive(etiqueta, valor),
            "valor sensible sin sanear",
        ),
    };
    (!saneado).then_some(detalle)
}

// --- Binarios ---------------------------------------------------------------

const FIRMA_PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Un binario debe figurar en `permitidos` con su sha256, y las imágenes no
/// pueden llevar metadatos de texto ni EXIF (se reconocen por su firma, no
/// por la extensión).
pub(crate) fn regla_binario(ruta: &str, bytes: &[u8], p: &Permitidos) -> Vec<&'static str> {
    let mut out = Vec::new();
    let sha: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    match p.sha_binario(ruta) {
        None => out.push("binario sin revisar"),
        Some(revisado) if revisado != sha => out.push("sha256 distinto del revisado"),
        Some(_) => {}
    }
    if bytes.starts_with(FIRMA_PNG) {
        out.extend(metadatos_png(bytes));
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        out.extend(metadatos_jpeg(bytes));
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        out.extend(metadatos_webp(bytes));
    }
    out
}

fn metadatos_png(bytes: &[u8]) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut i = FIRMA_PNG.len();
    while i < bytes.len() {
        let Some(cabecera) = bytes.get(i..i + 8) else {
            return vec!["png ilegible"];
        };
        let largo = u32::from_be_bytes([cabecera[0], cabecera[1], cabecera[2], cabecera[3]]);
        let tipo = &cabecera[4..8];
        match tipo {
            b"tEXt" | b"iTXt" | b"zTXt" => out.push("png con texto"),
            b"eXIf" => out.push("png con exif"),
            b"IEND" => break,
            _ => {}
        }
        i = i
            .saturating_add(12)
            .saturating_add(usize::try_from(largo).unwrap_or(usize::MAX));
    }
    out.dedup();
    out
}

fn metadatos_jpeg(bytes: &[u8]) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut i = 2;
    while i + 4 <= bytes.len() && bytes[i] == 0xff {
        let marca = bytes[i + 1];
        if marca == 0xda || marca == 0xd9 {
            break;
        }
        let largo = usize::from(u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]));
        let datos = bytes.get(i + 4..i + 2 + largo).unwrap_or(&[]);
        match marca {
            0xe1 if datos.starts_with(b"Exif\0") => out.push("jpeg con exif"),
            0xe1 => out.push("jpeg con xmp"),
            0xfe => out.push("jpeg con comentario"),
            _ => {}
        }
        i += 2 + largo;
    }
    out.dedup();
    out
}

fn metadatos_webp(bytes: &[u8]) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut i = 12;
    while let Some(cabecera) = bytes.get(i..i + 8) {
        let largo = u32::from_le_bytes([cabecera[4], cabecera[5], cabecera[6], cabecera[7]]);
        match &cabecera[..4] {
            b"EXIF" => out.push("webp con exif"),
            b"XMP " => out.push("webp con xmp"),
            _ => {}
        }
        let largo = usize::try_from(largo).unwrap_or(usize::MAX);
        i = i.saturating_add(8).saturating_add(largo + (largo & 1));
    }
    out
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn versiones() {
        assert!(es_version("v"));
        assert!(es_version("x v"));
        assert!(!es_version("dev"));
        assert!(es_version("version = \""));
        assert!(es_version("pkg=="));
        assert!(es_version("pkg >= "));
        assert!(!es_version("RP_ROUTER="));
        assert!(!es_version("host = "));
        assert!(!es_version("server="));
    }

    #[test]
    fn ipv6_con_prefijo() {
        assert!(primera_ipv6("addr:2001:db8::1").is_some());
        assert!(primera_ipv6("12:30:45").is_none());
    }
}
