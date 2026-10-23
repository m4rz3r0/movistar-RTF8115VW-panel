//! «Usuarios de la web del router»: cada usuario con sus acciones y el formulario de uno nuevo ([`Usuarios`]).
//!
//! Una fila protegida (el usuario con el que entra el panel, el del
//! operador) lleva su nota y ningún botón. Las demás, «Cambiar contraseña o
//! grupo» (un formulario en la propia fila, que pide la contraseña del
//! panel si hace falta y no se deshace) y «Borrar» (el diálogo de revisión,
//! con su aviso). «Crear usuario» prepara el alta y la abre en el mismo
//! diálogo. Va dentro del formulario del detalle: sin formularios propios,
//! con botones `type="button"`.

use super::aviso::Seguimiento;
use super::rejilla::Pagina;
use crate::ajustes_router::{AjusteVista, InstanciaVista};
use crate::contrasena_forma::{MIN_BYTES, validar_forma_hasta};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::i18n::traducir;
use crate::ui::panel_ajustes::{DialogoReautenticar, Reautenticacion, campo, repetible};
use crate::ui::secreto::Secreto;
use crate::usuarios_router::{
    GrupoUsuario, MAX_OTRO, OperacionUsuario, cambiar_usuario_router, preparar_usuario_router,
    texto_error_otro,
};
use crate::validacion_real::{Funcion3, sin_validar};
use ajustes::ValorLegible;
use leptos::{prelude::*, task::spawn_local};

/// Texto del tipo de cuenta `g`.
fn texto_grupo(g: GrupoUsuario) -> String {
    match g {
        GrupoUsuario::User => t!("ajuste-seguridad-usuarios_router-papel-usuario"),
        GrupoUsuario::Admin => t!("ajuste-seguridad-usuarios_router-papel-administrador"),
    }
}

/// Texto de la opción `clave` del campo «Tipo de cuenta» (`operador`, …).
fn texto_papel(clave: &str) -> String {
    traducir(
        &format!("ajuste-seguridad-usuarios_router-papel-{clave}"),
        None,
    )
}

/// El valor del campo `id` de la instancia, como texto u opción.
fn valor(i: &InstanciaVista, id: &str) -> String {
    match i.campos.iter().find(|c| c.id == id).map(|c| &c.valor) {
        Some(ValorLegible::Texto(t) | ValorLegible::Opcion(t)) => t.clone(),
        _ => String::new(),
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

/// Los usuarios de `a` con sus acciones y el formulario de uno nuevo.
/// `elemento` es el del detalle.
#[component]
pub(crate) fn Usuarios(a: AjusteVista, elemento: String) -> impl IntoView {
    let pagina = expect_context::<Pagina>();
    let s = expect_context::<Seguimiento>();
    let reauth = Reautenticacion::proveer();
    let error = RwSignal::new(None::<String>);
    let ocupado = RwSignal::new(false);
    let editando = RwSignal::new(None::<String>);
    let elemento = StoredValue::new(elemento);
    // Lo que aún no ha confirmado la validación real, desactivado con su
    // motivo (el servidor también lo rechaza).
    let motivo_usuarios = sin_validar(Funcion3::Usuarios);
    let motivo_otro = sin_validar(Funcion3::ContrasenaOtro);
    let sin_gestor = pagina.solo_lectura || motivo_usuarios.is_some();
    let sin_otro = pagina.solo_lectura || motivo_otro.is_some();
    let preparar = move |op: OperacionUsuario| {
        if ocupado.get_untracked() {
            return;
        }
        error.set(None);
        ocupado.set(true);
        // Cada usuario es como una fila de una lista: el cambio sigue la
        // suya (`<elemento>/<nombre>`), no el elemento entero, que al borrar
        // (o deshacer un alta) se cerraría con su detalle.
        let fila = format!("{}/{}", elemento.get_value(), op.usuario());
        spawn_local(async move {
            match preparar_usuario_router(op).await {
                Ok(g) => s.revisar(g, vec![fila]),
                Err(e) => {
                    error.try_set(Some(texto_error(&e)));
                }
            }
            ocupado.try_set(false);
        });
    };
    let filas = if a.instancias.is_empty() {
        view! { <p class="ayuda">{t!("ajuste-seguridad-usuarios_router-vacia")}</p> }.into_any()
    } else {
        a.instancias
            .iter()
            .map(|i| {
                fila(
                    i,
                    editando,
                    (sin_gestor, sin_otro),
                    ocupado.into(),
                    Callback::new(move |u: String| {
                        preparar(OperacionUsuario::Borrar { usuario: u })
                    }),
                )
            })
            .collect_view()
            .into_any()
    };
    let motivos = [motivo_usuarios, motivo_otro]
        .into_iter()
        .flatten()
        .map(|m| view! { <p class="nota-solo-lectura">{traducir(m, None)}</p> })
        .collect_view();
    view! {
        <div class="ajustes-usuarios usuarios-router">{filas}</div>
        <p class="error-formulario" role="alert">{move || error.get()}</p>
        {motivos}
        <NuevoUsuario
            bloqueado=sin_gestor
            ocupado
            preparar=Callback::new(preparar)
        />
        {move || reauth.abierto().then(|| view! { <DialogoReautenticar /> })}
    }
}

/// La fila de un usuario: su nombre y su tipo de cuenta y, si no está
/// protegido, sus acciones. `bloqueos`: sin crear ni borrar, sin cambiar.
fn fila(
    i: &InstanciaVista,
    editando: RwSignal<Option<String>>,
    bloqueos: (bool, bool),
    ocupado: Signal<bool>,
    borrar: Callback<String>,
) -> impl IntoView + use<> {
    let nombre = i.id.clone();
    let papel = RwSignal::new(valor(i, "papel"));
    let cabeza = {
        let nombre = nombre.clone();
        view! {
            <span class="ajustes-usuario-nombre">
                <strong>{nombre}</strong>
                " · "
                {move || texto_papel(&papel.get())}
            </span>
        }
    };
    let Some(nota) = i.nota.clone() else {
        let (sin_gestor, sin_otro) = bloqueos;
        let id_abrir = nombre.clone();
        let id_borrar = nombre.clone();
        let id_form = nombre.clone();
        let abierto = {
            let n = nombre.clone();
            move || editando.with(|e| e.as_deref() == Some(n.as_str()))
        };
        let abierto_aria = abierto.clone();
        let formulario = move || {
            abierto().then(|| {
                view! { <CambiarOtro usuario=id_form.clone() papel editando /> }
            })
        };
        return view! {
            <div class="ajustes-usuario">
                <div class="ajustes-usuario-fila">
                    {cabeza}
                    <span class="botones">
                        <button
                            type="button"
                            class="b-sec"
                            aria-expanded=move || abierto_aria().to_string()
                            aria-label=t!("ajuste-seguridad-usuarios_router-cambiar-de", nombre = nombre.clone())
                            disabled=sin_otro
                            on:click=move |_| editando.set(Some(id_abrir.clone()))
                        >
                            {t!("ajuste-seguridad-usuarios_router-cambiar")}
                        </button>
                        <button
                            type="button"
                            class="b-pel"
                            aria-label=t!("ajuste-seguridad-usuarios_router-borrar-de", nombre = nombre.clone())
                            disabled=move || sin_gestor || ocupado.get()
                            on:click=move |_| borrar.run(id_borrar.clone())
                        >
                            {t!("ajuste-seguridad-usuarios_router-borrar")}
                        </button>
                    </span>
                </div>
                {formulario}
            </div>
        }
        .into_any();
    };
    view! {
        <div class="ajustes-usuario">
            <div class="ajustes-usuario-fila">{cabeza}</div>
            <p class="nota">{traducir(&nota, None)}</p>
        </div>
    }
    .into_any()
}

/// «Cambiar contraseña o grupo» de `usuario`, en su fila. Pide la
/// contraseña del panel si hace falta; al acabar, la fila muestra el tipo
/// de cuenta nuevo (`papel`) y el aviso.
#[component]
fn CambiarOtro(
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
    let dom = format!(
        "usuario-{}",
        usuario.replace(|c: char| !c.is_ascii_alphanumeric(), "-")
    );
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
        <div class="formulario usuario-cambio" role="group" aria-label=t!("ajuste-seguridad-usuarios_router-cambiar")>
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
                    on:click=move |_| cambiar.run(())
                >
                    {t!("ajuste-seguridad-usuarios_router-cambiar-boton")}
                </button>
                <button
                    type="button"
                    class="b-txt"
                    disabled=move || pendiente.get()
                    on:click=move |_| editando.set(None)
                >
                    {t!("panel-cancelar")}
                </button>
            </div>
        </div>
    }
}

/// «Usuario nuevo»: nombre, contraseña con el ojo y tipo de cuenta; «Crear
/// usuario» prepara el alta (`preparar`).
#[component]
fn NuevoUsuario(
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
