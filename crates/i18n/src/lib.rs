//! Textos de la interfaz del panel en español e inglés.
//!
//! Fachada propia sobre `fluent-bundle`: los mensajes están en
//! `locales/{es,en}/*.ftl`, se incrustan con `include_str!` y cada idioma
//! monta su `FluentBundle` la primera vez que se usa. La misma crate sirve al
//! servidor (SSR) y al navegador (hidratación), así que los dos pintan el
//! mismo texto.
//!
//! Una clave que falta en un idioma se sirve en español y se avisa (registro
//! del servidor o consola del navegador). Las pruebas de `tests/claves.rs`
//! comprueban que los dos idiomas tienen las mismas claves y que existen las
//! que usa el panel.

use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use unic_langid::LanguageIdentifier;

pub use fluent_bundle::{FluentArgs, FluentValue};

/// Registra los ficheros `.ftl`: un fichero nuevo se añade solo aquí (y en
/// `locales/es` y `locales/en`).
macro_rules! locales {
    ($($nombre:literal),* $(,)?) => {
        /// Nombres de los ficheros de mensajes (sin `.ftl`), los mismos en
        /// cada idioma.
        pub const FICHEROS: &[&str] = &[$($nombre),*];
        const FUENTES_ES: &[&str] =
            &[$(include_str!(concat!("../locales/es/", $nombre, ".ftl"))),*];
        const FUENTES_EN: &[&str] =
            &[$(include_str!(concat!("../locales/en/", $nombre, ".ftl"))),*];
    };
}

locales!(
    "comun",
    "login",
    "estado",
    "historial",
    "biblioteca",
    "instalar",
    "panel",
    "router",
    "recomendaciones"
);

/// Idioma de la interfaz.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Idioma {
    #[default]
    Es,
    En,
}

impl Idioma {
    pub const TODOS: [Idioma; 2] = [Idioma::Es, Idioma::En];

    /// Código BCP 47 (`<html lang>`, cookie `rp_lang`).
    pub fn codigo(self) -> &'static str {
        match self {
            Idioma::Es => "es",
            Idioma::En => "en",
        }
    }

    /// Idioma de un código («es», «EN»…); `None` si no es uno de los dos.
    pub fn from_codigo(codigo: &str) -> Option<Idioma> {
        let c = codigo.trim();
        Idioma::TODOS
            .into_iter()
            .find(|i| c.eq_ignore_ascii_case(i.codigo()))
    }

    /// Idioma del valor de la cookie `rp_lang`; `None` si no es válido.
    pub fn from_cookie(valor: &str) -> Option<Idioma> {
        Idioma::from_codigo(valor)
    }

    /// Idioma preferido de una cabecera `Accept-Language`: el de mayor `q`
    /// entre los que hay (a igual `q`, el primero); español si no pide
    /// ninguno de los dos.
    pub fn from_accept_language(cabecera: &str) -> Idioma {
        let mut mejor: Option<(f32, Idioma)> = None;
        for parte in cabecera.split(',') {
            let mut trozos = parte.split(';');
            let etiqueta = trozos.next().unwrap_or("").trim();
            let q = trozos
                .find_map(|p| p.trim().strip_prefix("q="))
                .map_or(Some(1.0), |q| q.trim().parse::<f32>().ok());
            let Some(q) = q.filter(|q| *q > 0.0) else {
                continue;
            };
            let primaria = etiqueta.split('-').next().unwrap_or("");
            let Some(idioma) = Idioma::from_codigo(primaria) else {
                continue;
            };
            if mejor.is_none_or(|(mq, _)| q > mq) {
                mejor = Some((q, idioma));
            }
        }
        mejor.map_or(Idioma::Es, |(_, i)| i)
    }

    /// Separador decimal de los números.
    pub fn separador_decimal(self) -> char {
        match self {
            Idioma::Es => ',',
            Idioma::En => '.',
        }
    }

    fn fuentes(self) -> &'static [&'static str] {
        match self {
            Idioma::Es => FUENTES_ES,
            Idioma::En => FUENTES_EN,
        }
    }
}

type Bundle = FluentBundle<FluentResource>;

static ES: LazyLock<Bundle> = LazyLock::new(|| construir(Idioma::Es));
static EN: LazyLock<Bundle> = LazyLock::new(|| construir(Idioma::En));

fn bundle(idioma: Idioma) -> &'static Bundle {
    match idioma {
        Idioma::Es => &ES,
        Idioma::En => &EN,
    }
}

fn construir(idioma: Idioma) -> Bundle {
    let langid: LanguageIdentifier = idioma.codigo().parse().expect("código de idioma válido");
    let mut b = FluentBundle::new_concurrent(vec![langid]);
    b.set_use_isolating(false);
    for fuente in idioma.fuentes() {
        let recurso = FluentResource::try_new((*fuente).to_owned()).unwrap_or_else(|(r, e)| {
            avisar(&format!(
                "i18n: errores de sintaxis en un .ftl ({}): {e:?}",
                idioma.codigo()
            ));
            r
        });
        if let Err(e) = b.add_resource(recurso) {
            avisar(&format!(
                "i18n: claves repetidas ({}): {e:?}",
                idioma.codigo()
            ));
        }
    }
    b
}

/// Texto de `clave` en `idioma`, con sus argumentos. Si falta, el español
/// (y se avisa); si tampoco está, la propia clave.
pub fn tr(idioma: Idioma, clave: &str, args: Option<&FluentArgs>) -> String {
    if let Some(t) = formatear(idioma, clave, args) {
        return t;
    }
    avisar(&format!(
        "i18n: falta la clave «{clave}» en «{}»",
        idioma.codigo()
    ));
    if idioma != Idioma::Es
        && let Some(t) = formatear(Idioma::Es, clave, args)
    {
        return t;
    }
    clave.to_owned()
}

fn formatear(idioma: Idioma, clave: &str, args: Option<&FluentArgs>) -> Option<String> {
    let b = bundle(idioma);
    let patron = b.get_message(clave)?.value()?;
    let mut errores = Vec::new();
    let texto = b.format_pattern(patron, args, &mut errores).into_owned();
    if !errores.is_empty() {
        avisar(&format!(
            "i18n: «{clave}» ({}): {errores:?}",
            idioma.codigo()
        ));
    }
    Some(texto)
}

#[allow(unused_variables)]
fn avisar(mensaje: &str) {
    #[cfg(feature = "tracing")]
    tracing::warn!("{mensaje}");
    #[cfg(target_arch = "wasm32")]
    web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(mensaje));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codigo_y_cookie() {
        assert_eq!(Idioma::En.codigo(), "en");
        assert_eq!(Idioma::from_cookie("en"), Some(Idioma::En));
        assert_eq!(Idioma::from_cookie(" ES "), Some(Idioma::Es));
        assert_eq!(Idioma::from_cookie("xx"), None);
        assert_eq!(Idioma::from_cookie(""), None);
    }

    #[test]
    fn accept_language() {
        assert_eq!(Idioma::from_accept_language("en-GB,en;q=0.9"), Idioma::En);
        assert_eq!(Idioma::from_accept_language("fr"), Idioma::Es);
        assert_eq!(Idioma::from_accept_language(""), Idioma::Es);
        assert_eq!(
            Idioma::from_accept_language("fr-FR, en;q=0.5, es;q=0.8"),
            Idioma::Es
        );
        assert_eq!(Idioma::from_accept_language("de, en-US;q=0.7"), Idioma::En);
        // q=0 es «no lo quiero».
        assert_eq!(Idioma::from_accept_language("en;q=0"), Idioma::Es);
    }

    #[test]
    fn texto_y_argumentos() {
        assert_eq!(tr(Idioma::Es, "login-entrar", None), "Entrar");
        assert_eq!(tr(Idioma::En, "login-entrar", None), "Sign in");
        let mut a = FluentArgs::new();
        a.set("n", 3);
        assert_eq!(tr(Idioma::Es, "comun-cortes", Some(&a)), "3 cortes");
        assert_eq!(tr(Idioma::En, "comun-cortes", Some(&a)), "3 outages");
        let mut a = FluentArgs::new();
        a.set("n", 1);
        assert_eq!(tr(Idioma::En, "comun-cortes", Some(&a)), "1 outage");
    }

    #[test]
    fn paginas_del_historial_en_singular() {
        let args = |total: i64| {
            let mut a = FluentArgs::new();
            a.set("pagina", 1);
            a.set("paginas", 1);
            a.set("total", total);
            a
        };
        let t = |i, n| tr(i, "historial-pagina-de", Some(&args(n)));
        assert_eq!(t(Idioma::En, 1), "Page 1 of 1 · 1 entry");
        assert_eq!(t(Idioma::En, 5), "Page 1 of 1 · 5 entries");
        assert_eq!(t(Idioma::Es, 1), "Página 1 de 1 · 1 entrada");
        assert_eq!(t(Idioma::Es, 5), "Página 1 de 1 · 5 entradas");
    }

    #[test]
    fn clave_que_falta() {
        assert_eq!(tr(Idioma::En, "no-existe", None), "no-existe");
    }
}
