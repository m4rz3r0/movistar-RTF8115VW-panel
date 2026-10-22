//! Lo que solo se consulta: una lista sin componente propio y el enlace a la contraseña del router.
//!
//! Lo usan el detalle de un elemento ([`super::detalle`]) y las tarjetas de
//! los temas que aún no van en rejilla ([`super::heredada`]).

use super::campo::{etiqueta, valor_resumido};
use super::piezas::motivos;
use crate::ajustes_router::AjusteVista;
use crate::t;
use leptos::prelude::*;
use leptos_router::components::A;

/// Las instancias de una lista en una tabla, sin editar (o que no tiene
/// ninguna), y por qué no se editan.
pub(crate) fn lista(a: &AjusteVista) -> impl IntoView + use<> {
    let cuerpo = if a.instancias.is_empty() {
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
        {cuerpo}
        {if todo_consulta {
            motivos(&a.campos).into_any()
        } else {
            view! { <p class="nota-solo-lectura">{t!("ajustes-lista-sin-editar")}</p> }.into_any()
        }}
    }
}

/// La contraseña del router se cambia en «Configuración del router» (junto
/// a la que usa el panel para entrar): el texto que lo dice y el enlace.
pub(crate) fn enlace_contrasena_router() -> impl IntoView + use<> {
    let href = format!(
        "{}#{}",
        crate::ui::router::RUTA_CONEXION,
        crate::ui::router::ID_CAMBIAR_CONTRASENA
    );
    view! {
        <p class="ayuda">{t!("contrasena-router-enlace-texto")}</p>
        <p>
            <A href=href attr:class="b-sec">
                {t!("contrasena-router-enlace")}
            </A>
        </p>
    }
}
