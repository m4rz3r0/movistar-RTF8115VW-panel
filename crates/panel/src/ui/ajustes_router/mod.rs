//! Sección «Ajustes» (`/ajustes/<tema>`): los ajustes del router agrupados
//! por temas en lenguaje claro. `/ajustes` lleva a Internet.
//!
//! Cada página tiene las pestañas de los siete temas (por debajo de 640 px,
//! un selector), el aviso de solo lectura si el panel aún no escribe, las
//! tarjetas de lo esencial arriba y el resto bajo «Más opciones». Las
//! tarjetas ([`tarjeta::Tarjeta`]) se editan en el sitio y guardan con el
//! diálogo de grupo; los dispositivos y las IP fijas de Red local tienen su
//! propio componente ([`dispositivos`], [`reservas`]). Un tema que no
//! existe es «página no encontrada».

mod campo;
mod dispositivos;
mod reservas;
mod tarjeta;

use crate::ajustes_router::{CADUCIDAD_LECTURAS, TemaVista, leer_tema};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::i18n::traducir;
use crate::ui::{Estado, IconoEstado};
use ajustes::{Nivel, Tema};
use leptos::prelude::*;
use leptos_router::{
    NavigateOptions,
    components::{Outlet, Redirect},
    hooks::{use_navigate, use_params_map},
};
use std::{collections::HashSet, time::Duration};
use tarjeta::Tarjeta;

/// Espera antes de enseñar «Actualizando…»: una lectura más rápida (de la
/// caché del servidor) no hace parpadear la página.
const UMBRAL_ACTUALIZANDO: Duration = Duration::from_millis(150);

/// Tiempo con el ratón o el foco en una pestaña antes de precargar su tema:
/// pasar por encima de camino a otra no lee nada.
const RETARDO_PRECARGA: Duration = Duration::from_millis(150);

/// Id de la nota que dice qué significa «Oculto» (una por página; la
/// describen todos sus chips).
pub(crate) const NOTA_OCULTO: &str = "ajustes-nota-oculto";

/// Componentes de Red local (los nombres de `ajustes::Componente::slug`,
/// que en el navegador no está).
pub(crate) const DISPOSITIVOS: &str = "dispositivos";
pub(crate) const RESERVAS: &str = "reservas";

/// Primera página de la sección (la del menú).
pub const RUTA_AJUSTES: &str = "/ajustes/internet";

/// Id del título que recibe el foco cuando la página vuelve a leer el tema
/// (tras aplicar un cambio, las tarjetas se pintan de nuevo).
#[derive(Clone, Copy)]
pub(crate) struct Foco(pub RwSignal<Option<String>>);

/// Marco de la sección: la página de la ruta hija.
#[component]
pub fn SeccionAjustes() -> impl IntoView {
    view! { <Outlet /> }
}

/// `/ajustes`: lleva a Internet (302 en el servidor). En el navegador
/// sustituye la entrada del historial.
#[component]
pub fn InicioAjustes() -> impl IntoView {
    let options = NavigateOptions {
        replace: true,
        ..NavigateOptions::default()
    };
    view! { <Redirect path=RUTA_AJUSTES options /> }
}

fn ruta(t: Tema) -> String {
    format!("/ajustes/{}", t.slug())
}

fn nombre(t: Tema) -> String {
    traducir(&format!("ajustes-tema-{}", t.slug()), None)
}

/// Precarga de los temas: al dejar el ratón o el foco en una pestaña, lee
/// su tema en segundo plano para que el servidor lo tenga en su caché
/// corta cuando se pulse. Una sola a la vez (el servidor las lee en serie)
/// y cada tema una vez por caducidad de la caché.
#[derive(Clone, Copy)]
struct Precarga {
    programada: StoredValue<Option<TimeoutHandle>>,
    en_curso: StoredValue<bool>,
    hechas: StoredValue<HashSet<String>>,
}

impl Precarga {
    fn nueva() -> Precarga {
        Precarga {
            programada: StoredValue::new(None),
            en_curso: StoredValue::new(false),
            hechas: StoredValue::new(HashSet::new()),
        }
    }

    /// Precarga `t` si el ratón o el foco siguen en su pestaña dentro de
    /// [`RETARDO_PRECARGA`].
    fn programar(self, t: Tema, actual: Memo<String>) {
        self.cancelar();
        let slug = t.slug().to_owned();
        let precargar = move || {
            self.programada.set_value(None);
            let ya = self.hechas.with_value(|h| h.contains(&slug));
            if ya || self.en_curso.get_value() || actual.get_untracked() == slug {
                return;
            }
            self.en_curso.set_value(true);
            leptos::task::spawn_local(async move {
                if leer_tema(slug.clone()).await.is_ok() {
                    self.hechas.update_value(|h| {
                        h.insert(slug.clone());
                    });
                    // Pasada la caducidad, el servidor ya no lo tiene.
                    set_timeout(
                        move || self.hechas.update_value(|h| _ = h.remove(&slug)),
                        CADUCIDAD_LECTURAS,
                    );
                }
                self.en_curso.set_value(false);
            });
        };
        if let Ok(h) = set_timeout_with_handle(precargar, RETARDO_PRECARGA) {
            self.programada.set_value(Some(h));
        }
    }

    fn cancelar(self) {
        if let Some(h) = self.programada.get_value() {
            h.clear();
            self.programada.set_value(None);
        }
    }
}

/// Pestañas de los temas y, en pantallas estrechas, el selector que navega.
fn pestanas(slug: Memo<String>) -> impl IntoView {
    let precarga = Precarga::nueva();
    let enlaces = Tema::TODOS
        .into_iter()
        .map(|t| {
            let actual = move || (slug.get() == t.slug()).then_some("page");
            view! {
                <a
                    href=ruta(t)
                    aria-current=actual
                    on:mouseenter=move |_| precarga.programar(t, slug)
                    on:focus=move |_| precarga.programar(t, slug)
                    on:mouseleave=move |_| precarga.cancelar()
                    on:blur=move |_| precarga.cancelar()
                >
                    {nombre(t)}
                </a>
            }
        })
        .collect_view();
    // `selected` da el valor del HTML del servidor; después manda
    // `prop:value`, que sigue a la ruta (también al volver atrás).
    let inicial = slug.get_untracked();
    let opciones = Tema::TODOS
        .into_iter()
        .map(|t| {
            let elegido = inicial == t.slug();
            view! { <option value=t.slug() selected=elegido>{nombre(t)}</option> }
        })
        .collect_view();
    let navegar = use_navigate();
    let cambiar = move |ev: leptos::ev::Event| {
        if let Some(t) = Tema::from_slug(&event_target_value(&ev)) {
            navegar(&ruta(t), NavigateOptions::default());
        }
    };
    view! {
        <nav class="temas" aria-label=t!("ajustes-temas")>{enlaces}</nav>
        <p class="campo temas-selector">
            <label for="ajustes-tema">{t!("ajustes-tema-elegir")}</label>
            <select id="ajustes-tema" prop:value=move || slug.get() on:change=cambiar>
                {opciones}
            </select>
        </p>
    }
}

/// Página no encontrada dentro del marco (404 en el servidor).
fn no_encontrada() -> impl IntoView {
    #[cfg(feature = "ssr")]
    if let Some(r) = use_context::<leptos_axum::ResponseOptions>() {
        r.set_status(axum::http::StatusCode::NOT_FOUND);
    }
    view! {
        <header class="pagina-cabecera">
            <h1>{t!("comun-no-encontrada")}</h1>
        </header>
        <p>
            <a href=RUTA_AJUSTES>{t!("ajustes-volver")}</a>
        </p>
    }
}

/// `/ajustes/<tema>`.
#[component]
pub fn PaginaTema() -> impl IntoView {
    let params = use_params_map();
    let slug = Memo::new(move |_| params.with(|p| p.get("tema").unwrap_or_default()));
    let existe = Memo::new(move |_| Tema::from_slug(&slug.get()).is_some());
    let datos = Resource::new(
        move || slug.get(),
        |s| async move {
            match Tema::from_slug(&s) {
                Some(_) => leer_tema(s).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let foco = Foco(RwSignal::new(None));
    provide_context(foco);
    let recargar = Callback::new(move |()| datos.refetch());
    // Mientras se lee otro tema (o el mismo tras un cambio), lo anterior se
    // ve atenuado y con «Actualizando…», si tarda más que el umbral.
    let pendiente = RwSignal::new(false);
    let actualizando = RwSignal::new(false);
    Effect::new(move |_| {
        if pendiente.get() {
            set_timeout(
                move || {
                    if pendiente.get_untracked() {
                        actualizando.set(true);
                    }
                },
                UMBRAL_ACTUALIZANDO,
            );
        } else {
            actualizando.set(false);
        }
    });
    // Con la lectura nueva ya pintada, el foco al título pendiente.
    Effect::new(move |_| {
        if datos.with(|d| d.is_some())
            && let Some(id) = foco.0.get_untracked()
        {
            request_animation_frame(move || {
                tarjeta::enfocar(&id);
                foco.0.set(None);
            });
        }
    });
    move || {
        if !existe.get() {
            return no_encontrada().into_any();
        }
        view! {
            <header class="pagina-cabecera">
                <div>
                    <h1>{t!("ajustes-titulo")}</h1>
                    <p class="pagina-sub">{t!("ajustes-desc")}</p>
                </div>
            </header>
            {pestanas(slug)}
            <div class="tema-contenido" aria-busy=move || actualizando.get().then_some("true")>
                // Siempre en la página (vacío si no carga): el lector de
                // pantalla anuncia el texto una vez cada vez que aparece.
                <p class="tema-actualizando" role="status">
                    {move || actualizando.get().then(|| t!("ajustes-actualizando"))}
                </p>
                <Transition
                    fallback=|| view! { <p class="estado-espera">{t!("comun-cargando")}</p> }
                    set_pending=pendiente
                >
                {move || Suspend::new(async move {
                    match datos.await {
                        Ok(Some(t)) => view! { <ContenidoTema t recargar /> }.into_any(),
                        Ok(None) => ().into_any(),
                        Err(e) => {
                            view! {
                                <p class="aviso estado-error" role="alert">
                                    <IconoEstado estado=Estado::Error />
                                    <span>{texto_error(&e)}</span>
                                </p>
                            }
                                .into_any()
                        }
                    }
                })}
                </Transition>
            </div>
        }
        .into_any()
    }
}

/// Las tarjetas de un tema: lo esencial arriba; lo avanzado y lo oculto,
/// bajo «Más opciones».
#[component]
fn ContenidoTema(t: TemaVista, recargar: Callback<()>) -> impl IntoView {
    let solo_lectura = t.solo_lectura;
    // Los dispositivos y las IP fijas leen la red de casa una vez para los dos.
    let de_red = |c: &Option<String>| matches!(c.as_deref(), Some(DISPOSITIVOS | RESERVAS));
    if t.ajustes.iter().any(|a| de_red(&a.componente)) {
        let existentes = t
            .ajustes
            .iter()
            .find(|a| a.componente.as_deref() == Some(RESERVAS))
            .map(|a| reservas::existentes(&a.instancias))
            .unwrap_or_default();
        provide_context(dispositivos::DatosRed::nuevo(existentes));
    }
    let (basicos, otros): (Vec<_>, Vec<_>) = t
        .ajustes
        .into_iter()
        .partition(|a| a.nivel == Nivel::Basico);
    let tarjetas = move |lista: Vec<_>| {
        lista
            .into_iter()
            .map(|a| view! { <Tarjeta a solo_lectura recargar /> })
            .collect_view()
    };
    let aviso = solo_lectura.then(|| {
        view! {
            <p class="aviso estado-aviso" role="status">
                <IconoEstado estado=Estado::Aviso />
                <span>
                    {t!("ajustes-solo-lectura-aviso")} " "
                    <a href=crate::ui::router::RUTA_CONEXION>{t!("ajustes-solo-lectura-enlace")}</a>
                </span>
            </p>
        }
    });
    let tema = format!("ajustes-tema-{}-desc", t.tema);
    // Qué significa «Oculto», una sola vez, delante de las tarjetas ocultas.
    let nota_oculto = |hay: bool| {
        hay.then(|| {
            view! {
                <p class="nota-oculto" id=NOTA_OCULTO>
                    <span class="chip chip-nivel">{t!("ajustes-nivel-oculto")}</span>
                    " "
                    {t!("ajustes-nivel-oculto-desc")}
                </p>
            }
        })
    };
    let hay_oculto = otros.iter().any(|a| a.nivel == Nivel::Oculto);
    // Sin nada esencial (Sistema), plegarlo todo dejaría la página vacía:
    // sus tarjetas se ven directamente, cada una con su etiqueta de nivel.
    let cuerpo = if basicos.is_empty() {
        view! {
            {nota_oculto(hay_oculto)}
            <div class="ajustes-lista">{tarjetas(otros)}</div>
        }
        .into_any()
    } else {
        let mas = (!otros.is_empty()).then(|| {
            view! {
                <details class="mas-opciones">
                    <summary>
                        <span class="mas-opciones-titulo">{t!("ajustes-mas-opciones")}</span>
                        <span class="mas-opciones-desc">{t!("ajustes-mas-opciones-desc")}</span>
                    </summary>
                    {nota_oculto(hay_oculto)}
                    <div class="ajustes-lista">{tarjetas(otros)}</div>
                </details>
            }
        });
        view! {
            <div class="ajustes-lista">{tarjetas(basicos)}</div>
            {mas}
        }
        .into_any()
    };
    view! {
        <p class="tema-desc">{traducir(&tema, None)}</p>
        {aviso}
        {cuerpo}
    }
}
