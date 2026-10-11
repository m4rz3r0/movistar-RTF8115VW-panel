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

// --- Ronda de correcciones 1 ------------------------------------------------

/// IPv6 global sintética (no de documentación).
fn v6_global() -> String {
    format!("{}:{}:1:2::3", "2a01", "4f8")
}

#[test]
fn ipv6_tras_cualquier_prefijo() {
    for prefijo in ["v=spf1 ip6:", "ipv6:", "face:", "6:"] {
        let texto = format!("{prefijo}{} -all", v6_global());
        assert_eq!(reglas("a", &texto), [Regla::Ipv6], "{prefijo}");
    }
}

#[test]
fn mac_con_mas_separadores() {
    for forma in [
        mac(MAC_REAL, "_", true),
        mac(MAC_REAL, " ", false),
        // `arp` de BSD y macOS: grupos de una cifra.
        format!("0:{}", mac(MAC_REAL, ":", false).split_at(3).1),
        // Solo cifras, pero con separadores: sigue siendo una MAC.
        mac([0x00, 0x11, 0x22, 0x33, 0x44, 0x55], ":", false),
    ] {
        assert_eq!(
            reglas("a", &format!("x {forma} y")),
            [Regla::Mac],
            "{forma}"
        );
    }
    // Una lista de números separados por espacios no es una MAC.
    assert!(reglas("a", "10 20 30 40 50 60").is_empty());
}

#[test]
fn identificadores_con_separadores() {
    let n8 = "12345678";
    let n7 = "1234567";
    let casos = [
        format!("{n8}-Z"),
        format!("{}.{}.{}-Z", &n8[..2], &n8[2..5], &n8[5..]),
        format!("X-{n7}-L"),
        format!("{}-{}-{}", "612", "345", "678"),
        format!("{}.{}.{}", "612", "345", "678"),
        format!("{} {}", "612", "345678"),
        format!("{}-{}-{}-{}", "612", "34", "56", "78"),
        format!("00{}{}{}", "34", "6123", "45678"),
        format!("{}{}", "asky", "0123abcd"),
        // Serie GPON en hexadecimal: el fabricante en ASCII («ASKY»).
        format!("{}{}", "41534B59", "0123ABCD"),
    ];
    // Las MAC especiales sin separador no son series GPON.
    assert!(reglas("a", &"f".repeat(12)).is_empty());
    for caso in casos {
        assert_eq!(
            reglas("a", &format!("dato {caso} fin")),
            [Regla::Identificador],
            "{caso}"
        );
    }
}

#[test]
fn codificaciones_y_cifras_unicode() {
    let casos = [
        format!("ip 10{0}1{0}2{0}3", "%2E"),
        format!("ip 10{0}1{0}2{0}3", "&#46;"),
        format!("ip 10{0}1{0}2{0}3", "&#x2e;"),
        // Cifras de ancho completo.
        "ip \u{FF11}\u{FF10}.\u{FF11}.\u{FF12}.\u{FF13}".to_owned(),
        // Cifras arábigo-índicas.
        "ip \u{0661}\u{0660}.\u{0661}.\u{0662}.\u{0663}".to_owned(),
    ];
    for caso in casos {
        assert_eq!(reglas("a", &caso), [Regla::Ipv4], "{caso}");
    }
    let dominio = format!("mi{0}dominio{0}net", "&#46;");
    assert_eq!(reglas("a", &dominio), [Regla::Dominio]);
}

#[test]
fn ruta_no_desactiva_reglas_de_seguridad() {
    for regla in [
        "lista-secretos",
        "lista-prohibidos",
        "config-local",
        "ruta-prohibida",
    ] {
        let linea = format!("ruta ** {regla} — no");
        assert!(Permitidos::leer(&linea).is_err(), "{regla}");
    }
    assert!(Permitidos::leer("ruta docs/** ipv4 — ejemplos").is_ok());
}

// --- Ronda de correcciones 2 ------------------------------------------------

#[test]
fn mac_tras_palabra_hexadecimal() {
    for forma in [
        format!("MAC {}", mac(MAC_REAL, " ", true)),
        format!("mac:{}", mac(MAC_REAL, ":", false)),
        format!("mac-{}", mac(MAC_REAL, "-", false)),
        format!("mac_{}", mac(MAC_REAL, "_", false)),
        format!("mac:0:{}", mac(MAC_REAL, ":", false).split_at(3).1),
    ] {
        assert_eq!(reglas("a", &forma), [Regla::Mac], "{forma}");
    }
    // Una cola de IPv6 con grupos de una cifra no es una MAC.
    assert!(reglas("a", "fe80::1:2:3:4:5:6").is_empty());
}

#[test]
fn mac_formas_hp_y_cisco() {
    let plano = mac(MAC_REAL, "", false);
    let hp = format!("{}-{}", &plano[..6], &plano[6..]);
    let cisco = format!("{}.{}.{}", &plano[..4], &plano[4..8], &plano[8..]);
    for forma in [hp, cisco] {
        assert_eq!(
            reglas("a", &format!("eth {forma} up")),
            [Regla::Mac],
            "{forma}"
        );
    }
}

#[test]
fn clave_tras_identificador_con_separadores() {
    let texto = format!("{}_{}", "x_y_z_w_v_u_t_s", azar(35));
    assert_eq!(reglas("a", &texto), [Regla::Clave]);
}

#[test]
fn gpon_sin_falso_positivo_de_fecha() {
    assert!(reglas("a", &format!("{}{}", "data", "20261011")).is_empty());
    let serie = format!("{}{}", "HWTC", "12345678");
    assert_eq!(reglas("a", &serie), [Regla::Identificador]);
}

// --- Tarea 3: arrastrados de la Tarea 2 --------------------------------------

#[test]
fn mac_permitida_no_tapa_una_real_que_la_solapa() {
    let real = mac(MAC_REAL, " ", false);
    // `ff 00 1a 2b 3c 4d` (con el bit local) solapa la MAC real que empieza
    // un grupo después.
    let volcado = format!("ff {real} 6f");
    assert_eq!(reglas("a", &volcado), [Regla::Mac], "{volcado}");
    let tras_local = format!("pkt aa:{}", mac(MAC_REAL, ":", false));
    assert_eq!(reglas("a", &tras_local), [Regla::Mac], "{tras_local}");
}

#[test]
fn clave_con_token_enorme_es_rapida() {
    let token = "Zz_".repeat(40_000);
    let inicio = std::time::Instant::now();
    let h = revisar("a", &token);
    let tiempo = inicio.elapsed();
    assert!(h.is_empty(), "{h:?}");
    // Con el coste cuadrático anterior tardaba más de dos minutos en debug;
    // el límite es holgado para no fallar en una máquina cargada.
    assert!(tiempo.as_secs() < 10, "{tiempo:?}");
    // La clave al final de un token así se sigue viendo.
    let con_clave = format!("{token}{}", azar_url(40));
    assert_eq!(reglas("a", &con_clave), [Regla::Clave]);
}

#[test]
fn clave_con_mas_y_guion_bajo_no_revienta() {
    // `+` y `_` sin `/`: antes se revisaba a sí misma sin fin.
    let token = format!("{}+{}_{}", azar_url(20), azar_url(20), azar_url(20));
    let inicio = std::time::Instant::now();
    let _ = reglas("a", &token);
    assert!(inicio.elapsed().as_secs() < 10);
    let largo = format!("abc+def_{}", azar_url(40));
    assert_eq!(reglas("a", &largo), [Regla::Clave]);
}

/// Como [`azar`], sin `/` ni `+` (base64url sin separadores).
fn azar_url(n: usize) -> String {
    azar(n).replace(['/', '+'], "Q")
}

#[test]
fn correo_anonimo_de_github_no_da_dominio() {
    assert!(reglas("a", "Ana <123+ana@users.noreply.github.com>").is_empty());
    assert_eq!(reglas("a", "visita github.com"), [Regla::Dominio]);
}

#[test]
fn binario_con_varios_sha_revisados() {
    let a = png(false);
    let mut b = png(false);
    b.extend_from_slice(b"otra version");
    let mut c = png(false);
    c.extend_from_slice(b"sin revisar");
    let p = Permitidos::leer(&format!(
        "binario docs/x.png {} — v1\nbinario docs/x.png {} — v2\n",
        sha256(&a),
        sha256(&b)
    ))
    .unwrap();
    assert!(revisar_binario("docs/x.png", &a, &p).is_empty());
    assert!(revisar_binario("docs/x.png", &b, &p).is_empty());
    assert_eq!(revisar_binario("docs/x.png", &c, &p).len(), 1);
}
