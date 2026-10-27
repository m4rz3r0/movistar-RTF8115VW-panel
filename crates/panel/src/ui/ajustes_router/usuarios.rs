//! «Cuentas de la web del router»: cada cuenta con sus acciones y el formulario de una nueva ([`Usuarios`]).
//!
//! Una fila protegida (el usuario con el que entra el panel, el del
//! operador) lleva su nota y ningún botón. Las demás, «Cambiar contraseña o
//! grupo» (un formulario en la propia fila, que pide la contraseña del
//! panel si hace falta y no se deshace) y «Borrar» (el diálogo de revisión,
//! con su aviso). «Crear usuario» prepara el alta y la abre en el mismo
//! diálogo. Lo que el servidor tiene desactivado en este router
//! ([`crate::validacion_real`]) no sale: ni sus botones ni su formulario,
//! solo una nota de una línea con el motivo. Va dentro del formulario del
//! detalle: sin formularios propios, con botones `type="button"`.

use super::aviso::Seguimiento;
use super::rejilla::Pagina;
use crate::ajustes_router::{AjusteVista, EstadoLectura, InstanciaVista};
use crate::t;
use crate::ui::comun::Insignia;
use crate::ui::i18n::traducir;
use crate::ui::panel_ajustes::{DialogoReautenticar, Reautenticacion};
use crate::ui::{Estado, IconoEstado};
use crate::usuarios_router::{OperacionUsuario, preparar_usuario_router};
use crate::validacion_real::{Desactivadas, Funcion3};
use ajustes::ValorLegible;
use formularios::{CambiarOtro, NuevoUsuario, id_cambio};
use leptos::{prelude::*, task::spawn_local};
use leptos_router::components::A;

mod formularios;

/// Las acciones de las filas: cuáles salen (`borrar`, `cambiar`: las que
/// el servidor tiene habilitadas) y cuándo esperan (`sin_gestor`: sin
/// crear ni borrar; `solo_lectura`).
#[derive(Clone, Copy)]
struct Acciones {
    borrar: bool,
    cambiar: bool,
    sin_gestor: bool,
    solo_lectura: bool,
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

/// Lo que dice la lista cuando no tiene filas que pintar (`None` si las
/// tiene): sin leerla, su motivo; leída y `vacia`, que no hay usuarios.
#[derive(Debug, PartialEq, Eq)]
enum SinFilas {
    Motivo(String),
    Vacia,
}

/// Ver [`SinFilas`].
fn sin_filas(estado: &EstadoLectura, vacia: bool) -> Option<SinFilas> {
    match estado {
        EstadoLectura::SinLeer(m) => Some(SinFilas::Motivo(m.clone())),
        EstadoLectura::Leido => vacia.then_some(SinFilas::Vacia),
    }
}

/// Sin crear ni borrar usuarios: el panel en solo lectura o la lista sin
/// leer (no se sabe quién hay).
fn gestor_bloqueado(solo_lectura: bool, estado: &EstadoLectura) -> bool {
    solo_lectura || *estado != EstadoLectura::Leido
}

/// Los usuarios de `a` con sus acciones y el formulario de uno nuevo.
/// `elemento` es el del detalle; `desactivadas`, lo que el servidor tiene
/// desactivado (no sale).
#[component]
pub(crate) fn Usuarios(
    a: AjusteVista,
    elemento: String,
    desactivadas: Desactivadas,
) -> impl IntoView {
    let pagina = expect_context::<Pagina>();
    let s = expect_context::<Seguimiento>();
    let reauth = Reautenticacion::proveer();
    let error = RwSignal::new(None::<String>);
    let ocupado = RwSignal::new(false);
    let editando = RwSignal::new(None::<String>);
    let elemento = StoredValue::new(elemento);
    // Lo que el servidor tiene desactivado en este router no sale (y él lo
    // rechaza igual): solo su nota.
    let motivo_usuarios = desactivadas.motivo(Funcion3::Usuarios);
    let motivo_otro = desactivadas.motivo(Funcion3::ContrasenaOtro);
    let acciones = Acciones {
        borrar: motivo_usuarios.is_none(),
        cambiar: motivo_otro.is_none(),
        sin_gestor: gestor_bloqueado(pagina.solo_lectura, &a.estado),
        solo_lectura: pagina.solo_lectura,
    };
    // Crear o borrar pide la contraseña del panel si hace falta: el
    // diálogo repite la misma operación (`repetir`).
    let repetir = StoredValue::new(None::<Callback<OperacionUsuario>>);
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
        let otra_vez = op.clone();
        spawn_local(async move {
            match preparar_usuario_router(op).await {
                Ok(g) => s.revisar(g, vec![fila]),
                Err(e) => {
                    let reintentar = Callback::new(move |()| {
                        if let Some(cb) = repetir.get_value() {
                            cb.run(otra_vez.clone());
                        }
                    });
                    reauth.tratar(&e, error, reintentar);
                }
            }
            ocupado.try_set(false);
        });
    };
    repetir.set_value(Some(Callback::new(preparar)));
    // Lo que cambia el cambio en curso: las filas lo dicen ya, sin esperar
    // a que termine la comprobación (al terminar, la lista se vuelve a
    // leer). Un usuario que se está creando aún no está en la lista: va
    // detrás, con su insignia.
    let en_curso = Memo::new(move |_| s.filas_en_curso(&elemento.get_value()));
    let existentes: Vec<String> = a.instancias.iter().map(|i| i.id.clone()).collect();
    let nuevos = move || {
        en_curso
            .get()
            .into_iter()
            .filter(|n| !existentes.contains(n))
            .map(|n| fila_en_curso(&n, s, &elemento.get_value()))
            .collect_view()
    };
    let filas = match sin_filas(&a.estado, a.instancias.is_empty()) {
        Some(SinFilas::Motivo(m)) => view! {
            <p class="aviso estado-error">
                <IconoEstado estado=Estado::Error />
                <span>{m}</span>
            </p>
        }
        .into_any(),
        Some(SinFilas::Vacia) => {
            view! { <p class="ayuda">{t!("ajuste-seguridad-usuarios_router-vacia")}</p> }.into_any()
        }
        None => a
            .instancias
            .iter()
            .map(|i| {
                fila(
                    i,
                    format!("{}/{}", elemento.get_value(), i.id),
                    editando,
                    acciones,
                    ocupado.into(),
                    Callback::new(move |u: String| {
                        preparar(OperacionUsuario::Borrar { usuario: u })
                    }),
                )
            })
            .collect_view()
            .into_any(),
    };
    let motivos = [motivo_usuarios, motivo_otro]
        .into_iter()
        .flatten()
        .map(|m| view! { <p class="nota">{traducir(m, None)}</p> })
        .collect_view();
    let nuevo = acciones.borrar.then(|| {
        view! {
            <NuevoUsuario
                en_curso=en_curso.into()
                bloqueado=acciones.sin_gestor
                ocupado
                preparar=Callback::new(preparar)
            />
        }
    });
    view! {
        // Las dos listas se confundían: esta dice cuál no es y enlaza a la otra.
        <p class="aviso estado-neutro usuarios-router-no-panel">
            <span>
                {t!("usuarios-router-no-panel")} " "
                <A href="/panel">{t!("usuarios-router-no-panel-enlace")}</A>
                "."
            </span>
        </p>
        <div class="ajustes-usuarios usuarios-router">{filas} {nuevos}</div>
        <p class="error-formulario" role="alert">{move || error.get()}</p>
        {motivos}
        {nuevo}
        {move || reauth.abierto().then(|| view! { <DialogoReautenticar /> })}
    }
}

/// La fila de un usuario que se está creando: su nombre, la insignia del
/// cambio y qué falta.
fn fila_en_curso(nombre: &str, s: Seguimiento, elemento: &str) -> impl IntoView + use<> {
    let id = format!("{elemento}/{nombre}");
    let insignia = move || {
        s.insignia(&id)
            .map(|(estado, texto)| view! { <Insignia estado texto /> })
    };
    view! {
        <div class="ajustes-usuario usuario-en-curso">
            <div class="ajustes-usuario-fila">
                <span class="ajustes-usuario-nombre"><strong>{nombre.to_owned()}</strong></span>
                {insignia}
            </div>
            <p class="nota">{t!("ajustes-usuario-en-curso-nuevo")}</p>
        </div>
    }
}

/// La fila de un usuario: su nombre y su tipo de cuenta y, si no está
/// protegido, sus acciones (ver [`Acciones`]). `en_curso_id` es como la
/// nombra el cambio en curso (`<elemento>/<nombre>`): mientras la cambia,
/// lo dice su insignia y sus botones esperan.
fn fila(
    i: &InstanciaVista,
    en_curso_id: String,
    editando: RwSignal<Option<String>>,
    acciones: Acciones,
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
    let s = expect_context::<Seguimiento>();
    let en_curso_id = StoredValue::new(en_curso_id);
    let cambiando = move || en_curso_id.with_value(|id| s.insignia(id));
    let insignia = move || cambiando().map(|(estado, texto)| view! { <Insignia estado texto /> });
    let Some(nota) = i.nota.clone() else {
        let Acciones {
            borrar: con_borrar,
            cambiar: con_cambiar,
            sin_gestor,
            solo_lectura,
        } = acciones;
        let controla = id_cambio(&nombre);
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
                    {insignia}
                    {(con_cambiar || con_borrar).then(|| view! {
                        <span class="botones">
                            {con_cambiar.then(|| view! {
                                <button
                                    type="button"
                                    class="b-sec"
                                    aria-expanded=move || abierto_aria().to_string()
                                    aria-controls=controla
                                    aria-label=t!("ajuste-seguridad-usuarios_router-cambiar-de", nombre = nombre.clone())
                                    disabled=move || solo_lectura || ocupado.get() || cambiando().is_some()
                                    on:click=move |_| editando.set(Some(id_abrir.clone()))
                                >
                                    {t!("ajuste-seguridad-usuarios_router-cambiar")}
                                </button>
                            })}
                            {con_borrar.then(|| view! {
                                <button
                                    type="button"
                                    class="b-pel"
                                    aria-label=t!("ajuste-seguridad-usuarios_router-borrar-de", nombre = nombre.clone())
                                    disabled=move || sin_gestor || ocupado.get() || cambiando().is_some()
                                    on:click=move |_| borrar.run(id_borrar.clone())
                                >
                                    {t!("ajuste-seguridad-usuarios_router-borrar")}
                                </button>
                            })}
                        </span>
                    })}
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

#[cfg(test)]
mod tests {
    use super::{SinFilas, gestor_bloqueado, sin_filas};
    use crate::ajustes_router::EstadoLectura;

    /// Sin poder leer la lista, la tarjeta da el motivo y no dice que el
    /// router no tenga usuarios; leída y vacía, sí lo dice.
    #[test]
    fn la_lista_sin_leer_no_se_da_por_vacia() {
        let fallo = EstadoLectura::SinLeer("No se pudo leer".into());
        assert_eq!(
            sin_filas(&fallo, true),
            Some(SinFilas::Motivo("No se pudo leer".into()))
        );
        assert_eq!(
            sin_filas(&EstadoLectura::Leido, true),
            Some(SinFilas::Vacia)
        );
        assert_eq!(sin_filas(&EstadoLectura::Leido, false), None);
    }

    /// Sin la lista leída no se crea ni se borra nadie.
    #[test]
    fn sin_leer_la_lista_no_se_crea_usuario() {
        let fallo = EstadoLectura::SinLeer("No se pudo leer".into());
        assert!(gestor_bloqueado(false, &fallo));
        assert!(!gestor_bloqueado(false, &EstadoLectura::Leido));
        assert!(gestor_bloqueado(true, &EstadoLectura::Leido));
    }
}
