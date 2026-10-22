//! El detalle de una fila, que entra desde la derecha: su formulario o sus datos y «Borrar».
//!
//! Lleva la misma cabecera que el de una tarjeta (título, insignia, ✕ y, en
//! el móvil, «‹ Puertos»); Esc o ✕ lo cierran y la lista recupera su ancho.
//! El de la fila «nueva» (la dirección de la propia lista) es el formulario
//! vacío, que se cierra al aplicarse. Los detalles técnicos van una vez, al
//! pie de la lista. «Borrar» prepara la misma acción que la barra con esa
//! fila sola: el diálogo lo confirma con el botón de peligro.

use super::super::aviso::Seguimiento;
use super::super::detalle::{ID_TITULO_DETALLE, Registro};
use super::super::dispositivos::DetalleDispositivo;
use super::super::puertos::DetalleRegla;
use super::super::rejilla::Pagina;
use super::super::reservas::DetalleReserva;
use super::columnas::{Fila, Tipo, id_de_mac};
use crate::ajustes_router::{AjusteVista, GrupoVista, RedCasa, preparar_accion};
use crate::t;
use crate::ui::comun::Insignia;
use crate::ui::explorador::texto_error;
use crate::ui::i18n::traducir;
use leptos::{ev, prelude::*, task::spawn_local};

/// Título del detalle de la fila nueva.
fn titulo_nueva(tipo: Tipo) -> String {
    match tipo {
        Tipo::Reglas => t!("ajuste-puertos-reglas-nueva"),
        Tipo::Reservas => t!("ajuste-red_local-reservas-nueva"),
        Tipo::Dispositivos => String::new(),
    }
}

/// Nombre accesible de «Borrar» en la fila `nombre`.
fn borrar_de(tipo: Tipo, nombre: String) -> String {
    match tipo {
        Tipo::Reservas => t!("ajuste-red_local-reservas-borrar-de", equipo = nombre),
        _ => t!("ajuste-puertos-reglas-borrar-de", nombre = nombre),
    }
}

/// El detalle de `fila` de la lista `a` (o, sin ella, el de una nueva, con
/// el id `abierto`). `borrar`: la acción que la borra, si la lista la
/// tiene; `existentes`, las IP fijas (`(id, mac, ip)`), para saber si se
/// puede fijar la IP de un dispositivo.
#[component]
pub(crate) fn DetalleFila(
    tipo: Tipo,
    a: AjusteVista,
    fila: Option<Fila>,
    abierto: String,
    red: RedCasa,
    existentes: Vec<(String, String, String)>,
    borrar: Option<String>,
    registro: Registro,
) -> impl IntoView {
    let pagina = expect_context::<Pagina>();
    let s = expect_context::<Seguimiento>();
    let error = RwSignal::new(None::<String>);
    let editable = !pagina.solo_lectura;
    let id_fila = fila.as_ref().map(|f| f.id.clone());
    // Una instancia nueva, al aplicarse, ya está en su lista: su detalle
    // (vacío) se cierra.
    let preparado = {
        let (id_fila, abierto) = (id_fila.clone(), abierto.clone());
        Callback::new(move |g: GrupoVista| match &id_fila {
            Some(id) => s.revisar(g, vec![id.clone()]),
            None => s.revisar_y_cerrar(g, vec![abierto.clone()]),
        })
    };
    let cancelar = Callback::new(move |()| pagina.cerrar.run(()));
    let instancia = id_fila
        .as_ref()
        .and_then(|id| a.instancias.iter().find(|i| i.id == *id).cloned());
    let titulo = fila
        .as_ref()
        .map_or_else(|| titulo_nueva(tipo), |f| f.nombre.clone());
    let cuerpo = match tipo {
        Tipo::Reglas => view! {
            <DetalleRegla
                a=a.clone()
                instancia
                editable
                red=red.clone()
                error
                preparado
                cancelar
                registro
                fila=abierto.clone()
            />
        }
        .into_any(),
        Tipo::Reservas => view! {
            <DetalleReserva
                a=a.clone()
                instancia
                editable
                lan=red.lan
                error
                preparado
                cancelar
                registro
                fila=abierto.clone()
            />
        }
        .into_any(),
        Tipo::Dispositivos => red
            .dispositivos
            .iter()
            .flatten()
            .find(|d| id_de_mac(&d.mac) == abierto)
            .cloned()
            .map(|d| {
                view! {
                    <DetalleDispositivo
                        d
                        lan=red.lan
                        existentes
                        solo_lectura=!editable
                        error
                        preparado
                    />
                }
            })
            .into_any(),
    };
    // «Borrar», con el nombre de la fila, solo si existe y se puede.
    let boton_borrar = match (id_fila.clone(), borrar, editable) {
        (Some(id), Some(accion), true) => {
            let etiqueta = borrar_de(tipo, titulo.clone());
            let preparando = RwSignal::new(false);
            let pulsar = move |_| {
                if preparando.get_untracked() {
                    return;
                }
                preparando.set(true);
                error.set(None);
                let (tema, accion, id) = (pagina.tema.get_value(), accion.clone(), id.clone());
                spawn_local(async move {
                    // El detalle sigue abierto mientras se revisa (cancelar lo
                    // deja todo como estaba); al aplicarse, se cierra.
                    match preparar_accion(tema, accion, vec![id.clone()]).await {
                        Ok(g) => s.revisar(g, vec![id]),
                        Err(e) => {
                            error.try_set(Some(texto_error(&e)));
                        }
                    }
                    preparando.try_set(false);
                });
            };
            Some(view! {
                <div class="detalle-borrar">
                    <button
                        type="button"
                        class="b-pel"
                        aria-label=etiqueta
                        aria-disabled=move || preparando.get().then_some("true")
                        on:click=pulsar
                    >
                        {t!("ajustes-lista-borrar")}
                    </button>
                </div>
            })
        }
        _ => None,
    };
    let insignia = fila.map(|f| {
        let id = f.id.clone();
        move || {
            let (estado, texto) = s
                .insignia(&id)
                .unwrap_or((f.estado, f.texto_estado.clone()));
            view! { <Insignia estado texto /> }
        }
    });
    let teclas = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Escape" && !ev.default_prevented() {
            ev.prevent_default();
            pagina.cerrar.run(());
        }
    };
    let tema = pagina
        .tema
        .with_value(|t| traducir(&format!("ajustes-tema-{t}"), None));
    // Recién abierto, el foco a su título en cuanto está en la página.
    let h2 = NodeRef::<leptos::html::H2>::new();
    Effect::new(move |_| {
        if let Some(h) = h2.get()
            && pagina.enfocar_detalle.get_untracked()
        {
            pagina.enfocar_detalle.set(false);
            let _ = h.focus();
        }
    });
    let cerrar = move |_| pagina.cerrar.run(());
    view! {
        <section
            class="detalle detalle-lateral"
            aria-labelledby=ID_TITULO_DETALLE
            on:keydown=teclas
        >
            <header class="detalle-cabecera">
                <button type="button" class="b-txt detalle-volver" on:click=cerrar>
                    {t!("ajustes-detalle-volver", tema = tema)}
                </button>
                <h2 id=ID_TITULO_DETALLE tabindex="-1" node_ref=h2>{titulo}</h2>
                <span class="detalle-insignia">{insignia}</span>
                <button
                    type="button"
                    class="b-txt detalle-cerrar"
                    aria-label=t!("ajustes-detalle-cerrar")
                    on:click=cerrar
                >
                    "✕"
                </button>
            </header>
            {cuerpo}
            <p class="error-formulario" role="alert">{move || error.get()}</p>
            {boton_borrar}
        </section>
    }
}
