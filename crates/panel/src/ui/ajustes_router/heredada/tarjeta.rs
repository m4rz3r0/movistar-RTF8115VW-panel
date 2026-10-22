//! Tarjeta de un ajuste en los temas que aún no van en rejilla: título,
//! nivel, riesgo, descripción, «¿Qué hace esto?», valor actual resumido con
//! su recomendación, edición en el sitio («Editar» → campos → «Guardar»
//! abre [`DialogoGrupo`]; «Cancelar» lo deja como estaba) y los detalles
//! técnicos plegados.
//!
//! Un ajuste que no se pudo leer muestra el motivo y no se edita. Los
//! dispositivos, las IP fijas y los puertos abiertos tienen su componente;
//! la contraseña del router se cambia en «Configuración del router» y aquí
//! lleva un enlace; otro componente propio sería un hueco con su título y
//! descripción; una lista sin componente se ve como tabla.

use super::super::Foco;
use super::super::campo::{Campo, etiqueta, valor_resumido};
use super::super::dispositivos::Dispositivos;
use super::super::edicion::Edicion;
use super::super::piezas::{ayuda, cabecera, detalles_tecnicos, dom, enfocar, fila, motivos};
use super::super::puertos::Puertos;
use super::super::reservas::Reservas;
use crate::ajustes_router::{
    AjusteVista, EstadoLectura, GrupoVista, PeticionAjuste, preparar_ajuste,
};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::grupo::DialogoGrupo;
use crate::ui::i18n::traducir;
use crate::ui::{Estado, IconoEstado};
use leptos::{ev::SubmitEvent, html, prelude::*, task::spawn_local};
use leptos_router::components::A;
use std::collections::BTreeMap;

/// Tarjeta de un ajuste del tema. `solo_lectura`: el panel no escribe
/// (firmware sin confirmar). `recargar` vuelve a leer el tema tras aplicar.
#[component]
pub(crate) fn Tarjeta(
    a: AjusteVista,
    solo_lectura: bool,
    #[prop(into)] recargar: Callback<()>,
) -> impl IntoView {
    let id_dom = dom(&a);
    // Los nombres de los componentes son los del catálogo (`Componente::slug`).
    let cuerpo = if a.componente.as_deref() == Some(super::super::DISPOSITIVOS) {
        view! { <Dispositivos a solo_lectura recargar /> }.into_any()
    } else if a.componente.as_deref() == Some(super::super::RESERVAS) {
        view! { <Reservas a solo_lectura recargar /> }.into_any()
    } else if a.componente.as_deref() == Some(super::super::PUERTOS) {
        view! { <Puertos a solo_lectura recargar /> }.into_any()
    } else if a.componente.as_deref() == Some(super::super::CONTRASENA_ROUTER) {
        enlace_contrasena(&a).into_any()
    } else if a.componente.is_some() {
        hueco(&a).into_any()
    } else if a.lista {
        lista(&a).into_any()
    } else {
        view! { <TarjetaAjuste a solo_lectura recargar /> }.into_any()
    };
    view! {
        <article class="tarjeta ajuste" aria-labelledby=format!("{id_dom}-titulo")>
            {cuerpo}
        </article>
    }
}

/// La contraseña del router: se cambia en «Configuración del router»
/// (junto a la que usa el panel para entrar); aquí, el enlace.
fn enlace_contrasena(a: &AjusteVista) -> impl IntoView + use<> {
    let href = format!(
        "{}#{}",
        crate::ui::router::RUTA_CONEXION,
        crate::ui::router::ID_CAMBIAR_CONTRASENA
    );
    view! {
        {cabecera(a, false)}
        {ayuda(a)}
        <p class="ayuda">{t!("contrasena-router-enlace-texto")}</p>
        <p>
            <A href=href attr:class="b-sec">
                {t!("contrasena-router-enlace")}
            </A>
        </p>
    }
}

/// Ajuste con componente propio sin hacer: su título y su descripción.
fn hueco(a: &AjusteVista) -> impl IntoView + use<> {
    view! {
        {cabecera(a, false)}
        <p class="nota-hueco">{t!("ajustes-hueco")}</p>
    }
}

/// Lista sin componente propio: sus instancias en una tabla, sin editar.
fn lista(a: &AjusteVista) -> impl IntoView + use<> {
    let sin_leer = match &a.estado {
        EstadoLectura::SinLeer(m) => Some(m.clone()),
        EstadoLectura::Leido => None,
    };
    let cuerpo = if let Some(m) = sin_leer {
        view! { <p class="aviso estado-error" role="status"><IconoEstado estado=Estado::Error /><span>{m}</span></p> }
            .into_any()
    } else if a.instancias.is_empty() {
        view! { <p class="ayuda">{t!("ajustes-lista-vacia")}</p> }.into_any()
    } else {
        let cabeza = a
            .campos
            .iter()
            .map(|c| view! { <th scope="col">{etiqueta(c)}</th> })
            .collect_view();
        let filas = a
            .instancias
            .iter()
            .map(|i| {
                let celdas = i
                    .campos
                    .iter()
                    .map(|c| view! { <td>{valor_resumido(c)}</td> })
                    .collect_view();
                view! { <tr>{celdas}</tr> }
            })
            .collect_view();
        view! {
            <div class="tabla-scroll">
                <table class="tabla">
                    <thead><tr>{cabeza}</tr></thead>
                    <tbody>{filas}</tbody>
                </table>
            </div>
        }
        .into_any()
    };
    let todo_consulta = a.campos.iter().all(|c| c.solo_lectura.is_some());
    view! {
        {cabecera(a, false)}
        {ayuda(a)}
        {cuerpo}
        {if todo_consulta {
            motivos(&a.campos).into_any()
        } else {
            view! { <p class="nota-solo-lectura">{t!("ajustes-lista-sin-editar")}</p> }.into_any()
        }}
        {detalles_tecnicos(&a.clave, &a.tecnico)}
    }
}

/// Ajuste con la tarjeta genérica: vista resumida y edición en el sitio.
#[component]
fn TarjetaAjuste(a: AjusteVista, solo_lectura: bool, recargar: Callback<()>) -> impl IntoView {
    let id_dom = dom(&a);
    let leido = matches!(a.estado, EstadoLectura::Leido);
    let editable = leido && !solo_lectura && a.campos.iter().any(|c| c.solo_lectura.is_none());
    let edicion = Edicion::nueva(&a.campos);
    let editando = RwSignal::new(false);
    let preparando = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let grupo = RwSignal::new(None::<GrupoVista>);
    let boton_editar = NodeRef::<html::Button>::new();
    let foco = expect_context::<Foco>();
    let control = {
        let id_dom = id_dom.clone();
        move |i: usize| format!("{id_dom}-c{i}-control")
    };

    let abrir = {
        let control = control.clone();
        move |_| {
            editando.set(true);
            // El foco, al primer campo que se puede cambiar.
            let primero = edicion.primero();
            let control = control.clone();
            request_animation_frame(move || {
                if let Some(i) = primero {
                    enfocar(&control(i));
                }
            });
        }
    };
    let cancelar = move |_| {
        edicion.reiniciar();
        error.set(None);
        editando.set(false);
        request_animation_frame(move || {
            if let Some(b) = boton_editar.get_untracked() {
                let _ = b.focus();
            }
        });
    };

    let id_ajuste = a.id.clone();
    let guardar = {
        let control = control.clone();
        move |ev: SubmitEvent| {
            ev.prevent_default();
            if preparando.get_untracked() {
                return;
            }
            error.set(None);
            let pedir = match edicion.pedir() {
                Ok(p) => p,
                Err(i) => {
                    enfocar(&control(i));
                    return;
                }
            };
            if pedir.is_empty() {
                error.set(Some(t!("ajustes-error-sin-cambios")));
                return;
            }
            preparando.set(true);
            let p = PeticionAjuste {
                ajuste: id_ajuste.clone(),
                instancia: None,
                borrar: false,
                valores: pedir,
                esperados: BTreeMap::new(),
            };
            spawn_local(async move {
                match preparar_ajuste(p).await {
                    Ok(g) => grupo.set(Some(g)),
                    Err(e) => error.set(Some(texto_error(&e))),
                }
                preparando.set(false);
            });
        }
    };

    let dom_cerrar = id_dom.clone();
    let cerrar = Callback::new(move |aplicado: Option<i64>| {
        grupo.set(None);
        if aplicado.is_some() {
            editando.set(false);
            foco.0.set(Some(format!("{}-titulo", dom_cerrar)));
            recargar.run(());
        }
    });

    let sin_leer = match &a.estado {
        EstadoLectura::SinLeer(m) => Some(view! {
            <p class="aviso estado-error" role="status">
                <IconoEstado estado=Estado::Error />
                <span>{m.clone()}</span>
            </p>
        }),
        EstadoLectura::Leido => None,
    };
    let resumen = {
        let filas = a.campos.iter().map(fila).collect_view();
        let motivos = motivos(&a.campos);
        view! {
            <dl class="ajuste-valores">{filas}</dl>
            {motivos}
        }
    };
    let formulario = {
        let filas = a
            .campos
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if c.solo_lectura.is_some() {
                    view! {
                        <dl class="ajuste-valores">{fila(c)}</dl>
                    }
                    .into_any()
                } else {
                    let valor = edicion.valor(i);
                    let error = edicion.error(i);
                    view! {
                        <Campo
                            c=c.clone()
                            dom=format!("{id_dom}-c{i}")
                            valor
                            error
                            bloqueado=preparando
                        />
                    }
                    .into_any()
                }
            })
            .collect_view();
        let efecto = a.efecto.clone().map(|e| {
            view! {
                <p class="ajuste-efecto">
                    <strong>{t!("ajustes-efecto")}</strong>
                    ": "
                    {traducir(&e, None)}
                </p>
            }
        });
        let motivos = motivos(&a.campos);
        view! {
            <form class="ajuste-formulario" novalidate=true on:submit=guardar>
                <div class="ajuste-campos">{filas}</div>
                {motivos}
                {efecto}
                <p class="error-formulario" role="alert">{move || error.get()}</p>
                <div class="ajuste-acciones">
                    <button class="b-sec" type="button" disabled=move || preparando.get() on:click=cancelar>
                        {t!("ajustes-cancelar")}
                    </button>
                    // Sin `disabled`: el botón conserva el foco y el diálogo
                    // se lo devuelve al cerrarse (el envío ya se ignora
                    // mientras prepara).
                    <button
                        class="b-pri"
                        type="submit"
                        aria-disabled=move || preparando.get().then_some("true")
                    >
                        {move || if preparando.get() { t!("ajustes-grupo-preparando") } else { t!("ajustes-guardar") }}
                    </button>
                </div>
            </form>
        }
    };
    // La vista y el formulario se pintan una vez; solo se alterna cuál se ve.
    view! {
        {cabecera(&a, editable)}
        {ayuda(&a)}
        {sin_leer}
        {leido.then(|| view! {
            <div class="ajuste-resumen" class:oculto=move || editando.get()>
                {resumen}
                {editable.then(|| view! {
                    <div class="ajuste-acciones">
                        <button class="b-sec" type="button" node_ref=boton_editar on:click=abrir>
                            {t!("ajustes-editar")}
                        </button>
                    </div>
                })}
            </div>
        })}
        {editable.then(|| view! {
            <div class="ajuste-edicion" class:oculto=move || !editando.get()>{formulario}</div>
        })}
        {detalles_tecnicos(&a.clave, &a.tecnico)}
        {move || grupo.get().map(|g| view! { <DialogoGrupo grupo=g on_cerrar=cerrar /> })}
    }
}
