//! «IP fijas» ([`Reservas`]): las reservas DHCP en una tabla (apilada en
//! pantallas estrechas) con el dispositivo si se conoce, su MAC, su IP y si está
//! activa. «Añadir IP fija», «Editar» y «Borrar» preparan el cambio y lo
//! guardan con el diálogo de grupo. Antes de enviar, la página comprueba la
//! reserva como el servidor ([`validar_reserva`]) y dice el error junto al
//! campo.

use super::Foco;
use super::campo::{Campo, cambiado, etiqueta, inicial, texto_legible, validar};
use super::dispositivos::{DatosRed, RedLocal, aviso_error, nombre};
use super::tarjeta::{ayuda, cabecera, detalles_tecnicos, dom, enfocar};
use crate::ajustes_router::{
    AjusteVista, CampoVista, EstadoLectura, GrupoVista, InstanciaVista, PeticionAjuste,
    mac_comparable, preparar_ajuste, valida_la_reserva, validar_reserva,
};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::grupo::DialogoGrupo;
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;

/// Qué se está editando: una reserva nueva o la de ese id.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Modo {
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
pub(super) fn existentes(instancias: &[InstanciaVista]) -> Vec<(String, String, String)> {
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
/// general va a `error`, la única región de alerta de la tarjeta. Al editar,
/// envía la MAC que espera (`esperada`): si otra sesión cambió la reserva,
/// el servidor no la toca.
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
) -> impl IntoView {
    let valores: Vec<RwSignal<String>> = campos
        .iter()
        .map(|c| RwSignal::new(inicial(&c.valor)))
        .collect();
    let errores: Vec<RwSignal<Option<String>>> =
        campos.iter().map(|_| RwSignal::new(None)).collect();
    let preparando = RwSignal::new(false);
    let control = {
        let dom = dom.clone();
        move |i: usize| format!("{dom}-c{i}-control")
    };
    // El foco, al primer campo al abrirse.
    {
        let primero = control(0);
        request_animation_frame(move || enfocar(&primero));
    }
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
                    Ok(g) => preparado.run(g),
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
                    class="boton-sutil"
                    type="button"
                    disabled=move || preparando.get()
                    on:click=move |_| cancelar.run(())
                >
                    {t!("ajustes-cancelar")}
                </button>
                <button
                    class="boton"
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

#[component]
pub(crate) fn Reservas(
    a: AjusteVista,
    solo_lectura: bool,
    recargar: Callback<()>,
) -> impl IntoView {
    let datos = expect_context::<DatosRed>();
    let foco = expect_context::<Foco>();
    let id_dom = dom(&a);
    let leido = matches!(a.estado, EstadoLectura::Leido);
    let editable = leido && !solo_lectura;
    let grupo = RwSignal::new(None::<GrupoVista>);
    let modo = RwSignal::new(None::<Modo>);
    let borrando = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<String>);
    let plantilla = StoredValue::new(a.campos.clone());
    let instancias = StoredValue::new(a.instancias.clone());
    let anadir_id = format!("{id_dom}-anadir");
    let editar_id = {
        let id_dom = id_dom.clone();
        move |id: &str| format!("{id_dom}-editar-{id}")
    };

    // Al dejar el formulario, el foco vuelve al botón que lo abrió.
    let volver = {
        let anadir_id = anadir_id.clone();
        let editar_id = editar_id.clone();
        move |m: &Modo| match m {
            Modo::Nueva => anadir_id.clone(),
            Modo::Editar(id) => editar_id(id),
        }
    };
    let cancelar = {
        let volver = volver.clone();
        Callback::new(move |()| {
            if let Some(m) = modo.get_untracked() {
                let id = volver(&m);
                request_animation_frame(move || enfocar(&id));
            }
            modo.set(None);
        })
    };
    let preparado = Callback::new(move |g: GrupoVista| grupo.set(Some(g)));
    let titulo = format!("{id_dom}-titulo");
    let cerrar = Callback::new(move |aplicado: Option<i64>| {
        grupo.set(None);
        if aplicado.is_some() {
            modo.set(None);
            foco.0.set(Some(titulo.clone()));
            recargar.run(());
        }
    });
    let borrar = Callback::new(move |(id, mac): (String, String)| {
        if borrando.get_untracked().is_some() {
            return;
        }
        error.set(None);
        modo.set(None);
        borrando.set(Some(id.clone()));
        let p = PeticionAjuste {
            ajuste: "red_local.reservas".into(),
            instancia: Some(id),
            borrar: true,
            valores: BTreeMap::new(),
            esperados: [("mac".to_owned(), mac)].into(),
        };
        spawn_local(async move {
            match preparar_ajuste(p).await {
                Ok(g) => grupo.set(Some(g)),
                Err(e) => error.set(Some(texto_error(&e))),
            }
            borrando.set(None);
        });
    });

    let cabeza = move || {
        let col =
            |id: &str| plantilla.with_value(|cs| cs.iter().find(|c| c.id == id).map(etiqueta));
        let acciones = editable.then(|| {
            view! {
                <th scope="col"><span class="solo-lector">{t!("ajuste-red_local-reservas-col-acciones")}</span></th>
            }
        });
        view! {
            <thead>
                <tr>
                    <th scope="col">{t!("ajuste-red_local-reservas-col-equipo")}</th>
                    <th scope="col">{col("mac")}</th>
                    <th scope="col">{col("ip")}</th>
                    <th scope="col">{col("activa")}</th>
                    {acciones}
                </tr>
            </thead>
        }
    };
    let dom_form = format!("{id_dom}-form");

    let cuerpo = move |RedLocal { dispositivos, lan }: RedLocal| {
        // Sin dispositivos leídos, las reservas se ven igual, sin su nombre.
        let dispositivos = dispositivos.unwrap_or_default();
        let lista = instancias.get_value();
        let equipo = |mac: &str| {
            let m = mac_comparable(mac);
            nombre(
                dispositivos
                    .iter()
                    .find(|d| mac_comparable(&d.mac) == m)
                    .map_or("", |d| d.nombre.as_str()),
            )
        };
        let et = |id: &str| {
            plantilla.with_value(|cs| {
                cs.iter()
                    .find(|c| c.id == id)
                    .map(etiqueta)
                    .unwrap_or_default()
            })
        };
        let filas = lista
            .iter()
            .map(|i| {
                let quien = equipo(&valor(&i.campos, "mac"));
                let activa = i.campos.iter().find(|c| c.id == "activa").map(|c| {
                    texto_legible(c, &inicial(&c.valor))
                });
                let id = i.id.clone();
                let mac = valor(&i.campos, "mac");
                let acciones = editable.then(|| {
                    let (id_e, id_b) = (id.clone(), id.clone());
                    let mac_b = mac.clone();
                    let ocupado = {
                        let id = id.clone();
                        move || borrando.with(|b| b.as_deref() == Some(id.as_str()))
                    };
                    view! {
                        <td class="acciones-fila" data-etiqueta=t!("ajuste-red_local-reservas-col-acciones")>
                            <button
                                class="boton-mini"
                                type="button"
                                id=editar_id(&id)
                                aria-label=t!("ajuste-red_local-reservas-editar-de", equipo = quien.clone())
                                on:click=move |_| {
                                    error.set(None);
                                    modo.set(Some(Modo::Editar(id_e.clone())));
                                }
                            >
                                {t!("ajustes-editar")}
                            </button>
                            <button
                                class="boton-mini boton-peligro"
                                type="button"
                                aria-label=t!("ajuste-red_local-reservas-borrar-de", equipo = quien.clone())
                                aria-disabled=move || ocupado().then_some("true")
                                on:click=move |_| borrar.run((id_b.clone(), mac_b.clone()))
                            >
                                {t!("ajuste-red_local-reservas-borrar")}
                            </button>
                        </td>
                    }
                });
                view! {
                    <tr>
                        <th scope="row" data-etiqueta=t!("ajuste-red_local-reservas-col-equipo")>{quien.clone()}</th>
                        <td class="mono" data-etiqueta=et("mac")>{valor(&i.campos, "mac")}</td>
                        <td class="mono" data-etiqueta=et("ip")>{valor(&i.campos, "ip")}</td>
                        <td data-etiqueta=et("activa")>{activa}</td>
                        {acciones}
                    </tr>
                }
            })
            .collect_view();
        let tabla = if lista.is_empty() {
            view! { <p class="ayuda">{t!("ajuste-red_local-reservas-vacia")}</p> }.into_any()
        } else {
            view! {
                <div class="tabla-scroll tabla-apilable">
                    <table class="tabla tabla-apilada">
                        {cabeza()}
                        <tbody>{filas}</tbody>
                    </table>
                </div>
            }
            .into_any()
        };
        let todas = existentes(&lista);
        let dom_form = dom_form.clone();
        let formulario = move || {
            modo.get().map(|m| {
                let (campos, esperada) = match &m {
                    Modo::Nueva => (plantilla.get_value(), None),
                    Modo::Editar(id) => instancias.with_value(|is| {
                        is.iter()
                            .find(|i| i.id == *id)
                            .map(|i| (i.campos.clone(), Some(valor(&i.campos, "mac"))))
                            .unwrap_or_default()
                    }),
                };
                view! {
                    <FormReserva
                        campos
                        modo=m
                        esperada
                        error
                        existentes=todas.clone()
                        lan
                        dom=dom_form.clone()
                        preparado
                        cancelar
                    />
                }
            })
        };
        let anadir = editable.then(|| {
            let anadir_id = anadir_id.clone();
            view! {
                <div class="ajuste-acciones reservas-acciones" class:oculto=move || modo.get() == Some(Modo::Nueva)>
                    <button
                        class="boton-sutil"
                        type="button"
                        id=anadir_id
                        on:click=move |_| {
                            error.set(None);
                            modo.set(Some(Modo::Nueva));
                        }
                    >
                        {t!("ajuste-red_local-reservas-anadir")}
                    </button>
                </div>
            }
        });
        // La única región de alerta de la tarjeta: errores de borrar y del
        // formulario.
        view! {
            {tabla}
            {anadir}
            {formulario}
            <p class="error-formulario" role="alert">{move || error.get()}</p>
        }
        .into_any()
    };

    let contenido = match &a.estado {
        EstadoLectura::SinLeer(m) => aviso_error(m.clone()),
        EstadoLectura::Leido => view! {
            <Transition fallback=|| view! { <p class="estado-espera">{t!("comun-cargando")}</p> }>
                {
                    let cuerpo = cuerpo.clone();
                    move || {
                        let cuerpo = cuerpo.clone();
                        Suspend::new(async move { cuerpo(datos.leer().await) })
                    }
                }
            </Transition>
        }
        .into_any(),
    };
    view! {
        {cabecera(&a, editable)}
        {ayuda(&a)}
        {contenido}
        {detalles_tecnicos(&a.clave, &a.tecnico)}
        {move || grupo.get().map(|g| view! { <DialogoGrupo grupo=g on_cerrar=cerrar /> })}
    }
}
