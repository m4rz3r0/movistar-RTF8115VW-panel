//! Cabecera de la tarjeta: icono de emisión, nombre, papel, SSID, insignia e interruptor.

use super::super::ID_SOLO_LECTURA;
use super::super::aviso::Seguimiento;
use super::super::rejilla::Pagina;
use crate::ajustes_router::{ElementoVista, InterruptorVista, PeticionAjuste, preparar_ajuste};
use crate::t;
use crate::ui::comun::{Glifo, IconoEmision, Insignia, Interruptor};
use crate::ui::explorador::texto_error;
use crate::ui::i18n::traducir;
use ajustes::ValorLegible;
use leptos::{prelude::*, task::spawn_local};
use std::collections::BTreeMap;

/// Dispositivos conectados del elemento, si su tarjeta lo dice.
fn conectados(e: &ElementoVista) -> Option<u32> {
    e.datos
        .iter()
        .find(|d| d.clave == "ajustes-dato-conectados")
        .and_then(|d| match &d.valor {
            ValorLegible::Texto(n) => n.parse().ok(),
            _ => None,
        })
}

/// El interruptor del elemento: sin `efecto` (encender), aplica el cambio
/// sin preguntar; con él (apagar corta lo que emite), abre antes el
/// diálogo, que dice qué se pierde. En solo lectura, desactivado y con el
/// porqué.
fn interruptor(e: &ElementoVista, i: InterruptorVista, nombre: &str) -> impl IntoView + use<> {
    let pagina = expect_context::<Pagina>();
    let s = expect_context::<Seguimiento>();
    let id = StoredValue::new(e.id.clone());
    let leido = i.encendido;
    let encendido = Signal::derive(move || id.with_value(|id| s.interruptor(id)).unwrap_or(leido));
    // Las suyas («Relé DNS activado») o «<nombre> encendida/apagada».
    let (si, no) = i.etiquetas.clone().unwrap_or_else(|| {
        (
            t!("ajustes-interruptor-si", nombre = nombre.to_owned()),
            t!("ajustes-interruptor-no", nombre = nombre.to_owned()),
        )
    });
    let etiqueta = Signal::derive(move || {
        if encendido.get() {
            si.clone()
        } else {
            no.clone()
        }
    });
    let conectados = conectados(e);
    let i = StoredValue::new(i);
    let al_cambiar = Callback::new(move |nuevo: bool| {
        let (ajuste, campo, efecto) =
            i.with_value(|i| (i.ajuste.clone(), i.campo.clone(), i.efecto.is_some()));
        let id = id.get_value();
        let pregunta = efecto && !nuevo;
        if !pregunta && !s.ocupado() {
            s.empieza(&id, Some(nuevo), String::new());
        }
        let p = PeticionAjuste {
            ajuste,
            instancia: None,
            borrar: false,
            valores: BTreeMap::from([(campo, nuevo.to_string())]),
            esperados: BTreeMap::new(),
        };
        spawn_local(async move {
            match preparar_ajuste(p).await {
                Ok(mut g) if pregunta => {
                    if let Some(n) = conectados.filter(|n| *n > 0) {
                        g.aviso = Some(t!("ajustes-aviso-conectados", n = n));
                    }
                    s.revisar(g, vec![id]);
                }
                Ok(g) => s.aplicar(&g, Vec::new(), vec![id], false),
                Err(e) => s.falla(texto_error(&e)),
            }
        });
    });
    let solo_lectura = pagina.solo_lectura;
    // Mientras su propio cambio se aplica o se comprueba, no se pulsa otra
    // vez (un doble clic prepararía el valor contrario).
    let desactivado =
        Signal::derive(move || solo_lectura || id.with_value(|id| s.insignia(id).is_some()));
    view! {
        <Interruptor
            encendido
            etiqueta
            desactivado
            descrito=solo_lectura.then(|| ID_SOLO_LECTURA.to_owned())
            al_cambiar
        />
    }
}

/// La cabecera de la tarjeta de `e` (`nombre`, su título con el id
/// `titulo`). La insignia dice «Aplicando…» o «Comprobando…» mientras el
/// elemento cambia.
pub(super) fn cabecera(e: &ElementoVista, nombre: &str, titulo: String) -> impl IntoView + use<> {
    let s = expect_context::<Seguimiento>();
    let id = e.id.clone();
    let (estado, texto) = (e.estado, e.texto_estado.clone());
    // Sin un estado que decir (la contraseña del router), sin insignia,
    // salvo mientras cambia.
    let insignia = move || {
        let (estado, texto) = s.insignia(&id).unwrap_or((estado, texto.clone()));
        (!texto.is_empty()).then(|| view! { <Insignia estado texto /> })
    };
    let papel = e.papel.as_deref().map(|k| traducir(k, None));
    let ssid = e
        .ssid
        .clone()
        .map(|s| view! { <span class="tarjeta-ssid mono">{s}</span> });
    let sub = (papel.is_some() || ssid.is_some()).then(|| {
        view! { <p class="tarjeta-papel">{papel.map(|p| view! { <span>{p}</span> })} {ssid}</p> }
    });
    let interruptor = e.interruptor.clone().map(|i| interruptor(e, i, nombre));
    view! {
        <div class="tarjeta-cabecera">
            {match e.icono.clone() {
                Some(nombre) => view! { <Glifo nombre /> }.into_any(),
                None => view! { <IconoEmision estado=e.estado /> }.into_any(),
            }}
            <div class="tarjeta-nombre">
                <h2 id=titulo>{nombre.to_owned()}</h2>
                {sub}
            </div>
            <span class="tarjeta-insignia">{insignia}</span>
            {interruptor}
        </div>
    }
}
