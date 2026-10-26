//! La rejilla de un tema: resumen, barra, tarjetas y el detalle bajo su fila.
//!
//! Cada elemento abierto tiene su dirección (`/ajustes/<tema>/<elemento>`):
//! se puede compartir y «Atrás» cierra el detalle. El detalle se pinta a
//! todo el ancho justo debajo de la fila de su tarjeta (en el móvil, a
//! pantalla completa). Con el teclado, las flechas recorren las tarjetas,
//! espacio marca o desmarca, Intro abre y Esc cierra; al abrir, el foco va
//! al título del detalle y, al cerrar, vuelve a la tarjeta. Con cambios sin
//! guardar, cambiar de elemento, cerrar o «Atrás» pregunta antes.
//!
//! En Puertos y Red local, lo que no es una lista va en tarjetas y, debajo,
//! cada lista con sus columnas ([`super::lista`]): sus filas se abren,
//! marcan y recorren igual, con su propia dirección.

use super::aviso::Seguimiento;
use super::detalle::{Borradores, Detalle, sin_guardar};
use super::lista::{Lista, es_lista, filas_de_la_pagina};
use super::pestanas::Lectura;
use super::piezas::enfocar;
use super::resumen::ResumenTema;
use super::seleccion::{BarraSeleccion, Seleccion, marcables, nombre};
use super::sin_guardar::SinGuardar;
use super::tarjeta::{Tarjeta, id_tarjeta};
use crate::ajustes_router::{ElementoVista, PaginaTema, preparar_accion};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::{Estado, IconoEstado};
use leptos::{ev, prelude::*, task::spawn_local};
use leptos_router::{NavigateOptions, hooks::use_navigate};
use std::collections::BTreeSet;

/// Lo que comparten las tarjetas y el detalle de la rejilla.
#[derive(Clone, Copy)]
pub(crate) struct Pagina {
    /// Identificador corto del tema (`wifi`).
    pub tema: StoredValue<String>,
    /// El panel está en solo lectura: nada se cambia.
    pub solo_lectura: bool,
    /// Los elementos (tarjetas y filas) que se pueden marcar: los que
    /// comparten una acción con otro ([`marcables`]). Vacío: la pantalla
    /// no ofrece selección (ni marca, ni tecla, ni barra).
    pub marcables: StoredValue<BTreeSet<String>>,
    /// El elemento cuyo detalle se ve.
    pub abierto: RwSignal<Option<String>>,
    /// Lleva a la dirección de un elemento (abre su detalle).
    pub abrir: Callback<String>,
    /// Vuelve a la del tema (cierra el detalle).
    pub cerrar: Callback<()>,
    /// Vuelve a leer el tema del router (sin la caché).
    pub volver_a_leer: Callback<()>,
    /// El detalle que se pinte lleva el foco a su título (se acaba de abrir).
    pub enfocar_detalle: RwSignal<bool>,
}

impl Pagina {
    /// ¿Se puede marcar el elemento (tarjeta o fila) `id`?
    pub(crate) fn marcable(self, id: &str) -> bool {
        self.marcables.with_value(|m| m.contains(id))
    }

    /// ¿Ofrece la pantalla selección?
    pub(crate) fn con_seleccion(self) -> bool {
        self.marcables.with_value(|m| !m.is_empty())
    }
}

/// Las tarjetas de la rejilla, en orden.
fn tarjetas() -> Vec<leptos::web_sys::HtmlElement> {
    use leptos::wasm_bindgen::JsCast;
    let Ok(lista) = document().query_selector_all(".rejilla-elementos > .tarjeta-el") else {
        return Vec::new();
    };
    (0..lista.length())
        .filter_map(|i| lista.item(i))
        .filter_map(|n| n.dyn_into::<leptos::web_sys::HtmlElement>().ok())
        .collect()
}

/// Posición a la que llevan las flechas desde `i`, con `columnas` por fila
/// y `n` tarjetas; `None` si no se mueve.
fn destino(tecla: &str, i: usize, columnas: usize, n: usize) -> Option<usize> {
    let columnas = columnas.max(1);
    let j = match tecla {
        "ArrowRight" => i + 1,
        "ArrowLeft" => i.checked_sub(1)?,
        "ArrowDown" => i + columnas,
        "ArrowUp" => i.checked_sub(columnas)?,
        _ => return None,
    };
    (j < n).then_some(j)
}

/// Las teclas sobre la tarjeta del elemento `id` (no sobre sus controles).
pub(crate) fn teclas_de_tarjeta(ev: &ev::KeyboardEvent, id: &str, p: Pagina, sel: Seleccion) {
    if ev.target() != ev.current_target() {
        return;
    }
    match ev.key().as_str() {
        "Enter" => p.abrir.run(id.to_owned()),
        " " if p.marcable(id) => {
            ev.prevent_default();
            sel.alternar(id);
        }
        "Escape" if p.abierto.get_untracked().is_some() => p.cerrar.run(()),
        tecla @ ("ArrowRight" | "ArrowLeft" | "ArrowDown" | "ArrowUp") => {
            ev.prevent_default();
            let todas = tarjetas();
            let Some(i) = todas.iter().position(|t| t.id() == id_tarjeta(id)) else {
                return;
            };
            let arriba = todas.first().map(|t| t.offset_top());
            let columnas = todas
                .iter()
                .filter(|t| Some(t.offset_top()) == arriba)
                .count();
            if let Some(j) = destino(tecla, i, columnas, todas.len()) {
                let _ = todas[j].focus();
            }
        }
        _ => {}
    }
}

/// Al cerrar el detalle de `id`, el foco vuelve a lo que lo abrió: su fila,
/// su tarjeta o, en el de una instancia nueva (la dirección de su lista), el
/// botón que la crea.
fn enfocar_al_cerrar(id: &str) {
    let candidatos = [
        super::lista::id_fila(id),
        id_tarjeta(id),
        super::lista::id_nueva(id),
    ];
    if let Some(c) = candidatos
        .iter()
        .find(|c| document().get_element_by_id(c).is_some())
    {
        enfocar(c);
    }
}

/// Esqueletos con la forma de las tarjetas, mientras se lee el tema.
pub(crate) fn esqueleto() -> impl IntoView {
    let tarjeta = || {
        view! {
            <div class="tarjeta-el tarjeta-esqueleto">
                <span class="esqueleto-linea esqueleto-titulo"></span>
                <span class="esqueleto-linea"></span>
                <span class="esqueleto-linea esqueleto-corta"></span>
            </div>
        }
    };
    view! {
        <p class="solo-lector" role="status">{t!("comun-cargando")}</p>
        <div class="rejilla-elementos" aria-hidden="true">
            {tarjeta()} {tarjeta()} {tarjeta()} {tarjeta()}
        </div>
    }
}

/// El contenido de un tema en rejilla. `elemento` es el de la dirección.
#[component]
pub(crate) fn ContenidoRejilla(
    p: PaginaTema,
    volver_a_leer: Callback<()>,
    elemento: Memo<Option<String>>,
) -> impl IntoView {
    let sel = expect_context::<Seleccion>();
    let s = expect_context::<Seguimiento>();
    let navegar = use_navigate();
    let tema = StoredValue::new(p.tema.clone());
    // Las filas de las listas son elementos como las tarjetas: se abren,
    // se marcan y entran en la barra.
    let filas = filas_de_la_pagina(&p);
    let tarjetas_y_filas: Vec<ElementoVista> = p
        .elementos
        .iter()
        .filter(|e| !es_lista(e))
        .cloned()
        .chain(filas.iter().map(|(e, _)| e.clone()))
        .collect();
    Lectura::llega(p.leido_hace_s);
    let marcables = marcables(&p.acciones, &tarjetas_y_filas);
    sel.conservar(&marcables);
    let nombres: Vec<(String, String)> = p
        .elementos
        .iter()
        .map(|e| (e.id.clone(), nombre(e)))
        .chain(filas.iter().map(|(e, n)| (e.id.clone(), n.clone())))
        .collect();
    // Sin señales: el efecto de la dirección puede correr justo cuando la
    // página se está desmontando (al ir a otro tema).
    let ids: std::sync::Arc<[String]> = nombres.iter().map(|(id, _)| id.clone()).collect();
    // Tras un borrado, ya pintada la página sin lo borrado, el foco a la
    // fila vecina (o a «Abrir…/Añadir»).
    if let Some(f) = s.tomar_foco(&ids) {
        request_animation_frame(move || super::tras_borrar::enfocar_tras_borrar(&f));
    }
    let existe = move |id: &Option<String>| id.as_ref().filter(|i| ids.contains(i)).cloned();
    // Un elemento en la dirección que el tema no tiene: se dice.
    let desconocido = {
        let existe = existe.clone();
        move || {
            elemento
                .get()
                // Lo recién borrado (o creado desde su detalle) deja su
                // dirección: no es un error.
                .filter(|e| existe(&Some(e.clone())).is_none() && !s.cerrado(e))
                .map(|id| {
                    view! {
                        <p class="aviso estado-error" role="alert">
                            <IconoEstado estado=Estado::Error />
                            <span>{t!("ajustes-elemento-no-existe", id = id)}</span>
                        </p>
                    }
                })
        }
    };
    let borradores = expect_context::<Borradores>();
    let abierto = RwSignal::new(existe(&elemento.get_untracked()));
    let registro = RwSignal::new(None);
    let enfocar_detalle = RwSignal::new(false);
    let sucio = move || sin_guardar(registro, abierto.get_untracked().as_deref());
    // A dónde se iba con cambios sin guardar (la pregunta está abierta) y
    // si la dirección ya ha cambiado («Atrás»).
    let salida = RwSignal::new(None::<(Option<String>, bool)>);
    let ir = {
        let navegar = navegar.clone();
        move |id: Option<String>, sustituir: bool| {
            let ruta = tema.with_value(|t| match &id {
                Some(id) => format!("/ajustes/{t}/{id}"),
                None => format!("/ajustes/{t}"),
            });
            let options = NavigateOptions {
                replace: sustituir,
                ..NavigateOptions::default()
            };
            navegar(&ruta, options);
        }
    };
    // Lo que se ve pasa a ser `destino`; el foco, al título del detalle o
    // de vuelta a la tarjeta.
    let cambiar_a = move |destino: Option<String>| {
        let antes = abierto.get_untracked();
        abierto.set(destino.clone());
        // Al abrir, el detalle lleva el foco a su título en cuanto se pinta
        // (`enfocar_detalle`); al cerrar, vuelve a la tarjeta.
        match (destino, antes) {
            (Some(_), _) => enfocar_detalle.set(true),
            (None, Some(a)) => request_animation_frame(move || enfocar_al_cerrar(&a)),
            (None, None) => {}
        }
    };
    Effect::new(move |_| {
        let destino = existe(&elemento.get());
        if destino == abierto.get_untracked() {
            return;
        }
        if sucio() {
            salida.set(Some((destino, true)));
        } else {
            cambiar_a(destino);
        }
    });
    // Desde la página (otra tarjeta, ✕, Esc): con cambios sin guardar,
    // primero la pregunta; la dirección cambia solo si se descartan.
    let ir_a = {
        let ir = ir.clone();
        move |destino: Option<String>| {
            if destino == abierto.get_untracked() {
                return;
            }
            if sucio() {
                salida.set(Some((destino, false)));
            } else {
                ir(destino, false);
            }
        }
    };
    let pagina = Pagina {
        tema,
        solo_lectura: p.solo_lectura,
        marcables: StoredValue::new(marcables.clone()),
        abierto,
        abrir: Callback::new({
            let ir_a = ir_a.clone();
            move |id: String| ir_a(Some(id))
        }),
        cerrar: Callback::new({
            let ir_a = ir_a.clone();
            move |()| ir_a(None)
        }),
        volver_a_leer,
        enfocar_detalle,
    };
    provide_context(pagina);

    // Lo borrado (o creado desde su detalle) se cierra al aplicarse: la
    // dirección deja de nombrarlo, sin preguntar por lo editado en él.
    {
        let ir = ir.clone();
        Effect::new(move |_| {
            let en_la_direccion = elemento.get_untracked();
            if en_la_direccion.as_deref().is_some_and(|e| s.cerrado(e)) {
                registro.set(None);
                s.olvidar_cerrados();
                ir(None, true);
            }
        });
    }
    let accion = Callback::new(move |(accion, ids): (String, Vec<String>)| {
        let tema = tema.get_value();
        spawn_local(async move {
            match preparar_accion(tema, accion, ids.clone()).await {
                Ok(g) => s.revisar(g, ids),
                Err(e) => s.falla(texto_error(&e)),
            }
        });
    });
    let tarjetas = p
        .elementos
        .iter()
        .filter(|e| !es_lista(e))
        .map(|e| {
            let id = e.id.clone();
            let del_detalle = StoredValue::new(e.clone());
            let detalle = move || {
                (abierto.get().as_deref() == Some(id.as_str()))
                    .then(|| view! { <Detalle e=del_detalle.get_value() registro /> })
            };
            view! {
                <Tarjeta e=e.clone() />
                {detalle}
            }
        })
        .collect_view();
    let red = p.red.clone().unwrap_or_default();
    let existentes = p
        .ajustes()
        .find(|a| a.componente.as_deref() == Some(super::RESERVAS))
        // Sin escribir las IP fijas, la IP de un dispositivo no se fija.
        .filter(|a| crate::validacion_real::escribible(a))
        .map(|a| super::reservas::existentes(&a.instancias));
    let listas = p
        .elementos
        .iter()
        .filter(|e| es_lista(e))
        .map(|e| {
            view! {
                <Lista
                    e=e.clone()
                    red=red.clone()
                    acciones=p.acciones.clone()
                    existentes=existentes.clone()
                    registro
                />
            }
        })
        .collect_view();
    let nombre_abierto = move || {
        abierto
            .get()
            .and_then(|id| {
                nombres
                    .iter()
                    .find(|(i, _)| *i == id)
                    .map(|(_, n)| n.clone())
            })
            .unwrap_or_default()
    };
    let seguir = {
        let ir = ir.clone();
        move || {
            // Tras «Atrás», vuelve a la dirección de lo que se edita.
            if let Some((_, true)) = salida.get_untracked() {
                ir(abierto.get_untracked(), true);
            }
            salida.set(None);
        }
    };
    let descartar = move || {
        let Some((destino, navegado)) = salida.get_untracked() else {
            return;
        };
        salida.set(None);
        // Sin cambios que perder: ya no se pregunta.
        registro.set(None);
        if let Some(id) = abierto.get_untracked() {
            borradores.olvidar(&id);
        }
        if navegado {
            cambiar_a(destino);
        } else {
            ir(destino, false);
        }
    };
    view! {
        {desconocido}
        <ResumenTema datos=p.resumen.clone() />
        {pagina.con_seleccion().then(|| view! {
            <BarraSeleccion
                acciones=p.acciones.clone()
                elementos=tarjetas_y_filas
                solo_lectura=p.solo_lectura
                al_pulsar=accion
            />
        })}
        <div class="rejilla-elementos" class:hay-marcadas=move || { sel.cuantas() > 0 }>{tarjetas}</div>
        {listas}
        {move || salida.with(Option::is_some).then(|| {
            let seguir = seguir.clone();
            let descartar = descartar.clone();
            view! {
                <SinGuardar
                    nombre=nombre_abierto()
                    seguir=Callback::new(move |()| seguir())
                    descartar=Callback::new(move |()| descartar())
                />
            }
        })}
    }
}

#[cfg(test)]
mod tests {
    use super::destino;

    #[test]
    fn las_flechas_recorren_la_rejilla() {
        // Dos columnas, cinco tarjetas.
        assert_eq!(destino("ArrowRight", 0, 2, 5), Some(1));
        assert_eq!(destino("ArrowLeft", 0, 2, 5), None);
        assert_eq!(destino("ArrowDown", 1, 2, 5), Some(3));
        assert_eq!(destino("ArrowDown", 3, 2, 5), None);
        assert_eq!(destino("ArrowUp", 4, 2, 5), Some(2));
        assert_eq!(destino("ArrowUp", 1, 2, 5), None);
        assert_eq!(destino("ArrowRight", 4, 2, 5), None);
        // Una columna (el móvil).
        assert_eq!(destino("ArrowDown", 0, 1, 5), Some(1));
        assert_eq!(destino("Tab", 0, 1, 5), None);
    }
}
