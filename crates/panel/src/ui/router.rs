//! Sección «Configuración del router» (`/router/*`): todo lo que necesita el
//! panel para conectar con el router, cada cosa en su página (los ajustes
//! del propio router van en «Ajustes»). La primera es «Conexión con el
//! router» (`/router/conexion`): dirección, usuario y contraseña con los que
//! entra el panel, prueba de la conexión y confirmación del firmware.
//! `/router` lleva a ella.
//!
//! La misma página muestra la red del router detectada (conexión a internet,
//! LAN, telefonía y televisión) y deja elegir a mano la conexión a internet
//! o volver a detectarla.
//!
//! Cambiar las credenciales, confirmar el firmware, elegir la conexión a
//! internet y volver a detectar piden la contraseña del panel si no se ha
//! escrito en los últimos 5 minutos (el mismo diálogo que «Ajustes del
//! panel», [`DialogoReautenticar`]).

use super::panel_ajustes::{
    DialogoReautenticar, Reautenticacion, campo, mensajes, repetible, texto_error,
};
use super::{Estado, Insignia};
use crate::instalar::{ResumenRed, TipoRed};
use crate::panel_ajustes::{
    InfoRouter, confirmar_firmware, guardar_router, guardar_wan, leer_panel, probar_router,
    redetectar,
};
use crate::t;
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use leptos_router::{
    NavigateOptions,
    components::{Outlet, Redirect},
};

/// Primera página de la sección.
pub const RUTA_CONEXION: &str = "/router/conexion";

/// Marco de la sección: la página de la ruta hija.
#[component]
pub fn SeccionRouter() -> impl IntoView {
    view! { <Outlet /> }
}

/// `/router`: lleva a la primera página de la sección. En el navegador
/// sustituye la entrada del historial: «Atrás» no vuelve a `/router` (que
/// redirigiría otra vez), sino a la página de antes.
#[component]
pub fn InicioRouter() -> impl IntoView {
    let options = NavigateOptions {
        replace: true,
        ..NavigateOptions::default()
    };
    view! { <Redirect path=RUTA_CONEXION options /> }
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
                            <div class="ajustes-panel">
                                <Conexion
                                    url=a.router.url
                                    usuario=a.router.usuario
                                    solo_lectura=a.solo_lectura
                                    firmware=a.firmware
                                />
                                <RedDelRouter
                                    red=a.red
                                    wan_manual=a.wan_manual
                                    conexiones=a.conexiones
                                />
                            </div>
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
                Err(e) => {
                    reauth.tratar(&e, error, yo);
                }
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
                Err(e) => {
                    reauth.tratar(&e, error_firmware, yo);
                }
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
    }
}

fn nombre_tipo(t: TipoRed) -> String {
    match t {
        TipoRed::Pppoe => t!("router-red-tipo-pppoe"),
        TipoRed::Dhcp => t!("router-red-tipo-dhcp"),
        TipoRed::Estatica => t!("router-red-tipo-estatica"),
        TipoRed::Otro => t!("router-red-tipo-otro"),
    }
}

/// Resumen de una línea de la red detectada: «Internet: ip2 (PPPoE) · LAN
/// 192.0.2.0/24 · Telefonía: sí · Televisión: sí». Lo usan esta página y el
/// final del asistente de instalación.
pub(crate) fn resumen_red(red: &ResumenRed) -> impl IntoView + use<> {
    let internet = match (&red.wan, red.tipo) {
        (Some(w), Some(tipo)) => t!(
            "router-red-internet-tipo",
            conexion = w.clone(),
            tipo = nombre_tipo(tipo)
        ),
        (Some(w), None) => t!("router-red-internet", conexion = w.clone()),
        (None, _) => t!("router-red-internet-sin-detectar"),
    };
    let lan = match &red.lan {
        Some(l) => t!("router-red-lan", subred = l.clone()),
        None => t!("router-red-lan-sin-detectar"),
    };
    let telefonia = if red.voip {
        t!("router-red-telefonia-si")
    } else {
        t!("router-red-telefonia-no")
    };
    let television = if red.iptv {
        t!("router-red-television-si")
    } else {
        t!("router-red-television-no")
    };
    let items = [internet, lan, telefonia, television]
        .into_iter()
        .map(|texto| view! { <li>{texto}</li> })
        .collect_view();
    view! { <ul class="red-resumen">{items}</ul> }
}

/// Valor de la opción «automática» (la detectada) del selector.
const AUTOMATICA: &str = "";

/// Red del router detectada y elección de la conexión a internet.
#[component]
fn RedDelRouter(
    red: Option<ResumenRed>,
    wan_manual: Option<String>,
    conexiones: Vec<String>,
) -> impl IntoView {
    let reauth = expect_context::<Reautenticacion>();
    let red = RwSignal::new(red);
    // La guardada y la del selector.
    let guardada = RwSignal::new(wan_manual.clone());
    let elegida = RwSignal::new(wan_manual.clone().unwrap_or_default());
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    let conexiones = StoredValue::new(conexiones);

    let guardar = repetible(move |yo| {
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let manual = Some(elegida.get_untracked()).filter(|m| m != AUTOMATICA);
        spawn_local(async move {
            match guardar_wan(manual.clone()).await {
                Ok(()) => {
                    guardada.set(manual);
                    aviso.set(Some(t!("router-red-guardada")));
                }
                Err(e) => {
                    reauth.tratar(&e, error, yo);
                }
            }
            pendiente.set(false);
        });
    });
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        guardar.run(());
    };
    let volver_a_detectar = repetible(move |yo| {
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        spawn_local(async move {
            match redetectar().await {
                Ok(r) => {
                    red.set(Some(r));
                    aviso.set(Some(t!("router-red-redetectada")));
                }
                Err(e) => {
                    reauth.tratar(&e, error, yo);
                }
            }
            pendiente.set(false);
        });
    });

    let resumen = move || match red.get() {
        Some(r) => resumen_red(&r).into_any(),
        None => view! { <p>{t!("router-red-sin-detectar")}</p> }.into_any(),
    };
    // La elegida a mano que el router ya no tiene.
    let no_existe = move || {
        guardada
            .get()
            .filter(|m| !conexiones.with_value(|c| c.contains(m)))
            .map(|m| {
                view! {
                    <Insignia
                        estado=Estado::Aviso
                        texto=t!("router-red-manual-no-existe", conexion = m)
                    />
                }
            })
    };
    let automatica = move || match red.get().and_then(|r| r.wan) {
        Some(w) => t!("router-red-automatica", conexion = w),
        None => t!("router-red-automatica-sin-detectar"),
    };
    let opciones = move || {
        let mut nombres = conexiones.get_value();
        if let Some(m) = guardada.get()
            && !nombres.contains(&m)
        {
            nombres.push(m);
        }
        nombres
            .into_iter()
            .map(|n| {
                let (valor, texto, actual) = (n.clone(), n.clone(), n);
                view! {
                    <option value=valor selected=move || elegida.get() == actual>
                        {texto}
                    </option>
                }
            })
            .collect_view()
    };

    view! {
        <section class="tarjeta" aria-labelledby="router-red">
            <h2 id="router-red">{t!("router-red-titulo")}</h2>
            <div class="ajustes-estado">
                <p class="nota">{t!("router-red-texto")}</p>
                {resumen}
                {no_existe}
            </div>
            <form class="ajustes-formulario" on:submit=enviar>
                <p class="campo">
                    <label for="router-red-conexion">{t!("router-red-conexion")}</label>
                    <select
                        id="router-red-conexion"
                        aria-describedby="router-red-conexion-ayuda"
                        on:change=move |ev| elegida.set(event_target_value(&ev))
                    >
                        <option
                            value=AUTOMATICA
                            selected=move || elegida.get() == AUTOMATICA
                        >
                            {automatica}
                        </option>
                        {opciones}
                    </select>
                    <span class="nota" id="router-red-conexion-ayuda">
                        {t!("router-red-conexion-ayuda")}
                    </span>
                </p>
                {mensajes(error, aviso)}
                <div class="botones">
                    <button
                        class="boton-sutil"
                        type="button"
                        disabled=move || pendiente.get()
                        on:click=move |_| volver_a_detectar.run(())
                    >
                        {t!("router-red-redetectar")}
                    </button>
                    <button class="boton" type="submit" disabled=move || pendiente.get()>
                        {t!("router-red-guardar")}
                    </button>
                </div>
            </form>
        </section>
    }
}
