//! Idioma de la interfaz en Leptos: contexto, macro [`t!`](crate::t) y
//! cambio de idioma.
//!
//! El servidor decide el idioma de cada petición (cookie `rp_lang`, si no
//! `Accept-Language`, si no español), lo pone en `<html lang>` y en el
//! contexto. Al hidratar, el navegador lo lee de `<html lang>`: los dos
//! pintan lo mismo. Cambiar de idioma fija la cookie y recarga la página, así
//! que el idioma no cambia mientras la página vive.
//!
//! Fuera de un componente (server functions, manejadores de eventos) el
//! idioma sale de la petición en el servidor y del documento en el navegador.

use ::i18n::{FluentArgs, Idioma, tr};
use leptos::prelude::*;

/// Nombre de la cookie del idioma.
pub const COOKIE: &str = "rp_lang";

/// Idioma de la página en el contexto.
#[derive(Clone, Copy)]
struct IdiomaContexto(Signal<Idioma>);

/// Idioma de la página en el navegador, para el código que corre sin
/// contexto (manejadores de eventos, tareas).
#[cfg(feature = "hydrate")]
static IDIOMA_PAGINA: std::sync::OnceLock<Idioma> = std::sync::OnceLock::new();

/// Pone `idioma` en el contexto de la aplicación.
pub fn provide_idioma(idioma: Idioma) {
    #[cfg(feature = "hydrate")]
    let _ = IDIOMA_PAGINA.set(idioma);
    provide_context(IdiomaContexto(Signal::stored(idioma)));
}

/// Idioma de la página.
pub fn use_idioma() -> Signal<Idioma> {
    use_context::<IdiomaContexto>()
        .map(|c| c.0)
        .unwrap_or_else(|| Signal::stored(idioma_sin_contexto()))
}

/// Idioma actual: el del contexto (siguiéndolo si se lee dentro de un
/// cierre reactivo) o, sin contexto, el de la petición o el documento.
pub fn idioma_actual() -> Idioma {
    match use_context::<IdiomaContexto>() {
        Some(IdiomaContexto(s)) if leptos::reactive::graph::Observer::get().is_some() => s.get(),
        Some(IdiomaContexto(s)) => s.get_untracked(),
        None => idioma_sin_contexto(),
    }
}

fn idioma_sin_contexto() -> Idioma {
    #[cfg(feature = "ssr")]
    if let Some(i) = idioma_de_peticion() {
        return i;
    }
    #[cfg(feature = "hydrate")]
    if let Some(i) = IDIOMA_PAGINA.get() {
        return *i;
    }
    Idioma::Es
}

/// Idioma con el que arranca la aplicación: el de la petición en el
/// servidor; el de `<html lang>` (que puso el servidor) al hidratar.
pub fn idioma_inicial() -> Idioma {
    #[cfg(feature = "ssr")]
    if let Some(i) = idioma_de_peticion() {
        return i;
    }
    #[cfg(feature = "hydrate")]
    if let Some(i) = document()
        .document_element()
        .and_then(|e| e.get_attribute("lang"))
        .and_then(|l| Idioma::from_codigo(&l))
    {
        return i;
    }
    Idioma::Es
}

/// Idioma de la petición en curso, si hay una.
#[cfg(feature = "ssr")]
pub fn idioma_de_peticion() -> Option<Idioma> {
    use_context::<axum::http::request::Parts>().map(|p| idioma_de_cabeceras(&p.headers))
}

/// Cookie `rp_lang` válida; si no, `Accept-Language`; si no, español.
#[cfg(feature = "ssr")]
pub fn idioma_de_cabeceras(h: &axum::http::HeaderMap) -> Idioma {
    use axum::http::header;
    let cookie = h
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|c| c.trim().split_once('='))
        .find(|(k, _)| *k == COOKIE)
        .and_then(|(_, v)| Idioma::from_cookie(v));
    cookie.unwrap_or_else(|| {
        h.get(header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .map_or(Idioma::Es, Idioma::from_accept_language)
    })
}

/// Texto de `clave` en el idioma actual. Lo usa [`t!`](crate::t).
pub fn traducir(clave: &str, args: Option<&FluentArgs>) -> String {
    tr(idioma_actual(), clave, args)
}

/// Texto traducido: `t!("clave")` o `t!("clave", nombre = valor, …)`. Lee el
/// idioma del contexto (y lo sigue dentro de un cierre reactivo).
#[macro_export]
macro_rules! t {
    ($clave:literal) => {
        $crate::ui::i18n::traducir($clave, None)
    };
    ($clave:literal, $($nombre:ident = $valor:expr),+ $(,)?) => {{
        let mut args = ::i18n::FluentArgs::new();
        $(args.set(stringify!($nombre), $valor);)+
        $crate::ui::i18n::traducir($clave, Some(&args))
    }};
}

/// Fija el idioma de la interfaz (cookie `rp_lang` de un año; no es secreta).
/// El navegador recarga la página después.
#[server(endpoint = "cambiar_idioma")]
pub async fn cambiar_idioma(idioma: Idioma) -> Result<(), ServerFnError> {
    use axum::http::{HeaderValue, header};
    let cookie = format!(
        "{COOKIE}={}; Path=/; SameSite=Lax; Secure; Max-Age=31536000",
        idioma.codigo()
    );
    if let (Some(resp), Ok(v)) = (
        use_context::<leptos_axum::ResponseOptions>(),
        HeaderValue::from_str(&cookie),
    ) {
        resp.insert_header(header::SET_COOKIE, v);
    }
    Ok(())
}

/// Nombre de un idioma en ese mismo idioma.
fn nombre_idioma(i: Idioma) -> String {
    match i {
        Idioma::Es => crate::t!("comun-idioma-es"),
        Idioma::En => crate::t!("comun-idioma-en"),
    }
}

/// Selector de idioma: fija la cookie en el servidor y recarga la página.
/// Si no se puede, vuelve al idioma de la página y lo dice.
#[component]
pub fn SelectorIdioma() -> impl IntoView {
    let idioma = use_idioma();
    let error = RwSignal::new(false);
    let select = NodeRef::<leptos::html::Select>::new();
    let cambiar = move |ev| {
        let Some(nuevo) = Idioma::from_codigo(&event_target_value(&ev)) else {
            return;
        };
        error.set(false);
        leptos::task::spawn_local(async move {
            if cambiar_idioma(nuevo).await.is_ok() {
                let _ = window().location().reload();
            } else {
                if let Some(s) = select.get_untracked() {
                    s.set_value(idioma.get_untracked().codigo());
                }
                error.set(true);
            }
        });
    };
    let opciones = Idioma::TODOS
        .into_iter()
        .map(|i| {
            view! {
                <option value=i.codigo() lang=i.codigo() selected=move || idioma.get() == i>
                    {nombre_idioma(i)}
                </option>
            }
        })
        .collect_view();
    view! {
        <label class="idioma">
            <span class="solo-lector">{crate::t!("comun-idioma")}</span>
            <select class="idioma-selector" node_ref=select on:change=cambiar>
                {opciones}
            </select>
            <span class="error idioma-error" role="alert">
                {move || error.get().then(|| crate::t!("comun-error-idioma"))}
            </span>
        </label>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn sin_contexto_es_espanol() {
        assert_eq!(idioma_actual(), Idioma::Es);
        assert_eq!(crate::t!("login-entrar"), "Entrar");
    }

    #[test]
    fn el_contexto_manda() {
        let owner = Owner::new();
        owner.with(|| {
            provide_idioma(Idioma::En);
            assert_eq!(use_idioma().get_untracked(), Idioma::En);
            assert_eq!(crate::t!("login-entrar"), "Sign in");
            assert_eq!(crate::t!("comun-cortes", n = 2), "2 outages");
        });
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn cabeceras() {
        use axum::http::HeaderMap;
        let h = |pares: &[(&'static str, &'static str)]| {
            let mut m = HeaderMap::new();
            for (k, v) in pares {
                m.append(*k, v.parse().unwrap());
            }
            idioma_de_cabeceras(&m)
        };
        assert_eq!(h(&[]), Idioma::Es);
        assert_eq!(h(&[("cookie", "a=b; rp_lang=en")]), Idioma::En);
        assert_eq!(
            h(&[("cookie", "rp_lang=xx"), ("accept-language", "en-US")]),
            Idioma::En
        );
        assert_eq!(
            h(&[("cookie", "rp_lang=es"), ("accept-language", "en")]),
            Idioma::Es
        );
    }
}
