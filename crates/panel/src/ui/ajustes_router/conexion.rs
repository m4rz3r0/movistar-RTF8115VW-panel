//! Conexión a internet en el detalle de su elemento: un bloque por conexión, la de internet primero, y «Cómo pasar a monopuesto» ([`Conexion`]).
//!
//! Todo es de solo lectura. El servidor ya pone primero la conexión de
//! internet y marca las demás con su nota («del operador»). Los secretos
//! (la contraseña PPP) van censurados con el ojo, como en el resto de la
//! página. Lo vacío (el usuario de una conexión sin PPP) no se pinta. La
//! sección de monopuesto solo explica: no tiene botón.

use super::piezas::fila;
use crate::ajustes_router::{AjusteVista, CampoVista, InstanciaVista};
use crate::t;
use crate::ui::i18n::traducir;
use ajustes::ValorLegible;
use leptos::prelude::*;

/// Componente del ajuste de las conexiones.
pub(crate) const CONEXION: &str = "conexion";

/// ¿Se pinta el campo? No si está vacío (texto o secreto).
fn con_valor(c: &CampoVista) -> bool {
    match &c.valor {
        ValorLegible::Texto(t) => !t.trim().is_empty(),
        ValorLegible::Secreto => c.valor_secreto.as_deref().is_some_and(|v| !v.is_empty()),
        _ => true,
    }
}

/// Las líneas no vacías de un texto de varias líneas.
fn lineas(texto: &str) -> Vec<String> {
    texto
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

/// El bloque de una conexión: su título (la de internet o la nota de la
/// del operador) y sus valores.
fn bloque(i: &InstanciaVista) -> impl IntoView + use<> {
    let id = format!("conexion-{}-titulo", i.id);
    let titulo = match &i.nota {
        Some(nota) => traducir(nota, None),
        None => t!("ajuste-internet-conexion-internet"),
    };
    let clase = if i.nota.is_some() {
        "conexion-bloque conexion-operador"
    } else {
        "conexion-bloque conexion-internet"
    };
    let filas = i
        .campos
        .iter()
        .filter(|c| con_valor(c))
        .map(fila)
        .collect_view();
    let titulo_id = id.clone();
    view! {
        <section class=clase aria-labelledby=id>
            <h4 id=titulo_id>{titulo}</h4>
            <dl class="ajuste-valores">{filas}</dl>
        </section>
    }
}

/// Las conexiones de `a` y cómo pasar a monopuesto.
#[component]
pub(crate) fn Conexion(a: AjusteVista) -> impl IntoView {
    let bloques = a.instancias.iter().map(bloque).collect_view();
    let pasos = lineas(&t!("ajuste-internet-conexion-monopuesto-pasos"))
        .into_iter()
        .map(|p| view! { <li>{p}</li> })
        .collect_view();
    let avisos = lineas(&t!("ajuste-internet-conexion-monopuesto-avisos"))
        .into_iter()
        .map(|p| view! { <li>{p}</li> })
        .collect_view();
    view! {
        <div class="conexiones">{bloques}</div>
        <details class="ajuste-ayuda conexion-monopuesto">
            <summary>{t!("ajuste-internet-conexion-monopuesto-titulo")}</summary>
            <ol class="conexion-pasos">{pasos}</ol>
            <ul class="conexion-avisos">{avisos}</ul>
        </details>
    }
}

#[cfg(test)]
mod tests {
    use super::lineas;

    #[test]
    fn lineas_sin_vacias_ni_sangria() {
        assert_eq!(lineas("uno\n    dos\n\n tres "), ["uno", "dos", "tres"]);
    }
}
