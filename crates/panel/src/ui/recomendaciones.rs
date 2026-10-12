//! Sección «Recomendaciones» (`/recomendaciones/*`). Por ahora tiene una
//! página, «Tus preferencias» (`/recomendaciones/preferencias`), con los
//! parámetros de la instancia que usan prestablecidos y recomendaciones: DNS
//! preferidos, dispositivos de confianza, horario de mantenimiento y servidor
//! propio. `/recomendaciones` lleva a ella.

use super::panel_ajustes::{campo, mensajes, texto_error};
use crate::panel_ajustes::{CamposInstancia, MAX_DNS, guardar_instancia};
use crate::preferencias::{EquipoLan, Preferencias, leer_preferencias};
use crate::t;
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use leptos_router::{
    NavigateOptions,
    components::{Outlet, Redirect},
};

/// Página «Tus preferencias».
pub const RUTA_PREFERENCIAS: &str = "/recomendaciones/preferencias";

/// Marco de la sección: la página de la ruta hija.
#[component]
pub fn SeccionRecomendaciones() -> impl IntoView {
    view! { <Outlet /> }
}

/// `/recomendaciones`: lleva a «Tus preferencias». En el navegador sustituye
/// la entrada del historial: «Atrás» no vuelve a `/recomendaciones` (que
/// redirigiría otra vez), sino a la página de antes.
#[component]
pub fn InicioRecomendaciones() -> impl IntoView {
    let options = NavigateOptions {
        replace: true,
        ..NavigateOptions::default()
    };
    view! { <Redirect path=RUTA_PREFERENCIAS options /> }
}

/// Página «Tus preferencias».
#[component]
pub fn PaginaPreferencias() -> impl IntoView {
    let datos = Resource::new(|| (), |_| leer_preferencias());
    view! {
        <header class="pagina-cabecera">
            <div>
                <p class="pagina-antetitulo">{t!("recomendaciones-titulo")}</p>
                <h1>{t!("pref-titulo")}</h1>
                <p class="pagina-sub">{t!("pref-sub")}</p>
            </div>
        </header>
        <Suspense fallback=|| view! { <p>{t!("comun-cargando")}</p> }>
            {move || Suspend::new(async move {
                match datos.await {
                    Ok(p) => view! { <div class="ajustes-panel"><Formulario p /></div> }.into_any(),
                    Err(e) => {
                        view! { <p class="error-formulario" role="alert">{texto_error(&e)}</p> }
                            .into_any()
                    }
                }
            })}
        </Suspense>
    }
}

/// Lista escrita a mano: separada por comas, espacios o saltos de línea.
fn trocear(s: &str) -> Vec<String> {
    s.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|x| !x.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Sugerencias de DNS públicos: nombre, direcciones y política de
/// privacidad (claves i18n).
fn sugerencias_dns() -> [(String, String, String); 3] {
    [
        (
            t!("pref-dns-quad9"),
            t!("pref-dns-quad9-ips"),
            t!("pref-dns-quad9-politica"),
        ),
        (
            t!("pref-dns-mullvad"),
            t!("pref-dns-mullvad-ips"),
            t!("pref-dns-mullvad-politica"),
        ),
        (
            t!("pref-dns-cloudflare"),
            t!("pref-dns-cloudflare-ips"),
            t!("pref-dns-cloudflare-politica"),
        ),
    ]
}

/// Botones para añadir a los de confianza los equipos que ve el router.
fn botones_equipos(equipos: Vec<EquipoLan>, confianza: RwSignal<String>) -> impl IntoView {
    let anadir = move |mac: String| {
        confianza.update(|c| {
            let mut l = trocear(c);
            if !l.iter().any(|m| m.eq_ignore_ascii_case(&mac)) {
                l.push(mac);
            }
            *c = l.join(", ");
        });
    };
    (!equipos.is_empty()).then(|| {
        let botones = equipos
            .into_iter()
            .map(|e| {
                let nombre = if e.nombre.is_empty() {
                    t!("pref-equipo-sin-nombre")
                } else {
                    e.nombre.clone()
                };
                let mac = e.mac.clone();
                view! {
                    <li>
                        <button
                            class="boton-sutil"
                            type="button"
                            on:click=move |_| anadir(mac.clone())
                        >
                            {t!("pref-confianza-anadir", nombre = nombre)}
                            <span class="mono">{e.mac}</span>
                        </button>
                    </li>
                }
            })
            .collect_view();
        view! {
            <div class="ajustes-sugerencias">
                <p class="nota">{t!("pref-confianza-equipos")}</p>
                <ul class="lista-limpia ajustes-botones">{botones}</ul>
            </div>
        }
    })
}

/// Las preferencias en edición; guardar manda solo estos parámetros de la
/// instancia (no el idioma ni la zona de «Ajustes del panel»).
#[component]
fn Formulario(p: Preferencias) -> impl IntoView {
    let dns = RwSignal::new(p.dns_preferidos.join(", "));
    let confianza = RwSignal::new(p.dispositivos_confianza.join(", "));
    let mantenimiento = RwSignal::new(p.mantenimiento.unwrap_or_default());
    let servidor = RwSignal::new(p.servidor_propio.unwrap_or_default());
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let campos = CamposInstancia {
            dns_preferidos: Some(trocear(&dns.get_untracked())),
            dispositivos_confianza: Some(trocear(&confianza.get_untracked())),
            mantenimiento: Some(mantenimiento.get_untracked()),
            servidor_propio: Some(servidor.get_untracked()),
            ..CamposInstancia::default()
        };
        spawn_local(async move {
            match guardar_instancia(campos).await {
                // Los valores ya normalizados por el servidor.
                Ok(c) => {
                    if let Some(d) = c.dns_preferidos {
                        dns.set(d.join(", "));
                    }
                    if let Some(m) = c.dispositivos_confianza {
                        confianza.set(m.join(", "));
                    }
                    if let Some(m) = c.mantenimiento {
                        mantenimiento.set(m);
                    }
                    if let Some(v) = c.servidor_propio {
                        servidor.set(v);
                    }
                    aviso.set(Some(t!("panel-guardado")));
                }
                Err(e) => error.set(Some(texto_error(&e))),
            }
            pendiente.set(false);
        });
    };
    let sugerencias = sugerencias_dns()
        .into_iter()
        .map(|(nombre, ips, politica)| {
            let usar = t!("pref-dns-usar", nombre = nombre.clone());
            let enlace = t!("pref-dns-politica", nombre = nombre);
            view! {
                <li>
                    <button class="boton-sutil" type="button" on:click=move |_| dns.set(ips.clone())>
                        {usar}
                    </button>
                    <a href=politica rel="noopener noreferrer" target="_blank">
                        {enlace}
                    </a>
                </li>
            }
        })
        .collect_view();
    view! {
        <section class="tarjeta" aria-labelledby="pref-preferencias">
            <h2 id="pref-preferencias">{t!("pref-preferencias")}</h2>
            <form class="ajustes-formulario" on:submit=enviar>
                <p class="nota">{t!("pref-preferencias-texto")}</p>
                {campo(
                    "pref-dns",
                    t!("pref-dns"),
                    "text",
                    "off",
                    dns,
                    Some(t!("pref-dns-ayuda", n = MAX_DNS)),
                )}
                <div class="ajustes-sugerencias">
                    <p class="nota">{t!("pref-dns-sugerencias")}</p>
                    <ul class="lista-limpia ajustes-botones">{sugerencias}</ul>
                </div>
                {campo(
                    "pref-confianza",
                    t!("pref-confianza"),
                    "text",
                    "off",
                    confianza,
                    Some(t!("pref-confianza-ayuda")),
                )}
                {botones_equipos(p.equipos, confianza)}
                {campo(
                    "pref-mantenimiento",
                    t!("pref-mantenimiento"),
                    "text",
                    "off",
                    mantenimiento,
                    Some(t!("pref-mantenimiento-ayuda")),
                )}
                {campo(
                    "pref-servidor",
                    t!("pref-servidor"),
                    "text",
                    "off",
                    servidor,
                    Some(t!("pref-servidor-ayuda")),
                )}
                {mensajes(error, aviso)}
                <div class="botones">
                    <button class="boton" type="submit" disabled=move || pendiente.get()>
                        {t!("panel-guardar")}
                    </button>
                </div>
            </form>
        </section>
    }
}
