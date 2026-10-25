//! Conexión a internet en el detalle de su elemento: un bloque por conexión, la de internet primero, la fibra y «Cómo pasar a monopuesto» ([`Conexion`]).
//!
//! Todo es de solo lectura. El servidor ya pone primero la conexión de
//! internet y marca las demás con su nota («del operador», o «sin
//! identificar» todas si no se sabe cuál es). Los secretos (la contraseña
//! PPP, los de la fibra) van censurados con el ojo, como en el resto de la
//! página; uno que el router no deja leer lo dice. Lo vacío (el usuario de
//! una conexión sin PPP) no se pinta. La sección de monopuesto solo
//! explica: no tiene botón.

use super::piezas::{ayuda, detalles_tecnicos, fila};
use crate::ajustes_router::{AjusteVista, CampoVista, InstanciaVista};
use crate::t;
use crate::ui::i18n::traducir;
use ajustes::ValorLegible;
use leptos::prelude::*;

/// Componente del ajuste de las conexiones.
pub(crate) const CONEXION: &str = "conexion";

/// Nota de las conexiones cuando no se sabe cuál es la de internet.
const SIN_IDENTIFICAR: &str = "ajuste-internet-conexion-sin-identificar";

/// ¿Se pinta un campo con `valor` (y `secreto`, el valor leído de un
/// secreto)? No si está vacío de verdad. Un secreto que el router no deja
/// leer (`None`) sí: su campo lo dice.
fn se_pinta(valor: &ValorLegible, secreto: Option<&str>) -> bool {
    match valor {
        ValorLegible::Texto(t) => !t.trim().is_empty(),
        ValorLegible::Secreto => secreto != Some(""),
        _ => true,
    }
}

fn con_valor(c: &CampoVista) -> bool {
    se_pinta(&c.valor, c.valor_secreto.as_deref())
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
    let (titulo, clase) = match i.nota.as_deref() {
        Some(SIN_IDENTIFICAR) => (
            t!(
                "ajuste-internet-conexion-sin-identificar",
                id = i.id.clone()
            ),
            "conexion-bloque conexion-sin-identificar",
        ),
        Some(nota) => (traducir(nota, None), "conexion-bloque conexion-operador"),
        None => (
            t!("ajuste-internet-conexion-internet"),
            "conexion-bloque conexion-internet",
        ),
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

/// Un ajuste de consulta que acompaña a las conexiones (la fibra): su
/// título, su descripción y ayuda, sus valores y sus detalles técnicos.
fn anexo(f: &AjusteVista) -> impl IntoView + use<> {
    let id = format!("conexion-{}-titulo", f.id.replace('.', "-"));
    let filas = f
        .campos
        .iter()
        .filter(|c| con_valor(c))
        .map(fila)
        .collect_view();
    let titulo_id = id.clone();
    view! {
        <section class="conexion-bloque conexion-anexo" aria-labelledby=id>
            <h4 id=titulo_id>{traducir(&format!("{}-titulo", f.clave), None)}</h4>
            <p class="ajuste-desc">{traducir(&format!("{}-desc", f.clave), None)}</p>
            {ayuda(f)}
            <dl class="ajuste-valores">{filas}</dl>
            {detalles_tecnicos(&f.clave, &f.tecnico)}
        </section>
    }
}

/// Las conexiones de `a`, los ajustes que las acompañan (`anexos`: la
/// fibra) y cómo pasar a monopuesto.
#[component]
pub(crate) fn Conexion(a: AjusteVista, anexos: Vec<AjusteVista>) -> impl IntoView {
    let bloques = a.instancias.iter().map(bloque).collect_view();
    let anexos = anexos.iter().map(anexo).collect_view();
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
        {anexos}
        <details class="ajuste-ayuda conexion-monopuesto">
            <summary>{t!("ajuste-internet-conexion-monopuesto-titulo")}</summary>
            <ol class="conexion-pasos">{pasos}</ol>
            <ul class="conexion-avisos">{avisos}</ul>
        </details>
    }
}

#[cfg(test)]
mod tests {
    use super::{lineas, se_pinta};
    use ajustes::ValorLegible;

    #[test]
    fn lineas_sin_vacias_ni_sangria() {
        assert_eq!(lineas("uno\n    dos\n\n tres "), ["uno", "dos", "tres"]);
    }

    /// Lo vacío no se pinta; un secreto que el router no deja leer, sí.
    #[test]
    fn solo_se_oculta_lo_vacio() {
        assert!(!se_pinta(&ValorLegible::Texto("  ".into()), None));
        assert!(se_pinta(&ValorLegible::Texto("ip2".into()), None));
        assert!(!se_pinta(&ValorLegible::Secreto, Some("")));
        assert!(se_pinta(&ValorLegible::Secreto, Some("clave-de-prueba-1")));
        assert!(se_pinta(&ValorLegible::Secreto, None));
    }
}
