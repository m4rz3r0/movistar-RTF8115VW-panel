//! Diálogos modales: revisar y aplicar un cambio (también lo usa el
//! historial para deshacer) y revelar un secreto.
//!
//! Son `<dialog>` nativos abiertos con `showModal()`: el resto de la página
//! queda inerte (el foco no sale del diálogo), Esc lo cierra salvo mientras
//! se está aplicando y, al cerrarse, el foco vuelve a donde estaba.

use super::{
    CambioVista, EstadoDeCambio, MASCARA, PreparadoVista, Valor, confirmar_cambio, duracion,
    estado_cambio, revelar, texto_error,
};
use crate::ui::{Estado, Insignia};
use leptos::{ev::SubmitEvent, html, prelude::*, task::spawn_local};
use std::time::Duration;

/// Cada cuánto se pregunta por el estado de un cambio.
const SONDEO: Duration = Duration::from_secs(2);

/// Abre el `<dialog>` como modal en cuanto existe en el DOM.
fn abrir_modal(dlg: NodeRef<html::Dialog>) {
    Effect::new(move |_| {
        if let Some(d) = dlg.get()
            && !d.open()
        {
            let _ = d.show_modal();
        }
    });
}

fn valor(v: &Valor) -> AnyView {
    match v {
        Valor::Texto(t) => view! { <span class="mono">{t.clone()}</span> }.into_any(),
        Valor::Secreto => view! {
            <span class="secreto">{MASCARA}</span>
            <span class="solo-lector">"(valor oculto)"</span>
        }
        .into_any(),
        Valor::Ausente => view! {
            <span class="vacia" aria-hidden="true">"—"</span>
            <span class="solo-lector">"sin valor"</span>
        }
        .into_any(),
    }
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

    let titulo = match (preparado.borrar, preparado.palabra.is_some()) {
        (true, _) => "Borrar la instancia",
        (_, true) => "Ejecutar la acción",
        _ => "Revisar cambios",
    };
    let instancia = preparado
        .instancia
        .clone()
        .map(|i| format!(", instancia {i}"))
        .unwrap_or_default();
    let filas = preparado
        .campos
        .iter()
        .map(|c| {
            view! {
                <tr>
                    <th scope="row">
                        {c.etiqueta.clone()}
                        <span class="mono campo-nombre">{c.nombre.clone()}</span>
                    </th>
                    <td>{valor(&c.antes)}</td>
                    <td>{valor(&c.despues)}</td>
                </tr>
            }
        })
        .collect_view();
    let diff = if preparado.campos.is_empty() {
        view! { <p class="pagina-sub">"Sin campos que comparar."</p> }.into_any()
    } else {
        view! {
            <div class="tabla-scroll">
                <table class="tabla">
                    <thead>
                        <tr>
                            <th scope="col">"Campo"</th>
                            <th scope="col">"Antes"</th>
                            <th scope="col">"Después"</th>
                        </tr>
                    </thead>
                    <tbody>{filas}</tbody>
                </table>
            </div>
        }
        .into_any()
    };
    let vigilancia = duracion(preparado.vigilancia_s);
    let nota = if preparado.palabra.is_some() {
        format!(
            "Una acción no se puede revertir. Tras ejecutarla, el panel comprueba el router durante {vigilancia}."
        )
    } else {
        format!(
            "Tras aplicarlo, el panel vigila el router durante {vigilancia}: si deja de responder o se cae internet, vuelve a los valores anteriores."
        )
    };
    let riesgo = format!("{} · vigilancia {vigilancia}", preparado.riesgo.etiqueta());
    let campo_palabra = preparado.palabra.clone().map(|w| {
        let w2 = w.clone();
        view! {
            <p class="campo">
                <label for="dialogo-palabra">
                    "Escribe " <strong class="mono">{w}</strong> " para confirmar"
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
                    {format!("Escribe {w2} en mayúsculas")}
                </span>
            </p>
        }
    });

    let estado = move || {
        let texto = match (fase.get(), cambio.get()) {
            (Fase::Revisar, _) => return None,
            (Fase::Confirmando, _) => view! {
                <p class="estado-cambio">"Aplicando el cambio y comprobando que el router lo muestra…"</p>
            }
            .into_any(),
            (Fase::Seguimiento(id), None) => view! {
                <p class="estado-cambio">{format!("Cambio nº {id}: comprobando…")}</p>
            }
            .into_any(),
            (Fase::Seguimiento(id), Some(c)) => {
                let detalle = match c.estado {
                    EstadoDeCambio::Vigilando => Some(match c.restante_s {
                        Some(s) => format!("Vigilando el router: quedan {}.", duracion(s as u64)),
                        None => "Vigilando el router…".to_owned(),
                    }),
                    EstadoDeCambio::Aplicado => Some(
                        c.nota
                            .clone()
                            .map_or("Aplicado y verificado.".to_owned(), |n| format!("Aplicado ({n}).")),
                    ),
                    EstadoDeCambio::Revertido | EstadoDeCambio::Fallido => c.error.clone().or(c.nota.clone()),
                    _ => None,
                };
                let estado = match c.estado {
                    EstadoDeCambio::Vigilando | EstadoDeCambio::Aplicando => Estado::Neutro,
                    e => e.estado(),
                };
                view! {
                    <div class="estado-cambio">
                        <Insignia estado texto=format!("Cambio nº {id}: {}", c.estado.etiqueta()) />
                        {detalle.map(|d| view! { <p>{d}</p> })}
                    </div>
                }
                .into_any()
            }
        };
        Some(texto)
    };

    let pie = move || {
        match fase.get() {
        Fase::Seguimiento(_) => view! {
            <button class="boton" type="button" on:click=move |_| cerrar()>
                "Cerrar"
            </button>
        }
        .into_any(),
        f => view! {
            <button
                class="boton-sutil"
                type="button"
                disabled=f == Fase::Confirmando
                on:click=move |_| cerrar()
            >
                "Cancelar"
            </button>
            <button class="boton" type="submit" disabled=f == Fase::Confirmando>
                {if f == Fase::Confirmando { "Aplicando…" } else if p.with_value(|p| p.borrar) { "Borrar" } else { "Aplicar" }}
            </button>
        }
        .into_any(),
    }
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
                {diff}
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

/// Diálogo para revelar un secreto: pide la contraseña del panel.
#[component]
pub(super) fn DialogoRevelar(
    funcion: String,
    campo: String,
    instancia: Option<String>,
    params: Vec<(String, String)>,
    #[prop(into)] on_cerrar: Callback<()>,
    #[prop(into)] on_revelado: Callback<String>,
) -> impl IntoView {
    let dlg = NodeRef::<html::Dialog>::new();
    abrir_modal(dlg);
    let contrasena = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let pidiendo = RwSignal::new(false);
    let datos = StoredValue::new((funcion, campo.clone(), instancia, params));

    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        if pidiendo.get_untracked() {
            return;
        }
        error.set(None);
        pidiendo.set(true);
        let (f, c, i, p) = datos.get_value();
        let clave = contrasena.get_untracked();
        spawn_local(async move {
            match revelar(f, c, i, p, clave).await {
                Ok(v) => {
                    contrasena.set(String::new());
                    on_revelado.run(v);
                    if let Some(d) = dlg.get_untracked() {
                        d.close();
                    }
                }
                Err(e) => {
                    contrasena.set(String::new());
                    error.set(Some(texto_error(&e)));
                }
            }
            pidiendo.set(false);
        });
    };
    let cerrar = move |_| {
        if let Some(d) = dlg.get_untracked() {
            d.close();
        }
    };

    view! {
        <dialog
            class="dialogo dialogo-estrecho"
            node_ref=dlg
            aria-labelledby="revelar-titulo"
            aria-modal="true"
            on:close=move |_| on_cerrar.run(())
        >
            <form class="dialogo-cuerpo" on:submit=enviar>
                <header class="dialogo-cabecera">
                    <h2 id="revelar-titulo">"Revelar un secreto"</h2>
                </header>
                <p>
                    "Para ver " <span class="mono">{campo}</span>
                    " escribe la contraseña del panel. Se mostrará 30 s y quedará anotado en el historial (sin el valor)."
                </p>
                <p class="campo">
                    <label for="revelar-contrasena">"Contraseña del panel"</label>
                    <input
                        id="revelar-contrasena"
                        type="password"
                        autocomplete="current-password"
                        required
                        autofocus
                        prop:value=contrasena
                        on:input=move |ev| contrasena.set(event_target_value(&ev))
                    />
                </p>
                <p class="error-formulario" role="alert">
                    {move || error.get()}
                </p>
                <div class="dialogo-pie">
                    <button class="boton-sutil" type="button" on:click=cerrar>
                        "Cancelar"
                    </button>
                    <button class="boton" type="submit" disabled=move || pidiendo.get()>
                        {move || if pidiendo.get() { "Comprobando…" } else { "Revelar" }}
                    </button>
                </div>
            </form>
        </dialog>
    }
}
