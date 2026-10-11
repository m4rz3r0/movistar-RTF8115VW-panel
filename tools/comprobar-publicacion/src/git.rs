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
/// las cabeceras (autor, committer) y el mensaje de cada commit, las
/// etiquetas anotadas, cada nombre y cada blob **en cada ruta en la que
/// aparece** en cada revisión (también los ficheros borrados) y, si `revs`
/// está vacío, los nombres de las refs. Las notas se revisan como blobs (son
/// alcanzables desde `refs/notes/*`).
///
/// Los árboles se recorren desde la raíz de cada commit, del más antiguo al
/// más nuevo, sin repetir un mismo árbol en una misma ruta: el mismo
/// contenido en dos rutas se revisa en las dos (una puede estar prohibida o
/// tener una excepción distinta). Cada hallazgo se atribuye al primer commit
/// que tiene ese contenido en esa ruta.
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
    let tipo = tipos(repo, &shas)?;
    let es = |sha: &str, t: &str| tipo.get(sha).is_some_and(|x| x == t);

    // Commits, etiquetas y árboles (los blobs se leen al final).
    let mut out = Vec::new();
    let mut commits: Vec<(String, String)> = Vec::new();
    let mut arboles: HashMap<String, Vec<(String, String, String)>> = HashMap::new();
    let no_blobs: Vec<String> = shas.iter().filter(|s| !es(s, "blob")).cloned().collect();
    por_cada_objeto(repo, &no_blobs, |sha, tipo, datos| {
        match tipo {
            "commit" => {
                let texto = String::from_utf8_lossy(datos);
                out.extend(revisar_commit(sha, &texto, p, l));
                let arbol = partes_de_objeto(&texto)
                    .0
                    .into_iter()
                    .find(|(c, _)| c == "tree")
                    .map(|(_, v)| v)
                    .ok_or("un commit no tiene árbol")?;
                commits.push((sha.to_owned(), arbol));
            }
            "tag" => {
                let texto = String::from_utf8_lossy(datos);
                let ruta = format!("etiqueta {}", corto(sha));
                out.extend(revisar_etiqueta(&texto, &ruta, "", p, l));
            }
            "tree" => {
                arboles.insert(sha.to_owned(), entradas_de_arbol(datos, sha.len()));
            }
            _ => {}
        }
        Ok(())
    })?;

    // Recorrido: cada (árbol, ruta) una vez; cada (blob, ruta) una vez.
    let mut recorrido = Recorrido::default();
    // `rev-list` da los commits del más nuevo al más antiguo.
    for (commit, arbol) in commits.iter().rev() {
        recorrido.recorrer(&arboles, arbol, "", Some(commit), p, l)?;
    }
    // Lo que solo es alcanzable sin commit (una etiqueta que apunta a un
    // árbol o a un blob) se revisa en la ruta que da `rev-list`.
    for sha in &shas {
        let ruta = rutas.get(sha).map(String::as_str).unwrap_or("");
        if es(sha, "tree") && !recorrido.arboles_vistos.contains(sha) {
            recorrido.recorrer(&arboles, sha, ruta, None, p, l)?;
        } else if es(sha, "blob") && !recorrido.colocaciones.contains_key(sha) {
            recorrido.colocar(sha, ruta, None);
        }
    }

    let Recorrido {
        colocaciones,
        orden,
        nombres,
        ..
    } = recorrido;
    let mut numeradas = Arboles::default();
    let mut anotar = |commit: Option<&str>, ruta: &str, mut h: Vec<Hallazgo>, objeto: &str| {
        let oculta = !revisar_nombre_sin_notas(ruta, p, l).is_empty();
        let prefijo = match commit {
            Some(c) => {
                if oculta {
                    let k = numeradas.numero(repo, c, ruta)?;
                    ocultar_ruta(&mut h, k);
                }
                corto(c).to_owned()
            }
            None => {
                if oculta {
                    ocultar_ruta(&mut h, 0);
                }
                format!("objeto {}", corto(objeto))
            }
        };
        for x in &mut h {
            x.ruta = format!("{prefijo}:{}", x.ruta);
        }
        out.extend(h);
        Ok::<(), String>(())
    };
    for (commit, ruta, h) in nombres {
        anotar(commit.as_deref(), &ruta, h, "")?;
    }
    por_cada_objeto(repo, &orden, |sha, _, datos| {
        for (ruta, commit) in colocaciones.get(sha).into_iter().flatten() {
            let h = revisar_contenido(ruta, datos, p, l);
            if !h.is_empty() {
                anotar(commit.as_deref(), ruta, h, sha)?;
            }
        }
        Ok(())
    })?;

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

/// Estado del recorrido de los árboles de [`historial`].
#[derive(Default)]
struct Recorrido {
    /// `(árbol, ruta)` ya recorridos.
    visitados: HashSet<(String, String)>,
    arboles_vistos: HashSet<String>,
    /// Cada blob, con las rutas (y el commit) en que aparece.
    colocaciones: HashMap<String, Vec<(String, Option<String>)>>,
    /// Los blobs en el orden en que aparecen.
    orden: Vec<String>,
    colocados: HashSet<(String, String)>,
    /// Hallazgos de nombres: `(commit, ruta, hallazgos)`.
    nombres: Vec<(Option<String>, String, Vec<Hallazgo>)>,
    nombres_vistos: HashSet<(String, Regla, String)>,
}

impl Recorrido {
    fn recorrer(
        &mut self,
        arboles: &HashMap<String, Vec<(String, String, String)>>,
        raiz: &str,
        prefijo: &str,
        commit: Option<&String>,
        p: &Permitidos,
        l: &ListasLocales,
    ) -> Result<(), String> {
        let mut pila = vec![(raiz.to_owned(), prefijo.to_owned())];
        while let Some((arbol, base)) = pila.pop() {
            if !self.visitados.insert((arbol.clone(), base.clone())) {
                continue;
            }
            self.arboles_vistos.insert(arbol.clone());
            let entradas = arboles
                .get(&arbol)
                .ok_or("no se pudo leer un árbol del historial")?;
            for (modo, nombre, sha) in entradas {
                let ruta = if base.is_empty() {
                    nombre.clone()
                } else {
                    format!("{base}/{nombre}")
                };
                let mut h = revisar_nombre_en_git(&ruta, modo, p, l);
                // Un mismo nombre se informa una vez.
                h.retain(|x| {
                    self.nombres_vistos
                        .insert((ruta.clone(), x.regla, x.detalle.clone()))
                });
                if !h.is_empty() {
                    self.nombres.push((commit.cloned(), ruta.clone(), h));
                }
                match modo.as_str() {
                    "40000" => pila.push((sha.clone(), ruta)),
                    // Un submódulo no tiene contenido en este repositorio.
                    "160000" => {}
                    _ => self.colocar(sha, &ruta, commit),
                }
            }
        }
        Ok(())
    }

    fn colocar(&mut self, blob: &str, ruta: &str, commit: Option<&String>) {
        if !self.colocados.insert((blob.to_owned(), ruta.to_owned())) {
            return;
        }
        let v = self.colocaciones.entry(blob.to_owned()).or_default();
        if v.is_empty() {
            self.orden.push(blob.to_owned());
        }
        v.push((ruta.to_owned(), commit.cloned()));
    }
}

/// El contenido de `ruta` en el árbol de `rev`, si existe (por ejemplo, la
/// lista de permitidos del commit que se sube).
pub fn fichero_en_commit(repo: &Path, rev: &str, ruta: &str) -> Result<Option<Vec<u8>>, String> {
    if rev.starts_with('-') {
        return Err("una revisión no es válida".to_owned());
    }
    let objeto = format!("{rev}:{ruta}");
    let existe = git(repo)
        .args(["cat-file", "-e", &objeto])
        .status()
        .is_ok_and(|e| e.success());
    if !existe {
        return Ok(None);
    }
    salida(
        repo,
        &["cat-file", "blob", &objeto],
        "no se pudo leer un fichero del commit",
    )
    .map(Some)
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
        // `git push` (pack-objects) no usa `refs/replace/*` ni injertos: el
        // comprobador debe ver los mismos objetos que se envían.
        .env("GIT_NO_REPLACE_OBJECTS", "1")
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
    f: impl FnMut(&str, &str, &[u8]) -> Result<(), String>,
) -> Result<(), String> {
    lote(repo, shas, true, f)
}

/// El tipo de cada objeto (`git cat-file --batch-check`).
fn tipos(repo: &Path, shas: &[String]) -> Result<HashMap<String, String>, String> {
    let mut m = HashMap::new();
    lote(repo, shas, false, |sha, tipo, _| {
        m.insert(sha.to_owned(), tipo.to_owned());
        Ok(())
    })?;
    Ok(m)
}

/// `git cat-file --batch` (con el contenido) o `--batch-check` (sin él).
fn lote(
    repo: &Path,
    shas: &[String],
    con_contenido: bool,
    mut f: impl FnMut(&str, &str, &[u8]) -> Result<(), String>,
) -> Result<(), String> {
    const ERROR: &str = "no se pudieron leer los objetos del repositorio";
    if shas.is_empty() {
        return Ok(());
    }
    let mut hijo = git(repo)
        .args([
            "cat-file",
            if con_contenido {
                "--batch"
            } else {
                "--batch-check"
            },
        ])
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
        let mut datos = Vec::new();
        if con_contenido {
            datos = vec![0; tam + 1];
            if lector.read_exact(&mut datos).is_err() {
                resultado = Err(ERROR.to_owned());
                break;
            }
            datos.pop();
        }
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
    let sin_firma = sin_firma_final(mensaje);
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

/// El mensaje sin el bloque de firma (PGP o SSH) que cierra el objeto. Un
/// marcador en medio del mensaje, o sin su final, no oculta nada.
fn sin_firma_final(mensaje: &str) -> &str {
    // Partidas para que este fichero no parezca un bloque PEM.
    let tipos = [
        (
            concat!("-----", "BEGIN PGP SIGNATURE-----"),
            concat!("-----", "END PGP SIGNATURE-----"),
        ),
        (
            concat!("-----", "BEGIN SSH SIGNATURE-----"),
            concat!("-----", "END SSH SIGNATURE-----"),
        ),
    ];
    for (inicio, fin) in tipos {
        if let Some(i) = mensaje.rfind(inicio)
            && (i == 0 || mensaje[..i].ends_with('\n'))
            && mensaje[i..]
                .trim_end()
                .strip_suffix(fin)
                .is_some_and(|cuerpo| !cuerpo[inicio.len()..].contains("-----"))
        {
            return &mensaje[..i];
        }
    }
    mensaje
}

fn padres(commit: &str) -> Vec<String> {
    partes_de_objeto(commit)
        .0
        .into_iter()
        .filter(|(c, _)| c == "parent")
        .map(|(_, v)| v)
        .collect()
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
