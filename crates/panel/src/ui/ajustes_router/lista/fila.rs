//! Una fila de la lista: marca, nombre, columnas, estado, interruptor y «Abrir».
//!
//! Toda la fila abre su detalle (con el ratón o con Intro), salvo sus
//! controles. Con el teclado, las flechas arriba y abajo recorren las filas,
//! espacio marca o desmarca y Esc cierra el detalle abierto. El interruptor
//! se aplica sin preguntar (activar o desactivar no corta nada que haya que
//! avisar) y la insignia dice «Aplicando…» y «Comprobando…» mientras tanto.

use super::super::ID_SOLO_LECTURA;
use super::super::aviso::Seguimiento;
use super::super::rejilla::Pagina;
use super::super::seleccion::{Marca, Seleccion};
use super::DatosLista;
use super::columnas::{Fila, Tipo};
use crate::ajustes_router::{PeticionAjuste, identidad_de, preparar_ajuste};
use crate::t;
use crate::ui::comun::{Insignia, Interruptor};
use crate::ui::explorador::texto_error;
use leptos::{ev, prelude::*, task::spawn_local};
use std::collections::BTreeMap;

/// Id en la página de la fila `id`.
pub(crate) fn id_fila(id: &str) -> String {
    format!("fila-{id}")
}

/// ¿El clic fue en un control de la fila (que hace lo suyo, no abrir)?
fn en_un_control(ev: &ev::MouseEvent) -> bool {
    use leptos::wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<leptos::web_sys::Element>().ok())
        .and_then(|e| e.closest("button, a, input, select, label").ok().flatten())
        .is_some()
}

/// Lleva el foco a la fila de al lado de `id` (`abajo` o arriba), si la hay.
fn mover(id: &str, abajo: bool) {
    use leptos::wasm_bindgen::JsCast;
    let Some(fila) = document().get_element_by_id(&id_fila(id)) else {
        return;
    };
    let otra = if abajo {
        fila.next_element_sibling()
    } else {
        fila.previous_element_sibling()
    };
    if let Some(o) = otra.and_then(|o| o.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
        let _ = o.focus();
    }
}

/// Nombre accesible del interruptor de la fila `nombre`.
fn etiqueta_interruptor(tipo: Tipo, nombre: String) -> String {
    match tipo {
        Tipo::Reservas => t!("ajuste-red_local-reservas-activa-de", equipo = nombre),
        _ => t!("ajuste-puertos-reglas-activa-de", nombre = nombre),
    }
}

/// La fila `f` de la lista `d`.
#[component]
pub(crate) fn FilaLista(f: Fila, d: DatosLista) -> impl IntoView {
    let pagina = expect_context::<Pagina>();
    let sel = expect_context::<Seleccion>();
    let s = expect_context::<Seguimiento>();
    let id = StoredValue::new(f.id.clone());
    let abrir = move || pagina.abrir.run(id.get_value());
    let marcada = move || id.with_value(|id| sel.marcada(id));
    let abierta = move || {
        pagina
            .abierto
            .with(|a| id.with_value(|id| a.as_ref() == Some(id)))
    };
    let teclas = move |ev: ev::KeyboardEvent| {
        if ev.target() != ev.current_target() {
            return;
        }
        match ev.key().as_str() {
            "Enter" => abrir(),
            " " if d.marcar => {
                ev.prevent_default();
                id.with_value(|id| sel.alternar(id));
            }
            "Escape" if pagina.abierto.get_untracked().is_some() => pagina.cerrar.run(()),
            "ArrowDown" | "ArrowUp" => {
                ev.prevent_default();
                id.with_value(|id| mover(id, ev.key() == "ArrowDown"));
            }
            _ => {}
        }
    };
    let (estado, texto_estado) = (f.estado, f.texto_estado.clone());
    let insignia = move || {
        let (estado, texto) = id
            .with_value(|id| s.insignia(id))
            .unwrap_or((estado, texto_estado.clone()));
        view! { <Insignia estado texto /> }
    };
    let cabeceras = d.cabeceras.get_value();
    let celdas = f
        .celdas
        .iter()
        .zip(&cabeceras)
        .filter(|(c, _)| c.interruptor.is_none())
        .map(|(c, cabecera)| {
            let nota = c
                .nota
                .clone()
                .map(|n| view! { <span class="lista-nota mono">{n}</span> });
            view! {
                <td class="lista-dato" data-etiqueta=cabecera.clone()>
                    <span class="lista-valor" class:mono=c.mono class:chip=c.chip>
                        {c.texto.clone()}
                    </span>
                    {nota}
                </td>
            }
        })
        .collect_view();
    let interruptor = d.con_interruptor.then(|| {
        let leido = f.activa.unwrap_or(false);
        let encendido =
            Signal::derive(move || id.with_value(|id| s.interruptor(id)).unwrap_or(leido));
        let desactivado =
            Signal::derive(move || !d.editable || id.with_value(|id| s.insignia(id).is_some()));
        let etiqueta = etiqueta_interruptor(d.tipo, f.nombre.clone());
        let al_cambiar = Callback::new(move |nuevo: bool| {
            let Some(i) = id.with_value(|id| {
                d.instancias
                    .with_value(|is| is.iter().find(|i| i.id == *id).cloned())
            }) else {
                return;
            };
            if !s.ocupado() {
                s.empieza(&i.id, Some(nuevo), String::new());
            }
            let p = PeticionAjuste {
                ajuste: d.ajuste.get_value(),
                instancia: Some(i.id.clone()),
                borrar: false,
                valores: BTreeMap::from([(d.campo_interruptor.get_value(), nuevo.to_string())]),
                esperados: identidad_de(d.tipo.componente(), &i.campos),
            };
            spawn_local(async move {
                match preparar_ajuste(p).await {
                    Ok(g) => s.aplicar(&g, Vec::new(), vec![i.id], false),
                    Err(e) => s.falla(texto_error(&e)),
                }
            });
        });
        view! {
            <td class="lista-int">
                <Interruptor
                    encendido
                    etiqueta
                    desactivado
                    descrito=pagina.solo_lectura.then(|| ID_SOLO_LECTURA.to_owned())
                    al_cambiar
                />
            </td>
        }
    });
    let sub = f.sub.clone().map(|s| {
        view! { <span class="lista-sub" class:mono=f.sub_mono>{s}</span> }
    });
    let nombre = f.nombre.clone();
    view! {
        <tr
            id=id_fila(&f.id)
            class="lista-fila"
            class:marcada=marcada
            class:abierta=abierta
            tabindex="0"
            on:click=move |ev| {
                if !en_un_control(&ev) {
                    abrir();
                }
            }
            on:keydown=teclas
        >
            {d.marcar.then(|| {
                view! {
                    <td class="lista-marca">
                        <Marca id=f.id.clone() nombre=nombre.clone() />
                    </td>
                }
            })}
            <th scope="row" class="lista-nombre">
                <span class="lista-nombre-texto">{f.nombre.clone()}</span>
                {sub}
            </th>
            {celdas}
            <td class="lista-estado">{insignia}</td>
            {interruptor}
            <td class="lista-abrir">
                <button
                    type="button"
                    class="b-txt"
                    aria-label=t!("ajustes-abrir-nombre", nombre = nombre.clone())
                    on:click=move |_| abrir()
                >
                    {t!("ajustes-abrir")}
                </button>
            </td>
        </tr>
    }
}
