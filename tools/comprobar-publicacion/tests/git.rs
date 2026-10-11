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
        self.comprobar_en(&self.dir, args)
    }

    /// `git` con `entrada` por la entrada estándar.
    fn git_con_entrada(&self, args: &[&str], entrada: &str) -> String {
        use std::io::Write;
        let mut hijo = self
            .git_cmd()
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        hijo.stdin
            .take()
            .unwrap()
            .write_all(entrada.as_bytes())
            .unwrap();
        let o = hijo.wait_with_output().unwrap();
        assert!(o.status.success(), "git {args:?}");
        String::from_utf8(o.stdout).unwrap().trim().to_owned()
    }

    fn comprobar_en(&self, dir: &Path, args: &[&str]) -> Salida {
        let o = Command::new(env!("CARGO_BIN_EXE_comprobar-publicacion"))
            .env("HOME", &self.casa)
            .env_remove("XDG_CONFIG_HOME")
            .env("RP_PUBLICACION_PROHIBIDOS", &self.lista)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args(args)
            .arg(dir)
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
    // Una etiqueta firmada: el bloque de firma final (base64) no se revisa.
    let inicio = concat!("-----", "BEGIN PGP SIGNATURE-----");
    let fin = concat!("-----", "END PGP SIGNATURE-----");
    let firma = format!("iQIzBAABCAAdFiEE{}", "Ab3dEf7hIj".repeat(5));
    let objeto = format!(
        "object {c1}\ntype commit\ntag v2\ntagger prueba <prueba@users.noreply.github.com> 1700000000 +0000\n\nv2\n{inicio}\n\n{firma}\n{fin}\n"
    );
    let v2 = r.git_con_entrada(&["mktag"], &objeto);
    r.git(&["update-ref", "refs/tags/v2", &v2]);
    let s = r.historial();
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert_eq!(
        s.hallazgos(),
        [format!("{}:a.txt:1  ipv4  [ipv4 privada]", corto(&c1))]
    );
    // Un marcador de firma que no cierra el objeto no oculta lo que le sigue.
    let falsa = format!("v3\n\n{inicio}\n{VALOR}\n");
    r.git(&["tag", "-a", "v3", "-m", &falsa]);
    let v3 = r.git(&["rev-parse", "v3"]);
    let s = r.historial();
    assert!(
        s.stdout
            .contains(&format!("etiqueta {}:4  lista-prohibidos", corto(&v3))),
        "{}",
        s.stdout
    );
}

#[test]
fn historial_ignora_replace_y_grafts() {
    let r = Repo::nuevo();
    r.escribir("a.txt", &format!("{VALOR}\n"));
    r.commit("primero");
    fs::remove_file(r.dir.join("a.txt")).unwrap();
    r.escribir("b.txt", "neutro\n");
    r.commit("lo borra");
    // Un injerto «corta» el historial para git log, pero git push envía los
    // objetos originales: el comprobador debe ver lo mismo que el push.
    r.git(&["replace", "--graft", "HEAD"]);
    let s = r.comprobar(&["--historial", "--rev", "main"]);
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert!(s.stdout.contains("lista-prohibidos"), "{}", s.stdout);
}

#[test]
fn historial_revisa_cada_ruta_de_un_contenido_compartido() {
    let r = Repo::nuevo();
    let privada = ["10", "1", "2", "3"].join(".");
    r.escribir(
        "tools/publicacion-permitidos.txt",
        "ruta docs/** ipv4 — ejemplos\n",
    );
    // El mismo árbol en una ruta normal y en una prohibida, y el mismo blob
    // en una ruta con excepción y en otra sin ella.
    r.escribir("-copia/plan.md", "neutro\n");
    r.escribir(".superpowers/sdd/x/plan.md", "neutro\n");
    r.escribir("docs/a.md", &format!("ip {privada}\n"));
    r.escribir("src/a.md", &format!("ip {privada}\n"));
    let c1 = r.commit("primero");
    let s = r.historial();
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    let mut h = s.hallazgos();
    h.sort();
    let c = corto(&c1);
    assert_eq!(
        h,
        [
            format!("{c}:.superpowers/sdd/x/plan.md:0  ruta-prohibida  [.superpowers/]"),
            format!("{c}:src/a.md:1  ipv4  [ipv4 privada]"),
        ]
    );
}

#[test]
fn permitidos_del_commit_no_del_arbol_de_trabajo() {
    let r = Repo::nuevo();
    r.escribir("a.txt", "neutro\n");
    let c1 = r.commit("primero");
    r.escribir("a.txt", &format!("neutro\n{VALOR}\n"));
    let c2 = r.commit("lo añade");
    // Una excepción sin confirmar no vale para lo que se sube.
    r.escribir(
        "tools/publicacion-permitidos.txt",
        &format!("valor {VALOR} — prueba\n"),
    );
    assert_eq!(r.diff(&c1, &c2).codigo, Some(1));
    assert_eq!(r.comprobar(&["--historial", "--rev", &c2]).codigo, Some(1));
    // Confirmada, sí.
    let c3 = r.commit("excepción");
    assert_eq!(r.diff(&c1, &c3).codigo, Some(0));
    assert_eq!(r.comprobar(&["--historial", "--rev", &c3]).codigo, Some(0));
}

#[test]
fn listas_del_repositorio_principal_desde_un_worktree() {
    let r = Repo::nuevo();
    r.escribir(".gitignore", ".superpowers/\n");
    r.escribir("a.txt", "neutro\n");
    r.commit("primero");
    r.escribir(".superpowers/sdd/x/real-secrets.txt", "rubi-escondido\n");
    let wt = r.dir.parent().unwrap().join("wt");
    r.git(&["worktree", "add", "-q", "-b", "otra", wt.to_str().unwrap()]);
    fs::write(wt.join("b.txt"), "rubi-escondido\n").unwrap();
    let o = r
        .git_cmd()
        .arg("-C")
        .arg(&wt)
        .args(["add", "-A"])
        .output()
        .unwrap();
    assert!(o.status.success());
    let o = r
        .git_cmd()
        .arg("-C")
        .arg(&wt)
        .args(["commit", "-qm", "en el worktree"])
        .output()
        .unwrap();
    assert!(o.status.success());
    let s = r.comprobar_en(&wt, &["--historial", "--rev", "otra"]);
    assert_eq!(s.codigo, Some(1), "{}{}", s.stdout, s.stderr);
    assert!(s.stdout.contains("lista-secretos"), "{}", s.stdout);
    assert!(!s.stdout.contains("rubi"), "{}", s.stdout);
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
