//! Pruebas del modo `--arbol` y del binario.

use comprobar_publicacion::{ListasLocales, Permitidos, Regla, arbol};
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

fn obligatorios(raiz: &Path) {
    for f in [
        "LICENSE",
        "AVISO-LEGAL.md",
        "README.md",
        "README.en.md",
        "deploy/config.example",
    ] {
        escribir(raiz, f, "texto neutro\n");
    }
}

fn arbol_de(raiz: &Path, solo: &[PathBuf]) -> Vec<comprobar_publicacion::Hallazgo> {
    arbol(
        raiz,
        solo,
        &Permitidos::leer("").unwrap(),
        &ListasLocales::vacia(),
    )
}

#[test]
fn rutas_prohibidas_y_obligatorias() {
    let dir = tempfile::tempdir().unwrap();
    escribir(dir.path(), ".superpowers/x", "nada\n");
    escribir(dir.path(), "datos/panel.db", "nada\n");
    escribir(dir.path(), "README.md", "nada\n");
    let h = arbol_de(dir.path(), &[]);
    let prohibidas: Vec<&str> = h
        .iter()
        .filter(|h| h.regla == Regla::RutaProhibida)
        .map(|h| h.ruta.as_str())
        .collect();
    assert_eq!(prohibidas, [".superpowers/x", "datos/panel.db"], "{h:?}");
    let faltan: Vec<&str> = h
        .iter()
        .filter(|h| h.regla == Regla::FaltaFichero)
        .map(|h| h.ruta.as_str())
        .collect();
    assert_eq!(
        faltan,
        [
            "AVISO-LEGAL.md",
            "LICENSE",
            "README.en.md",
            "deploy/config.example"
        ]
    );
}

#[test]
fn arbol_limpio_no_da_nada() {
    let dir = tempfile::tempdir().unwrap();
    obligatorios(dir.path());
    escribir(dir.path(), "src/main.rs", "fn main() {} // 192.0.2.1\n");
    assert!(arbol_de(dir.path(), &[]).is_empty());
}

#[test]
fn solo_limita_y_no_pide_obligatorios() {
    let dir = tempfile::tempdir().unwrap();
    let privada = ["10", "2", "3", "4"].join(".");
    escribir(dir.path(), "a/x.txt", &format!("ip {privada}\n"));
    escribir(dir.path(), "b/y.txt", &format!("ip {privada}\n"));
    let h = arbol_de(dir.path(), &[PathBuf::from("./a/")]);
    assert_eq!(h.len(), 1, "{h:?}");
    assert_eq!(h[0].ruta, "a/x.txt");
    assert_eq!(h[0].regla, Regla::Ipv4);
}

#[test]
fn binario_sin_extension_conocida() {
    let dir = tempfile::tempdir().unwrap();
    obligatorios(dir.path());
    fs::write(dir.path().join("volcado.bin"), [0u8, 1, 2, 0xff, 0xfe]).unwrap();
    let h = arbol_de(dir.path(), &[]);
    assert_eq!(h.len(), 1, "{h:?}");
    assert_eq!(h[0].regla, Regla::Binario);
}

#[test]
fn nombre_de_fichero_con_dato_no_se_imprime() {
    let dir = tempfile::tempdir().unwrap();
    let privada = ["10", "7", "7", "7"].join(".");
    escribir(dir.path(), &format!("notas-{privada}.txt"), "nada\n");
    let h = arbol_de(dir.path(), &[]);
    let ip: Vec<_> = h.iter().filter(|h| h.regla == Regla::Ipv4).collect();
    assert_eq!(ip.len(), 1, "{h:?}");
    assert!(!ip[0].to_string().contains(&privada), "{}", ip[0]);
}

#[test]
fn usa_git_ls_files_en_un_repositorio() {
    let dir = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(args)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?}");
    };
    git(&["init", "-q"]);
    obligatorios(dir.path());
    escribir(dir.path(), ".gitignore", "/ignorado/\n");
    let privada = ["10", "5", "5", "5"].join(".");
    escribir(dir.path(), "ignorado/x.txt", &format!("{privada}\n"));
    escribir(dir.path(), "nuevo.txt", &format!("{privada}\n"));
    let h = arbol_de(dir.path(), &[]);
    // El ignorado no cuenta; el nuevo sin añadir sí, porque se publicaría.
    assert_eq!(h.len(), 1, "{h:?}");
    assert_eq!(h[0].ruta, "nuevo.txt");
}

#[test]
fn nunca_imprime_el_valor() {
    let dir = tempfile::tempdir().unwrap();
    let privada = ["10", "9", "8", "7"].join(".");
    let mac = ["00", "1a", "2b", "3c", "4d", "5e"].join(":");
    escribir(dir.path(), "notas.txt", &format!("{privada} {mac}\n"));
    let out = Command::new(env!("CARGO_BIN_EXE_comprobar-publicacion"))
        .arg("--arbol")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let todo = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(todo.contains("notas.txt:1"), "{todo}");
    assert!(!todo.contains(&privada), "{todo}");
    let plano = todo.to_ascii_lowercase().replace([':', '-', '.'], "");
    assert!(!plano.contains(&mac.replace(':', "")), "{todo}");
    assert!(!plano.contains("1a2b"), "{todo}");
}

#[test]
fn salidas_del_binario() {
    let dir = tempfile::tempdir().unwrap();
    obligatorios(dir.path());
    let bin = env!("CARGO_BIN_EXE_comprobar-publicacion");
    let limpio = Command::new(bin)
        .arg("--arbol")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(limpio.status.code(), Some(0));
    assert!(limpio.stdout.is_empty());
    let mal = Command::new(bin).arg("--inventado").output().unwrap();
    assert_eq!(mal.status.code(), Some(2));
    let sin_modo = Command::new(bin).output().unwrap();
    assert_eq!(sin_modo.status.code(), Some(2));
    // Una lista de permitidos con una línea sin motivo es un error de uso.
    escribir(
        dir.path(),
        "tools/publicacion-permitidos.txt",
        "valor 9.9.9.9\n",
    );
    let roto = Command::new(bin)
        .arg("--arbol")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(roto.status.code(), Some(2));
}
