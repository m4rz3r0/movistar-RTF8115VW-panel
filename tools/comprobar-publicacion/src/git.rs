//! Modos `--diff` y `--historial`: revisan lo que git publicaría, leyendo los
//! objetos con `git` (`rev-list`, `ls-tree`, `cat-file --batch`), sin tocar
//! el árbol de trabajo.
//!
//! La ruta de un hallazgo es `<commit corto>:<ruta>` (contenido o nombre de
//! un fichero), `commit <corto>` (cabeceras y mensaje de un commit; las
//! cabeceras van en la línea 0), `etiqueta <corto>` (una etiqueta anotada) o
//! `ref n.º K` (el nombre de una ref). Si el nombre de un fichero contiene un
//! dato, en lugar de la ruta se muestra su número en
//! `git ls-tree -r -t --name-only <commit>` ([`ocultar_ruta`]).
//!
//! Los errores nunca repiten argumentos, rutas ni la salida de git: pueden
//! contener un dato.

use crate::{
    Hallazgo, ListasLocales, Permitidos, Regla,
    arbol::{binario_por_extension, prohibida, revisar_contenido},
    ocultar_ruta, revisar_nombre, revisar_texto,
};
use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

/// Revisa lo que añade el rango `rango` (`A..B`, como en `git rev-list`):
/// por cada commit, sus cabeceras y su mensaje, el nombre de cada fichero
/// que cambia y, de su contenido, las líneas que no estaban en ningún padre.
/// Un fichero nuevo, binario o que no es UTF-8 se revisa entero; uno borrado
/// no se revisa.
///
/// Se revisan todos los commits del rango, no solo el resultado final: un
/// dato añadido y quitado dentro del rango también se publicaría.
pub fn diff(
    repo: &Path,
    rango: &str,
    p: &Permitidos,
    l: &ListasLocales,
) -> Result<Vec<Hallazgo>, String> {
    if rango.starts_with('-') {
        return Err("el rango no es válido".to_owned());
    }
    let commits = lineas(&salida(
        repo,
        &["rev-list", "--end-of-options", rango],
        "el rango no es válido",
    )?);
    let objetos = leer_objetos(repo, &commits)?;
    let mut arboles = Arboles::default();
    let mut out = Vec::new();
    for sha in &commits {
        let Some((_, datos)) = objetos.get(sha) else {
            return Err("no se pudo leer un commit del rango".to_owned());
        };
        let commit = String::from_utf8_lossy(datos);
        out.extend(revisar_commit(sha, &commit, p, l));
        out.extend(revisar_cambios(
            repo,
            sha,
            &padres(&commit),
            &mut arboles,
            p,
            l,
        )?);
    }
    Ok(out)
}

/// Revisa todo lo alcanzable desde `revs` (todas las refs si está vacío):
/// cada blob (todas las versiones de todos los ficheros, también de los
/// borrados), el nombre de cada entrada de cada árbol, las cabeceras (autor,
/// committer) y el mensaje de cada commit, las etiquetas anotadas y, si
/// `revs` está vacío, los nombres de las refs. Las notas se revisan como
/// blobs (son alcanzables desde `refs/notes/*`).
pub fn historial(
    repo: &Path,
    revs: &[String],
    p: &Permitidos,
    l: &ListasLocales,
) -> Result<Vec<Hallazgo>, String> {
    if revs.iter().any(|r| r.starts_with('-')) {
        return Err("una revisión no es válida".to_owned());
    }
    let mut args = vec!["rev-list", "--objects"];
    if revs.is_empty() {
        args.push("--all");
    } else {
        args.push("--end-of-options");
        args.extend(revs.iter().map(String::as_str));
    }
    let lista = salida(repo, &args, "no se pudo listar el historial")?;
    // `<sha>` o `<sha> <ruta>`.
    let mut rutas: HashMap<String, String> = HashMap::new();
    let mut shas = Vec::new();
    for linea in String::from_utf8_lossy(&lista).lines() {
        let (sha, ruta) = linea.split_once(' ').unwrap_or((linea, ""));
        rutas.insert(sha.to_owned(), ruta.to_owned());
        shas.push(sha.to_owned());
    }

    let mut out = Vec::new();
    // Hallazgos de blobs y árboles, con la ruta sin prefijo y si se oculta.
    let mut de_objetos: Vec<(String, Vec<Hallazgo>, bool)> = Vec::new();
    let mut nombres_vistos: HashSet<(String, Regla, String)> = HashSet::new();
    por_cada_objeto(repo, &shas, |sha, tipo, datos| {
        let ruta = rutas.get(sha).map(String::as_str).unwrap_or("");
        match tipo {
            "commit" => out.extend(revisar_commit(sha, &String::from_utf8_lossy(datos), p, l)),
            "tag" => {
                let texto = String::from_utf8_lossy(datos);
                out.extend(revisar_etiqueta(
                    &texto,
                    &format!("etiqueta {}", corto(sha)),
                    "",
                    p,
                    l,
                ));
            }
            "tree" => {
                for (modo, nombre, _) in entradas_de_arbol(datos, sha.len()) {
                    let ruta = if ruta.is_empty() {
                        nombre
                    } else {
                        format!("{ruta}/{nombre}")
                    };
                    let mut h = revisar_nombre_en_git(&ruta, &modo, p, l);
                    // Un árbol cambia en cada revisión: un mismo nombre se
                    // informa una vez.
                    h.retain(|x| nombres_vistos.insert((ruta.clone(), x.regla, x.detalle.clone())));
                    if !h.is_empty() {
                        let oculta = !revisar_nombre_sin_notas(&ruta, p, l).is_empty();
                        de_objetos.push((sha.to_owned(), h, oculta));
                    }
                }
            }
            "blob" => {
                let h = revisar_contenido(ruta, datos, p, l);
                if !h.is_empty() {
                    let oculta = !revisar_nombre_sin_notas(ruta, p, l).is_empty();
                    de_objetos.push((sha.to_owned(), h, oculta));
                }
            }
            _ => {}
        }
        Ok(())
    })?;

    if !de_objetos.is_empty() {
        let introducido = commits_que_introducen(repo, revs)?;
        let mut arboles = Arboles::default();
        for (sha, mut h, oculta) in de_objetos {
            let Some(commit) = introducido.get(&sha) else {
                // Solo alcanzable sin commit (una etiqueta que apunta a un
                // blob o a un árbol): se nombra por el objeto.
                for x in &mut h {
                    x.ruta = if oculta {
                        format!("objeto {}", corto(&sha))
                    } else {
                        format!("objeto {}:{}", corto(&sha), x.ruta)
                    };
                }
                out.extend(h);
                continue;
            };
            if oculta {
                // Todos los hallazgos de un objeto comparten la ruta.
                let ruta = h[0].ruta.clone();
                let k = arboles.numero(repo, commit, &ruta)?;
                ocultar_ruta(&mut h, k);
            }
            for x in &mut h {
                x.ruta = format!("{}:{}", corto(commit), x.ruta);
            }
            out.extend(h);
        }
    }

    if revs.is_empty() {
        let refs = salida(
            repo,
            &["for-each-ref", "--format=%(refname)"],
            "no se pudieron listar las refs",
        )?;
        out.extend(revisar_refs(&lineas(&refs), p, l));
    }
    Ok(out)
}

/// Revisa nombres de refs (ramas, etiquetas). La ruta es `ref n.º K`, con `K`
/// la posición en `nombres`: el nombre nunca se muestra.
pub fn revisar_refs(nombres: &[String], p: &Permitidos, l: &ListasLocales) -> Vec<Hallazgo> {
    nombres
        .iter()
        .enumerate()
        .flat_map(|(k, nombre)| {
            let mut h = revisar_nombre(nombre, p, l);
            for x in &mut h {
                x.ruta = format!("ref n.º {}", k + 1);
            }
            h
        })
        .collect()
}

fn corto(sha: &str) -> &str {
    &sha[..sha.len().min(7)]
}

fn git(repo: &Path) -> Command {
    let mut c = Command::new("git");
    c.arg("-C")
        .arg(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    c
}

/// Salida de `git args`; si falla, `error` (sin repetir nada de lo pedido).
fn salida(repo: &Path, args: &[&str], error: &str) -> Result<Vec<u8>, String> {
    let o = git(repo)
        .args(args)
        .output()
        .map_err(|_| error.to_owned())?;
    if o.status.success() {
        Ok(o.stdout)
    } else {
        Err(error.to_owned())
    }
}

fn lineas(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Lee los objetos con `git cat-file --batch` y llama a `f(sha, tipo, datos)`
/// con cada uno, en orden.
fn por_cada_objeto(
    repo: &Path,
    shas: &[String],
    mut f: impl FnMut(&str, &str, &[u8]) -> Result<(), String>,
) -> Result<(), String> {
    const ERROR: &str = "no se pudieron leer los objetos del repositorio";
    if shas.is_empty() {
        return Ok(());
    }
    let mut hijo = git(repo)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| ERROR.to_owned())?;
    let mut entrada = hijo.stdin.take().ok_or(ERROR)?;
    let pedidos: Vec<String> = shas.to_vec();
    // Se escribe en otro hilo: si no, con muchos objetos se bloquean las dos
    // tuberías.
    let escritor = std::thread::spawn(move || -> std::io::Result<()> {
        for s in &pedidos {
            writeln!(entrada, "{s}")?;
        }
        Ok(())
    });
    let mut lector = BufReader::new(hijo.stdout.take().ok_or(ERROR)?);
    let mut resultado = Ok(());
    for _ in shas {
        let mut cabecera = String::new();
        if lector.read_line(&mut cabecera).unwrap_or(0) == 0 {
            resultado = Err(ERROR.to_owned());
            break;
        }
        let campos: Vec<&str> = cabecera.split_whitespace().collect();
        let [sha, tipo, tam] = campos[..] else {
            resultado = Err(ERROR.to_owned());
            break;
        };
        let Ok(tam) = tam.parse::<usize>() else {
            resultado = Err(ERROR.to_owned());
            break;
        };
        let mut datos = vec![0; tam + 1];
        if lector.read_exact(&mut datos).is_err() {
            resultado = Err(ERROR.to_owned());
            break;
        }
        datos.pop();
        if let Err(e) = f(sha, tipo, &datos) {
            resultado = Err(e);
            break;
        }
    }
    if resultado.is_err() {
        let _ = hijo.kill();
    }
    drop(lector);
    let escrito = escritor.join().map_err(|_| ERROR.to_owned())?;
    let estado = hijo.wait().map_err(|_| ERROR.to_owned())?;
    resultado?;
    if escrito.is_err() || !estado.success() {
        return Err(ERROR.to_owned());
    }
    Ok(())
}

/// Los objetos pedidos, como `sha → (tipo, datos)`.
fn leer_objetos(
    repo: &Path,
    shas: &[String],
) -> Result<HashMap<String, (String, Vec<u8>)>, String> {
    let mut m = HashMap::new();
    por_cada_objeto(repo, shas, |sha, tipo, datos| {
        m.insert(sha.to_owned(), (tipo.to_owned(), datos.to_vec()));
        Ok(())
    })?;
    Ok(m)
}

/// Entradas de un objeto árbol: `(modo, nombre, sha)`.
fn entradas_de_arbol(datos: &[u8], largo_sha: usize) -> Vec<(String, String, String)> {
    let bytes_sha = largo_sha / 2;
    let mut out = Vec::new();
    let mut i = 0;
    while i < datos.len() {
        let Some(espacio) = datos[i..].iter().position(|&b| b == b' ') else {
            break;
        };
        let Some(nul) = datos[i + espacio..].iter().position(|&b| b == 0) else {
            break;
        };
        let modo = String::from_utf8_lossy(&datos[i..i + espacio]).into_owned();
        let nombre =
            String::from_utf8_lossy(&datos[i + espacio + 1..i + espacio + nul]).into_owned();
        let inicio = i + espacio + nul + 1;
        let Some(sha) = datos.get(inicio..inicio + bytes_sha) else {
            break;
        };
        let sha: String = sha.iter().map(|b| format!("{b:02x}")).collect();
        out.push((modo, nombre, sha));
        i = inicio + bytes_sha;
    }
    out
}

/// Ruta de un blob del árbol de notas (`refs/notes/*`): el hash del objeto
/// anotado, quizá repartido en directorios de dos cifras.
fn es_ruta_de_nota(ruta: &str) -> bool {
    let partes: Vec<&str> = ruta.split('/').collect();
    let total: usize = partes.iter().map(|p| p.len()).sum();
    let (_, dirs) = partes.split_last().expect("split da al menos una parte");
    (total == 40 || total == 64)
        && dirs.iter().all(|d| d.len() == 2)
        && partes
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// [`revisar_nombre`] sin la regla de claves en las rutas de notas (que son
/// hashes).
fn revisar_nombre_sin_notas(ruta: &str, p: &Permitidos, l: &ListasLocales) -> Vec<Hallazgo> {
    let mut h = revisar_nombre(ruta, p, l);
    if es_ruta_de_nota(ruta) {
        h.retain(|x| x.regla != Regla::Clave);
    }
    h
}

/// El nombre de una entrada de un árbol y, si no es un directorio, si su
/// ruta está prohibida.
fn revisar_nombre_en_git(
    ruta: &str,
    modo: &str,
    p: &Permitidos,
    l: &ListasLocales,
) -> Vec<Hallazgo> {
    let mut h = revisar_nombre_sin_notas(ruta, p, l);
    if modo != "40000"
        && let Some(detalle) = prohibida(ruta)
    {
        h.push(Hallazgo {
            ruta: ruta.to_owned(),
            linea: 0,
            regla: Regla::RutaProhibida,
            detalle: detalle.to_owned(),
        });
    }
    h
}

/// Separa un objeto commit o etiqueta en cabeceras (con sus líneas de
/// continuación unidas) y mensaje.
fn partes_de_objeto(texto: &str) -> (Vec<(String, String)>, &str) {
    let (cabeza, mensaje) = texto.split_once("\n\n").unwrap_or((texto, ""));
    let mut cabeceras: Vec<(String, String)> = Vec::new();
    for linea in cabeza.lines() {
        if let Some(resto) = linea.strip_prefix(' ') {
            if let Some((_, valor)) = cabeceras.last_mut() {
                valor.push('\n');
                valor.push_str(resto);
            }
        } else {
            let (clave, valor) = linea.split_once(' ').unwrap_or((linea, ""));
            cabeceras.push((clave.to_owned(), valor.to_owned()));
        }
    }
    (cabeceras, mensaje)
}

/// `Nombre <correo>` de una cabecera `author`/`committer`/`tagger`, sin la
/// fecha.
fn identidad(valor: &str) -> &str {
    match valor.rfind('>') {
        Some(i) => &valor[..=i],
        None => valor,
    }
}

/// Revisa un texto de una sola línea de cabecera: hallazgos en la línea 0 con
/// `prefijo` delante del detalle.
fn revisar_cabecera(
    ruta: &str,
    prefijo: &str,
    texto: &str,
    p: &Permitidos,
    l: &ListasLocales,
) -> Vec<Hallazgo> {
    revisar_texto(ruta, texto, p, l)
        .into_iter()
        .map(|mut h| {
            h.linea = 0;
            h.detalle = format!("{prefijo}: {}", h.detalle);
            h
        })
        .collect()
}

/// Cabeceras de identidad y mensaje de un commit. Las firmas, el árbol y los
/// padres no se revisan (son hashes y bloques firmados); una etiqueta
/// incrustada (`mergetag`) se revisa como etiqueta.
fn revisar_commit(sha: &str, texto: &str, p: &Permitidos, l: &ListasLocales) -> Vec<Hallazgo> {
    let ruta = format!("commit {}", corto(sha));
    let (cabeceras, mensaje) = partes_de_objeto(texto);
    let mut out = Vec::new();
    for (clave, valor) in &cabeceras {
        match clave.as_str() {
            "author" => out.extend(revisar_cabecera(&ruta, "autor", identidad(valor), p, l)),
            "committer" => {
                out.extend(revisar_cabecera(&ruta, "committer", identidad(valor), p, l));
            }
            "mergetag" => out.extend(
                revisar_etiqueta(valor, &ruta, "mergetag ", p, l)
                    .into_iter()
                    .map(|mut h| {
                        h.linea = 0;
                        h
                    }),
            ),
            "tree" | "parent" | "gpgsig" | "gpgsig-sha256" | "encoding" => {}
            _ => out.extend(revisar_cabecera(&ruta, "cabecera", valor, p, l)),
        }
    }
    out.extend(revisar_texto(&ruta, mensaje, p, l));
    out
}

/// Nombre, autor y mensaje de una etiqueta anotada (sin su firma).
fn revisar_etiqueta(
    texto: &str,
    ruta: &str,
    prefijo: &str,
    p: &Permitidos,
    l: &ListasLocales,
) -> Vec<Hallazgo> {
    let (cabeceras, mensaje) = partes_de_objeto(texto);
    let mut out = Vec::new();
    for (clave, valor) in &cabeceras {
        match clave.as_str() {
            "tag" => out.extend(revisar_cabecera(
                ruta,
                &format!("{prefijo}nombre"),
                valor,
                p,
                l,
            )),
            "tagger" => out.extend(revisar_cabecera(
                ruta,
                &format!("{prefijo}etiquetador"),
                identidad(valor),
                p,
                l,
            )),
            "object" | "type" => {}
            _ => out.extend(revisar_cabecera(
                ruta,
                &format!("{prefijo}cabecera"),
                valor,
                p,
                l,
            )),
        }
    }
    // Partidas para que este fichero no parezca un bloque PEM.
    let firmas = [
        concat!("-----", "BEGIN PGP SIGNATURE-----"),
        concat!("-----", "BEGIN SSH SIGNATURE-----"),
    ];
    let sin_firma = firmas
        .iter()
        .filter_map(|f| mensaje.find(f))
        .min()
        .map_or(mensaje, |i| &mensaje[..i]);
    out.extend(
        revisar_texto(ruta, sin_firma, p, l)
            .into_iter()
            .map(|mut h| {
                if !prefijo.is_empty() {
                    h.detalle = format!("{prefijo}{}", h.detalle);
                }
                h
            }),
    );
    out
}

fn padres(commit: &str) -> Vec<String> {
    partes_de_objeto(commit)
        .0
        .into_iter()
        .filter(|(c, _)| c == "parent")
        .map(|(_, v)| v)
        .collect()
}

/// Cada objeto (blob o árbol) alcanzable desde `revs` (o desde todas las
/// refs), con el commit más antiguo que lo introduce.
fn commits_que_introducen(repo: &Path, revs: &[String]) -> Result<HashMap<String, String>, String> {
    let mut args = vec![
        "log",
        "--format=%x01%H %T",
        "--raw",
        "--root",
        "-r",
        "-t",
        "-m",
        "--no-renames",
        "--no-abbrev",
    ];
    if revs.is_empty() {
        args.push("--all");
    } else {
        args.push("--end-of-options");
        args.extend(revs.iter().map(String::as_str));
    }
    let log = salida(repo, &args, "no se pudo leer el historial")?;
    let mut m = HashMap::new();
    let mut actual = String::new();
    // `git log` va del más nuevo al más antiguo: el último que se apunta es
    // el que lo introduce.
    for linea in String::from_utf8_lossy(&log).lines() {
        if let Some(resto) = linea.strip_prefix('\u{1}') {
            let (commit, arbol) = resto.split_once(' ').unwrap_or((resto, ""));
            actual = commit.to_owned();
            m.insert(arbol.to_owned(), actual.clone());
        } else if let Some(raw) = linea.strip_prefix(':') {
            let campos: Vec<&str> = raw.split('\t').next().unwrap_or("").split(' ').collect();
            if let Some(nuevo) = campos.get(3) {
                m.insert((*nuevo).to_owned(), actual.clone());
            }
        }
    }
    Ok(m)
}

/// `(modo, sha)` de cada fichero de un commit, por ruta; y la lista de rutas
/// de `ls-tree -r -t` para numerar las que se ocultan.
#[derive(Default)]
struct Arboles {
    ficheros: HashMap<String, HashMap<String, (String, String)>>,
    numeradas: HashMap<String, Vec<String>>,
}

impl Arboles {
    fn ficheros(
        &mut self,
        repo: &Path,
        commit: &str,
    ) -> Result<&HashMap<String, (String, String)>, String> {
        if !self.ficheros.contains_key(commit) {
            let s = salida(
                repo,
                &["ls-tree", "-r", "-z", "--full-tree", commit],
                "no se pudo leer el árbol de un commit",
            )?;
            let mut m = HashMap::new();
            for entrada in String::from_utf8_lossy(&s).split('\0') {
                // `<modo> <tipo> <sha>\t<ruta>`
                let Some((cabeza, ruta)) = entrada.split_once('\t') else {
                    continue;
                };
                let campos: Vec<&str> = cabeza.split(' ').collect();
                if let [modo, _, sha] = campos[..] {
                    m.insert(ruta.to_owned(), (modo.to_owned(), sha.to_owned()));
                }
            }
            self.ficheros.insert(commit.to_owned(), m);
        }
        Ok(&self.ficheros[commit])
    }

    /// Posición (desde 1) de `ruta` en `git ls-tree -r -t --name-only
    /// <commit>`; 0 si no está.
    fn numero(&mut self, repo: &Path, commit: &str, ruta: &str) -> Result<usize, String> {
        if !self.numeradas.contains_key(commit) {
            let s = salida(
                repo,
                &[
                    "ls-tree",
                    "-r",
                    "-t",
                    "-z",
                    "--name-only",
                    "--full-tree",
                    commit,
                ],
                "no se pudo leer el árbol de un commit",
            )?;
            let v = String::from_utf8_lossy(&s)
                .split('\0')
                .filter(|r| !r.is_empty())
                .map(str::to_owned)
                .collect();
            self.numeradas.insert(commit.to_owned(), v);
        }
        Ok(self.numeradas[commit]
            .iter()
            .position(|r| r == ruta)
            .map_or(0, |i| i + 1))
    }
}

/// Los ficheros que `commit` cambia respecto de **todos** sus padres (lo que
/// coincide con algún padre ya estaba publicado o se revisa en ese padre).
fn revisar_cambios(
    repo: &Path,
    commit: &str,
    padres: &[String],
    arboles: &mut Arboles,
    p: &Permitidos,
    l: &ListasLocales,
) -> Result<Vec<Hallazgo>, String> {
    let ficheros = arboles.ficheros(repo, commit)?.clone();
    let mut de_padres = Vec::new();
    for padre in padres {
        de_padres.push(arboles.ficheros(repo, padre)?.clone());
    }
    // (ruta, sha nuevo, shas de los padres)
    let mut cambios: Vec<(String, String, Vec<String>)> = Vec::new();
    let mut rutas: Vec<&String> = ficheros.keys().collect();
    rutas.sort();
    for ruta in rutas {
        let (modo, sha) = &ficheros[ruta];
        let anteriores: Vec<&(String, String)> =
            de_padres.iter().filter_map(|f| f.get(ruta)).collect();
        if anteriores.iter().any(|(_, s)| s == sha) {
            continue;
        }
        // Un submódulo no tiene contenido aquí, pero su nombre sí se revisa.
        let sha = if modo == "160000" {
            String::new()
        } else {
            sha.clone()
        };
        cambios.push((
            ruta.clone(),
            sha,
            anteriores.iter().map(|(_, s)| s.clone()).collect(),
        ));
    }

    let pedidos: Vec<String> = cambios
        .iter()
        .flat_map(|(_, sha, anteriores)| std::iter::once(sha).chain(anteriores))
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let blobs = leer_objetos(repo, &pedidos)?;
    let contenido = |sha: &str| blobs.get(sha).map(|(_, d)| d.as_slice());

    let mut out = Vec::new();
    for (ruta, sha, anteriores) in &cambios {
        let mut h = revisar_nombre_en_git(ruta, "", p, l);
        let oculta = !revisar_nombre_sin_notas(ruta, p, l).is_empty();
        if let Some(nuevo) = contenido(sha) {
            h.extend(revisar_anadido(ruta, nuevo, anteriores, &contenido, p, l));
        }
        if oculta {
            let k = arboles.numero(repo, commit, ruta)?;
            ocultar_ruta(&mut h, k);
        }
        for x in &mut h {
            x.ruta = format!("{}:{}", corto(commit), x.ruta);
        }
        out.extend(h);
    }
    Ok(out)
}

/// Lo que añade `nuevo` respecto de las versiones `anteriores` del fichero:
/// entero si no había versión anterior o si no es texto UTF-8; si no, las
/// líneas que no estaban en ninguna versión anterior.
fn revisar_anadido<'a>(
    ruta: &str,
    nuevo: &[u8],
    anteriores: &[String],
    contenido: &impl Fn(&str) -> Option<&'a [u8]>,
    p: &Permitidos,
    l: &ListasLocales,
) -> Vec<Hallazgo> {
    let texto = std::str::from_utf8(nuevo)
        .ok()
        .filter(|_| !nuevo.contains(&0) && !binario_por_extension(ruta));
    let (Some(texto), false) = (texto, anteriores.is_empty()) else {
        return revisar_contenido(ruta, nuevo, p, l);
    };
    let antes: Vec<String> = anteriores
        .iter()
        .filter_map(|s| contenido(s))
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .collect();
    let previas: HashSet<&str> = antes.iter().flat_map(|t| t.lines()).collect();
    let mut out = Vec::new();
    for (i, linea) in texto.lines().enumerate() {
        if previas.contains(linea) {
            continue;
        }
        for mut h in revisar_texto(ruta, linea, p, l) {
            h.linea = i + 1;
            out.push(h);
        }
    }
    out
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn rutas_de_notas() {
        let h = &"0123456789abcdef".repeat(3)[..40];
        assert!(es_ruta_de_nota(h));
        assert!(!es_ruta_de_nota(&h[1..]));
        assert!(es_ruta_de_nota(&format!("{}/{}", &h[..2], &h[2..])));
        assert!(!es_ruta_de_nota(&format!("docs/{}", &h[4..])));
        assert!(!es_ruta_de_nota("abc"));
    }

    #[test]
    fn identidades_y_cabeceras() {
        assert_eq!(identidad("Ana <a@b.c> 1700000000 +0200"), "Ana <a@b.c>");
        let (c, m) = partes_de_objeto("tree x\ngpgsig a\n b\nauthor y\n\nmensaje\n");
        assert_eq!(c[1], ("gpgsig".to_owned(), "a\nb".to_owned()));
        assert_eq!(c[2].0, "author");
        assert_eq!(m, "mensaje\n");
    }
}
