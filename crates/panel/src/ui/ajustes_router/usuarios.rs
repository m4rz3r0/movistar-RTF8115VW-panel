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
use crate::ajustes_router::{AjusteVista, EstadoLectura, InstanciaVista};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::i18n::traducir;
use crate::ui::panel_ajustes::{DialogoReautenticar, Reautenticacion};
use crate::ui::{Estado, IconoEstado};
use crate::usuarios_router::{OperacionUsuario, preparar_usuario_router};
use crate::validacion_real::{Funcion3, sin_validar};
use ajustes::ValorLegible;
use formularios::{CambiarOtro, NuevoUsuario, id_cambio};
use leptos::{prelude::*, task::spawn_local};

mod formularios;

/// Ids de los textos que dicen por qué crear o borrar usuarios, o cambiar
/// otro, está desactivado por la validación real (`aria-describedby` de sus
/// botones).
const MOTIVO_USUARIOS: &str = "usuarios-router-motivo-usuarios";
const MOTIVO_OTRO: &str = "usuarios-router-motivo-otro";

/// Lo que desactiva las acciones de las filas: sin crear ni borrar, sin
/// cambiar otro, y el id del motivo de cada uno si viene de la validación
/// real.
#[derive(Clone, Copy)]
struct Bloqueos {
    sin_gestor: bool,
    sin_otro: bool,
    motivo_gestor: Option<&'static str>,
    motivo_otro: Option<&'static str>,
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

/// Sin crear ni borrar usuarios: el panel en solo lectura, la función sin
/// validar o la lista sin leer (no se sabe quién hay).
fn gestor_bloqueado(solo_lectura: bool, sin_validar: bool, estado: &EstadoLectura) -> bool {
    solo_lectura || sin_validar || *estado != EstadoLectura::Leido
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
    let bloqueos = Bloqueos {
        sin_gestor: gestor_bloqueado(pagina.solo_lectura, motivo_usuarios.is_some(), &a.estado),
        sin_otro: pagina.solo_lectura || motivo_otro.is_some(),
        motivo_gestor: motivo_usuarios.map(|_| MOTIVO_USUARIOS),
        motivo_otro: motivo_otro.map(|_| MOTIVO_OTRO),
    };
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
                    editando,
                    bloqueos,
                    ocupado.into(),
                    Callback::new(move |u: String| {
                        preparar(OperacionUsuario::Borrar { usuario: u })
                    }),
                )
            })
            .collect_view()
            .into_any(),
    };
    let motivos = [
        (motivo_usuarios, MOTIVO_USUARIOS),
        (motivo_otro, MOTIVO_OTRO),
    ]
    .into_iter()
    .filter_map(|(m, id)| m.map(|m| (m, id)))
    .map(|(m, id)| view! { <p class="nota-solo-lectura" id=id>{traducir(m, None)}</p> })
    .collect_view();
    view! {
        <div class="ajustes-usuarios usuarios-router">{filas}</div>
        <p class="error-formulario" role="alert">{move || error.get()}</p>
        {motivos}
        <NuevoUsuario
            bloqueado=bloqueos.sin_gestor
            ocupado
            preparar=Callback::new(preparar)
            motivo=bloqueos.motivo_gestor
        />
        {move || reauth.abierto().then(|| view! { <DialogoReautenticar /> })}
    }
}

/// La fila de un usuario: su nombre y su tipo de cuenta y, si no está
/// protegido, sus acciones (ver [`Bloqueos`]).
fn fila(
    i: &InstanciaVista,
    editando: RwSignal<Option<String>>,
    bloqueos: Bloqueos,
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
        let Bloqueos {
            sin_gestor,
            sin_otro,
            motivo_gestor,
            motivo_otro,
        } = bloqueos;
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
                    <span class="botones">
                        <button
                            type="button"
                            class="b-sec"
                            aria-expanded=move || abierto_aria().to_string()
                            aria-controls=controla
                            aria-describedby=motivo_otro
                            aria-label=t!("ajuste-seguridad-usuarios_router-cambiar-de", nombre = nombre.clone())
                            disabled=move || sin_otro || ocupado.get()
                            on:click=move |_| editando.set(Some(id_abrir.clone()))
                        >
                            {t!("ajuste-seguridad-usuarios_router-cambiar")}
                        </button>
                        <button
                            type="button"
                            class="b-pel"
                            aria-label=t!("ajuste-seguridad-usuarios_router-borrar-de", nombre = nombre.clone())
                            aria-describedby=motivo_gestor
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
        assert!(gestor_bloqueado(false, false, &fallo));
        assert!(!gestor_bloqueado(false, false, &EstadoLectura::Leido));
        assert!(gestor_bloqueado(true, false, &EstadoLectura::Leido));
        assert!(gestor_bloqueado(false, true, &EstadoLectura::Leido));
    }
}
