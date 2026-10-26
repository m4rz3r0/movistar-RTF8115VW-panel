//! Los formularios de la tarjeta de usuarios: «Cambiar contraseña o grupo» en la fila ([`CambiarOtro`]) y «Usuario nuevo» ([`NuevoUsuario`]).

use crate::contrasena_forma::{MIN_BYTES, validar_forma_hasta};
use crate::t;
use crate::ui::panel_ajustes::{Reautenticacion, campo, repetible};
use crate::ui::secreto::Secreto;
use crate::usuarios_router::{
    GrupoUsuario, MAX_OTRO, OperacionUsuario, cambiar_usuario_router, texto_error_otro,
};
use leptos::{prelude::*, task::spawn_local};

/// Id del formulario de cambio de `usuario` en su fila (para
/// `aria-controls`).
pub(super) fn id_cambio(usuario: &str) -> String {
    format!("{}-cambio", dom(usuario))
}

/// Prefijo de los ids de la fila de `usuario`.
fn dom(usuario: &str) -> String {
    format!(
        "usuario-{}",
        usuario.replace(|c: char| !c.is_ascii_alphanumeric(), "-")
    )
}

/// Texto del tipo de cuenta `g`.
fn texto_grupo(g: GrupoUsuario) -> String {
    match g {
        GrupoUsuario::User => t!("ajuste-seguridad-usuarios_router-papel-usuario"),
        GrupoUsuario::Admin => t!("ajuste-seguridad-usuarios_router-papel-administrador"),
    }
}

/// Elegir el tipo de cuenta: «Usuario» o «Administrador». `nombre` agrupa
/// los botones de opción (uno por formulario).
fn grupos(elegido: RwSignal<GrupoUsuario>, nombre: String) -> impl IntoView {
    let opciones = GrupoUsuario::TODOS
        .into_iter()
        .map(|g| {
            let id = format!("{nombre}-{}", g.del_router());
            view! {
                <div class="opcion">
                    <input
                        id=id.clone()
                        type="radio"
                        name=nombre.clone()
                        value=g.del_router()
                        checked=elegido.get_untracked() == g
                        prop:checked=move || elegido.get() == g
                        on:change=move |_| elegido.set(g)
                    />
                    <label class="opcion-nombre" for=id>{texto_grupo(g)}</label>
                </div>
            }
        })
        .collect_view();
    view! {
        <fieldset class="opciones">
            <legend>{t!("ajuste-seguridad-usuarios_router-papel")}</legend>
            {opciones}
        </fieldset>
    }
}

/// Campo de una contraseña nueva con el ojo y su ayuda.
fn contrasena(
    valor: RwSignal<String>,
    id: String,
    error: RwSignal<Option<String>>,
    bloqueado: Signal<bool>,
) -> impl IntoView {
    let ayuda = format!("{id}-ayuda");
    view! {
        <div class="campo campo-secreto">
            <label for=id.clone()>{t!("ajuste-seguridad-usuarios_router-contrasena")}</label>
            <Secreto
                valor
                nombre=t!("ajuste-seguridad-usuarios_router-contrasena")
                editable=true
                id
                descrito=ayuda.clone()
                bloqueado
                error
                maxlength=MAX_OTRO
            />
            <span class="nota" id=ayuda>
                {t!("ajuste-seguridad-usuarios_router-contrasena-ayuda", min = MIN_BYTES, max = MAX_OTRO)}
            </span>
        </div>
    }
}

/// «Cambiar contraseña o grupo» de `usuario`, en su fila. Pide la
/// contraseña del panel si hace falta; al acabar, la fila muestra el tipo
/// de cuenta nuevo (`papel`) y el aviso.
#[component]
pub(super) fn CambiarOtro(
    usuario: String,
    papel: RwSignal<String>,
    editando: RwSignal<Option<String>>,
) -> impl IntoView {
    let reauth = expect_context::<Reautenticacion>();
    let nueva = RwSignal::new(String::new());
    let grupo =
        RwSignal::new(GrupoUsuario::de_clave(&papel.get_untracked()).unwrap_or(GrupoUsuario::User));
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    let dom = dom(&usuario);
    let id = id_cambio(&usuario);
    let usuario = StoredValue::new(usuario);
    let cambiar = repetible(move |yo| {
        let clave = nueva.get_untracked();
        if let Err(e) = validar_forma_hasta(&clave, MAX_OTRO) {
            error.set(Some(texto_error_otro(e)));
            return;
        }
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let g = grupo.get_untracked();
        let u = usuario.get_value();
        spawn_local(async move {
            match cambiar_usuario_router(u.clone(), clave, g).await {
                Ok(()) => {
                    nueva.try_set(String::new());
                    papel.try_set(g.clave().to_owned());
                    aviso.try_set(Some(t!(
                        "ajuste-seguridad-usuarios_router-cambiado",
                        nombre = u
                    )));
                }
                Err(e) => reauth.tratar(&e, error, yo),
            }
            pendiente.try_set(false);
        });
    });
    view! {
        <div id=id class="formulario usuario-cambio" role="group" aria-label=t!("ajuste-seguridad-usuarios_router-cambiar")>
            <p class="nota">{t!("ajuste-seguridad-usuarios_router-cambiar-nota")}</p>
            {contrasena(nueva, format!("{dom}-nueva"), error, pendiente.into())}
            {grupos(grupo, format!("{dom}-grupo"))}
            <p class="error-formulario" role="alert">{move || error.get()}</p>
            <p class="aviso-formulario" role="status">{move || aviso.get()}</p>
            <div class="botones">
                <button
                    type="button"
                    class="b-sec"
                    disabled=move || pendiente.get()
                    on:click=move |_| editando.set(None)
                >
                    {t!("panel-cancelar")}
                </button>
                <button
                    type="button"
                    class="b-pri"
                    aria-busy=move || pendiente.get().then_some("true")
                    disabled=move || pendiente.get()
                    on:click=move |_| cambiar.run(())
                >
                    {t!("ajuste-seguridad-usuarios_router-cambiar-boton")}
                </button>
            </div>
        </div>
    }
}

/// «Usuario nuevo»: nombre, contraseña con el ojo y tipo de cuenta; «Crear
/// usuario» prepara el alta (`preparar`). No sale si el servidor tiene
/// desactivado crear y borrar usuarios.
#[component]
pub(super) fn NuevoUsuario(
    /// Los usuarios que cambia el cambio en curso: si entre ellos está el
    /// que se ha escrito, ya se está creando y el formulario se vacía.
    en_curso: Signal<Vec<String>>,
    bloqueado: bool,
    ocupado: RwSignal<bool>,
    preparar: Callback<OperacionUsuario>,
) -> impl IntoView {
    let nombre = RwSignal::new(String::new());
    let clave = RwSignal::new(String::new());
    let grupo = RwSignal::new(GrupoUsuario::User);
    let error = RwSignal::new(None::<String>);
    let crear = move |_| {
        if bloqueado || ocupado.get_untracked() {
            return;
        }
        if let Err(e) = validar_forma_hasta(&clave.get_untracked(), MAX_OTRO) {
            error.set(Some(texto_error_otro(e)));
            return;
        }
        error.set(None);
        preparar.run(OperacionUsuario::Crear {
            usuario: nombre.get_untracked().trim().to_owned(),
            contrasena: clave.get_untracked(),
            grupo: grupo.get_untracked(),
        });
    };
    Effect::new(move |_| {
        // Primero la señal, para que el efecto la siga siempre.
        let curso = en_curso.get();
        let escrito = nombre.get_untracked().trim().to_owned();
        if !escrito.is_empty() && curso.contains(&escrito) {
            nombre.set(String::new());
            clave.set(String::new());
            grupo.set(GrupoUsuario::User);
        }
    });
    let sin_tocar = Signal::derive(move || bloqueado || ocupado.get());
    view! {
        <section class="formulario usuario-nuevo" aria-labelledby="usuarios-router-nuevo-titulo">
            <h4 id="usuarios-router-nuevo-titulo">{t!("ajuste-seguridad-usuarios_router-crear-titulo")}</h4>
            {campo(
                "usuarios-router-nuevo-nombre",
                t!("ajuste-seguridad-usuarios_router-nuevo-nombre"),
                "text",
                "off",
                nombre,
                Some(t!("ajuste-seguridad-usuarios_router-nuevo-nombre-ayuda")),
            )}
            {contrasena(clave, "usuarios-router-nuevo-contrasena".into(), error, sin_tocar)}
            {grupos(grupo, "usuarios-router-nuevo-grupo".into())}
            <p class="error-formulario" role="alert">{move || error.get()}</p>
            <div class="botones">
                <button
                    type="button"
                    class="b-pri"
                    disabled=move || sin_tocar.get()
                    on:click=crear
                >
                    {t!("ajuste-seguridad-usuarios_router-crear")}
                </button>
            </div>
        </section>
    }
}
