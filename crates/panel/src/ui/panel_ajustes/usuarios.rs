//! «Quién entra en este panel»: los usuarios del panel, con sus acciones y
//! el formulario de uno nuevo ([`SeccionUsuarios`]).
//!
//! No son las cuentas del router (Ajustes → Seguridad → «Cuentas de la web
//! del router»): la sección lo dice y enlaza allí, porque las dos listas se
//! confundían. Lo que no se deshace avisa antes: cambiar una contraseña
//! (cierra las sesiones de ese usuario) lo dice junto al campo; borrar pide
//! confirmación en un diálogo.

use super::{MIN_CONTRASENA, Reautenticacion, mensajes, repetible};
use crate::panel_ajustes::{borrar_usuario, cambiar_contrasena, crear_usuario};
use crate::t;
use crate::ui::contrasena_campo::CampoContrasena;
use leptos::{ev::SubmitEvent, html, prelude::*, task::spawn_local};
use leptos_router::components::A;

/// Dónde están las cuentas del router.
const USUARIOS_ROUTER: &str = "/ajustes/seguridad/usuarios";

/// Por qué no se puede borrar `usuario`, si no se puede: es el tuyo, o el
/// único que queda.
fn sin_borrar(usuario: &str, yo: &str, cuantos: usize) -> Option<String> {
    if usuario == yo {
        Some(t!("panel-usuario-no-borrar-tu"))
    } else if cuantos <= 1 {
        Some(t!("panel-usuario-no-borrar-ultimo"))
    } else {
        None
    }
}

/// Id del motivo por el que no se borra `usuario` (`aria-describedby`).
fn id_motivo(usuario: &str) -> String {
    format!(
        "panel-usuario-{}-motivo",
        usuario.replace(|c: char| !c.is_ascii_alphanumeric(), "-")
    )
}

#[component]
pub(super) fn SeccionUsuarios(usuarios: Vec<String>, yo: String) -> impl IntoView {
    let reauth = expect_context::<Reautenticacion>();
    let usuarios = RwSignal::new(usuarios);
    let yo = StoredValue::new(yo);
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    // Usuario cuya contraseña se cambia / cuyo borrado se confirma.
    let editando = RwSignal::new(None::<String>);
    let borrando = RwSignal::new(None::<String>);
    let clave_nueva = RwSignal::new(String::new());
    let nombre = RwSignal::new(String::new());
    let clave = RwSignal::new(String::new());

    let borrar = repetible(move |yo_cb| {
        let Some(u) = borrando.get_untracked() else {
            return;
        };
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        spawn_local(async move {
            match borrar_usuario(u.clone()).await {
                Ok(()) => {
                    usuarios.update(|l| l.retain(|x| *x != u));
                    borrando.set(None);
                    aviso.set(Some(t!("panel-usuario-borrado", usuario = u)));
                }
                Err(e) => {
                    borrando.set(None);
                    reauth.tratar(&e, error, yo_cb);
                }
            }
            pendiente.set(false);
        });
    });
    let cambiar = repetible(move |yo_cb| {
        let Some(u) = editando.get_untracked() else {
            return;
        };
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let c = clave_nueva.get_untracked();
        spawn_local(async move {
            match cambiar_contrasena(u.clone(), c).await {
                Ok(()) => {
                    editando.set(None);
                    clave_nueva.set(String::new());
                    aviso.set(Some(t!("panel-usuario-cambiada", usuario = u)));
                }
                Err(e) => reauth.tratar(&e, error, yo_cb),
            }
            pendiente.set(false);
        });
    });
    let crear = repetible(move |yo_cb| {
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let (u, c) = (nombre.get_untracked(), clave.get_untracked());
        spawn_local(async move {
            match crear_usuario(u.clone(), c).await {
                Ok(()) => {
                    usuarios.update(|l| l.push(u.clone()));
                    nombre.set(String::new());
                    clave.set(String::new());
                    aviso.set(Some(t!("panel-usuario-creado", usuario = u)));
                }
                Err(e) => reauth.tratar(&e, error, yo_cb),
            }
            pendiente.set(false);
        });
    });

    let fila = move |u: String| {
        let es_yo = yo.with_value(|y| *y == u);
        let (u1, u2, u3) = (u.clone(), u.clone(), u.clone());
        let editar_este = move || editando.get().as_deref() == Some(u1.as_str());
        let aviso_cambio = if es_yo {
            t!("panel-usuario-cambiar-aviso-tuyo")
        } else {
            t!("panel-usuario-cambiar-aviso-otro", usuario = u.clone())
        };
        let formulario = move || {
            editar_este().then(|| {
                let aviso_cambio = aviso_cambio.clone();
                view! {
                    <form
                        class="ajustes-usuario-form"
                        on:submit=move |ev: SubmitEvent| {
                            ev.prevent_default();
                            cambiar.run(());
                        }
                    >
                        <CampoContrasena
                            id="panel-usuario-clave-nueva"
                            etiqueta=t!("panel-usuario-clave-nueva", n = MIN_CONTRASENA)
                            autocompletar="new-password"
                            valor=clave_nueva
                            requerido=true
                        />
                        <p class="aviso estado-aviso">{aviso_cambio}</p>
                        <div class="botones">
                            <button
                                class="b-sec"
                                type="button"
                                on:click=move |_| editando.set(None)
                            >
                                {t!("panel-cancelar")}
                            </button>
                            <button
                                class="b-pri"
                                type="submit"
                                aria-busy=move || pendiente.get().then_some("true")
                                disabled=move || pendiente.get()
                            >
                                {t!("panel-usuario-cambiar")}
                            </button>
                        </div>
                    </form>
                }
            })
        };
        let motivo =
            Signal::derive(move || usuarios.with(|l| sin_borrar(&u2, &yo.get_value(), l.len())));
        let motivo_id = id_motivo(&u);
        let describe = {
            let id = motivo_id.clone();
            move || motivo.get().map(|_| id.clone())
        };
        let (ue, ub) = (u3.clone(), u3.clone());
        view! {
            <li class="ajustes-usuario">
                <div class="ajustes-usuario-fila">
                    <span class="ajustes-usuario-nombre">
                        <strong>{u.clone()}</strong>
                        {es_yo.then(|| view! { " " <span class="nota">{t!("panel-usuario-tu")}</span> })}
                    </span>
                    <div class="botones">
                        <button
                            class="b-sec"
                            type="button"
                            aria-label=t!("panel-usuario-contrasena-de", usuario = ue.clone())
                            aria-expanded=move || editando.with(|e| e.as_deref() == Some(ue.as_str())).to_string()
                            on:click={
                                let ue = u3.clone();
                                move |_| {
                                    clave_nueva.set(String::new());
                                    editando.set(Some(ue.clone()));
                                }
                            }
                        >
                            {t!("panel-usuario-contrasena")}
                        </button>
                        <button
                            class="b-pel"
                            type="button"
                            aria-label=t!("panel-usuario-borrar-de", usuario = ub.clone())
                            aria-describedby=describe
                            disabled=move || motivo.get().is_some() || pendiente.get()
                            on:click=move |_| {
                                editando.set(None);
                                borrando.set(Some(ub.clone()));
                            }
                        >
                            {t!("panel-usuario-borrar")}
                        </button>
                    </div>
                </div>
                {move || motivo.get().map(|m| view! { <p class="nota" id=motivo_id.clone()>{m}</p> })}
                {formulario}
            </li>
        }
    };

    view! {
        <section class="tarjeta" aria-labelledby="panel-usuarios">
            <h2 id="panel-usuarios">{t!("panel-usuarios")}</h2>
            <p class="nota">{t!("panel-usuarios-texto")}</p>
            <p class="aviso estado-neutro panel-usuarios-no-router">
                <span>
                    {t!("panel-usuarios-no-router")} " "
                    <A href=USUARIOS_ROUTER>{t!("panel-usuarios-no-router-enlace")}</A>
                    "."
                </span>
            </p>
            <ul class="lista-limpia ajustes-usuarios">
                <For each=move || usuarios.get() key=|u| u.clone() children=fila />
            </ul>
            {mensajes(error, aviso)}
            <form
                class="ajustes-formulario"
                on:submit=move |ev: SubmitEvent| {
                    ev.prevent_default();
                    crear.run(());
                }
            >
                <h3>{t!("panel-usuario-nuevo")}</h3>
                {super::campo("panel-usuario-nombre", t!("panel-usuario-nombre"), "text", "off", nombre, None)}
                <CampoContrasena
                    id="panel-usuario-clave"
                    etiqueta=t!("panel-usuario-clave", n = MIN_CONTRASENA)
                    autocompletar="new-password"
                    valor=clave
                    requerido=true
                />
                <div class="botones">
                    <button
                        class="b-pri"
                        type="submit"
                        aria-busy=move || pendiente.get().then_some("true")
                        disabled=move || pendiente.get()
                    >
                        {t!("panel-usuario-crear")}
                    </button>
                </div>
            </form>
            {move || borrando.get().map(|u| view! { <ConfirmarBorrado usuario=u borrando pendiente borrar /> })}
        </section>
    }
}

/// Confirma el borrado de `usuario`: qué pasa (deja de entrar, se cierran
/// sus sesiones, no se deshace, el router no cambia) y el botón de peligro.
#[component]
fn ConfirmarBorrado(
    usuario: String,
    borrando: RwSignal<Option<String>>,
    pendiente: RwSignal<bool>,
    borrar: Callback<()>,
) -> impl IntoView {
    let dlg = NodeRef::<html::Dialog>::new();
    crate::ui::explorador::abrir_modal(dlg);
    let cerrar = move || {
        if let Some(d) = dlg.get_untracked() {
            d.close();
        }
    };
    view! {
        <dialog
            class="dialogo dialogo-estrecho"
            node_ref=dlg
            aria-labelledby="panel-borrar-titulo"
            aria-describedby="panel-borrar-texto"
            aria-modal="true"
            on:cancel=move |ev: leptos::ev::Event| {
                if pendiente.get_untracked() {
                    ev.prevent_default();
                }
            }
            on:close=move |_| {
                if !pendiente.get_untracked() {
                    borrando.set(None);
                }
            }
        >
            <form
                class="dialogo-cuerpo"
                on:submit=move |ev: SubmitEvent| {
                    ev.prevent_default();
                    borrar.run(());
                }
            >
                <header class="dialogo-cabecera">
                    <h2 id="panel-borrar-titulo">
                        {t!("panel-usuario-borrar-titulo", usuario = usuario.clone())}
                    </h2>
                </header>
                <p id="panel-borrar-texto">
                    {t!("panel-usuario-borrar-texto", usuario = usuario.clone())}
                </p>
                <p class="nota">{t!("panel-usuario-borrar-router")}</p>
                <div class="dialogo-pie">
                    <button
                        class="b-sec"
                        type="button"
                        autofocus
                        disabled=move || pendiente.get()
                        on:click=move |_| cerrar()
                    >
                        {t!("panel-cancelar")}
                    </button>
                    <button
                        class="b-pel"
                        type="submit"
                        aria-busy=move || pendiente.get().then_some("true")
                        disabled=move || pendiente.get()
                    >
                        {t!("panel-usuario-borrar-confirmar", usuario = usuario.clone())}
                    </button>
                </div>
            </form>
        </dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::sin_borrar;

    /// No se borra uno a sí mismo ni al último; a otro, sí.
    #[test]
    fn quien_no_se_puede_borrar() {
        assert!(sin_borrar("ana", "ana", 2).is_some());
        assert!(sin_borrar("luis", "ana", 1).is_some());
        assert_eq!(sin_borrar("luis", "ana", 2), None);
    }
}
