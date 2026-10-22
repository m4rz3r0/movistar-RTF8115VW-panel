//! «IP fijas»: el detalle de una reserva DHCP de la lista
//! ([`DetalleReserva`]), con su formulario; «Añadir IP fija» abre el mismo
//! formulario vacío. Antes de enviar, la página comprueba la reserva como el
//! servidor ([`validar_reserva`]) y dice el error junto al campo. La lista
//! misma es la genérica ([`super::lista`]).

use super::campo::{Campo, cambiado, inicial, validar};
use super::detalle::Registro;
use super::lista::seguir_lo_editado;
use super::piezas::{enfocar, fila as fila_de_campo};
use crate::ajustes_router::{
    AjusteVista, CampoVista, GrupoVista, InstanciaVista, PeticionAjuste, preparar_ajuste,
    valida_la_reserva, validar_reserva,
};
use crate::t;
use crate::ui::explorador::texto_error;
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;

/// Qué se está editando: una reserva nueva o la de ese id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Modo {
    Nueva,
    Editar(String),
}

/// Valor en edición del campo `id` de una instancia.
fn valor(campos: &[CampoVista], id: &str) -> String {
    campos
        .iter()
        .find(|c| c.id == id)
        .map(|c| inicial(&c.valor))
        .unwrap_or_default()
}

/// Reservas como triples `(id, mac, ip)`.
pub(crate) fn existentes(instancias: &[InstanciaVista]) -> Vec<(String, String, String)> {
    instancias
        .iter()
        .map(|i| {
            (
                i.id.clone(),
                valor(&i.campos, "mac"),
                valor(&i.campos, "ip"),
            )
        })
        .collect()
}

/// Formulario de una reserva, nueva (con la plantilla) o existente. Llama a
/// `preparado` con el resumen del cambio y a `cancelar` al dejarlo. Su error
/// general va a `error`, la única región de alerta del detalle. Al editar,
/// envía la MAC que espera (`esperada`): si otra sesión cambió la reserva,
/// el servidor no la toca. Lo editado se apunta en `registro` con el id de
/// su fila (`fila`) y sobrevive a que la página vuelva a leer.
#[component]
fn FormReserva(
    campos: Vec<CampoVista>,
    modo: Modo,
    esperada: Option<String>,
    error: RwSignal<Option<String>>,
    existentes: Vec<(String, String, String)>,
    lan: Option<(Ipv4Addr, Ipv4Addr)>,
    dom: String,
    preparado: Callback<GrupoVista>,
    cancelar: Callback<()>,
    registro: Registro,
    fila: String,
) -> impl IntoView {
    let valores: Vec<RwSignal<String>> = campos
        .iter()
        .map(|c| RwSignal::new(inicial(&c.valor)))
        .collect();
    let olvidar = seguir_lo_editado(
        fila,
        campos
            .iter()
            .zip(&valores)
            .map(|(c, v)| (c.id.clone(), *v, inicial(&c.valor)))
            .collect(),
        registro,
    );
    let errores: Vec<RwSignal<Option<String>>> =
        campos.iter().map(|_| RwSignal::new(None)).collect();
    let preparando = RwSignal::new(false);
    let control = {
        let dom = dom.clone();
        move |i: usize| format!("{dom}-c{i}-control")
    };
    let nueva = modo == Modo::Nueva;
    let titulo = if nueva {
        t!("ajuste-red_local-reservas-nueva")
    } else {
        t!("ajuste-red_local-reservas-cambiar")
    };
    let instancia = match &modo {
        Modo::Nueva => None,
        Modo::Editar(id) => Some(id.clone()),
    };
    let datos = StoredValue::new((campos.clone(), valores.clone(), errores.clone()));

    let guardar = {
        let control = control.clone();
        move |ev: SubmitEvent| {
            ev.prevent_default();
            if preparando.get_untracked() {
                return;
            }
            error.set(None);
            let mut pedir = BTreeMap::new();
            let mut primero_mal = None::<usize>;
            datos.with_value(|(cs, vs, es)| {
                let v = |i: usize| vs[i].get_untracked();
                for (i, c) in cs.iter().enumerate() {
                    es[i].set(None);
                    // Una nueva lleva todos los campos; al editar, los cambiados.
                    if !nueva && !cambiado(c, &v(i)) {
                        continue;
                    }
                    match validar(c, &v(i)) {
                        Ok(()) => {
                            pedir.insert(c.id.clone(), v(i).trim().to_owned());
                        }
                        Err(m) => {
                            es[i].set(Some(m));
                            primero_mal.get_or_insert(i);
                        }
                    }
                }
                // Lo mismo que comprobará el servidor, junto al campo (no al
                // desactivarla: se puede apagar aunque ya no valdría).
                let validar_todo = valida_la_reserva(&PeticionAjuste {
                    ajuste: String::new(),
                    instancia: instancia.clone(),
                    borrar: false,
                    valores: pedir.clone(),
                    esperados: BTreeMap::new(),
                });
                let pos = |id: &str| cs.iter().position(|c| c.id == id);
                if primero_mal.is_none()
                    && validar_todo
                    && let (Some(lan), Some(m), Some(p)) = (lan, pos("mac"), pos("ip"))
                    && let Ok(ip) = v(p).trim().parse::<Ipv4Addr>()
                    && let Err(e) =
                        validar_reserva(&v(m), ip, lan, &existentes, instancia.as_deref())
                {
                    let i = if e.campo() == "mac" { m } else { p };
                    es[i].set(Some(e.texto(lan.0)));
                    primero_mal = Some(i);
                }
            });
            if let Some(i) = primero_mal {
                enfocar(&control(i));
                return;
            }
            if pedir.is_empty() {
                error.set(Some(t!("ajustes-error-sin-cambios")));
                return;
            }
            preparando.set(true);
            let p = PeticionAjuste {
                ajuste: "red_local.reservas".into(),
                instancia: instancia.clone(),
                borrar: false,
                valores: pedir,
                esperados: esperada
                    .iter()
                    .map(|m| ("mac".to_owned(), m.clone()))
                    .collect(),
            };
            spawn_local(async move {
                match preparar_ajuste(p).await {
                    Ok(g) => {
                        olvidar.run(());
                        preparado.run(g);
                    }
                    Err(e) => error.set(Some(texto_error(&e))),
                }
                preparando.set(false);
            });
        }
    };

    let filas = campos
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            view! {
                <Campo
                    c
                    dom=format!("{dom}-c{i}")
                    valor=valores[i]
                    error=errores[i]
                    bloqueado=preparando
                />
            }
        })
        .collect_view();
    view! {
        <form class="ajuste-formulario reserva-formulario" novalidate=true on:submit=guardar>
            <h3>{titulo}</h3>
            <div class="ajuste-campos">{filas}</div>
            <div class="ajuste-acciones">
                <button
                    class="b-sec"
                    type="button"
                    disabled=move || preparando.get()
                    on:click=move |_| cancelar.run(())
                >
                    {t!("ajustes-cancelar")}
                </button>
                <button
                    class="b-pri"
                    type="submit"
                    aria-disabled=move || preparando.get().then_some("true")
                >
                    {move || {
                        if preparando.get() { t!("ajustes-grupo-preparando") } else { t!("ajustes-guardar") }
                    }}
                </button>
            </div>
        </form>
    }
}

/// El detalle de una IP fija de la lista (`instancia`) o, sin ella, el
/// formulario de una nueva (con la plantilla de `a`). Sin `editable` (solo
/// lectura), sus valores sin editar. `lan`: IP y máscara del router, para
/// validar antes de enviar.
#[component]
pub(crate) fn DetalleReserva(
    a: AjusteVista,
    instancia: Option<InstanciaVista>,
    editable: bool,
    lan: Option<(Ipv4Addr, Ipv4Addr)>,
    error: RwSignal<Option<String>>,
    preparado: Callback<GrupoVista>,
    cancelar: Callback<()>,
    registro: Registro,
    fila: String,
) -> impl IntoView {
    if !editable {
        let campos = instancia.map(|i| i.campos).unwrap_or_default();
        let filas = campos.iter().map(fila_de_campo).collect_view();
        return view! { <dl class="ajuste-valores">{filas}</dl> }.into_any();
    }
    let (campos, modo, esperada) = match &instancia {
        None => (a.campos.clone(), Modo::Nueva, None),
        Some(i) => (
            i.campos.clone(),
            Modo::Editar(i.id.clone()),
            Some(valor(&i.campos, "mac")),
        ),
    };
    view! {
        <FormReserva
            campos
            modo
            esperada
            error
            existentes=existentes(&a.instancias)
            lan
            dom=format!("reserva-{fila}")
            preparado
            cancelar
            registro
            fila
        />
    }
    .into_any()
}
