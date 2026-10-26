//! `docs/api.md`: el catálogo revisado en formato legible.

use catalogo::{
    Acceso, Area, Catalogo, Funcion, Origen, Riesgo, Tipo, TipoParam,
    mask::{Mascara, Nivel},
};
use regex::Regex;
use std::{collections::BTreeSet, fmt::Write as _, path::Path};

fn tipo(t: Tipo) -> &'static str {
    match t {
        Tipo::Lectura => "lectura",
        Tipo::Escritura => "escritura",
        Tipo::Accion => "acción",
    }
}

fn origen(o: Origen) -> &'static str {
    match o {
        Origen::Basica => "UI básica",
        Origen::Avanzada => "UI avanzada",
        Origen::Oculta => "oculta",
        Origen::Incompleta => "incompleta",
    }
}

fn riesgo(r: Riesgo) -> &'static str {
    match r {
        Riesgo::Bajo => "bajo",
        Riesgo::Medio => "medio",
        Riesgo::Alto => "alto",
    }
}

fn acceso(a: Acceso) -> &'static str {
    match a {
        Acceso::PuedeCortar => "se vigila",
        Acceso::NoCorta => "sin vigilancia",
        Acceso::Wifi => "se vigila si el panel va por WiFi",
    }
}

fn tipo_param(t: TipoParam) -> &'static str {
    match t {
        TipoParam::Ip => "ip",
        TipoParam::Mac => "mac",
        TipoParam::Bool => "bool",
        TipoParam::Enum => "enum",
        TipoParam::Int => "int",
        TipoParam::Text => "text",
        TipoParam::Secret => "secret",
    }
}

/// Nombres de elementos de las respuestas saneadas de `crates/hgu/fixtures`
/// para la función `name` (sin valores).
fn elementos(fixtures: &Path, name: &str) -> Vec<String> {
    let etiqueta = Regex::new(r"<([A-Za-z_][\w.-]*)>").expect("regex válida");
    let mut out = BTreeSet::new();
    let Ok(rd) = std::fs::read_dir(fixtures) else {
        return Vec::new();
    };
    for e in rd.filter_map(|e| e.ok()) {
        let f = e.file_name().to_string_lossy().into_owned();
        let stem = f.trim_end_matches(".xml");
        if stem.split("__").next() != Some(name) {
            continue;
        }
        if let Ok(t) = std::fs::read_to_string(e.path()) {
            for c in etiqueta.captures_iter(&t) {
                if !["Response", "Result", name].contains(&&c[1]) {
                    out.insert(c[1].to_owned());
                }
            }
        }
    }
    out.into_iter().collect()
}

fn escapar(s: &str) -> String {
    s.replace('|', "\\|")
}

fn funcion(md: &mut String, f: &Funcion, m: &Mascara, fixtures: &Path) {
    let _ = writeln!(md, "### `{}`\n", f.name);
    let mut linea = format!(
        "*{}* · {} · riesgo {}",
        tipo(f.tipo),
        origen(f.origen),
        riesgo(f.riesgo)
    );
    if let Some(a) = f.acceso {
        let _ = write!(linea, " · {}", acceso(a));
    }
    if let Some(p) = &f.pagina {
        let _ = write!(linea, " · página `{p}`");
    }
    let _ = writeln!(md, "{linea}\n\n{}\n", f.descripcion);
    if let Some(b) = &f.bloqueada {
        let _ = writeln!(md, "**Bloqueada:** {b}\n");
    }
    if let Some(lp) = &f.lectura_pareada {
        let params: Vec<String> = lp
            .params
            .iter()
            .map(|(k, v)| format!("`{k}={v}`"))
            .collect();
        let _ = writeln!(
            md,
            "Lectura pareada: `{}`{}\n",
            lp.funcion,
            if params.is_empty() {
                String::new()
            } else {
                format!(" con {}", params.join(", "))
            }
        );
    }
    if let Some(l) = &f.lista {
        let mut linea = format!("Lista: instancia por `{}`", l.id_param);
        if l.all {
            linea.push_str("; `all=ALL` lista todas");
        }
        if l.del {
            linea.push_str("; `del=DEL` borra");
        }
        let _ = writeln!(md, "{linea}\n");
    }
    if !f.parametros.is_empty() {
        md.push_str("| Parámetro | Etiqueta | Tipo | Opciones |\n|---|---|---|---|\n");
        for p in &f.parametros {
            let mut t = tipo_param(p.tipo).to_owned();
            if let Some(pt) = p.partido {
                let _ = write!(t, " ({:?})", pt);
            }
            if p.tipo != TipoParam::Secret && m.es_secreto(&p.nombre) {
                t.push_str(" ⚠");
            }
            let op: Vec<String> = p
                .opciones
                .iter()
                .map(|(v, e)| format!("`{v}` {e}"))
                .collect();
            let _ = writeln!(
                md,
                "| `{}` | {} | {} | {} |",
                p.nombre,
                escapar(&p.etiqueta),
                t,
                escapar(&op.join(", "))
            );
        }
        md.push('\n');
    }
    if f.tipo == Tipo::Lectura {
        let els = elementos(fixtures, &f.name);
        if !els.is_empty() {
            let els: Vec<String> = els
                .iter()
                .map(|e| match m.nivel(e) {
                    Some(Nivel::Secreto) => format!("`{e}` 🔒"),
                    Some(Nivel::Sensible) => format!("`{e}` (sensible)"),
                    None => format!("`{e}`"),
                })
                .collect();
            let _ = writeln!(md, "Elementos de la respuesta: {}\n", els.join(", "));
        }
    }
}

/// Texto de `docs/api.md`.
pub fn generar(cat: &Catalogo, fixtures: &Path) -> Result<String, String> {
    let m = Mascara::load().map_err(|e| e.to_string())?;
    let mut md = String::from(
        "# API del router Askey RTF8115VW (Movistar/O2)\n\n\
         <!-- Generado por `cargo run -p gen-catalogo -- --api-md` a partir de\n\
         crates/catalogo/funciones.toml. No editar a mano. -->\n\n\
         Las funciones del propio panel (`/api/…`) están en\n\
         [funciones-del-panel.md](funciones-del-panel.md).\n\n\
         ## Cómo se llama\n\n\
         - **UI avanzada**: `POST /cgi-bin/<función>.xml` con los parámetros como\n  \
           formulario. Inicio de sesión con `cbCheckLogin.xml` (`rqUsername`,\n  \
           `rqPasswd` ofuscados con XOR 0x1f, `rqTimeout`); la respuesta es\n  \
           `<Response><función>…<Result>0</Result></función></Response>` (`Result` 0\n  \
           = correcto, 1 = error con `ErrorString`).\n\
         - Las escrituras llevan además `sessionKey`, la clave de sesión que da la\n  \
           página indicada. Las listas se leen con `all=ALL` (ids) y cada\n  \
           instancia con `<obj>_0i=<id>` o `<obj>_id=<id>`; los borrados llevan\n  \
           `del=DEL`; `-1` como id crea una instancia nueva.\n\
         - Los campos IP/MAC partidos (`<nombre>-0..-3` o `-0..-5`) se envían unidos\n  \
           en `<nombre>` (`beforeSendForm` de `tabs.js`).\n\
         - **UI básica**: `GET /cgi-bin/<página>.cmd|.cgi|.wl?…` con los valores en\n  \
           la URL, `sessionKey` y la cookie `_httpdSessionId_`; las claves WiFi y\n  \
           contraseñas van ofuscadas con XOR 0x1f.\n\
         - Riesgo: vigilancia tras el cambio de 30 s (bajo), 2 min (medio) o 5 min\n  \
           (alto). Las lecturas no tienen riesgo. Las funciones **bloqueadas** no se\n  \
           invocan nunca desde el panel.\n\
         - Vigilancia: solo se vigila lo que puede cortar el acceso del panel al\n  \
           router o la conexión a internet («se vigila»); lo demás se da por\n  \
           aplicado en cuanto la relectura muestra lo nuevo («sin vigilancia»).\n  \
           La WiFi se vigila salvo que el panel compruebe que llega por cable.\n\
         - En la lectura pareada, `{param}` es el valor de ese parámetro en la\n  \
           escritura. Los elementos marcados con 🔒 son secretos (en pantalla,\n  \
           censurados con el ojo) y los marcados «(sensible)», datos personales\n  \
           que en pantalla se ven en claro; los dos se enmascaran siempre en los\n  \
           registros (`crates/catalogo/secretos.toml`).\n\n",
    );
    let _ = writeln!(
        md,
        "## Resumen\n\n| Área | Funciones | Lecturas | Escrituras | Acciones | Bloqueadas |\n|---|---|---|---|---|---|"
    );
    for a in Area::TODAS {
        let fs: Vec<&Funcion> = cat.por_area(*a).collect();
        let n = |t: Tipo| fs.iter().filter(|f| f.tipo == t).count();
        let _ = writeln!(
            md,
            "| [{}](#{}) | {} | {} | {} | {} | {} |",
            a.etiqueta(),
            ancla(a.etiqueta()),
            fs.len(),
            n(Tipo::Lectura),
            n(Tipo::Escritura),
            n(Tipo::Accion),
            fs.iter().filter(|f| f.bloqueada.is_some()).count()
        );
    }
    md.push('\n');
    for a in Area::TODAS {
        let _ = writeln!(md, "## {}\n", a.etiqueta());
        for f in cat.por_area(*a) {
            funcion(&mut md, f, &m, fixtures);
        }
    }
    Ok(md)
}

fn ancla(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == ' ')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// Escribe `docs/api.md`.
pub fn escribir(salida: &Path) -> Result<(), String> {
    let cat = Catalogo::load().map_err(|e| e.to_string())?;
    let fixtures = crate::raiz().join("crates/hgu/fixtures");
    let md = generar(&cat, &fixtures)?;
    std::fs::write(salida, md).map_err(|e| format!("{}: {e}", salida.display()))?;
    eprintln!("{} funciones → {}", cat.todas().len(), salida.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_md_esta_al_dia() {
        let cat = Catalogo::load().unwrap();
        let md = generar(&cat, &crate::raiz().join("crates/hgu/fixtures")).unwrap();
        let actual = std::fs::read_to_string(crate::raiz().join("docs/api.md")).unwrap_or_default();
        assert!(
            md == actual,
            "docs/api.md desactualizado: cargo run -p gen-catalogo -- --api-md"
        );
        for f in cat.todas() {
            assert!(md.contains(&format!("### `{}`", f.name)), "{}", f.name);
        }
    }

    /// Cada función del panel que documenta `docs/funciones-del-panel.md`
    /// existe (`endpoint = "…"`).
    #[test]
    fn funciones_del_panel_existen() {
        let doc =
            std::fs::read_to_string(crate::raiz().join("docs/funciones-del-panel.md")).unwrap();
        let mut fuentes = String::new();
        let mut pendientes = vec![crate::raiz().join("crates/panel/src")];
        while let Some(dir) = pendientes.pop() {
            for e in std::fs::read_dir(&dir).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    pendientes.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    fuentes.push_str(&std::fs::read_to_string(&p).unwrap());
                }
            }
        }
        let fila = Regex::new(r"(?m)^\| `([a-z_]+)` \|").unwrap();
        let nombres: Vec<&str> = fila
            .captures_iter(&doc)
            .map(|c| c.get(1).unwrap().as_str())
            .collect();
        assert!(nombres.len() >= 10, "{nombres:?}");
        for n in nombres {
            assert!(
                fuentes.contains(&format!("endpoint = \"{n}\"")),
                "funciones-del-panel.md: {n} no es una server function del panel"
            );
        }
    }

    /// La documentación describe la API con texto propio: ningún fragmento
    /// literal de JavaScript o HTML de las páginas del router.
    #[test]
    fn api_md_sin_codigo_del_firmware() {
        let cat = Catalogo::load().unwrap();
        let md = generar(&cat, &crate::raiz().join("crates/hgu/fixtures")).unwrap();
        let toml =
            std::fs::read_to_string(crate::raiz().join("crates/catalogo/funciones.toml")).unwrap();
        for (nombre, texto) in [("docs/api.md", &md), ("funciones.toml", &toml)] {
            for fragmento in [
                "function(",
                "function (",
                "<div",
                "<script",
                "var ",
                "$(",
                "document.",
            ] {
                assert!(
                    !texto.contains(fragmento),
                    "{nombre} contiene «{fragmento}», código literal del firmware"
                );
            }
        }
    }
}
