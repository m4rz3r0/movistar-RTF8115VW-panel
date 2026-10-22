//! Extracción mecánica del borrador de catálogo a partir de las páginas y
//! scripts capturados del router.

use catalogo::{
    Area, Funcion, LecturaPareada, Lista, Origen, Parametro, Partido, Riesgo, Tipo, TipoParam,
    mask::Mascara,
};
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
    sync::LazyLock,
};

fn re(s: &str) -> Regex {
    Regex::new(s).expect("regex válida")
}

/// Página o script capturado, con los comentarios sustituidos por espacios.
struct Fuente {
    /// Ruta publicada: `dhcp.asp`, `js/dhcp.js`, `upload/cfgupload.asp`.
    ruta: String,
    texto: String,
    /// Texto sin quitar comentarios (para ver qué hay comentado).
    original: String,
}

impl Fuente {
    fn es_pagina(&self) -> bool {
        self.ruta.ends_with(".asp")
    }
}

/// Lee `*.asp` y `js__*.js` de `dir` y de `dir/wl` (páginas encontradas por
/// diccionario, no enlazadas).
fn cargar(dir: &Path) -> Result<Vec<Fuente>, String> {
    let mut out = Vec::new();
    for d in [dir.to_path_buf(), dir.join("wl")] {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        let mut nombres: Vec<String> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".asp") || (n.starts_with("js__") && n.ends_with(".js")))
            .collect();
        nombres.sort();
        for n in nombres {
            let bytes = std::fs::read(d.join(&n)).map_err(|e| format!("{n}: {e}"))?;
            let original = String::from_utf8_lossy(&bytes).into_owned();
            let ruta = n.replace("__", "/");
            let texto = if ruta.ends_with(".asp") {
                limpiar_asp(&original)
            } else {
                limpiar_js(&original)
            };
            out.push(Fuente {
                ruta,
                texto,
                original,
            });
        }
    }
    if out.is_empty() {
        return Err(format!("{}: no hay páginas ni scripts", dir.display()));
    }
    Ok(out)
}

/// Sustituye los comentarios JS por espacios (conservando saltos de línea),
/// respetando cadenas.
fn limpiar_js(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == '"' || c == '\'' || c == '`' {
            out.push(c);
            i += 1;
            while i < b.len() && b[i] != c && b[i] != '\n' {
                if b[i] == '\\' && i + 1 < b.len() {
                    out.push(b[i]);
                    i += 1;
                }
                out.push(b[i]);
                i += 1;
            }
            if i < b.len() {
                out.push(b[i]);
                i += 1;
            }
        } else if c == '/' && b.get(i + 1) == Some(&'/') {
            while i < b.len() && b[i] != '\n' {
                out.push(' ');
                i += 1;
            }
        } else if c == '/' && b.get(i + 1) == Some(&'*') {
            out.push_str("  ");
            i += 2;
            while i < b.len() && !(b[i] == '*' && b.get(i + 1) == Some(&'/')) {
                out.push(if b[i] == '\n' { '\n' } else { ' ' });
                i += 1;
            }
            i = (i + 2).min(b.len());
            out.push_str("  ");
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

static SCRIPT: LazyLock<Regex> = LazyLock::new(|| re(r"(?is)(<script\b[^>]*>)(.*?)(</script>)"));
static HTML_COMENTARIO: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<!--.*?-->"));

/// Quita los comentarios HTML fuera de los scripts y los JS dentro.
fn limpiar_asp(s: &str) -> String {
    let mut out = String::new();
    let mut ultimo = 0;
    for c in SCRIPT.captures_iter(s) {
        let todo = c.get(0).expect("grupo 0");
        out.push_str(&HTML_COMENTARIO.replace_all(&s[ultimo..todo.start()], ""));
        out.push_str(&c[1]);
        out.push_str(&limpiar_js(&c[2]));
        out.push_str(&c[3]);
        ultimo = todo.end();
    }
    out.push_str(&HTML_COMENTARIO.replace_all(&s[ultimo..], ""));
    out
}

/// Índice del paréntesis/llave que cierra el que hay en `abre`.
fn cierre(t: &str, abre: usize) -> Option<usize> {
    let b = t.as_bytes();
    let mut nivel = 0i32;
    let mut i = abre;
    let mut cadena: Option<u8> = None;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = cadena {
            if c == b'\\' {
                i += 1;
            } else if c == q {
                cadena = None;
            }
        } else {
            match c {
                b'"' | b'\'' | b'`' => cadena = Some(c),
                b'(' | b'{' | b'[' => nivel += 1,
                b')' | b'}' | b']' => {
                    nivel -= 1;
                    if nivel == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Separa por comas de primer nivel.
fn argumentos(t: &str) -> Vec<&str> {
    let b = t.as_bytes();
    let (mut nivel, mut cadena, mut ini) = (0i32, None::<u8>, 0);
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = cadena {
            if c == b'\\' {
                i += 1;
            } else if c == q {
                cadena = None;
            }
        } else {
            match c {
                b'"' | b'\'' | b'`' => cadena = Some(c),
                b'(' | b'{' | b'[' => nivel += 1,
                b')' | b'}' | b']' => nivel -= 1,
                b',' if nivel == 0 => {
                    out.push(t[ini..i].trim());
                    ini = i + 1;
                }
                _ => {}
            }
        }
        i += 1;
    }
    if !t[ini..].trim().is_empty() {
        out.push(t[ini..].trim());
    }
    out
}

static CADENA: LazyLock<Regex> = LazyLock::new(|| re(r#"^\s*["']([^"']*)["']\s*$"#));
static CLAVE: LazyLock<Regex> = LazyLock::new(|| re(r#"^\s*["']?([\w-]+)["']?\s*:\s*(.*)$"#));

/// Claves `k: v` de un literal de objeto `{...}` con el valor si es literal.
fn objeto(t: &str) -> Vec<(String, Option<String>)> {
    let t = t.trim();
    let dentro = t
        .strip_prefix('{')
        .and_then(|t| t.strip_suffix('}'))
        .unwrap_or(t);
    argumentos(dentro)
        .into_iter()
        .filter_map(|a| {
            let c = CLAVE.captures(a)?;
            Some((
                c[1].to_owned(),
                CADENA.captures(&c[2]).map(|v| v[1].to_owned()),
            ))
        })
        .collect()
}

/// Una invocación de una función `cb*.xml` en el código.
#[derive(Debug, Default)]
struct Sitio {
    cb: String,
    fuente: String,
    params: Vec<(String, Option<String>)>,
    formularios: Vec<String>,
}

static POST: LazyLock<Regex> = LazyLock::new(|| re(r"\$\.(post|ajax)\s*\("));
static GOFORM: LazyLock<Regex> = LazyLock::new(|| re(r#"goformCbk\s*=\s*["'](\w+)["']"#));
static CB_URL: LazyLock<Regex> = LazyLock::new(|| re(r"/cgi-bin/(\w+)\.xml"));
static SERIALIZE: LazyLock<Regex> =
    LazyLock::new(|| re(r#"\$\(\s*(?:["']#([\w-]+)["']|(this)|(\w+))\s*\)\s*\.serialize\(\)"#));
static SUBMIT: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"\$\(\s*["']([^"']+)["']\s*\)\s*\.\s*(?:submit\s*\(|on\s*\(\s*["']submit)"#)
});
static FUNCION_CON: LazyLock<Regex> = LazyLock::new(|| re(r"function\s+(\w+)\s*\(\s*(\w+)"));
static APPEND_OCULTO: LazyLock<Regex> =
    LazyLock::new(|| re(r#"name=\\?["'](\w+)\\?["']\s+value=\\?["']([\w-]*)\\?["']?"#));
static FETCH: LazyLock<Regex> = LazyLock::new(|| re(r#"fetch\s*\(\s*["']/cgi-bin/(\w+)\.xml"#));
static APPEND_PARAM: LazyLock<Regex> = LazyLock::new(|| re(r#"\.append\(\s*["'](\w+)["']"#));
static FORM_ACTION: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"(?i)<form\b[^>]*\bid\s*=\s*["']([\w-]+)["'][^>]*action\s*=\s*["']/cgi-bin/(\w+)\.xml"#)
});

fn llamadas(fuentes: &[Fuente]) -> Vec<Sitio> {
    let mut out = Vec::new();
    for f in fuentes {
        let t = &f.texto;
        for m in POST.find_iter(t) {
            let abre = m.end() - 1;
            let Some(fin) = cierre(t, abre) else { continue };
            let args = argumentos(&t[abre + 1..fin]);
            let (url, datos) = if &t[m.start() + 2..m.start() + 6] == "ajax" {
                let Some(obj) = args.first() else { continue };
                let campos = obj
                    .trim()
                    .strip_prefix('{')
                    .and_then(|o| o.strip_suffix('}'))
                    .map(argumentos)
                    .unwrap_or_default();
                let mut url = "";
                let mut datos = "";
                for c in campos {
                    if let Some(c) = CLAVE.captures(c) {
                        let v = c.get(2).expect("grupo").as_str();
                        match &c[1] {
                            "url" => url = v,
                            "data" => datos = v,
                            _ => {}
                        }
                    }
                }
                (url, datos)
            } else {
                (
                    args.first().copied().unwrap_or(""),
                    args.get(1).copied().unwrap_or(""),
                )
            };
            let cb = if url.contains("goformCbk") {
                GOFORM
                    .captures_iter(&t[..m.start()])
                    .last()
                    .map(|c| c[1].to_owned())
            } else {
                CB_URL.captures(url).map(|c| c[1].to_owned())
            };
            let Some(cb) = cb else { continue };
            let mut s = Sitio {
                cb,
                fuente: f.ruta.clone(),
                ..Sitio::default()
            };
            let datos = datos.trim();
            let inicio_funcion = t[..m.start()].rfind("function").unwrap_or(0);
            if datos.starts_with('{') {
                s.params = objeto(datos);
            } else if let Some(c) = SERIALIZE.captures(datos) {
                if let Some(id) = c.get(1) {
                    s.formularios.push(id.as_str().to_owned());
                } else if c.get(2).is_some() {
                    if let Some(c) = SUBMIT.captures_iter(&t[..m.start()]).last() {
                        s.formularios.extend(
                            c[1].split(',')
                                .filter_map(|id| id.trim().strip_prefix('#'))
                                .map(str::to_owned),
                        );
                    }
                } else if let Some(var) = c.get(3) {
                    let var = var.as_str();
                    if let Some(fc) = FUNCION_CON
                        .captures_iter(&t[..m.start()])
                        .filter(|fc| &fc[2] == var)
                        .last()
                    {
                        let llamada = re(&format!(r#"\b{}\s*\(\s*["']#([\w-]+)["']"#, &fc[1]));
                        for g in fuentes {
                            for c in llamada.captures_iter(&g.texto) {
                                s.formularios.push(c[1].to_owned());
                            }
                        }
                    }
                }
                for c in APPEND_OCULTO.captures_iter(&t[inicio_funcion..m.start()]) {
                    let v = (!c[2].is_empty()).then(|| c[2].to_owned());
                    s.params.push((c[1].to_owned(), v));
                }
            } else if let Some(var) = datos
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .filter(|v| !v.is_empty() && !datos.starts_with("function"))
            {
                // Objeto construido en una variable: `var x = {..}; x["k"] = "v";`
                let antes = &t[inicio_funcion..m.start()];
                let decl = re(&format!(r"\b{var}\s*=\s*\{{"));
                if let Some(d) = decl.find(antes)
                    && let Some(fin) = cierre(antes, d.end() - 1)
                {
                    s.params.extend(objeto(&antes[d.end() - 1..=fin]));
                }
                let asig = re(&format!(
                    r#"\b{var}\s*(?:\[\s*["'](\w+)["']\s*\]|\.(\w+))\s*=\s*(?:["']([^"']*)["'])?"#
                ));
                for c in asig.captures_iter(antes) {
                    let k = c.get(1).or(c.get(2)).expect("clave").as_str().to_owned();
                    s.params.push((k, c.get(3).map(|v| v.as_str().to_owned())));
                }
            }
            s.formularios.sort();
            s.formularios.dedup();
            out.push(s);
        }
        for c in FETCH.captures_iter(t) {
            let ini = c.get(0).expect("grupo 0").start();
            let mut desde = ini.saturating_sub(600);
            while !t.is_char_boundary(desde) {
                desde += 1;
            }
            let antes = &t[desde..ini];
            let antes = &antes[antes.rfind("URLSearchParams").unwrap_or(0)..];
            out.push(Sitio {
                cb: c[1].to_owned(),
                fuente: f.ruta.clone(),
                params: APPEND_PARAM
                    .captures_iter(antes)
                    .map(|a| (a[1].to_owned(), None))
                    .collect(),
                formularios: Vec::new(),
            });
        }
        for c in FORM_ACTION.captures_iter(&f.texto) {
            out.push(Sitio {
                cb: c[2].to_owned(),
                fuente: f.ruta.clone(),
                params: Vec::new(),
                formularios: vec![c[1].to_owned()],
            });
        }
    }
    out
}

/// Campo de un formulario.
#[derive(Debug, Clone)]
struct Campo {
    nombre: String,
    etiqueta: String,
    tipo: TipoParam,
    opciones: Vec<(String, String)>,
    partido: Option<Partido>,
}

static FORM: LazyLock<Regex> = LazyLock::new(|| re(r"(?is)<form\b([^>]*)>(.*?)</form>"));
static ATRIB: LazyLock<Regex> =
    LazyLock::new(|| re(r#"([\w-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#));
static CONTROL: LazyLock<Regex> = LazyLock::new(|| re(r"(?is)<(input|select|textarea)\b([^>]*)>"));
static OPCION: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?is)<option\b[^>]*value\s*=\s*["']([^"']*)["'][^>]*>([^<]*)"#));
static TEXTO: LazyLock<Regex> = LazyLock::new(|| re(r">([^<>]*[A-Za-z][^<>]*)<"));
static PARTE: LazyLock<Regex> = LazyLock::new(|| re(r"^(.+)-(\d)$"));
static ENTERO_NOMBRE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(_id|_0i|_idx|port|_time|mtu|metric|order|priority|weight|interval|count|_max|_min|vid|expires|threshold|timeout|_len|ptime|_gain|_value_min|_value_max)$",
    )
});

/// Texto hasta el `</select>` que cierra.
fn opciones_select(t: &str) -> &str {
    let fin = t.to_ascii_lowercase().find("</select>").unwrap_or(0);
    &t[..fin]
}

fn atributos(t: &str) -> HashMap<String, String> {
    ATRIB
        .captures_iter(t)
        .map(|c| {
            let v = c
                .get(2)
                .or(c.get(3))
                .or(c.get(4))
                .map_or("", |m| m.as_str());
            (c[1].to_ascii_lowercase(), v.to_owned())
        })
        .collect()
}

fn limpiar_etiqueta(s: &str) -> String {
    let s = s
        .replace("&nbsp;", " ")
        .replace("&aacute;", "á")
        .replace("&eacute;", "é")
        .replace("&iacute;", "í")
        .replace("&oacute;", "ó")
        .replace("&uacute;", "ú")
        .replace("&ntilde;", "ñ");
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| c == ':' || c == '*' || c.is_whitespace())
        .to_owned()
}

fn humanizar(nombre: &str) -> String {
    let s = nombre.replace(['_', '-'], " ");
    let mut c = s.chars();
    c.next()
        .map(|p| p.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// Tipo de un parámetro por su nombre cuando no hay formulario.
fn tipo_por_nombre(m: &Mascara, n: &str) -> TipoParam {
    if m.es_secreto(n) {
        TipoParam::Secret
    } else if ENTERO_NOMBRE.is_match(n) {
        TipoParam::Int
    } else {
        TipoParam::Text
    }
}

/// Formularios de las páginas: `(página, id) -> campos`.
fn formularios(fuentes: &[Fuente], m: &Mascara) -> HashMap<(String, String), Vec<Campo>> {
    let mut out = HashMap::new();
    for f in fuentes.iter().filter(|f| f.es_pagina()) {
        for c in FORM.captures_iter(&f.texto) {
            let a = atributos(&c[1]);
            let Some(id) = a.get("id").or(a.get("name")).cloned() else {
                continue;
            };
            let cuerpo = &c[2];
            let mut campos: Vec<Campo> = Vec::new();
            let mut partes: BTreeMap<String, (usize, String)> = BTreeMap::new();
            for ctl in CONTROL.captures_iter(cuerpo) {
                let todo = ctl.get(0).expect("grupo 0");
                let a = atributos(&ctl[2]);
                let Some(nombre) = a.get("name").cloned().filter(|n| !n.is_empty()) else {
                    continue;
                };
                let tipo_html = a
                    .get("type")
                    .map(|t| t.to_ascii_lowercase())
                    .unwrap_or_else(|| ctl[1].to_ascii_lowercase());
                if matches!(
                    tipo_html.as_str(),
                    "submit" | "button" | "reset" | "image" | "file"
                ) || nombre == "sessionKey"
                {
                    continue;
                }
                let despues = cuerpo[todo.end()..].split('<').next().unwrap_or("");
                let antes = TEXTO
                    .captures_iter(&cuerpo[..todo.start()])
                    .last()
                    .map(|c| c[1].to_owned())
                    .unwrap_or_default();
                let etiqueta_despues = limpiar_etiqueta(despues);
                let etiqueta_antes = limpiar_etiqueta(&antes);
                if let Some(p) = PARTE.captures(&nombre) {
                    let n: usize = p[2].parse().unwrap_or(0);
                    let e = partes
                        .entry(p[1].to_owned())
                        .or_insert_with(|| (0, etiqueta_antes.clone()));
                    e.0 = e.0.max(n + 1);
                    continue;
                }
                let mut campo = Campo {
                    nombre: nombre.clone(),
                    etiqueta: if etiqueta_antes.is_empty() || tipo_html == "hidden" {
                        humanizar(&nombre)
                    } else {
                        etiqueta_antes
                    },
                    tipo: TipoParam::Text,
                    opciones: Vec::new(),
                    partido: None,
                };
                match tipo_html.as_str() {
                    "radio" => {
                        let v = a.get("value").cloned().unwrap_or_default();
                        if let Some(prev) = campos.iter_mut().find(|c| c.nombre == nombre) {
                            prev.opciones.push((v, etiqueta_despues));
                            continue;
                        }
                        campo.tipo = TipoParam::Enum;
                        campo.opciones.push((v, etiqueta_despues));
                    }
                    "checkbox" => {
                        campo.tipo = TipoParam::Bool;
                        if !etiqueta_despues.is_empty() {
                            campo.etiqueta = etiqueta_despues;
                        }
                    }
                    "select" => {
                        campo.opciones = OPCION
                            .captures_iter(opciones_select(&cuerpo[todo.end()..]))
                            .map(|o| (o[1].to_owned(), limpiar_etiqueta(&o[2])))
                            .collect();
                        campo.tipo = if campo.opciones.is_empty() {
                            tipo_por_nombre(m, &nombre)
                        } else {
                            TipoParam::Enum
                        };
                    }
                    "password" => campo.tipo = TipoParam::Secret,
                    _ => campo.tipo = tipo_por_nombre(m, &nombre),
                }
                if m.es_secreto(&nombre) {
                    campo.tipo = TipoParam::Secret;
                    campo.opciones.clear();
                }
                if let Some(prev) = campos.iter_mut().find(|c| c.nombre == nombre) {
                    // Repetido (p. ej. versión móvil): se queda el primero.
                    if prev.tipo == TipoParam::Text && campo.tipo != TipoParam::Text {
                        *prev = campo;
                    }
                    continue;
                }
                campos.push(campo);
            }
            for (base, (n, etiqueta)) in partes {
                let (tipo, partido) = match n {
                    6 => (TipoParam::Mac, Partido::Mac6),
                    _ => (TipoParam::Ip, Partido::Ip4),
                };
                let etiqueta = if etiqueta.is_empty() {
                    humanizar(&base)
                } else {
                    etiqueta
                };
                campos.retain(|c| c.nombre != base);
                campos.push(Campo {
                    nombre: base,
                    etiqueta,
                    tipo,
                    opciones: Vec::new(),
                    partido: Some(partido),
                });
            }
            out.insert((f.ruta.clone(), id), campos);
        }
    }
    out
}

/// Una URL de la UI básica (`*.cmd|.cgi|.wl`) construida en el código.
struct Url {
    nombre: String,
    pagina: String,
    claves: Vec<String>,
}

static URL_INICIO: LazyLock<Regex> =
    LazyLock::new(|| re(r#"\w+\s*=\s*['"][^'"]*?\b(\w+\.(?:cmd|cgi|wl))\b[^'"]*['"]"#));
static URL_FIN: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?:\blocation(?:\.href)?\s*=|['"]location=|\$\.(?:get|post|ajax)\s*\(|window\.open|\bfunction\s+\w+\s*\()"#,
    )
});
static QUERY: LazyLock<Regex> = LazyLock::new(|| re(r#"(?:[?&]|['"])(\w+)="#));
static ACCION: LazyLock<Regex> = LazyLock::new(|| re(r"action=(\w+)"));

fn urls(fuentes: &[Fuente]) -> Vec<Url> {
    let mut out = Vec::new();
    for f in fuentes.iter().filter(|f| f.es_pagina()) {
        let t = &f.texto;
        let inicios: Vec<_> = URL_INICIO.captures_iter(t).collect();
        for (i, c) in inicios.iter().enumerate() {
            let ini = c.get(0).expect("grupo 0").start();
            let tope = inicios
                .get(i + 1)
                .map_or(t.len(), |s| s.get(0).expect("grupo 0").start())
                .min(ini + 12000);
            let tras = c.get(0).expect("grupo 0").end();
            let fin = URL_FIN
                .find(&t[tras..tope])
                .map_or(tope, |m| tras + m.end());
            let bloque = &t[ini..fin];
            let mut claves: Vec<String> = Vec::new();
            for q in QUERY.captures_iter(bloque) {
                let k = q[1].to_owned();
                if !["action", "sessionKey", "location"].contains(&k.as_str())
                    && !claves.contains(&k)
                {
                    claves.push(k);
                }
            }
            let mut acciones: Vec<String> = ACCION
                .captures_iter(bloque)
                .map(|a| a[1].to_owned())
                .collect();
            acciones.dedup();
            let ep = c[1].to_owned();
            if acciones.is_empty() {
                out.push(Url {
                    nombre: ep,
                    pagina: f.ruta.clone(),
                    claves,
                });
            } else {
                for a in acciones {
                    out.push(Url {
                        nombre: format!("{ep}?action={a}"),
                        pagina: f.ruta.clone(),
                        claves: claves.clone(),
                    });
                }
            }
        }
    }
    out
}

static REFERENCIA: LazyLock<Regex> = LazyLock::new(|| re(r#"([\w/.-]+\.asp)\b"#));

/// Páginas alcanzables desde los menús (sin contar referencias comentadas).
fn alcanzables(fuentes: &[Fuente]) -> BTreeSet<String> {
    let por_ruta: HashMap<&str, &Fuente> = fuentes.iter().map(|f| (f.ruta.as_str(), f)).collect();
    let mut vistas = BTreeSet::new();
    let mut pila: Vec<String> = [
        "index.asp",
        "login.asp",
        "monu.asp",
        "top.asp",
        "bottom.asp",
        "te_acceso_router.asp",
        "te_header.asp",
        "te_footer.asp",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    while let Some(p) = pila.pop() {
        if !vistas.insert(p.clone()) {
            continue;
        }
        let mut textos = Vec::new();
        if let Some(f) = por_ruta.get(p.as_str()) {
            textos.push(f.texto.as_str());
            for s in SCRIPT_SRC.captures_iter(&f.texto) {
                if let Some(js) = por_ruta.get(s[1].trim_start_matches('/')) {
                    textos.push(js.texto.as_str());
                }
            }
        }
        for t in textos {
            for r in REFERENCIA.captures_iter(t) {
                let r = r[1].trim_start_matches('/').to_owned();
                if por_ruta.contains_key(r.as_str()) && !vistas.contains(&r) {
                    pila.push(r);
                }
            }
        }
    }
    vistas
}

static SCRIPT_SRC: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)<script\b[^>]*src\s*=\s*["']([^"'?]+\.js)"#));

/// Páginas que incluyen cada script (o la propia página si es un `.asp`).
fn paginas_de(fuentes: &[Fuente], ruta: &str) -> Vec<String> {
    if ruta.ends_with(".asp") {
        return vec![ruta.to_owned()];
    }
    let mut v: Vec<String> = fuentes
        .iter()
        .filter(|f| f.es_pagina())
        .filter(|f| {
            SCRIPT_SRC
                .captures_iter(&f.texto)
                .any(|c| c[1].trim_start_matches('/') == ruta)
        })
        .map(|f| f.ruta.clone())
        .collect();
    v.sort();
    v
}

/// Área por página.
fn area_de_pagina(p: &str) -> Area {
    let p = p.rsplit('/').next().unwrap_or(p).trim_end_matches(".asp");
    match p {
        "ipv4" | "dhcp" | "bridging" | "filtering" | "lanstats" | "hoststats" | "arpstats"
        | "te_red_local" | "te_mapa_red_local" => Area::LanDhcp,
        "ipv6" | "te_ipv6" => Area::Ipv6,
        "wanintf"
        | "wanstats"
        | "routing_rules"
        | "default_gateway"
        | "rip"
        | "te_multipuesto_monopuesto" => Area::Wan,
        "natintf" | "natdmz" | "natalg" | "natloopback" | "natportmap" | "upnp" | "te_puertos" => {
            Area::Nat
        }
        "dnsrelay" | "ddns" => Area::Dns,
        "wifi" | "wifi5g" | "te_wifi" | "te_wifi_5ghz" | "te_wifi_guest" => Area::Wifi,
        "multicastintf" | "igmp" => Area::Multicast,
        "queue" | "classification" | "marking" => Area::Qos,
        "voicebasic" | "voiceadvance" | "pbxstats" => Area::Voip,
        "cwmp" => Area::Tr069,
        "user_profile"
        | "user_profile_add"
        | "user_profile_modifyOthers"
        | "te_cambio_contrasena"
        | "te_acceso_router"
        | "login" => Area::Usuarios,
        "scbidirectionalflt_ls" | "scbidirectionalflt_ed" => Area::Cortafuegos,
        "notsupport" => Area::Ocultas,
        _ => Area::Sistema,
    }
}

fn tipo_de(name: &str) -> Tipo {
    let base = name.split('?').next().unwrap_or(name);
    let accion = name.split("action=").nth(1).unwrap_or("");
    if matches!(
        base,
        "cbReboot"
            | "cbSaveConfig"
            | "cbRestoreConfig"
            | "cbFWUpgradeByFTP"
            | "cbDownloadConfig"
            | "cbCheckLogin"
            | "cbCheckLogout"
            | "cbModifyPwd"
            | "cbModifyPwdOther"
            | "cbSyncTime"
            | "restoreinfo.cgi"
            | "te_acceso_router.cgi"
            | "te_logout.cgi"
            | "te_cambio_contrasena.cgi"
    ) || base.starts_with("cbTrigger")
    {
        Tipo::Accion
    } else if base.starts_with("cbGet")
        || base.starts_with("cbList")
        || base.starts_with("cbCheck")
        || base.contains("Get")
        || base.contains("refresh")
        || matches!(accion, "retrieve" | "view")
    {
        Tipo::Lectura
    } else {
        Tipo::Escritura
    }
}

fn riesgo_de(f: &Funcion) -> Riesgo {
    if f.tipo == Tipo::Lectura {
        return Riesgo::Bajo;
    }
    match f.area {
        _ if f.name == "cbReboot" || f.name.starts_with("cbSetLanIPv4") => Riesgo::Alto,
        Area::Wan | Area::Cortafuegos | Area::Tr069 => Riesgo::Alto,
        Area::Wifi | Area::Dns => Riesgo::Medio,
        _ if f.name == "cbSetDhcpv4ServerPool" => Riesgo::Medio,
        _ => Riesgo::Bajo,
    }
}

/// Páginas que nunca deben cargarse o subidas: `(ruta, motivo)`.
const PELIGROSAS: &[(&str, &str)] = &[
    (
        "syslog.asp",
        "al cargarse llama a cbSetSyslog (exportación del registro)",
    ),
    (
        "backup_restore.asp",
        "al cargarse llama a cbSaveConfig (copia de la configuración)",
    ),
    (
        "upload/cfgupload.asp",
        "sube y aplica un fichero de configuración",
    ),
    ("upload/imgupgrade.asp", "sube y graba un firmware"),
    (
        "te_uploadsettings.asp",
        "sube y aplica un fichero de configuración (UI básica)",
    ),
    ("te_uploadinfo.asp", "sube y graba un firmware (UI básica)"),
];

/// Respuesta capturada que no es `<Response>` (la función no existe).
fn respuesta_rota(dir: &Path, cb: &str) -> bool {
    let p = dir.join("xml").join(format!("{cb}.xml"));
    std::fs::read_to_string(p).is_ok_and(|t| !t.contains("<Response>"))
}

fn parametro(c: &Campo) -> Parametro {
    Parametro {
        nombre: c.nombre.clone(),
        etiqueta: c.etiqueta.clone(),
        tipo: c.tipo,
        opciones: c.opciones.clone(),
        partido: c.partido,
        tapado: false,
    }
}

fn pendiente(origen: &str) -> String {
    format!("PENDIENTE: {origen}")
}

/// Extrae el borrador de catálogo.
pub fn extraer(dir: &Path) -> Result<Vec<Funcion>, String> {
    let m = Mascara::load().map_err(|e| e.to_string())?;
    let fuentes = cargar(dir)?;
    let enlazadas = alcanzables(&fuentes);
    let forms = formularios(&fuentes, &m);
    let sitios = llamadas(&fuentes);

    let origen_pagina = |p: &str| -> Origen {
        if p.starts_with("te_") {
            Origen::Basica
        } else if enlazadas.contains(p) {
            Origen::Avanzada
        } else {
            Origen::Oculta
        }
    };

    let mut por_cb: BTreeMap<&str, Vec<&Sitio>> = BTreeMap::new();
    for s in &sitios {
        por_cb.entry(s.cb.as_str()).or_default().push(s);
    }

    let mut out: Vec<Funcion> = Vec::new();
    for (cb, ss) in &por_cb {
        let mut paginas: Vec<String> = ss
            .iter()
            .flat_map(|s| paginas_de(&fuentes, &s.fuente))
            .collect();
        paginas.sort();
        paginas.dedup();
        let mut params: Vec<Parametro> = Vec::new();
        let mut pagina_form = None;
        let añadir = |p: Parametro, params: &mut Vec<Parametro>| {
            if p.nombre != "sessionKey" && !params.iter().any(|q| q.nombre == p.nombre) {
                params.push(p);
            }
        };
        for s in ss {
            for id in &s.formularios {
                for p in &paginas {
                    if let Some(campos) = forms.get(&(p.clone(), id.clone())) {
                        pagina_form.get_or_insert_with(|| p.clone());
                        for c in campos {
                            añadir(parametro(c), &mut params);
                        }
                    }
                }
            }
            for (k, v) in &s.params {
                let tipo = tipo_por_nombre(&m, k);
                let etiqueta = match v {
                    Some(v) if k == "all" || k == "del" => {
                        format!("{} (siempre «{v}»)", humanizar(k))
                    }
                    _ => humanizar(k),
                };
                añadir(
                    Parametro {
                        nombre: k.clone(),
                        etiqueta,
                        tipo,
                        opciones: Vec::new(),
                        partido: None,
                        tapado: false,
                    },
                    &mut params,
                );
            }
        }
        let origenes: Vec<Origen> = paginas.iter().map(|p| origen_pagina(p)).collect();
        let origen = if respuesta_rota(dir, cb) {
            Origen::Incompleta
        } else if origenes.contains(&Origen::Avanzada) {
            Origen::Avanzada
        } else if origenes.contains(&Origen::Basica) {
            Origen::Basica
        } else {
            Origen::Oculta
        };
        let pagina = pagina_form.or_else(|| {
            paginas
                .iter()
                .find(|p| origen_pagina(p) == origen)
                .or(paginas.first())
                .cloned()
        });
        let area = pagina.as_deref().map_or(Area::Sistema, area_de_pagina);
        let all = ss.iter().any(|s| {
            s.params
                .iter()
                .any(|(k, v)| k == "all" && v.as_deref() == Some("ALL"))
        });
        let del = ss.iter().any(|s| {
            s.params
                .iter()
                .any(|(k, v)| k == "del" && v.as_deref() == Some("DEL"))
        });
        let id_param = params
            .iter()
            .map(|p| p.nombre.as_str())
            .filter(|n| n.ends_with("_0i") || n.ends_with("_id"))
            .max_by_key(|n| n.len())
            .map(str::to_owned);
        let lista = match (&id_param, all || del) {
            (Some(id), true) => Some(Lista {
                id_param: id.clone(),
                all,
                del,
            }),
            _ => None,
        };
        if del && lista.is_some() {
            // `del=DEL` lo añade el gestor de cambios al borrar.
            params.retain(|p| p.nombre != "del");
        }
        let mut f = Funcion {
            name: (*cb).to_owned(),
            area,
            origen,
            tipo: tipo_de(cb),
            pagina,
            descripcion: pendiente(&paginas.join(", ")),
            parametros: params,
            lectura_pareada: None,
            lista,
            riesgo: Riesgo::Bajo,
            bloqueada: None,
        };
        f.riesgo = riesgo_de(&f);
        out.push(f);
    }

    // UI básica y otras URL `.cmd|.cgi|.wl`.
    let mut vistas: BTreeMap<String, usize> = BTreeMap::new();
    for u in urls(&fuentes) {
        let params: Vec<Parametro> = u
            .claves
            .iter()
            .map(|k| Parametro {
                nombre: k.clone(),
                etiqueta: humanizar(k),
                tipo: tipo_por_nombre(&m, k),
                opciones: Vec::new(),
                partido: None,
                tapado: false,
            })
            .collect();
        if let Some(&i) = vistas.get(&u.nombre) {
            for p in params {
                if out[i].parametro(&p.nombre).is_none() {
                    out[i].parametros.push(p);
                }
            }
            continue;
        }
        let mut f = Funcion {
            name: u.nombre.clone(),
            area: area_de_pagina(&u.pagina),
            origen: origen_pagina(&u.pagina),
            tipo: tipo_de(&u.nombre),
            pagina: Some(u.pagina.clone()),
            descripcion: pendiente(&u.pagina),
            parametros: params,
            lectura_pareada: None,
            lista: None,
            riesgo: Riesgo::Bajo,
            bloqueada: None,
        };
        f.riesgo = riesgo_de(&f);
        vistas.insert(u.nombre, out.len());
        out.push(f);
    }

    // Páginas ocultas (no alcanzables desde ningún menú) y peligrosas.
    for f in fuentes.iter().filter(|f| f.es_pagina()) {
        let peligrosa = PELIGROSAS.iter().find(|(p, _)| *p == f.ruta);
        let oculta = !enlazadas.contains(&f.ruta) && !f.ruta.starts_with("te_");
        if peligrosa.is_none() && !oculta {
            continue;
        }
        let mut func = Funcion {
            name: f.ruta.clone(),
            area: area_de_pagina(&f.ruta),
            origen: if f.ruta.starts_with("te_") {
                Origen::Basica
            } else if enlazadas.contains(&f.ruta) {
                Origen::Avanzada
            } else {
                Origen::Oculta
            },
            tipo: if peligrosa.is_some() {
                Tipo::Accion
            } else {
                Tipo::Lectura
            },
            pagina: Some(f.ruta.clone()),
            descripcion: pendiente(if f.original.contains("<!--") {
                "página con partes comentadas"
            } else {
                "página"
            }),
            parametros: Vec::new(),
            lectura_pareada: None,
            lista: None,
            riesgo: Riesgo::Bajo,
            bloqueada: peligrosa.map(|(_, m)| m.to_string()),
        };
        func.riesgo = riesgo_de(&func);
        if !out.iter().any(|g| g.name == func.name) {
            out.push(func);
        }
    }

    emparejar(&mut out, &sitios);
    Ok(out)
}

/// Lectura pareada de cada escritura: `Set`→`Get`, `Add`/`Delete`→lista.
fn emparejar(out: &mut [Funcion], sitios: &[Sitio]) {
    let nombres: BTreeSet<String> = out
        .iter()
        .filter(|f| f.tipo == Tipo::Lectura)
        .map(|f| f.name.clone())
        .collect();
    for f in out.iter_mut().filter(|f| f.tipo == Tipo::Escritura) {
        let candidatos = [
            f.name.replacen("Set", "Get", 1),
            f.name.replacen("cbAdd", "cbGet", 1) + "List",
            f.name.replacen("cbDelete", "cbGet", 1) + "List",
        ];
        let Some(get) = candidatos.into_iter().find(|c| nombres.contains(c)) else {
            continue;
        };
        // Parámetros con los que se lee una instancia (no `all`).
        let claves: BTreeSet<&str> = sitios
            .iter()
            .filter(|s| s.cb == get && !s.params.iter().any(|(k, _)| k == "all"))
            .flat_map(|s| s.params.iter().map(|(k, _)| k.as_str()))
            .collect();
        let params = claves
            .into_iter()
            .filter_map(|k| {
                let id = k.strip_suffix("_0i").map(|b| format!("{b}_id"));
                [Some(k.to_owned()), id]
                    .into_iter()
                    .flatten()
                    .find(|n| f.parametro(n).is_some())
                    .map(|n| (k.to_owned(), format!("{{{n}}}")))
            })
            .collect();
        f.lectura_pareada = Some(LecturaPareada {
            funcion: get,
            params,
        });
    }
}

#[derive(serde::Serialize)]
struct Fichero<'a> {
    funcion: &'a [Funcion],
}

/// Serializa el borrador.
pub fn a_toml(funciones: &[Funcion]) -> Result<String, String> {
    let cuerpo = toml::to_string(&Fichero { funcion: funciones }).map_err(|e| e.to_string())?;
    Ok(format!(
        "# BORRADOR generado por gen-catalogo: revisar a mano antes de copiar a\n\
         # crates/catalogo/funciones.toml (descripciones, áreas, tipos, riesgos).\n\n{cuerpo}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quita_comentarios_respetando_cadenas() {
        let t = limpiar_js("a = '//no'; // sí\n/* b\n c */ d = \"/*x*/\";");
        assert!(t.contains("'//no'") && t.contains("\"/*x*/\""));
        assert!(!t.contains("sí") && !t.contains(" b"));
        assert_eq!(t.lines().count(), 3);
        let h =
            limpiar_asp("<a url=\"x.asp\"><!-- <a url=\"y.asp\"> --><script>// z.asp\n</script>");
        assert!(h.contains("x.asp") && !h.contains("y.asp") && !h.contains("z.asp"));
    }

    #[test]
    fn llamadas_con_objeto_formulario_y_variable() {
        let js = Fuente {
            ruta: "js/x.js".into(),
            original: String::new(),
            texto: limpiar_js(
                r##"
function getX(zi) { var goformCbk = "cbGetX";
  $.post("/cgi-bin/" + goformCbk + ".xml", { "x_0i": zi, "all": "ALL" }, function(r) {}, "xml"); }
function setX(selector) {
  $(selector).append('<input type="hidden" name="sessionKey" value="' + k + '">');
  var goformCbk = "cbSetX";
  $.post("/cgi-bin/" + goformCbk + ".xml", $(selector).serialize(), function(r) {}); }
$("#F").submit(function(e) { setX("#F"); });
function delX(id) { var obj = { "x_id": id }; obj["del"] = "DEL";
  var goformCbk = "cbSetX"; $.post("/cgi-bin/" + goformCbk + ".xml", obj, function(r) {}); }
"##,
            ),
        };
        let s = llamadas(std::slice::from_ref(&js));
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].cb, "cbGetX");
        assert_eq!(
            s[0].params,
            [("x_0i".into(), None), ("all".into(), Some("ALL".into()))]
        );
        assert_eq!(s[1].formularios, ["F"]);
        assert_eq!(
            s[2].params,
            [("x_id".into(), None), ("del".into(), Some("DEL".into()))]
        );
    }

    #[test]
    fn formularios_con_campos_partidos() {
        let m = Mascara::load().unwrap();
        let p = Fuente {
            ruta: "x.asp".into(),
            original: String::new(),
            texto: limpiar_asp(
                r#"<form id="F"><input type="radio" name="adm_state" value="1">Enable
<input type="radio" name="adm_state" value="0">Disable
<label>Start</label><input type="text" name="ip-0"><input name="ip-1"><input name="ip-2"><input name="ip-3"><input type="hidden" name="ip">
<input name="mac-0"><input name="mac-1"><input name="mac-2"><input name="mac-3"><input name="mac-4"><input name="mac-5">
<select name="mode"><option value="a">A</option><option value="b">B</option></select>
<input type="checkbox" name="wmm">WMM <input type="text" name="key_phrase"> <input type="hidden" name="x_id">
<input type="text" name="acs_username"><input type="text" name="acs_URL">
<input type="submit" value="Apply"></form>"#,
            ),
        };
        let f = formularios(std::slice::from_ref(&p), &m);
        let c = &f[&("x.asp".to_owned(), "F".to_owned())];
        let por = |n: &str| c.iter().find(|c| c.nombre == n).unwrap();
        assert_eq!(por("adm_state").tipo, TipoParam::Enum);
        assert_eq!(por("adm_state").opciones.len(), 2);
        assert_eq!(por("ip").partido, Some(Partido::Ip4));
        assert_eq!(por("ip").etiqueta, "Start");
        assert_eq!(por("mac").tipo, TipoParam::Mac);
        assert_eq!(por("mode").opciones[1], ("b".into(), "B".into()));
        assert_eq!(por("wmm").tipo, TipoParam::Bool);
        assert_eq!(por("key_phrase").tipo, TipoParam::Secret);
        // Un usuario o una URL es texto, que se ve en claro; solo las
        // credenciales son secretas.
        assert_eq!(por("acs_username").tipo, TipoParam::Text);
        assert_eq!(por("acs_URL").tipo, TipoParam::Text);
        assert_eq!(por("x_id").tipo, TipoParam::Int);
        assert!(c.iter().all(|c| !c.nombre.contains('-')));
    }

    #[test]
    fn urls_de_la_ui_basica() {
        let p = Fuente {
            ruta: "te_x.asp".into(),
            original: String::new(),
            texto: limpiar_asp(
                "<script>var loc = '/cgi-bin/te_x.cmd?action=add&portName=' + a;\n\
                 loc += '&wlWpaPsk=' + b; loc += '&sessionKey=' + k;\n\
                 var code = 'location=\"' + loc + '\"'; eval(code);\n\
                 var loc = '/cgi-bin/te_x.wl?wlSsidIdx=0'; location = loc;</script>",
            ),
        };
        let u = urls(std::slice::from_ref(&p));
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].nombre, "te_x.cmd?action=add");
        assert_eq!(u[0].claves, ["portName", "wlWpaPsk"]);
        assert_eq!(u[1].nombre, "te_x.wl");
        assert_eq!(u[1].claves, ["wlSsidIdx"]);
    }
}
