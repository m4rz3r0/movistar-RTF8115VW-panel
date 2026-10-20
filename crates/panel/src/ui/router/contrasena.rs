//! Tarjeta «Cambiar la contraseña del router» de «Conexión con el router»
//! ([`CambiarContrasena`]): un solo campo «Contraseña nueva» con el ojo, el
//! aviso de que el panel la usará desde ese momento y «Cambiar». Pide la
//! contraseña del panel (si no se ha escrito en los últimos 5 minutos) y
//! después confirma con un diálogo propio; no pasa por el gestor de cambios
//! (ver [`crate::contrasena_router`]).

use crate::contrasena_router::{
    MAX_BYTES, MIN_BYTES, cambiar_contrasena_router, comprobar_contrasena_router,
    texto_error_nueva, validar_forma,
};
use crate::t;
use crate::ui::panel_ajustes::{Reautenticacion, mensajes, repetible};
use crate::ui::secreto::Secreto;
use leptos::{ev::SubmitEvent, html, prelude::*, task::spawn_local};

/// Id de la tarjeta (para el enlace desde Ajustes → Seguridad).
pub const ID_TARJETA: &str = "router-cambiar-contrasena";

/// Tarjeta para cambiar la contraseña del router. Al cambiarla, la tarjeta
/// de las credenciales pasa a tener la nueva: `contrasena_panel` es su
/// campo y `guardada`, la que tiene guardada el panel.
#[component]
pub fn CambiarContrasena(
    contrasena_panel: RwSignal<String>,
    guardada: RwSignal<String>,
) -> impl IntoView {
    let reauth = expect_context::<Reautenticacion>();
    let nueva = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    let confirmando = RwSignal::new(false);
    // El error es de lo escrito (no del router): el campo se marca.
    let nueva_mala = RwSignal::new(false);
    let titulo = NodeRef::<html::H2>::new();
    let dialogo = NodeRef::<html::Dialog>::new();

    // Con la contraseña del panel ya escrita: comprueba la nueva y pide la
    // confirmación.
    let comprobar = repetible(move |yo| {
        let clave = nueva.get_untracked();
        if let Err(e) = validar_forma(&clave) {
            nueva_mala.set(true);
            error.set(Some(texto_error_nueva(e)));
            return;
        }
        nueva_mala.set(false);
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        spawn_local(async move {
            match comprobar_contrasena_router(clave).await {
                Ok(()) => confirmando.set(true),
                Err(e) => reauth.tratar(&e, error, yo),
            }
            pendiente.set(false);
        });
    });
    let cambiar = repetible(move |yo| {
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let clave = nueva.get_untracked();
        spawn_local(async move {
            match cambiar_contrasena_router(clave.clone()).await {
                Ok(()) => {
                    guardada.set(clave.clone());
                    contrasena_panel.set(clave);
                    nueva.set(String::new());
                    aviso.set(Some(t!("contrasena-router-cambiada")));
                }
                Err(e) => reauth.tratar(&e, error, yo),
            }
            pendiente.set(false);
            // Se cierra el diálogo y el foco va al título de la tarjeta (el
            // resultado se lee justo debajo).
            if let Some(d) = dialogo.get_untracked() {
                d.close();
            }
            confirmando.set(false);
            if let Some(h) = titulo.get_untracked() {
                let _ = h.focus();
            }
        });
    });
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        if !pendiente.get_untracked() {
            comprobar.run(());
        }
    };
    // Al escribir se borra el error, y con él la marca.
    let invalido = Signal::derive(move || nueva_mala.get() && error.with(|e| e.is_some()));
    let ayuda = t!(
        "contrasena-router-nueva-ayuda",
        min = MIN_BYTES,
        max = MAX_BYTES
    );

    view! {
        <section class="tarjeta" id=ID_TARJETA aria-labelledby="router-cambiar-titulo">
            <h2 id="router-cambiar-titulo" tabindex="-1" node_ref=titulo>{t!("contrasena-router-titulo")}</h2>
            <form class="ajustes-formulario" on:submit=enviar>
                <p class="nota">{t!("contrasena-router-texto")}</p>
                <div class="campo campo-secreto">
                    <label for="router-contrasena-nueva">{t!("contrasena-router-nueva")}</label>
                    <Secreto
                        valor=nueva
                        nombre=t!("contrasena-router-nueva")
                        editable=true
                        id="router-contrasena-nueva"
                        descrito="router-contrasena-nueva-ayuda"
                        invalido
                        bloqueado=pendiente
                        error
                    />
                    <span class="nota" id="router-contrasena-nueva-ayuda">{ayuda}</span>
                </div>
                {mensajes(error, aviso)}
                <div class="botones">
                    <button class="boton" type="submit" disabled=move || pendiente.get()>
                        {t!("contrasena-router-cambiar")}
                    </button>
                </div>
            </form>
        </section>
        {move || {
            confirmando
                .get()
                .then(|| {
                    view! {
                        <DialogoConfirmar
                            dlg=dialogo
                            pendiente
                            confirmar=Callback::new(move |()| cambiar.run(()))
                            cerrar=Callback::new(move |()| confirmando.set(false))
                        />
                    }
                })
        }}
    }
}

/// Diálogo de confirmación del cambio. Mientras se cambia, no se cierra
/// (ni con Escape).
#[component]
fn DialogoConfirmar(
    dlg: NodeRef<html::Dialog>,
    pendiente: RwSignal<bool>,
    confirmar: Callback<()>,
    cerrar: Callback<()>,
) -> impl IntoView {
    crate::ui::explorador::abrir_modal(dlg);
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        if !pendiente.get_untracked() {
            confirmar.run(());
        }
    };
    let cancelar = move |_| {
        if let Some(d) = dlg.get_untracked() {
            d.close();
        }
    };
    view! {
        <dialog
            class="dialogo dialogo-estrecho"
            node_ref=dlg
            aria-labelledby="router-confirmar-titulo"
            aria-describedby="router-confirmar-texto"
            aria-modal="true"
            on:cancel=move |ev: leptos::ev::Event| {
                if pendiente.get_untracked() {
                    ev.prevent_default();
                }
            }
            on:close=move |_| cerrar.run(())
        >
            <form class="dialogo-cuerpo" on:submit=enviar>
                <header class="dialogo-cabecera">
                    <h2 id="router-confirmar-titulo">{t!("contrasena-router-confirmar-titulo")}</h2>
                </header>
                <p id="router-confirmar-texto">{t!("contrasena-router-confirmar-texto")}</p>
                <p class="nota">{t!("contrasena-router-confirmar-nota")}</p>
                <div class="dialogo-pie">
                    <button
                        class="boton-sutil"
                        type="button"
                        disabled=move || pendiente.get()
                        on:click=cancelar
                    >
                        {t!("panel-cancelar")}
                    </button>
                    <button class="boton" type="submit" autofocus disabled=move || pendiente.get()>
                        {move || {
                            if pendiente.get() {
                                t!("contrasena-router-cambiando")
                            } else {
                                t!("contrasena-router-confirmar-boton")
                            }
                        }}
                    </button>
                </div>
            </form>
        </dialog>
    }
}
