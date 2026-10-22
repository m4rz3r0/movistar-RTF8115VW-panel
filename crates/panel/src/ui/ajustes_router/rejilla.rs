//! La rejilla de un tema: resumen, barra, tarjetas y el detalle bajo su fila.
//!
//! Cada elemento abierto tiene su dirección (`/ajustes/<tema>/<elemento>`):
//! se puede compartir y «Atrás» cierra el detalle. El detalle se pinta a
//! todo el ancho justo debajo de la fila de su tarjeta (en el móvil, a
//! pantalla completa). Con el teclado, las flechas recorren las tarjetas,
//! espacio marca o desmarca, Intro abre y Esc cierra; al abrir, el foco va
//! al título del detalle y, al cerrar, vuelve a la tarjeta. Con cambios sin
//! guardar, cambiar de elemento, cerrar o «Atrás» pregunta antes.

use super::aviso::Seguimiento;
use super::detalle::{Detalle, ID_TITULO_DETALLE, sin_guardar};
use super::piezas::enfocar;
use super::resumen::ResumenTema;
use super::seleccion::{BarraSeleccion, Seleccion, nombre};
use super::tarjeta::{Tarjeta, id_tarjeta};
use crate::ajustes_router::{PaginaTema, preparar_accion};
use crate::t;
use crate::ui::explorador::texto_error;
use leptos::{ev, html, prelude::*, task::spawn_local};
use leptos_router::{NavigateOptions, hooks::use_navigate};

/// Lo que comparten las tarjetas y el detalle de la rejilla.
#[derive(Clone, Copy)]
pub(crate) struct Pagina {
    /// Identificador corto del tema (`wifi`).
    pub tema: StoredValue<String>,
    /// El panel está en solo lectura: nada se cambia.
    pub solo_lectura: bool,
    /// El elemento cuyo detalle se ve.
    pub abierto: RwSignal<Option<String>>,
    /// Lleva a la dirección de un elemento (abre su detalle).
    pub abrir: Callback<String>,
    /// Vuelve a la del tema (cierra el detalle).
    pub cerrar: Callback<()>,
    /// Vuelve a leer el tema.
    pub recargar: Callback<()>,
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
        " " => {
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
    recargar: Callback<()>,
    elemento: Memo<Option<String>>,
) -> impl IntoView {
    let sel = expect_context::<Seleccion>();
    let s = expect_context::<Seguimiento>();
    let navegar = use_navigate();
    let tema = StoredValue::new(p.tema.clone());
    let existe = {
        let ids: Vec<String> = p.elementos.iter().map(|e| e.id.clone()).collect();
        move |id: &Option<String>| id.as_ref().filter(|i| ids.contains(i)).cloned()
    };
    let abierto = RwSignal::new(existe(&elemento.get_untracked()));
    let registro = RwSignal::new(None);
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
        request_animation_frame(move || match (destino, antes) {
            (Some(_), _) => enfocar(ID_TITULO_DETALLE),
            (None, Some(a)) => enfocar(&id_tarjeta(&a)),
            (None, None) => {}
        });
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
        abierto,
        abrir: Callback::new({
            let ir_a = ir_a.clone();
            move |id: String| ir_a(Some(id))
        }),
        cerrar: Callback::new({
            let ir_a = ir_a.clone();
            move |()| ir_a(None)
        }),
        recargar,
    };
    provide_context(pagina);

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
    let nombre_abierto = {
        let elementos = p.elementos.clone();
        move || {
            abierto
                .get()
                .and_then(|id| elementos.iter().find(|e| e.id == id).map(nombre))
                .unwrap_or_default()
        }
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
        if navegado {
            cambiar_a(destino);
        } else {
            ir(destino, false);
        }
    };
    view! {
        <ResumenTema datos=p.resumen.clone() />
        <BarraSeleccion
            acciones=p.acciones.clone()
            elementos=p.elementos.clone()
            solo_lectura=p.solo_lectura
            al_pulsar=accion
        />
        <div class="rejilla-elementos" class:hay-marcadas=move || { sel.cuantas() > 0 }>{tarjetas}</div>
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

/// La pregunta de los cambios sin guardar: seguir editando o descartarlos.
/// Esc sigue editando.
#[component]
fn SinGuardar(nombre: String, seguir: Callback<()>, descartar: Callback<()>) -> impl IntoView {
    let dlg = NodeRef::<html::Dialog>::new();
    Effect::new(move |_| {
        if let Some(d) = dlg.get()
            && !d.open()
        {
            let _ = d.show_modal();
        }
    });
    view! {
        <dialog
            class="dialogo dialogo-sin-guardar"
            node_ref=dlg
            aria-labelledby="sin-guardar-titulo"
            aria-modal="true"
            on:cancel=move |ev: leptos::ev::Event| {
                ev.prevent_default();
                seguir.run(());
            }
        >
            <div class="dialogo-cuerpo">
                <h2 id="sin-guardar-titulo">{t!("ajustes-sin-guardar-titulo")}</h2>
                <p>{t!("ajustes-sin-guardar-texto", nombre = nombre)}</p>
                <div class="dialogo-pie">
                    <button type="button" class="b-pel" on:click=move |_| descartar.run(())>
                        {t!("ajustes-sin-guardar-descartar")}
                    </button>
                    <button type="button" class="b-pri" autofocus on:click=move |_| seguir.run(())>
                        {t!("ajustes-sin-guardar-seguir")}
                    </button>
                </div>
            </div>
        </dialog>
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
