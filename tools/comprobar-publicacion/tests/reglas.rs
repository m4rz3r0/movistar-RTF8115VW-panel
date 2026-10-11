//! Pruebas de las reglas genéricas sobre textos y binarios sueltos.
//!
//! Los valores que deben dar hallazgo se componen en tiempo de ejecución: este
//! fichero también pasa por el comprobador y no puede llevarlos escritos.

use comprobar_publicacion::{
    Hallazgo, ListasLocales, Permitidos, Regla, revisar_binario, revisar_texto,
};

fn ninguno() -> Permitidos {
    Permitidos::leer("").unwrap()
}

fn revisar(ruta: &str, texto: &str) -> Vec<Hallazgo> {
    revisar_texto(ruta, texto, &ninguno(), &ListasLocales::vacia())
}

fn reglas(ruta: &str, texto: &str) -> Vec<Regla> {
    revisar(ruta, texto).into_iter().map(|h| h.regla).collect()
}

/// Dirección IPv4 a partir de sus octetos.
fn ip(o: [u8; 4]) -> String {
    o.map(|n| n.to_string()).join(".")
}

/// MAC con el separador dado, en minúsculas o mayúsculas.
fn mac(o: [u8; 6], sep: &str, mayus: bool) -> String {
    let s = o.map(|n| format!("{n:02x}")).join(sep);
    if mayus { s.to_ascii_uppercase() } else { s }
}

const MAC_REAL: [u8; 6] = [0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e];

#[test]
fn ipv4_privada_es_hallazgo() {
    let texto = format!("host {}", ip([10, 1, 2, 3]));
    let h = revisar("docs/x.md", &texto);
    assert_eq!(h.len(), 1, "{h:?}");
    assert_eq!(h[0].regla, Regla::Ipv4);
    assert_eq!(h[0].linea, 1);
    for permitida in ["192.0.2.7", "127.0.0.1", "255.255.255.0", "192.168.1.1"] {
        assert!(
            reglas("docs/x.md", &format!("ip {permitida}")).is_empty(),
            "{permitida}"
        );
    }
    for otra in [
        ip([172, 16, 0, 1]),
        ip([192, 168, 7, 2]),
        ip([100, 64, 0, 9]),
        ip([8, 8, 4, 4]),
    ] {
        assert_eq!(reglas("a", &otra), [Regla::Ipv4], "{otra}");
    }
}

#[test]
fn ip_en_url_y_con_puerto() {
    let url = format!("http://{}:8080/x", ip([172, 20, 0, 9]));
    assert_eq!(reglas("a", &url), [Regla::Ipv4]);
    assert!(reglas("Cargo.toml", "version = \"1.2.3.4\"").is_empty());
    assert!(reglas("a", "suelta v1.2.3.4").is_empty());
    // Un prefijo de letras que no es una versión no la oculta.
    assert_eq!(
        reglas("a", &format!("host{}", ip([10, 3, 3, 3]))),
        [Regla::Ipv4]
    );
    assert_eq!(
        reglas("a", &format!("dev{}", ip([10, 3, 3, 3]))),
        [Regla::Ipv4]
    );
    assert!(reglas("a", "oid 1.3.6.1.4.1.9").is_empty());
    // Una asignación cualquiera no es un número de versión.
    let asignacion = format!("RP_ROUTER={}", ip([10, 0, 0, 5]));
    assert_eq!(reglas("a", &asignacion), [Regla::Ipv4]);
    // Octetos con ceros a la izquierda.
    let ceros = ["010", "001", "002", "003"].join(".");
    assert_eq!(reglas("a", &format!("gw {ceros}")), [Regla::Ipv4]);
}

#[test]
fn ipv6_global() {
    let global = format!("addr:{}:{}::1/64", "2a01", "4f8");
    assert_eq!(reglas("a", &global), [Regla::Ipv6]);
    assert!(reglas("a", "doc 2001:db8::1 y fd00::1 y fe80::1").is_empty());
}

#[test]
fn mac_en_todas_las_formas() {
    for forma in [
        mac(MAC_REAL, ":", true),
        mac(MAC_REAL, "-", false),
        mac(MAC_REAL, "", false),
    ] {
        let h = revisar("a", &format!("equipo {forma} fin"));
        assert_eq!(h.len(), 1, "{h:?}");
        assert_eq!(h[0].regla, Regla::Mac);
    }
    for valida in [
        "02:00:00:00:00:01",
        "00:00:00:00:00:00",
        "ff:ff:ff:ff:ff:ff",
        "00:00:5e:00:53:2a",
    ] {
        assert!(reglas("a", valida).is_empty(), "{valida}");
    }
    // Un guion bajo delante no la oculta.
    let pegada = format!("mac_{}", mac(MAC_REAL, "", false));
    assert_eq!(reglas("a", &pegada), [Regla::Mac]);
    // Un contador decimal de 12 cifras no es una MAC.
    assert!(reglas("a", "bytes 123456789012").is_empty());
}

#[test]
fn dominio_real_y_documental() {
    let real = format!("Mi.Dominio.{}", "NET");
    assert_eq!(reglas("a", &real), [Regla::Dominio]);
    // Un campo de Rust seguido de `_` no es un dominio.
    assert!(reglas("a", "if m.es_valido() {}").is_empty());
    for valido in [
        "panel.example",
        "nas.home.arpa",
        "www.example.com",
        "localhost",
    ] {
        assert!(reglas("a", valido).is_empty(), "{valido}");
    }
}

#[test]
fn permitidos_valen() {
    let p = Permitidos::leer(&format!(
        "# comentario\n\
         \n\
         valor 9.9.9.9 — DNS público\n\
         red {}/15 — red de pruebas de rendimiento\n\
         dominio crates.io — registro de Rust\n\
         ruta docs/** ipv4 — documentación con ejemplos\n",
        ip([198, 18, 0, 0])
    ))
    .unwrap();
    let l = ListasLocales::vacia();
    assert!(revisar_texto("a", "dns 9.9.9.9", &p, &l).is_empty());
    let banco = ip([198, 19, 1, 2]);
    assert_eq!(reglas("a", &banco), [Regla::Ipv4]);
    assert!(revisar_texto("a", &banco, &p, &l).is_empty());
    assert!(revisar_texto("a", "https://static.crates.io/x", &p, &l).is_empty());
    let privada = ip([10, 4, 4, 4]);
    assert!(revisar_texto("docs/sub/x.md", &privada, &p, &l).is_empty());
    assert_eq!(revisar_texto("src/x.rs", &privada, &p, &l).len(), 1);
}

#[test]
fn correo_ajeno() {
    let ajeno = format!("{}@{}", "persona", "correo.example");
    assert_eq!(reglas("a", &ajeno), [Regla::Correo]);
    // El dominio de estas sí pasa por la regla de dominios (y `permitidos`).
    for valido in [
        "Co-Authored-By: Claude <noreply@anthropic.com>",
        "1234+alguien@users.noreply.github.com",
    ] {
        assert!(!reglas("a", valido).contains(&Regla::Correo), "{valido}");
    }
}

#[test]
fn claves_y_huellas() {
    let huella = format!("SHA256:{}", "Ab1+".repeat(11).get(..43).unwrap());
    let ssh = format!("ssh-ed25519 {}C3NzaC1lZDI1NTE5", "AAAA");
    let pem = format!(
        "{}BEGIN OPENSSH PRIVATE KEY{}",
        "-".repeat(5),
        "-".repeat(5)
    );
    let hex = sha256(b"clave");
    let b64 = azar(40);
    for texto in [huella, ssh, pem, hex, b64] {
        assert_eq!(reglas("a", &texto), [Regla::Clave], "{texto}");
    }
    // Con separadores de base64url sueltos sigue siendo una clave.
    let url = format!("{}_{}", azar(20), azar(21));
    assert_eq!(reglas("a", &url), [Regla::Clave]);
    // Rutas, identificadores y palabras largas no son claves.
    assert!(reglas("a", "crates/panel/public/fonts/jetbrains-mono").is_empty());
    assert!(reglas("a", "configuracionDelServidorDeNombresPrincipal").is_empty());
    assert!(reglas("a", "cbGetWifiAccPt__dpm_cfg_wifi_acc_pt_0i").is_empty());
}

#[test]
fn identificadores_gpon_y_telefono() {
    let gpon = format!("serie SN_{}{}", "ASKY", "0123ABCD");
    assert_eq!(reglas("a", &gpon), [Regla::Identificador]);
    let movil = format!("tel +34{}{}", "6000", "00000");
    assert_eq!(reglas("a", &movil), [Regla::Identificador]);
    let espacios = format!("tel {} {} {}", "612", "345", "678");
    assert_eq!(reglas("a", &espacios), [Regla::Identificador]);
    // DNI y NIE con letra de control válida.
    let dni = format!("dni {}Z", "12345678");
    assert_eq!(reglas("a", &dni), [Regla::Identificador]);
    let nie = format!("nie X{}L", "1234567");
    assert_eq!(reglas("a", &nie), [Regla::Identificador]);
}

#[test]
fn ficheros_de_bloqueo_solo_correo() {
    let hex = &sha256(b"bloqueo");
    let lock = format!("checksum = \"{hex}\"\nsource = \"{}\"", ip([10, 1, 1, 1]));
    assert!(reglas("Cargo.lock", &lock).is_empty());
    assert!(reglas("e2e/package-lock.json", &lock).is_empty());
    let correo = format!("{}@{}", "persona", "correo.example");
    assert_eq!(reglas("Cargo.lock", &correo), [Regla::Correo]);
    assert_eq!(reglas("otro/Cargo.toml", hex), [Regla::Clave]);
}

#[test]
fn fixture_con_secreto() {
    let xml = "<Response><password>abc</password></Response>";
    assert_eq!(reglas("crates/hgu/fixtures/cbX.xml", xml), [Regla::Fixture]);
    let saneada = "<Response><password>X-REDACTED-X</password><password></password></Response>";
    assert!(reglas("crates/hgu/fixtures/cbX.xml", saneada).is_empty());
    // Una etiqueta sin clase en fixtures-tags.txt también es hallazgo.
    let nueva = "<Response><etiqueta_nueva_sin_revisar>1</etiqueta_nueva_sin_revisar></Response>";
    assert_eq!(
        reglas("crates/hgu/fixtures-v2/a.xml", nueva),
        [Regla::Fixture]
    );
    // Fuera de las fixtures no se aplica.
    assert!(reglas("docs/api.md", xml).is_empty());
}

#[test]
fn permitido_sin_motivo_es_error() {
    assert!(Permitidos::leer("valor 9.9.9.9").is_err());
    assert!(Permitidos::leer("valor 9.9.9.9 —   ").is_err());
    assert!(Permitidos::leer("red 192.0.2.0/33 — mal").is_err());
    assert!(Permitidos::leer("ruta docs/** inventada — mal").is_err());
    assert!(Permitidos::leer("binario a.png 1234 — sha corto").is_err());
    assert!(Permitidos::leer("otro x — tipo desconocido").is_err());
    assert!(Permitidos::leer("valor 9.9.9.9 — DNS público").is_ok());
}

/// PNG de 1×1 con un fragmento de texto opcional.
fn png(texto: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut bytes, 1, 1);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Eight);
        if texto {
            enc.add_text_chunk("Comment".into(), "hecho en casa".into())
                .unwrap();
        }
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[0]).unwrap();
    }
    bytes
}

/// Cadena base64 seudoaleatoria, determinista.
fn azar(n: usize) -> String {
    let alfabeto: Vec<u8> = (b'A'..=b'Z')
        .chain(b'a'..=b'z')
        .chain(b'0'..=b'9')
        .chain(*b"+/")
        .collect();
    let mut x: u32 = 12345;
    (0..n)
        .map(|_| {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
            alfabeto[((x >> 16) % 64) as usize] as char
        })
        .collect()
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn png_con_texto_es_hallazgo() {
    let con = png(true);
    let p = Permitidos::leer(&format!("binario img/a.png {} — revisada", sha256(&con))).unwrap();
    let h = revisar_binario("img/a.png", &con, &p);
    assert_eq!(h.len(), 1, "{h:?}");
    assert_eq!(h[0].regla, Regla::Binario);

    let sin = png(false);
    let p = Permitidos::leer(&format!("binario img/a.png {} — revisada", sha256(&sin))).unwrap();
    assert!(revisar_binario("img/a.png", &sin, &p).is_empty());
    // Sin entrada en permitidos, o con otra suma, es hallazgo.
    assert_eq!(revisar_binario("img/b.png", &sin, &p).len(), 1);
    assert_eq!(revisar_binario("img/a.png", &con, &p).len(), 2);
}

#[test]
fn display_sin_valor() {
    let texto = format!("{} {}", ip([10, 9, 8, 7]), mac(MAC_REAL, ":", false));
    for h in revisar("docs/x.md", &texto) {
        let s = h.to_string();
        assert!(s.starts_with("docs/x.md:1  "), "{s}");
        assert!(!s.contains("10.9"), "{s}");
        assert!(!s.contains("1a"), "{s}");
    }
}
