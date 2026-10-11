//! Pruebas de `--diff`, `--historial` y del gancho `pre-push`, con
//! repositorios git temporales y una lista de prohibidos sintética pasada por
//! `RP_PUBLICACION_PROHIBIDOS`.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Valor prohibido de las pruebas (solo existe en la lista temporal).
const VALOR: &str = "zafiro-escondido";

struct Repo {
    _t: tempfile::TempDir,
    dir: PathBuf,
    casa: PathBuf,
    lista: PathBuf,
}

impl Repo {
    fn nuevo() -> Repo {
        let t = tempfile::tempdir().unwrap();
        let dir = t.path().join("repo");
        let casa = t.path().join("casa");
        let lista = t.path().join("prohibidos.txt");
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir_all(&casa).unwrap();
        fs::write(&lista, format!("{VALOR}\n")).unwrap();
        let r = Repo {
            _t: t,
            dir,
            casa,
            lista,
        };
        r.git(&["init", "-q", "-b", "main"]);
        r
    }

    fn git_cmd(&self) -> Command {
        let mut c = Command::new("git");
        c.arg("-C")
            .arg(&self.dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "prueba")
            .env("GIT_AUTHOR_EMAIL", "prueba@users.noreply.github.com")
            .env("GIT_COMMITTER_NAME", "prueba")
            .env("GIT_COMMITTER_EMAIL", "prueba@users.noreply.github.com");
        c
    }

    fn git(&self, args: &[&str]) -> String {
        let o = self.git_cmd().args(args).output().unwrap();
        assert!(o.status.success(), "git {args:?}: {o:?}");
        String::from_utf8(o.stdout).unwrap().trim().to_owned()
    }

    fn escribir(&self, ruta: &str, texto: &str) {
        let p = self.dir.join(ruta);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, texto).unwrap();
    }

    /// `git add -A` y commit con `mensaje`; devuelve el hash.
    fn commit(&self, mensaje: &str) -> String {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "--allow-empty", "-m", mensaje]);
        self.git(&["rev-parse", "HEAD"])
    }

    fn comprobar(&self, args: &[&str]) -> Salida {
        let o = Command::new(env!("CARGO_BIN_EXE_comprobar-publicacion"))
            .env("HOME", &self.casa)
            .env_remove("XDG_CONFIG_HOME")
            .env("RP_PUBLICACION_PROHIBIDOS", &self.lista)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args(args)
            .arg(&self.dir)
            .output()
            .unwrap();
        let s = Salida {
            codigo: o.status.code(),
            stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
        };
        // Ninguna salida contiene el valor, en ninguna capitalización.
        for t in [&s.stdout, &s.stderr] {
            assert!(!t.to_lowercase().contains("zafiro"), "{t}");
        }
        s
    }

    fn diff(&self, a: &str, b: &str) -> Salida {
        self.comprobar(&["--diff", &format!("{a}..{b}")])
    }

    fn historial(&self) -> Salida {
        self.comprobar(&["--historial"])
    }
}

struct Salida {
    codigo: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Salida {
    fn hallazgos(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }
}

fn corto(sha: &str) -> &str {
    &sha[..7]
}

#[test]
fn diff_solo_lineas_anadidas() {
    let r = Repo::nuevo();
    r.escribir("a.txt", &format!("uno\n{VALOR}\ntres\n"));
    let c1 = r.commit("primero");
    r.escribir("a.txt", "uno\ntres\n");
    let c2 = r.commit("quita una línea");
    let s = r.diff(&c1, &c2);
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
    assert!(s.stdout.is_empty(), "{}", s.stdout);

    r.escribir("a.txt", &format!("uno\ntres\ncuatro {VALOR}\n"));
    let c3 = r.commit("añade una línea");
    let s = r.diff(&c2, &c3);
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert_eq!(
        s.hallazgos(),
        [format!(
            "{}:a.txt:3  lista-prohibidos  [prohibidos:#1]",
            corto(&c3)
        )]
    );

    // Una línea que ya estaba (movida) no es una línea añadida.
    r.escribir("a.txt", "tres\nuno\n");
    r.escribir("b.txt", "nuevo y neutro\n");
    let c4 = r.commit("mueve líneas");
    let s = r.diff(&c3, &c4);
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
}

#[test]
fn diff_revisa_mensajes_y_commits_intermedios() {
    let r = Repo::nuevo();
    r.escribir("a.txt", "neutro\n");
    let c1 = r.commit("primero");
    r.escribir("a.txt", &format!("neutro\n{VALOR}\n"));
    let c2 = r.commit("lo añade");
    r.escribir("a.txt", "neutro\n");
    r.commit("lo quita");
    let c4 = r.commit(&format!("mensaje\n\ncon {VALOR} en el cuerpo"));
    // El valor ya no está en el último árbol, pero sí en lo que se sube.
    let s = r.diff(&c1, &c4);
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    let mut h = s.hallazgos();
    h.sort();
    let mut esperado = [
        format!("{}:a.txt:2  lista-prohibidos  [prohibidos:#1]", corto(&c2)),
        format!("commit {}:3  lista-prohibidos  [prohibidos:#1]", corto(&c4)),
    ];
    esperado.sort();
    assert_eq!(h, esperado);
}

#[test]
fn diff_fichero_nuevo_binario_y_borrado() {
    let r = Repo::nuevo();
    r.escribir("a.txt", "neutro\n");
    r.escribir("quitar.txt", "neutro\n");
    let c1 = r.commit("primero");
    fs::remove_file(r.dir.join("quitar.txt")).unwrap();
    r.escribir("datos.bin", &format!("\0\0{VALOR}\0"));
    r.escribir(&format!("docs/{VALOR}.md"), "neutro\n");
    let c2 = r.commit("binario, nombre y borrado");
    let s = r.diff(&c1, &c2);
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    let h = s.stdout.as_str();
    assert!(
        h.contains(&format!("{}:datos.bin:1  lista-prohibidos", corto(&c2))),
        "{h}"
    );
    assert!(h.contains("binario sin revisar"), "{h}");
    // El nombre lleva el valor: la ruta no se muestra.
    assert!(h.contains(&format!("{}:[fichero n.º", corto(&c2))), "{h}");
}

#[test]
fn diff_rango_no_valido_sale_con_2_sin_repetirlo() {
    let r = Repo::nuevo();
    r.commit("primero");
    let s = r.comprobar(&["--diff", "zafiro..HEAD"]);
    assert_eq!(s.codigo, Some(2), "{}{}", s.stdout, s.stderr);
    let s = r.comprobar(&["--diff", "--zafiro"]);
    assert_eq!(s.codigo, Some(2), "{}{}", s.stdout, s.stderr);
}

#[test]
fn historial_revisa_mensajes_y_etiquetas() {
    let r = Repo::nuevo();
    r.escribir("a.txt", "neutro\n");
    let c1 = r.commit(&format!("primero con {VALOR}"));
    r.git(&[
        "tag",
        "-a",
        "v1",
        "-m",
        &format!("versión\n\nnotas {VALOR}"),
    ]);
    let tag = r.git(&["rev-parse", "v1"]);
    let s = r.historial();
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    let mut h = s.hallazgos();
    h.sort();
    assert_eq!(
        h,
        [
            format!("commit {}:1  lista-prohibidos  [prohibidos:#1]", corto(&c1)),
            format!(
                "etiqueta {}:3  lista-prohibidos  [prohibidos:#1]",
                corto(&tag)
            ),
        ]
    );
}

#[test]
fn historial_ve_ficheros_borrados() {
    let r = Repo::nuevo();
    r.escribir("a.txt", &format!("{VALOR}\n"));
    let c1 = r.commit("primero");
    fs::remove_file(r.dir.join("a.txt")).unwrap();
    r.escribir("b.txt", "neutro\n");
    r.commit("lo borra");
    let s = r.historial();
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert_eq!(
        s.hallazgos(),
        [format!(
            "{}:a.txt:1  lista-prohibidos  [prohibidos:#1]",
            corto(&c1)
        )]
    );
}

#[test]
fn historial_revisa_autores_refs_y_nombres_de_cada_revision() {
    let r = Repo::nuevo();
    r.escribir(&format!("{VALOR}/a.txt"), "neutro\n");
    r.commit("primero");
    // El nombre con el valor solo existe en la primera revisión.
    r.git(&["mv", VALOR, "neutro"]);
    r.commit("renombra");
    r.escribir("b.txt", "neutro\n");
    r.git(&["add", "-A"]);
    let o = r
        .git_cmd()
        .env("GIT_AUTHOR_NAME", format!("Ana {VALOR}"))
        .env("GIT_COMMITTER_EMAIL", format!("{VALOR}@example.com"))
        .args(["commit", "-q", "-m", "autor"])
        .output()
        .unwrap();
    assert!(o.status.success());
    r.git(&["branch", &format!("rama-{VALOR}")]);
    let s = r.historial();
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    let h = s.stdout.as_str();
    // Autor y committer del mismo commit (cabeceras: línea 0).
    assert!(
        h.contains(":0  lista-prohibidos  [autor: prohibidos:#1]"),
        "{h}"
    );
    assert!(
        h.contains(":0  lista-prohibidos  [committer: prohibidos:#1]"),
        "{h}"
    );
    assert!(h.contains(":0  correo  [committer: correo]"), "{h}");
    assert!(h.contains("ref n.º"), "{h}");
    assert!(h.contains(":[fichero n.º"), "{h}");
}

#[test]
fn historial_limitado_a_una_revision() {
    let r = Repo::nuevo();
    r.escribir("a.txt", "neutro\n");
    let c1 = r.commit("primero");
    r.git(&["checkout", "-q", "-b", "otra"]);
    r.escribir("a.txt", &format!("{VALOR}\n"));
    r.commit("en otra rama");
    let s = r.comprobar(&["--historial", "--rev", &c1]);
    assert_eq!(s.codigo, Some(0), "{}{}", s.stdout, s.stderr);
    let s = r.comprobar(&["--historial", "--rev", "otra"]);
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
}

#[test]
fn historial_reglas_genericas_y_firmas() {
    let r = Repo::nuevo();
    let privada = ["10", "1", "2", "3"].join(".");
    r.escribir("a.txt", &format!("host {privada}\n"));
    let c1 = r.commit("primero");
    // Una nota (sus rutas son hashes) y una etiqueta sin datos no dan nada.
    r.git(&["notes", "add", "-m", "nota neutra", &c1]);
    r.git(&["tag", "-a", "v1", "-m", "v1"]);
    let s = r.historial();
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert_eq!(
        s.hallazgos(),
        [format!("{}:a.txt:1  ipv4  [ipv4 privada]", corto(&c1))]
    );
}

#[test]
fn pre_push_rechaza_historial_privado() {
    let guion = Path::new(env!("CARGO_MANIFEST_DIR")).join("../hooks/prueba-pre-push.sh");
    let o = Command::new("sh")
        .arg(&guion)
        .env(
            "RP_COMPROBAR_PUBLICACION",
            env!("CARGO_BIN_EXE_comprobar-publicacion"),
        )
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
}
