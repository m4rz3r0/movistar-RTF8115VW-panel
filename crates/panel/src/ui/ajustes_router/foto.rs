//! «Cómo estaba al instalar» (`/ajustes/foto-de-partida`): lo que ha
//! cambiado en el router desde la foto de partida del asistente, por temas,
//! con «Volver a este valor» en lo que se puede restaurar (abre el diálogo
//! de grupo, como guardar un ajuste).
//!
//! Solo se pinta lo que tiene contenido: sin foto, un texto que lo explica;
//! sin diferencias, una línea que lo dice (sin tabla vacía); lo que la foto
//! no tiene, en una lista plegada. Las altas y bajas de las listas son
//! informativas, sin botón. Un secreto que ha cambiado se ve en sus dos
//! campos censurados con el ojo (de solo lectura).

use super::campo::{etiqueta, valor_resumido};
use super::tarjeta::enfocar;
use super::{RUTA_AJUSTES, nombre, ruta};
use crate::ajustes_router::{
    CampoVista, ComparacionVista, Diferencia, DiferenciaVista, GrupoVista, comparar_con_partida,
    preparar_volver_a_partida,
};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::grupo::DialogoGrupo;
use crate::ui::historial::fecha_hora;
use crate::ui::i18n::traducir;
use crate::ui::{Estado, IconoEstado};
use ajustes::Tema;
use leptos::{prelude::*, task::spawn_local};

/// Ruta de la página.
pub const RUTA_FOTO: &str = "/ajustes/foto-de-partida";

/// Id del título de la página: recibe el foco tras volver a un valor (la
/// fila que tenía el foco desaparece al releer).
const TITULO: &str = "foto-titulo";

/// Lo que comparten las filas de la página: el grupo del diálogo abierto y
/// cómo volver a leer la comparación.
#[derive(Clone, Copy)]
struct Pagina {
    grupo: RwSignal<Option<GrupoVista>>,
    solo_lectura: bool,
}

/// `/ajustes/foto-de-partida`.
#[component]
pub fn PaginaFoto() -> impl IntoView {
    let datos = Resource::new(|| (), |()| comparar_con_partida());
    let grupo = RwSignal::new(None::<GrupoVista>);
    let cerrar = Callback::new(move |aplicado: Option<i64>| {
        grupo.set(None);
        if aplicado.is_some() {
            datos.refetch();
            enfocar(TITULO);
        }
    });
    view! {
        <header class="pagina-cabecera">
            <div>
                <h1 id=TITULO tabindex="-1">{t!("ajustes-foto-titulo")}</h1>
                <p class="pagina-sub">{t!("ajustes-foto-desc")}</p>
            </div>
        </header>
        <p class="foto-volver">
            <a href=RUTA_AJUSTES>{t!("ajustes-foto-ir-ajustes")}</a>
        </p>
        <Transition fallback=|| view! { <p class="estado-espera">{t!("comun-cargando")}</p> }>
            {move || Suspend::new(async move {
                match datos.await {
                    Ok(c) => {
                        let pagina = Pagina { grupo, solo_lectura: c.solo_lectura };
                        view! { <Comparacion c pagina /> }.into_any()
                    }
                    Err(e) => view! {
                        <p class="aviso estado-error" role="alert">
                            <IconoEstado estado=Estado::Error />
                            <span>{texto_error(&e)}</span>
                        </p>
                    }
                    .into_any(),
                }
            })}
        </Transition>
        {move || grupo.get().map(|g| view! { <DialogoGrupo grupo=g on_cerrar=cerrar /> })}
    }
}

/// La comparación: la fecha de la foto, las diferencias por temas y lo que
/// no tiene dato.
#[component]
fn Comparacion(c: ComparacionVista, pagina: Pagina) -> impl IntoView {
    let Some(momento) = c.momento else {
        return view! {
            <p class="aviso estado-neutro foto-aviso" role="status">
                <IconoEstado estado=Estado::Neutro />
                <span>
                    {t!("ajustes-foto-sin-foto")} " "
                    <a href="/historial">{t!("ajustes-foto-ir-historial")}</a>
                </span>
            </p>
        }
        .into_any();
    };
    let tomada = t!("ajustes-foto-tomada", fecha = fecha_hora(momento, false));
    let cuerpo = if c.diferencias.is_empty() {
        view! {
            <p class="aviso estado-ok foto-aviso" role="status">
                <IconoEstado estado=Estado::Ok />
                <span>{t!("ajustes-foto-sin-diferencias")}</span>
            </p>
        }
        .into_any()
    } else {
        por_temas(c.diferencias, pagina).into_any()
    };
    let sin_leer = (!c.sin_leer.is_empty()).then(|| {
        view! {
            <div class="aviso estado-aviso foto-aviso" role="status">
                <IconoEstado estado=Estado::Aviso />
                <div>
                    <p>{t!("ajustes-foto-sin-leer")}</p>
                    <ul class="foto-nombres">{titulos(&c.sin_leer)}</ul>
                </div>
            </div>
        }
    });
    let sin_dato = (!c.sin_dato.is_empty()).then(|| {
        view! {
            <details class="foto-sin-dato">
                <summary>{t!("ajustes-foto-sin-dato")} " (" {c.sin_dato.len()} ")"</summary>
                <p>{t!("ajustes-foto-sin-dato-desc")}</p>
                <ul class="foto-nombres">{titulos(&c.sin_dato)}</ul>
            </details>
        }
    });
    view! {
        <p class="foto-fecha">{tomada}</p>
        {sin_leer}
        {cuerpo}
        {sin_dato}
    }
    .into_any()
}

/// Títulos de los ajustes de los prefijos `claves`, como lista.
fn titulos(claves: &[String]) -> impl IntoView + use<> {
    claves
        .iter()
        .map(|k| view! { <li>{traducir(&format!("{k}-titulo"), None)}</li> })
        .collect_view()
}

/// Las diferencias agrupadas por tema (en el orden de las pestañas) y, en
/// cada tema, por ajuste.
fn por_temas(diferencias: Vec<DiferenciaVista>, pagina: Pagina) -> impl IntoView {
    Tema::TODOS
        .into_iter()
        .filter_map(|t| {
            let del_tema: Vec<DiferenciaVista> = diferencias
                .iter()
                .filter(|d| d.tema == t.slug())
                .cloned()
                .collect();
            (!del_tema.is_empty()).then(|| {
                let id = format!("foto-tema-{}", t.slug());
                let titulo = id.clone();
                view! {
                    <section class="foto-tema" aria-labelledby=titulo>
                        <header class="foto-tema-cabecera">
                            <h2 id=id>{nombre(t)}</h2>
                            <a href=ruta(t)>{t!("ajustes-foto-ver-tema", tema = nombre(t))}</a>
                        </header>
                        {por_ajustes(del_tema, pagina)}
                    </section>
                }
            })
        })
        .collect_view()
}

/// Una tarjeta por ajuste con sus diferencias, en el orden en que llegan.
fn por_ajustes(diferencias: Vec<DiferenciaVista>, pagina: Pagina) -> impl IntoView {
    let mut grupos: Vec<(String, Vec<DiferenciaVista>)> = Vec::new();
    for d in diferencias {
        match grupos.iter_mut().find(|(id, _)| *id == d.ajuste) {
            Some((_, v)) => v.push(d),
            None => grupos.push((d.ajuste.clone(), vec![d])),
        }
    }
    grupos
        .into_iter()
        .map(|(_, ds)| {
            let clave = ds[0].clave.clone();
            let lista = ds.iter().any(|d| d.instancia.is_some());
            let filas = ds
                .into_iter()
                .map(|d| fila(d, pagina))
                .collect_view();
            view! {
                <article class="tarjeta foto-ajuste">
                    <h3>{traducir(&format!("{clave}-titulo"), None)}</h3>
                    <div class="foto-filas">{filas}</div>
                    {lista.then(|| view! { <p class="foto-nota">{t!("ajustes-foto-lista-nota")}</p> })}
                </article>
            }
        })
        .collect_view()
}

/// Una diferencia: un valor (con su botón si se puede volver a él), o una
/// instancia de una lista que se añadió o se quitó.
fn fila(d: DiferenciaVista, pagina: Pagina) -> AnyView {
    match d.cambio {
        Diferencia::Valor {
            partida,
            actual,
            restaurable,
        } => {
            let id = format!(
                "foto-{}-{}{}",
                d.ajuste.replace('.', "-"),
                partida.id,
                d.instancia
                    .as_deref()
                    .map(|i| format!("-{i}"))
                    .unwrap_or_default()
            );
            let boton = (restaurable && !pagina.solo_lectura).then(|| {
                view! { <BotonVolver ajuste=d.ajuste campo=partida.id.clone() descrito=id.clone() pagina /> }
            });
            view! {
                <div class="foto-fila">
                    <p class="foto-campo" id=id>{etiqueta(&partida)}</p>
                    <dl class="foto-valores">
                        <div>
                            <dt>{t!("ajustes-foto-al-instalar")}</dt>
                            <dd>{valor_resumido(&partida)}</dd>
                        </div>
                        <div>
                            <dt>{t!("ajustes-foto-ahora")}</dt>
                            <dd>{valor_resumido(&actual)}</dd>
                        </div>
                    </dl>
                    {boton}
                </div>
            }
            .into_any()
        }
        Diferencia::Alta(campos) => instancia(t!("ajustes-foto-alta"), &campos),
        Diferencia::Baja(campos) => instancia(t!("ajustes-foto-baja"), &campos),
    }
}

/// Una instancia de una lista: qué le pasó y sus campos.
fn instancia(que: String, campos: &[CampoVista]) -> AnyView {
    let valores = campos
        .iter()
        .map(|c| {
            view! {
                <div class="ajuste-fila">
                    <dt>{etiqueta(c)}</dt>
                    <dd>{valor_resumido(c)}</dd>
                </div>
            }
        })
        .collect_view();
    view! {
        <div class="foto-fila">
            <p class="foto-campo">{que}</p>
            <dl class="ajuste-valores">{valores}</dl>
        </div>
    }
    .into_any()
}

/// «Volver a este valor»: prepara el cambio y abre el diálogo; un error se
/// ve junto al botón.
#[component]
fn BotonVolver(ajuste: String, campo: String, descrito: String, pagina: Pagina) -> impl IntoView {
    let preparando = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let volver = move |_| {
        if preparando.get_untracked() {
            return;
        }
        preparando.set(true);
        error.set(None);
        let (ajuste, id) = (ajuste.clone(), campo.clone());
        spawn_local(async move {
            match preparar_volver_a_partida(ajuste, Some(id), None).await {
                Ok(g) => pagina.grupo.set(Some(g)),
                Err(e) => error.set(Some(texto_error(&e))),
            }
            preparando.set(false);
        });
    };
    view! {
        <div class="foto-acciones">
            // Sin `disabled`: el botón conserva el foco y el diálogo se lo
            // devuelve al cerrarse.
            <button
                class="boton-sutil"
                type="button"
                // El campo al que vuelve: hay un botón por fila.
                aria-describedby=descrito
                aria-disabled=move || preparando.get().then_some("true")
                on:click=volver
            >
                {move || {
                    if preparando.get() { t!("ajustes-grupo-preparando") } else { t!("ajustes-foto-volver") }
                }}
            </button>
            <p class="error-formulario" role="alert">{move || error.get()}</p>
        </div>
    }
}
