//! El detalle de una regla de la lista: su formulario o, sin editar, sus valores.

use super::super::detalle::Registro;
use super::super::piezas::fila as fila_de_campo;
use super::formulario::FormRegla;
use super::*;
use crate::ajustes_router::AjusteVista;

/// El detalle de la regla `instancia` o, sin ella, el formulario de una
/// nueva (con la plantilla de `a`). Sin `editable` (solo lectura), o con
/// una regla que la página no entiende, sus valores sin editar (se puede
/// borrar igual). Si su dispositivo de destino ya no está en la red, lo
/// dice y qué hacer.
#[component]
pub(crate) fn DetalleRegla(
    a: AjusteVista,
    instancia: Option<InstanciaVista>,
    editable: bool,
    red: RedCasa,
    error: RwSignal<Option<String>>,
    preparado: Callback<GrupoVista>,
    cancelar: Callback<()>,
    registro: Registro,
    fila: String,
) -> impl IntoView {
    let red = RedPuertos::de(&red);
    let perdido = instancia
        .as_ref()
        .is_some_and(|i| red.destino_perdido(&valor_de(&i.campos, "destino")));
    let aviso = perdido.then(|| {
        view! { <p class="regla-perdida">{t!("ajuste-puertos-reglas-destino-perdido-desc")}</p> }
    });
    let entendida = instancia.as_ref().is_none_or(|i| regla_de(i).is_some());
    let cuerpo = if editable && entendida {
        let existentes: Vec<ReglaPuertos> = a.instancias.iter().filter_map(regla_de).collect();
        view! {
            <FormRegla
                plantilla=a.campos.clone()
                editando=instancia
                existentes
                red
                error
                dom=format!("regla-{fila}")
                preparado
                cancelar
                registro
                fila
            />
        }
        .into_any()
    } else {
        let campos = instancia.map(|i| i.campos).unwrap_or_default();
        let filas = campos.iter().map(fila_de_campo).collect_view();
        view! { <dl class="ajuste-valores">{filas}</dl> }.into_any()
    };
    view! {
        {aviso}
        {cuerpo}
    }
}
