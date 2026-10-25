//! Las fixtures saneadas no llevan secretos ni direcciones reales.

use super::*;

static IPV4: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b\d{1,3}(?:\.\d{1,3}){3}\b").unwrap());

/// Valores reales que no deben aparecer, de `$RTF_SECRETS_FILE` (uno por línea).
///
/// Lo genera `tools/sanear-fixtures.sh` con los valores retirados de las
/// capturas, y se le pueden añadir a mano valores conocidos por otra vía que no
/// salen en las capturas (contraseña del router, claves WiFi…): la herramienta
/// conserva las líneas existentes al regenerarlo.
fn real_secrets() -> HashSet<String> {
    match std::env::var_os("RTF_SECRETS_FILE") {
        Some(path) => std::fs::read_to_string(path)
            .expect("RTF_SECRETS_FILE legible")
            .lines()
            .filter(|l| l.len() >= 4)
            .map(str::to_owned)
            .collect(),
        None => HashSet::new(),
    }
}

/// Valores inventados que una variante pone a propósito en un secreto, para
/// probar que no salen del sitio: (fixture, etiqueta, valor). Valen en lugar
/// de [`sanitize::REDACTED`] solo en esa fixture de
/// `fixtures-variantes/conexion-secretos` y en esa etiqueta.
const VALORES_DE_PRUEBA: [(&str, &str, &str); 2] = [
    (
        "conexion-secretos/cbGetWanIPIntf.xml",
        "password",
        "clave-ppp-de-prueba",
    ),
    (
        "conexion-secretos/cbGetGPONInfo.xml",
        "GponSerialNumber",
        "SERIE-DE-PRUEBA",
    ),
];

/// Comprueba un valor de la fixture `file` (de un elemento o de un parámetro
/// del nombre del fichero; `<variante>/<fichero>` en una variante).
fn check_value(file: &str, tag: &str, value: &str, secrets: &HashSet<String>) {
    use sanitize::TagClass;
    let class = sanitize::tag_class(tag)
        .unwrap_or_else(|| panic!("{file}: <{tag}> no está en fixtures-tags.txt"));
    assert!(
        !secrets.contains(value),
        "{file}: <{tag}> contiene un secreto real"
    );
    assert!(
        !secrets
            .iter()
            .any(|s| s.len() >= 8 && value.contains(s.as_str())),
        "{file}: <{tag}> contiene un secreto real dentro del valor"
    );
    if value.is_empty() {
        return;
    }
    let pseudo = |prefix: &str| {
        value
            .strip_prefix(prefix)
            .is_some_and(|n| n.parse::<u32>().is_ok())
    };
    match class {
        TagClass::Redact if VALORES_DE_PRUEBA.contains(&(file, tag, value)) => {}
        TagClass::Redact => assert_eq!(value, sanitize::REDACTED, "{file}: <{tag}> sin sanear"),
        TagClass::Ssid => assert!(pseudo("RED-"), "{file}: <{tag}> sin sanear"),
        TagClass::Host => assert!(pseudo("host-"), "{file}: <{tag}> sin sanear"),
        TagClass::Desc => assert!(
            pseudo("desc-") || ["LAN", "Default"].contains(&value) || value.parse::<u64>().is_ok(),
            "{file}: <{tag}> sin sanear"
        ),
        TagClass::Keep => assert!(
            value == sanitize::REDACTED || !sanitize::is_sensitive(tag, value),
            "{file}: <{tag}> sin sanear"
        ),
    }
    for mac in sanitize::macs_in(tag, value) {
        assert!(
            sanitize::is_fake_mac(&mac),
            "{file}: <{tag}> contiene una MAC real"
        );
    }
    assert!(
        sanitize::global_ipv6_in(value).is_empty(),
        "{file}: <{tag}> contiene una IPv6 global"
    );
    for m in IPV4.find_iter(value) {
        if let Ok(ip) = m.as_str().parse::<Ipv4Addr>() {
            assert!(!sanitize::is_public_ipv4(ip), "{file}: <{tag}> IP pública");
        }
    }
}

#[test]
fn fixtures_have_no_secrets() {
    let secrets = real_secrets();
    // Las de serie y las de cada variante (`fixtures-variantes/*/`).
    let mut dirs = vec![fixtures_dir()];
    let variantes = variantes_dir();
    for entry in std::fs::read_dir(&variantes).expect("directorio de variantes") {
        let path = entry.unwrap().path();
        assert!(
            path.is_dir(),
            "{}: se esperaba un directorio",
            path.display()
        );
        dirs.push(path);
    }
    assert!(dirs.len() > 1, "faltan las variantes");
    let mut files = 0;
    for dir in dirs {
        let variante = dir
            .strip_prefix(&variantes)
            .ok()
            .map(|d| d.to_string_lossy().into_owned());
        for entry in std::fs::read_dir(&dir).expect("directorio de fixtures") {
            let path = entry.unwrap().path();
            let file = path.file_name().unwrap().to_string_lossy().into_owned();
            let Some(stem) = file.strip_suffix(".xml") else {
                continue;
            };
            let name = match &variante {
                Some(v) => format!("{v}/{file}"),
                None => file.clone(),
            };
            files += 1;
            for (k, v) in sanitize::split_fixture_name(stem).1 {
                check_value(&name, &k, &v, &secrets);
            }
            let raw = std::fs::read_to_string(&path).unwrap();
            for (tag, value) in leaves(&raw) {
                check_value(&name, &tag, &value, &secrets);
            }
        }
    }
    assert!(files > 50, "faltan fixtures ({files})");
}

/// Los valores de prueba valen solo en su fixture y su etiqueta.
#[test]
fn valores_de_prueba_solo_en_su_sitio() {
    let ninguno = HashSet::new();
    let (file, tag, value) = VALORES_DE_PRUEBA[0];
    check_value(file, tag, value, &ninguno);
    for (f, t) in [
        ("cbGetWanIPIntf.xml", tag),
        ("dhcp-sin-voip/cbGetWanIPIntf.xml", tag),
        (file, "username"),
    ] {
        let r = std::panic::catch_unwind(|| check_value(f, t, value, &ninguno));
        assert!(r.is_err(), "{f} <{t}> admite el valor de prueba");
    }
}

/// Cada entrada de la caché ARP tiene su MAC: una MAC repetida en varias IP
/// haría pasar a un dispositivo por varios.
#[test]
fn arp_sin_mac_repetida() {
    let raw = std::fs::read_to_string(fixtures_dir().join("cbGetArpCache.xml")).unwrap();
    let doc = roxmltree::Document::parse(&raw).expect("fixture XML válida");
    let registros = doc
        .descendants()
        .filter(|n| n.has_tag_name("arp_record"))
        .count();
    let macs: HashSet<&str> = doc
        .descendants()
        .filter(|n| n.has_tag_name("hwaddr"))
        .filter_map(|n| n.text())
        .collect();
    assert!(registros > 1, "faltan entradas ARP");
    assert_eq!(macs.len(), registros, "MAC repetida en la caché ARP");
}
