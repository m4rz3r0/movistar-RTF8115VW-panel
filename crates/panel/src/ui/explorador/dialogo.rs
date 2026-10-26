//! Diálogos modales: revisar y aplicar un cambio (también lo usa el
//! historial para deshacer).
//!
//! Son `<dialog>` nativos abiertos con `showModal()`: el resto de la página
//! queda inerte (el foco no sale del diálogo), Esc lo cierra salvo mientras
//! se está aplicando y, al cerrarse, el foco vuelve a donde estaba.

use super::{
    CambioVista, CampoCambio, EstadoDeCambio, MASCARA, PreparadoVista, Valor, confirmar_cambio,
    duracion, estado_cambio, texto_error,
};
use crate::t;
use crate::ui::comun::{EstadoVisual, Insignia};
use crate::ui::{Estado, IconoEstado};
use leptos::{ev::SubmitEvent, html, prelude::*, task::spawn_local};
use std::time::Duration;

/// Cada cuánto se pregunta por el estado de un cambio.
const SONDEO: Duration = Duration::from_secs(2);

/// Abre el `<dialog>` como modal en cuanto existe en el DOM (lo usan todos
/// los diálogos del panel).
pub(crate) fn abrir_modal(dlg: NodeRef<html::Dialog>) {
    Effect::new(move |_| {
        if let Some(d) = dlg.get()
            && !d.open()
        {
            let _ = d.show_modal();
        }
    });
}

fn valor(v: &Valor, nombre: &str) -> AnyView {
    match v {
        Valor::Clave(v) => view! {
            <crate::ui::secreto::SecretoLeido valor=Some(v.clone()) nombre=nombre.to_owned() />
        }
        .into_any(),
        Valor::Texto(t) => view! { <span class="mono">{t.clone()}</span> }.into_any(),
        Valor::Secreto => view! {
            <span class="secreto">{MASCARA}</span>
            <span class="solo-lector">{t!("comun-valor-oculto")}</span>
        }
        .into_any(),
        Valor::Opcion { valor, etiqueta } => {
            let texto = Valor::texto_opcion(valor, etiqueta.as_deref());
            view! { <span lang="es">{texto}</span> }.into_any()
        }
        Valor::Ausente => view! {
            <span class="vacia" aria-hidden="true">"—"</span>
            <span class="solo-lector">{t!("comun-sin-valor")}</span>
        }
        .into_any(),
    }
}

/// Tabla de un diff: campo (con su nombre técnico, salvo con
/// `sin_nombres`: el Historial lo deja en sus detalles técnicos), antes y
/// después. Los secretos van enmascarados o, con su valor
/// ([`Valor::Clave`]), censurados con el ojo; las opciones, con su etiqueta.
#[component]
pub fn TablaDiff(campos: Vec<CampoCambio>, #[prop(optional)] sin_nombres: bool) -> impl IntoView {
    if campos.is_empty() {
        return view! { <p class="pagina-sub">{t!("biblioteca-diff-vacio")}</p> }.into_any();
    }
    let filas = campos
        .iter()
        .map(|c| {
            let campo = if sin_nombres {
                view! { <span>{c.etiqueta.clone()}</span> }.into_any()
            } else {
                view! {
                    <span lang="es">{c.etiqueta.clone()}</span>
                    <span class="mono campo-nombre">{c.nombre.clone()}</span>
                }
                .into_any()
            };
            let (antes, despues) = (
                t!("historial-valor-anterior", campo = c.etiqueta.clone()),
                t!("historial-valor-nuevo", campo = c.etiqueta.clone()),
            );
            view! {
                <tr>
                    <th scope="row">{campo}</th>
                    <td>{valor(&c.antes, &antes)}</td>
                    <td>{valor(&c.despues, &despues)}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <div class="tabla-scroll" tabindex="0">
            <table class="tabla">
                <thead>
                    <tr>
                        <th scope="col">{t!("biblioteca-col-campo")}</th>
                        <th scope="col">{t!("biblioteca-col-antes")}</th>
                        <th scope="col">{t!("biblioteca-col-despues")}</th>
                    </tr>
                </thead>
                <tbody>{filas}</tbody>
            </table>
        </div>
    }
    .into_any()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fase {
    Revisar,
    /// `confirmar_cambio` en marcha: no se puede cerrar.
    Confirmando,
    Seguimiento(i64),
}

/// Diálogo de un cambio preparado: diff (secretos enmascarados), riesgo y
/// vigilancia, palabra de confirmación si es una acción, «Aplicar» y el
/// seguimiento del estado (cada 2 s) hasta aplicado, revertido o fallido.
#[component]
pub fn DialogoCambio(
    preparado: PreparadoVista,
    /// Se llama al cerrarse el diálogo.
    #[prop(into)]
    on_cerrar: Callback<()>,
    /// Se llama una vez, cuando el cambio llega a un estado final.
    #[prop(into, optional)]
    on_terminado: Option<Callback<EstadoDeCambio>>,
    /// Cambio que se deshace (cambia el título y el botón).
    #[prop(optional)]
    deshace: Option<i64>,
) -> impl IntoView {
    let dlg = NodeRef::<html::Dialog>::new();
    abrir_modal(dlg);
    let fase = RwSignal::new(Fase::Revisar);
    let error = RwSignal::new(None::<String>);
    let palabra = RwSignal::new(String::new());
    let cambio = RwSignal::new(None::<CambioVista>);
    let terminado =
        Memo::new(move |_| cambio.with(|c| c.as_ref().is_some_and(|c| c.estado.final_())));
    let avisado = StoredValue::new(false);
    let p = StoredValue::new(preparado.clone());

    let aplicar = move |ev: SubmitEvent| {
        ev.prevent_default();
        if fase.get_untracked() != Fase::Revisar {
            return;
        }
        error.set(None);
        fase.set(Fase::Confirmando);
        let token = p.with_value(|p| p.token.clone());
        let palabra = Some(palabra.get_untracked()).filter(|p| !p.trim().is_empty());
        spawn_local(async move {
            match confirmar_cambio(token, palabra).await {
                Ok(id) => fase.set(Fase::Seguimiento(id)),
                Err(e) => {
                    error.set(Some(texto_error(&e)));
                    fase.set(Fase::Revisar);
                }
            }
        });
    };

    // Seguimiento: ahora y cada 2 s hasta un estado final.
    Effect::new(move |_| {
        let Fase::Seguimiento(id) = fase.get() else {
            return;
        };
        if terminado.get() {
            return;
        }
        let consultar = move || {
            spawn_local(async move {
                match estado_cambio(id).await {
                    Ok(c) => {
                        let e = c.estado;
                        cambio.set(Some(c));
                        if e.final_() && !avisado.get_value() {
                            avisado.set_value(true);
                            if let Some(cb) = on_terminado {
                                cb.run(e);
                            }
                        }
                    }
                    Err(e) => error.set(Some(texto_error(&e))),
                }
            });
        };
        consultar();
        let h = set_interval_with_handle(consultar, SONDEO).ok();
        on_cleanup(move || {
            if let Some(h) = h {
                h.clear();
            }
        });
    });

    let cerrar = move || {
        if fase.get_untracked() != Fase::Confirmando
            && let Some(d) = dlg.get_untracked()
        {
            d.close();
        }
    };

    let titulo = match (deshace, preparado.borrar, preparado.palabra.is_some()) {
        (Some(n), ..) => t!("biblioteca-dialogo-deshacer", n = n),
        (_, true, _) => t!("biblioteca-dialogo-borrar"),
        (_, _, true) => t!("biblioteca-dialogo-accion"),
        _ => t!("biblioteca-dialogo-revisar"),
    };
    let instancia = preparado
        .instancia
        .clone()
        .map(|i| t!("biblioteca-dialogo-instancia", i = i))
        .unwrap_or_default();
    let vigilancia = duracion(preparado.vigilancia_s);
    let sin_vigilancia = preparado.vigilancia_s == 0;
    let nota = match (preparado.palabra.is_some(), sin_vigilancia) {
        (true, false) => t!("biblioteca-nota-accion", vigilancia = vigilancia.as_str()),
        (true, true) => t!("biblioteca-nota-accion-sin-vigilancia"),
        (false, false) => t!("biblioteca-nota-cambio", vigilancia = vigilancia.as_str()),
        (false, true) => t!("biblioteca-nota-cambio-sin-vigilancia"),
    };
    let riesgo = if sin_vigilancia {
        t!(
            "biblioteca-riesgo-sin-vigilancia",
            riesgo = preparado.riesgo.etiqueta()
        )
    } else {
        t!(
            "biblioteca-riesgo-vigilancia",
            riesgo = preparado.riesgo.etiqueta(),
            vigilancia = vigilancia.as_str()
        )
    };
    let campo_palabra = preparado.palabra.clone().map(|w| {
        let w2 = w.clone();
        view! {
            <p class="campo">
                <label for="dialogo-palabra">
                    {t!("biblioteca-escribe")}
                    " "
                    <strong class="mono" lang="es">{w}</strong>
                    " "
                    {t!("biblioteca-para-confirmar")}
                </label>
                <input
                    id="dialogo-palabra"
                    type="text"
                    autocomplete="off"
                    autocapitalize="characters"
                    spellcheck="false"
                    required
                    disabled=move || fase.get() != Fase::Revisar
                    prop:value=palabra
                    on:input=move |ev| palabra.set(event_target_value(&ev))
                    aria-describedby="dialogo-palabra-ayuda"
                />
                <span class="solo-lector" id="dialogo-palabra-ayuda">
                    {t!("biblioteca-escribe-mayusculas", palabra = w2)}
                </span>
            </p>
        }
    });

    let estado = move || {
        let texto = match (fase.get(), cambio.get()) {
            (Fase::Revisar, _) => return None,
            (Fase::Confirmando, _) => view! {
                <p class="estado-cambio">{t!("biblioteca-aplicando-comprobando")}</p>
            }
            .into_any(),
            (Fase::Seguimiento(id), None) => view! {
                <p class="estado-cambio">{t!("biblioteca-cambio-comprobando", id = id)}</p>
            }
            .into_any(),
            (Fase::Seguimiento(id), Some(c)) => {
                let detalle = match c.estado {
                    EstadoDeCambio::Vigilando => Some(match c.restante_s {
                        Some(s) => t!("biblioteca-vigilando-quedan", resto = duracion(s as u64)),
                        None => t!("biblioteca-vigilando"),
                    }),
                    EstadoDeCambio::Aplicado => Some(
                        c.nota
                            .clone()
                            .map_or(t!("biblioteca-aplicado-verificado"), |n| {
                                t!("biblioteca-aplicado-nota", nota = n)
                            }),
                    ),
                    EstadoDeCambio::Revertido
                    | EstadoDeCambio::Fallido
                    | EstadoDeCambio::RevertidoParcial => c.error.clone().or(c.nota.clone()),
                    _ => None,
                };
                let estado = match c.estado {
                    EstadoDeCambio::Vigilando | EstadoDeCambio::Aplicando => EstadoVisual::Proceso,
                    e => e.estado(),
                };
                view! {
                    <div class="estado-cambio">
                        <Insignia estado texto=t!("biblioteca-cambio-estado", id = id, estado = c.estado.etiqueta()) />
                        {detalle.map(|d| view! { <p>{d}</p> })}
                    </div>
                }
                .into_any()
            }
        };
        Some(texto)
    };

    let pie = move || match fase.get() {
        Fase::Seguimiento(_) => view! {
            <button class="b-pri" type="button" on:click=move |_| cerrar()>
                {t!("biblioteca-cerrar")}
            </button>
        }
        .into_any(),
        f => view! {
            <button
                class="b-sec"
                type="button"
                disabled=f == Fase::Confirmando
                on:click=move |_| cerrar()
            >
                {t!("biblioteca-cancelar")}
            </button>
            <button class="b-pri" type="submit" disabled=f == Fase::Confirmando>
                {if f == Fase::Confirmando {
                    t!("biblioteca-aplicando")
                } else if deshace.is_some() {
                    t!("biblioteca-deshacer")
                } else if p.with_value(|p| p.borrar) {
                    t!("biblioteca-borrar")
                } else {
                    t!("biblioteca-aplicar")
                }}
            </button>
        }
        .into_any(),
    };

    view! {
        <dialog
            class="dialogo"
            node_ref=dlg
            aria-labelledby="dialogo-titulo"
            aria-modal="true"
            on:cancel=move |ev: leptos::ev::Event| {
                if fase.get_untracked() == Fase::Confirmando {
                    ev.prevent_default();
                }
            }
            on:close=move |_| on_cerrar.run(())
        >
            <form class="dialogo-cuerpo" on:submit=aplicar>
                <header class="dialogo-cabecera">
                    <h2 id="dialogo-titulo">{titulo}</h2>
                    <Insignia estado=preparado.riesgo.estado() texto=riesgo />
                </header>
                <p>
                    <span class="mono">{preparado.funcion.clone()}</span>
                    {instancia}
                </p>
                <TablaDiff campos=preparado.campos.clone() />
                {preparado.aviso.clone().map(|a| view! {
                    <p class="aviso estado-aviso" role="status">
                        <IconoEstado estado=Estado::Aviso />
                        <span>{a}</span>
                    </p>
                })}
                <p class="pagina-sub">{nota}</p>
                {campo_palabra}
                <p class="error-formulario" role="alert">
                    {move || error.get()}
                </p>
                <div class="dialogo-estado" aria-live="polite">
                    {estado}
                </div>
                <div class="dialogo-pie">{pie}</div>
            </form>
        </dialog>
    }
}
