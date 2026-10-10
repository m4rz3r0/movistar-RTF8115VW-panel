//! Asistente de primera instalación (`/instalar`): administrador, router,
//! lectura inicial y, para terminar, enlace a los parámetros opcionales de la
//! instancia.

use super::{Estado, Insignia, mensaje_del_servidor};
use crate::instalar::{
    EstadoInstalacion, InfoRouter, ResumenFoto, crear_admin, fase_instalacion, guardar_router,
    lectura_inicial, probar_router,
};
use crate::t;
use crate::ui::i18n::SelectorIdioma;
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};

/// Longitud mínima de la contraseña del panel (la misma que comprueba el
/// servidor).
const MIN_CONTRASENA: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Paso {
    Admin,
    Router,
    Lectura,
    Fin,
}

impl Paso {
    fn inicial(f: EstadoInstalacion) -> Paso {
        match (f.admin_creado, f.router_configurado) {
            (false, _) => Paso::Admin,
            (true, false) => Paso::Router,
            (true, true) => Paso::Lectura,
        }
    }
}

fn texto_error(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(m) => mensaje_del_servidor(m),
        _ => t!("comun-error-conexion"),
    }
}

/// Página del asistente.
#[component]
pub fn PaginaInstalar() -> impl IntoView {
    let fase = Resource::new(|| (), |_| fase_instalacion());
    view! {
        <main class="acceso">
            <section class="acceso-tarjeta instalar" aria-labelledby="instalar-titulo">
                <div class="acceso-cabeza">
                    <p class="acceso-marca">"RTF8115VW"</p>
                    <SelectorIdioma />
                </div>
                <h1 id="instalar-titulo">{t!("instalar-titulo")}</h1>
                <Suspense fallback=|| view! { <p>{t!("comun-cargando")}</p> }>
                    {move || Suspend::new(async move {
                        match fase.await {
                            Ok(f) => view! { <Asistente inicial=Paso::inicial(f) /> }.into_any(),
                            Err(e) => {
                                view! { <p class="error" role="alert">{texto_error(&e)}</p> }
                                    .into_any()
                            }
                        }
                    })}
                </Suspense>
            </section>
        </main>
    }
}

/// Los pasos, con el actual marcado.
fn pasos(paso: RwSignal<Paso>) -> impl IntoView {
    let items = [
        (Paso::Admin, t!("instalar-paso-admin")),
        (Paso::Router, t!("instalar-paso-router")),
        (Paso::Lectura, t!("instalar-paso-lectura")),
        (Paso::Fin, t!("instalar-paso-fin")),
    ]
    .into_iter()
    .map(|(p, texto)| {
        view! {
            <li aria-current=move || (paso.get() == p).then_some("step")>{texto}</li>
        }
    })
    .collect_view();
    view! { <ol class="instalar-pasos" aria-label=t!("instalar-pasos")>{items}</ol> }
}

#[component]
fn Asistente(inicial: Paso) -> impl IntoView {
    let paso = RwSignal::new(inicial);
    let token = RwSignal::new(String::new());
    let info = RwSignal::new(None::<InfoRouter>);
    let resumen = RwSignal::new(None::<ResumenFoto>);
    let cuerpo = move || match paso.get() {
        Paso::Admin => view! { <PasoAdmin paso token /> }.into_any(),
        Paso::Router => view! { <PasoRouter paso token info /> }.into_any(),
        Paso::Lectura => view! { <PasoLectura paso token info resumen /> }.into_any(),
        Paso::Fin => view! { <PasoFin resumen /> }.into_any(),
    };
    view! {
        {pasos(paso)}
        {cuerpo}
    }
}

/// Campo de texto ligado a `valor`.
fn campo(
    id: &'static str,
    etiqueta: String,
    tipo: &'static str,
    autocompletar: &'static str,
    valor: RwSignal<String>,
) -> impl IntoView {
    view! {
        <p class="campo">
            <label for=id>{etiqueta}</label>
            <input
                id=id
                name=id
                type=tipo
                autocomplete=autocompletar
                autocapitalize="none"
                spellcheck="false"
                required
                prop:value=valor
                on:input=move |ev| valor.set(event_target_value(&ev))
            />
        </p>
    }
}

fn error_de(error: RwSignal<Option<String>>) -> impl IntoView {
    view! { <p class="error" role="alert" aria-live="assertive">{move || error.get()}</p> }
}

#[component]
fn PasoAdmin(paso: RwSignal<Paso>, token: RwSignal<String>) -> impl IntoView {
    let usuario = RwSignal::new(String::new());
    let contrasena = RwSignal::new(String::new());
    let repetir = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        if contrasena.get_untracked() != repetir.get_untracked() {
            error.set(Some(t!("instalar-no-coinciden")));
            return;
        }
        error.set(None);
        pendiente.set(true);
        let (t, u, c) = (
            token.get_untracked().trim().to_owned(),
            usuario.get_untracked(),
            contrasena.get_untracked(),
        );
        spawn_local(async move {
            match crear_admin(t, u, c).await {
                Ok(()) => paso.set(Paso::Router),
                Err(e) => error.set(Some(texto_error(&e))),
            }
            pendiente.set(false);
        });
    };
    view! {
        <form on:submit=enviar>
            <p class="pagina-sub">{t!("instalar-admin-texto")}</p>
            {campo("instalar-token", t!("instalar-token"), "text", "off", token)}
            <p class="nota">{t!("instalar-token-ayuda")}</p>
            {campo("instalar-usuario", t!("instalar-usuario"), "text", "username", usuario)}
            {campo(
                "instalar-contrasena",
                t!("instalar-contrasena", n = MIN_CONTRASENA),
                "password",
                "new-password",
                contrasena,
            )}
            {campo("instalar-repetir", t!("instalar-repetir"), "password", "new-password", repetir)}
            {error_de(error)}
            <button class="boton" type="submit" disabled=move || pendiente.get()>
                {t!("instalar-crear-admin")}
            </button>
        </form>
    }
}

/// Modelo, firmware y si está probado.
fn detectado(i: &InfoRouter) -> impl IntoView + use<> {
    let (estado, nota) = if i.probado {
        (Estado::Ok, t!("instalar-firmware-probado"))
    } else {
        (Estado::Aviso, t!("instalar-firmware-no-probado"))
    };
    view! {
        <div class="instalar-detectado" role="status">
            <p>
                {t!(
                    "instalar-detectado", modelo = i.modelo.clone(), firmware = i.firmware.clone()
                )}
            </p>
            <Insignia estado texto=nota />
        </div>
    }
}

#[component]
fn PasoRouter(
    paso: RwSignal<Paso>,
    token: RwSignal<String>,
    info: RwSignal<Option<InfoRouter>>,
) -> impl IntoView {
    let url = RwSignal::new(String::new());
    let usuario = RwSignal::new(String::new());
    let contrasena = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    let llamar = move |guardar: bool| {
        error.set(None);
        pendiente.set(true);
        let (t, a, u, c) = (
            token.get_untracked().trim().to_owned(),
            url.get_untracked(),
            usuario.get_untracked(),
            contrasena.get_untracked(),
        );
        spawn_local(async move {
            let r = if guardar {
                guardar_router(t, a, u, c).await
            } else {
                probar_router(t, a, u, c).await
            };
            match r {
                Ok(i) => {
                    info.set(Some(i));
                    if guardar {
                        paso.set(Paso::Lectura);
                    }
                }
                Err(e) => error.set(Some(texto_error(&e))),
            }
            pendiente.set(false);
        });
    };
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        llamar(true);
    };
    view! {
        <form on:submit=enviar>
            <p class="pagina-sub">{t!("instalar-router-texto")}</p>
            <p class="campo">
                <label for="instalar-router-url">{t!("instalar-router-url")}</label>
                <input
                    id="instalar-router-url"
                    name="instalar-router-url"
                    type="url"
                    inputmode="url"
                    autocomplete="url"
                    autocapitalize="none"
                    spellcheck="false"
                    required
                    placeholder=t!("instalar-router-url-ejemplo")
                    prop:value=url
                    on:input=move |ev| url.set(event_target_value(&ev))
                />
            </p>
            {campo(
                "instalar-router-usuario",
                t!("instalar-router-usuario"),
                "text",
                "off",
                usuario,
            )}
            {campo(
                "instalar-router-contrasena",
                t!("instalar-router-contrasena"),
                "password",
                "off",
                contrasena,
            )}
            {move || info.get().map(|i| detectado(&i))}
            {error_de(error)}
            <div class="instalar-botones">
                <button
                    class="boton-sutil"
                    type="button"
                    disabled=move || pendiente.get()
                    on:click=move |_| llamar(false)
                >
                    {t!("instalar-probar")}
                </button>
                <button class="boton" type="submit" disabled=move || pendiente.get()>
                    {t!("instalar-guardar")}
                </button>
            </div>
        </form>
    }
}

#[component]
fn PasoLectura(
    paso: RwSignal<Paso>,
    token: RwSignal<String>,
    info: RwSignal<Option<InfoRouter>>,
    resumen: RwSignal<Option<ResumenFoto>>,
) -> impl IntoView {
    let error = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    let leer = move |_| {
        error.set(None);
        pendiente.set(true);
        let t = token.get_untracked().trim().to_owned();
        spawn_local(async move {
            match lectura_inicial(t).await {
                Ok(r) if r.funciones_leidas > 0 => {
                    resumen.set(Some(r));
                    paso.set(Paso::Fin);
                }
                Ok(r) => error.set(Some(t!("instalar-fallidas", n = r.fallidas.len()))),
                Err(e) => error.set(Some(texto_error(&e))),
            }
            pendiente.set(false);
        });
    };
    view! {
        {move || info.get().map(|i| detectado(&i))}
        <p class="pagina-sub">{t!("instalar-lectura-texto")}</p>
        {error_de(error)}
        <button class="boton" type="button" disabled=move || pendiente.get() on:click=leer>
            {move || if pendiente.get() { t!("instalar-leyendo") } else { t!("instalar-leer") }}
        </button>
    }
}

#[component]
fn PasoFin(resumen: RwSignal<Option<ResumenFoto>>) -> impl IntoView {
    let detalle = move || {
        resumen.get().map(|r| {
            let fallidas = (!r.fallidas.is_empty()).then(|| {
                view! {
                    <Insignia
                        estado=Estado::Aviso
                        texto=t!("instalar-fallidas", n = r.fallidas.len())
                    />
                }
            });
            view! {
                <p>{t!("instalar-leidas", n = r.funciones_leidas)}</p>
                {fallidas}
            }
        })
    };
    view! {
        <div role="status">
            <p>{t!("instalar-fin-texto")}</p>
            {detalle}
        </div>
        <p class="pagina-sub">{t!("instalar-fin-opcional")}</p>
        <div class="instalar-botones">
            <a class="boton-sutil" href="/panel">{t!("instalar-ir-ajustes")}</a>
            <a class="boton" href="/" rel="external">{t!("instalar-ir-estado")}</a>
        </div>
    }
}
