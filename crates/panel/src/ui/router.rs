//! Sección «Router» (`/router/*`): los ajustes del router, cada uno en su
//! página. La primera es «Conexión con el router» (`/router/conexion`):
//! dirección, usuario y contraseña con los que entra el panel, prueba de la
//! conexión y confirmación del firmware. `/router` lleva a ella.
//!
//! Cambiar las credenciales y confirmar el firmware piden la contraseña del
//! panel si no se ha escrito en los últimos 5 minutos (el mismo diálogo que
//! «Ajustes del panel», [`DialogoReautenticar`]).

use super::panel_ajustes::{
    DialogoReautenticar, Reautenticacion, campo, mensajes, repetible, texto_error,
};
use super::{Estado, Insignia};
use crate::panel_ajustes::{
    InfoRouter, confirmar_firmware, guardar_router, leer_panel, probar_router,
};
use crate::t;
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use leptos_router::components::{Outlet, Redirect};

/// Primera página de la sección.
pub const RUTA_CONEXION: &str = "/router/conexion";

/// Marco de la sección: la página de la ruta hija.
#[component]
pub fn SeccionRouter() -> impl IntoView {
    view! { <Outlet /> }
}

/// `/router`: lleva a la primera página de la sección.
#[component]
pub fn InicioRouter() -> impl IntoView {
    view! { <Redirect path=RUTA_CONEXION /> }
}

/// Página «Conexión con el router».
#[component]
pub fn PaginaConexion() -> impl IntoView {
    let datos = Resource::new(|| (), |_| leer_panel());
    let reauth = Reautenticacion::proveer();
    view! {
        <header class="pagina-cabecera">
            <div>
                <h1>{t!("router-conexion")}</h1>
                <p class="pagina-sub">{t!("router-conexion-sub")}</p>
            </div>
        </header>
        <Suspense fallback=|| view! { <p>{t!("comun-cargando")}</p> }>
            {move || Suspend::new(async move {
                match datos.await {
                    Ok(a) => {
                        view! {
                            <Conexion
                                url=a.router.url
                                usuario=a.router.usuario
                                solo_lectura=a.solo_lectura
                                firmware=a.firmware
                            />
                        }
                            .into_any()
                    }
                    Err(e) => {
                        view! { <p class="error-formulario" role="alert">{texto_error(&e)}</p> }
                            .into_any()
                    }
                }
            })}
        </Suspense>
        {move || reauth.abierto().then(|| view! { <DialogoReautenticar /> })}
    }
}

/// Credenciales del router y firmware.
#[component]
fn Conexion(
    url: String,
    usuario: String,
    solo_lectura: bool,
    firmware: Option<String>,
) -> impl IntoView {
    let reauth = expect_context::<Reautenticacion>();
    let configurado = !url.is_empty();
    let url = RwSignal::new(url);
    let usuario = RwSignal::new(usuario);
    let contrasena = RwSignal::new(String::new());
    let info = RwSignal::new(None::<InfoRouter>);
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    let solo_lectura = RwSignal::new(solo_lectura);
    let error_firmware = RwSignal::new(None::<String>);
    let datos = move || {
        (
            url.get_untracked(),
            usuario.get_untracked(),
            contrasena.get_untracked(),
        )
    };

    let probar = move |_| {
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let (u, us, c) = datos();
        spawn_local(async move {
            match probar_router(u, us, c).await {
                Ok(i) => info.set(Some(i)),
                Err(e) => error.set(Some(texto_error(&e))),
            }
            pendiente.set(false);
        });
    };
    let guardar = repetible(move |yo| {
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let (u, us, c) = datos();
        spawn_local(async move {
            match guardar_router(u, us, c).await {
                Ok(i) => {
                    // Con un firmware probado se quita el solo lectura; con
                    // uno sin probar, queda como estaba (o confirmado).
                    if i.probado {
                        solo_lectura.set(false);
                    }
                    info.set(Some(i));
                    contrasena.set(String::new());
                    aviso.set(Some(t!("router-guardadas")));
                }
                Err(e) => reauth.tratar(&e, error, yo),
            }
            pendiente.set(false);
        });
    });
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        guardar.run(());
    };
    let confirmar = repetible(move |yo| {
        error_firmware.set(None);
        spawn_local(async move {
            match confirmar_firmware().await {
                Ok(()) => solo_lectura.set(false),
                Err(e) => reauth.tratar(&e, error_firmware, yo),
            }
        });
    });

    let firmware_texto = match firmware {
        Some(f) => t!("router-firmware", firmware = f),
        None => t!("router-firmware-desconocido"),
    };
    let modo = move || {
        if solo_lectura.get() {
            view! {
                <Insignia estado=Estado::Aviso texto=t!("router-modo-solo-lectura") />
                <p class="nota">{t!("router-confirmar-texto")}</p>
                <p class="error-formulario" role="alert">{move || error_firmware.get()}</p>
                <div class="botones">
                    <button class="boton" type="button" on:click=move |_| confirmar.run(())>
                        {t!("router-confirmar")}
                    </button>
                </div>
            }
            .into_any()
        } else {
            view! { <Insignia estado=Estado::Ok texto=t!("router-modo-escritura") /> }.into_any()
        }
    };
    let detectado = move || {
        info.get().map(|i| {
            view! {
                <p>
                    {t!("instalar-detectado", modelo = i.modelo.clone(), firmware = i.firmware.clone())}
                </p>
            }
        })
    };

    view! {
        <div class="ajustes-panel">
            <section class="tarjeta" aria-labelledby="router-credenciales">
                <h2 id="router-credenciales">{t!("router-credenciales")}</h2>
                {(!configurado)
                    .then(|| view! { <p class="nota">{t!("router-sin-configurar")}</p> })}
                <form class="ajustes-formulario" on:submit=enviar>
                    <p class="nota">{t!("router-texto")}</p>
                    {campo("panel-router-url", t!("router-url"), "url", "off", url, None)}
                    {campo(
                        "panel-router-usuario",
                        t!("router-usuario"),
                        "text",
                        "off",
                        usuario,
                        None,
                    )}
                    {campo(
                        "panel-router-contrasena",
                        t!("router-contrasena"),
                        "password",
                        "new-password",
                        contrasena,
                        None,
                    )}
                    {detectado}
                    {mensajes(error, aviso)}
                    <div class="botones">
                        <button
                            class="boton-sutil"
                            type="button"
                            disabled=move || pendiente.get()
                            on:click=probar
                        >
                            {t!("router-probar")}
                        </button>
                        <button class="boton" type="submit" disabled=move || pendiente.get()>
                            {t!("router-guardar")}
                        </button>
                    </div>
                </form>
            </section>
            <section class="tarjeta" aria-labelledby="router-firmware">
                <h2 id="router-firmware">{t!("router-firmware-titulo")}</h2>
                <div class="ajustes-estado">
                    <p>{firmware_texto}</p>
                    {modo}
                </div>
            </section>
        </div>
    }
}
