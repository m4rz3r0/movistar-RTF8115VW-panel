//! Pruebas de las listas locales (`lista-secretos`, `lista-prohibidos` y
//! `config-local`) a través del binario.
//!
//! Cada prueba aísla el entorno: `HOME` y `XDG_CONFIG_HOME` apuntan a
//! directorios temporales y la lista de prohibidos se pasa (o no) por
//! `RP_PUBLICACION_PROHIBIDOS`. Así nunca se leen las listas del dueño.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn escribir(raiz: &Path, ruta: &str, texto: &str) {
    let p = raiz.join(ruta);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, texto).unwrap();
}

struct Salida {
    codigo: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Entorno aislado: un `HOME` vacío y nada de `XDG_CONFIG_HOME` ni de
/// configuración global de git.
fn orden(casa: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_comprobar-publicacion"));
    c.env("HOME", casa)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("RP_PUBLICACION_PROHIBIDOS")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    c
}

fn ejecutar(mut c: Command) -> Salida {
    let o = c.output().unwrap();
    Salida {
        codigo: o.status.code(),
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

/// `--arbol DIR --solo RUTA` con la lista de prohibidos `prohibidos` (si la hay).
fn arbol_solo(dir: &Path, casa: &Path, prohibidos: Option<&Path>, solo: &str) -> Salida {
    let mut c = orden(casa);
    if let Some(p) = prohibidos {
        c.env("RP_PUBLICACION_PROHIBIDOS", p);
    }
    c.arg("--arbol").arg(dir).arg("--solo").arg(solo);
    ejecutar(c)
}

/// Comprueba que ninguna salida contiene `valor`, en ninguna capitalización.
fn sin_valor(s: &Salida, valor: &str) {
    let v = valor.to_lowercase();
    assert!(
        !s.stdout.to_lowercase().contains(&v),
        "stdout: {}",
        s.stdout
    );
    assert!(
        !s.stderr.to_lowercase().contains(&v),
        "stderr: {}",
        s.stderr
    );
}

fn temporales() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("arbol");
    let casa = t.path().join("casa");
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(&casa).unwrap();
    (t, dir, casa)
}

#[test]
fn lista_prohibidos_insensible_y_subdominios() {
    let (t, dir, casa) = temporales();
    let lista = t.path().join("prohibidos.txt");
    fs::write(&lista, "# comentario\n\ncasa.example.net\n").unwrap();
    escribir(&dir, "a.md", "entra en PANEL.casa.example.net ya\n");
    escribir(&dir, "b.md", "nada que ver: example.net\n");
    let s = arbol_solo(&dir, &casa, Some(&lista), "a.md");
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert!(
        s.stdout.contains("a.md:1  lista-prohibidos"),
        "{}",
        s.stdout
    );
    sin_valor(&s, "casa.example");
    let s = arbol_solo(&dir, &casa, Some(&lista), "b.md");
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
}

#[test]
fn prohibidos_desde_xdg_y_desde_home() {
    let (t, dir, casa) = temporales();
    escribir(&dir, "a.md", "texto con zafiro dentro\n");
    // `~/.config/routerpanel/…` si no hay `XDG_CONFIG_HOME`.
    escribir(
        &casa,
        ".config/routerpanel/publicacion-prohibidos.txt",
        "zafiro\n",
    );
    let s = arbol_solo(&dir, &casa, None, "a.md");
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert!(s.stdout.contains("lista-prohibidos"), "{}", s.stdout);
    // `$XDG_CONFIG_HOME/routerpanel/…` tiene prioridad sobre `~/.config`.
    let xdg = t.path().join("xdg");
    escribir(
        &xdg,
        "routerpanel/publicacion-prohibidos.txt",
        "otra-cosa\n",
    );
    let mut c = orden(&casa);
    c.env("XDG_CONFIG_HOME", &xdg)
        .arg("--arbol")
        .arg(&dir)
        .args(["--solo", "a.md"]);
    let s = ejecutar(c);
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
}

#[test]
fn entrada_corta_solo_palabra() {
    let (t, dir, casa) = temporales();
    let lista = t.path().join("prohibidos.txt");
    fs::write(&lista, "abcd\n").unwrap();
    escribir(&dir, "largo.md", "abcdef y xabcd\n");
    escribir(&dir, "suelto.md", "x abcd y\n");
    escribir(&dir, "mayus.md", "id_ABCD_2\n");
    let s = arbol_solo(&dir, &casa, Some(&lista), "largo.md");
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
    let s = arbol_solo(&dir, &casa, Some(&lista), "suelto.md");
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert!(s.stdout.contains("lista-prohibidos"), "{}", s.stdout);
    let s = arbol_solo(&dir, &casa, Some(&lista), "mayus.md");
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
}

#[test]
fn config_local_da_clave_no_valor() {
    let (_t, dir, casa) = temporales();
    escribir(
        &dir,
        "deploy/config.local",
        "# comentario\nJAIL_PANEL_IP=\"jaula-turquesa\"\nVACIA=\nexport OTRA='zocalo-ambar'\n",
    );
    escribir(&dir, "a.md", "se llama Jaula-Turquesa\n");
    escribir(&dir, "b.md", "y la otra zocalo-ambar\n");
    let s = arbol_solo(&dir, &casa, None, "a.md");
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert!(
        s.stdout
            .contains("a.md:1  config-local  [config:JAIL_PANEL_IP]"),
        "{}",
        s.stdout
    );
    sin_valor(&s, "jaula-turquesa");
    let s = arbol_solo(&dir, &casa, None, "b.md");
    assert!(s.stdout.contains("[config:OTRA]"), "{}", s.stdout);
    // Una clave vacía no hace que cualquier línea dé hallazgo.
    escribir(&dir, "c.md", "texto neutro\n");
    let s = arbol_solo(&dir, &casa, None, "c.md");
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
}

/// Las claves de ajuste que no son datos de nadie (`TRABAJOS_COMPILACION`,
/// un número de trabajos) no cuentan: si no, un «4» daría hallazgo en
/// cualquier línea con ese número. Las demás claves sí, aunque sean cortas.
#[test]
fn config_local_sin_claves_de_ajuste() {
    let (_t, dir, casa) = temporales();
    escribir(
        &dir,
        "deploy/config.local",
        "TRABAJOS_COMPILACION=4\nUID_COMPILAR=4\n",
    );
    escribir(&dir, "a.md", "con 4 trabajos\n");
    let s = arbol_solo(&dir, &casa, None, "a.md");
    assert!(
        !s.stdout.contains("[config:TRABAJOS_COMPILACION]"),
        "{}",
        s.stdout
    );
    // El mismo valor en otra clave sigue contando.
    assert!(s.stdout.contains("[config:UID_COMPILAR]"), "{}", s.stdout);
}

#[test]
fn lista_de_secretos_por_numero_de_entrada() {
    let (_t, dir, casa) = temporales();
    escribir(
        &dir,
        ".superpowers/sdd/fase-a/real-secrets.txt",
        "rubi-escondido\nperla-oculta\n",
    );
    escribir(&dir, "a.md", "aqui va PERLA-OCULTA\n");
    let s = arbol_solo(&dir, &casa, None, "a.md");
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert!(
        s.stdout.contains("a.md:1  lista-secretos  [lista:#2]"),
        "{}",
        s.stdout
    );
    sin_valor(&s, "perla-oculta");
}

#[test]
fn entrada_mac_en_cualquier_forma() {
    let (t, dir, casa) = temporales();
    let lista = t.path().join("prohibidos.txt");
    // Administrada localmente: las reglas genéricas la dejan pasar, la lista no.
    let o = [0x02u8, 0x00, 0x00, 0xaa, 0xbb, 0xcc];
    let con = |sep: &str| o.map(|n| format!("{n:02x}")).join(sep);
    fs::write(&lista, format!("{}\n", con(":"))).unwrap();
    let cisco = format!("{}.{}.{}", &con("")[..4], &con("")[4..8], &con("")[8..]);
    let formas = [
        con("-").to_uppercase(),
        con("_"),
        con(""),
        cisco.to_uppercase(),
        o.map(|n| format!("{n:x}")).join(":"),
    ];
    for (i, forma) in formas.iter().enumerate() {
        let ruta = format!("m{i}.md");
        escribir(&dir, &ruta, &format!("mac={forma};\n"));
        let s = arbol_solo(&dir, &casa, Some(&lista), &ruta);
        assert_eq!(s.codigo, Some(1), "forma {i}: {}{}", s.stdout, s.stderr);
        assert!(s.stdout.contains("lista-prohibidos"), "{}", s.stdout);
    }
}

#[test]
fn entrada_permitida_se_salta() {
    let (t, dir, casa) = temporales();
    let lista = t.path().join("prohibidos.txt");
    fs::write(&lista, "ambar-publico\n").unwrap();
    escribir(
        &dir,
        "tools/publicacion-permitidos.txt",
        "valor ambar-publico — es un nombre del proyecto\n",
    );
    escribir(&dir, "a.md", "ambar-publico\n");
    let s = arbol_solo(&dir, &casa, Some(&lista), "a.md");
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
}

#[test]
fn nombre_con_valor_de_lista_no_se_imprime() {
    let (t, dir, casa) = temporales();
    let lista = t.path().join("prohibidos.txt");
    fs::write(&lista, "zafiro\n").unwrap();
    escribir(&dir, "docs/notas-zafiro.md", "texto neutro\n");
    let s = arbol_solo(&dir, &casa, Some(&lista), "docs");
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert!(s.stdout.contains("lista-prohibidos"), "{}", s.stdout);
    sin_valor(&s, "zafiro");
}

#[test]
fn sin_listas_avisa_y_sigue() {
    let (t, dir, casa) = temporales();
    escribir(&dir, "a.md", "texto neutro\n");
    let s = arbol_solo(&dir, &casa, None, "a.md");
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
    assert!(s.stdout.is_empty(), "{}", s.stdout);
    let avisos: Vec<&str> = s
        .stderr
        .lines()
        .filter(|l| l.contains("aviso") && l.contains("se omite"))
        .collect();
    assert_eq!(avisos.len(), 3, "{}", s.stderr);
    assert!(s.stderr.contains("lista-secretos"), "{}", s.stderr);
    assert!(s.stderr.contains("lista-prohibidos"), "{}", s.stderr);
    assert!(s.stderr.contains("config-local"), "{}", s.stderr);
    // Una lista indicada que no se puede leer también avisa, sin la ruta.
    let falta = t.path().join("no-existe-zafiro.txt");
    let s = arbol_solo(&dir, &casa, Some(&falta), "a.md");
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
    sin_valor(&s, "zafiro");
    // `--sin-locales` (CI): ni listas ni avisos.
    let mut c = orden(&casa);
    c.arg("--arbol")
        .arg(&dir)
        .args(["--solo", "a.md", "--sin-locales"]);
    let s = ejecutar(c);
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
    assert!(!s.stderr.contains("se omite"), "{}", s.stderr);
}

#[test]
fn sin_locales_no_usa_las_listas() {
    let (t, dir, casa) = temporales();
    let lista = t.path().join("prohibidos.txt");
    fs::write(&lista, "zafiro\n").unwrap();
    escribir(&dir, "a.md", "zafiro\n");
    let mut c = orden(&casa);
    c.env("RP_PUBLICACION_PROHIBIDOS", &lista)
        .args(["--arbol", "--sin-locales"])
        .arg(&dir)
        .args(["--solo", "a.md"]);
    // `--sin-locales` puede ir antes o después del directorio.
    let s = ejecutar(c);
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
}

#[test]
fn listas_explicitas_por_opcion_y_por_entorno() {
    let (t, dir, casa) = temporales();
    let otra = t.path().join("listas");
    escribir(
        &otra,
        ".superpowers/sdd/x/real-secrets.txt",
        "rubi-escondido\n",
    );
    escribir(&dir, "a.md", "rubi-escondido\n");
    // Sin indicar dónde están, no se encuentran.
    let s = arbol_solo(&dir, &casa, None, "a.md");
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
    let mut c = orden(&casa);
    c.arg("--arbol")
        .arg(&dir)
        .args(["--listas"])
        .arg(&otra)
        .args(["--solo", "a.md"]);
    let s = ejecutar(c);
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert!(s.stdout.contains("lista-secretos"), "{}", s.stdout);
    let mut c = orden(&casa);
    c.env("RP_PUBLICACION_LISTAS", &otra)
        .arg("--arbol")
        .arg(&dir)
        .args(["--solo", "a.md"]);
    let s = ejecutar(c);
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    sin_valor(&s, "rubi");
}

#[test]
fn exigir_listas_falla_si_falta_alguna() {
    let (t, dir, casa) = temporales();
    escribir(&dir, "a.md", "texto neutro\n");
    let exigir = |prohibidos: Option<&Path>| {
        let mut c = orden(&casa);
        if let Some(p) = prohibidos {
            c.env("RP_PUBLICACION_PROHIBIDOS", p);
        }
        c.arg("--arbol")
            .arg(&dir)
            .args(["--exigir-listas", "--solo", "a.md"]);
        ejecutar(c)
    };
    // Faltan los secretos y config.local.
    let s = exigir(None);
    assert_eq!(s.codigo, Some(2), "{}{}", s.stdout, s.stderr);
    assert!(s.stderr.contains("lista-secretos"), "{}", s.stderr);
    escribir(
        &dir,
        ".superpowers/sdd/x/real-secrets.txt",
        "rubi-escondido\n",
    );
    let s = exigir(None);
    assert_eq!(s.codigo, Some(2), "{}{}", s.stdout, s.stderr);
    assert!(s.stderr.contains("config-local"), "{}", s.stderr);
    escribir(&dir, "deploy/config.local", "CLAVE=jaula-turquesa\n");
    // Falta la lista de prohibidos en su sitio por defecto: también es un error.
    let s = exigir(None);
    assert_eq!(s.codigo, Some(2), "{}{}", s.stdout, s.stderr);
    assert!(s.stderr.contains("lista-prohibidos"), "{}", s.stderr);
    escribir(
        &casa,
        ".config/routerpanel/publicacion-prohibidos.txt",
        "zafiro\n",
    );
    let s = exigir(None);
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
    // Una lista de prohibidos indicada que no se puede leer también falla.
    let s = exigir(Some(&t.path().join("no-existe.txt")));
    assert_eq!(s.codigo, Some(2), "{}{}", s.stdout, s.stderr);
    assert!(s.stderr.contains("lista-prohibidos"), "{}", s.stderr);
}

#[test]
fn entrada_con_barra_se_busca_en_la_ruta_entera() {
    let (t, dir, casa) = temporales();
    let lista = t.path().join("prohibidos.txt");
    fs::write(&lista, "carpeta/zafiro-x\n").unwrap();
    escribir(&dir, "carpeta/zafiro-x/a.txt", "texto neutro\n");
    let s = arbol_solo(&dir, &casa, Some(&lista), "carpeta");
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert!(s.stdout.contains("lista-prohibidos"), "{}", s.stdout);
    assert_eq!(s.stdout.lines().count(), 1, "{}", s.stdout);
    sin_valor(&s, "zafiro");
}
