//! El aviso de los cambios en curso (con sus botones) y el diálogo de revisión de la página.

use super::*;
use crate::ui::panel_ajustes::{DialogoReautenticar, Reautenticacion, repetible};
use crate::ui::vigilancia::BotonesEnCurso;

/// El diálogo de revisión abierto, que solo revisa: «Aplicar» lo cierra y
/// la página sigue el cambio.
#[component]
pub(crate) fn DialogoDeLaPagina() -> impl IntoView {
    let s = expect_context::<Seguimiento>();
    move || {
        s.dialogo.get().map(|r| {
            let Revision {
                grupo,
                elementos,
                deshacer,
                cerrar,
                extra,
            } = r;
            let g = grupo.clone();
            let nuevo_grupo = extra.as_ref().map(|e| e.nuevo_grupo);
            view! {
                <DialogoGrupo
                    grupo=g
                    deshacer
                    antes_de_pasos=extra.as_ref().map(|e| e.antes_de_pasos.clone())
                    nuevo_grupo
                    ocupado=extra.as_ref().map(|e| e.ocupado)
                    on_cerrar=move |_| s.dialogo.set(None)
                    al_aplicar=move |(_, desmarcados): (String, Vec<usize>)| {
                        // Se aplica el último preparado (otra opción elegida).
                        let ultimo = nuevo_grupo.and_then(|n| n.get_untracked());
                        s.aplicar(ultimo.as_ref().unwrap_or(&grupo), desmarcados, elementos.clone(), cerrar)
                    }
                />
            }
        })
    }
}

/// Texto del estado de un cambio en el aviso.
fn estado_texto(a: &CambioActivo) -> String {
    match a.estado.as_ref().map(|c| c.estado) {
        None | Some(EstadoDeCambio::Preparado | EstadoDeCambio::Aplicando) => {
            t!("ajustes-insignia-aplicando")
        }
        // Con lo que le queda: el cambio ya está en el router, pero aún no
        // se da por bueno (y la lista, hasta entonces, no se vuelve a leer).
        Some(EstadoDeCambio::Vigilando) => match a.estado.as_ref().and_then(|c| c.restante_s) {
            Some(s) => t!(
                "ajustes-aviso-comprobando",
                resto = crate::ui::explorador::duracion(s.max(0) as u64)
            ),
            None => t!("ajustes-insignia-comprobando"),
        },
        Some(EstadoDeCambio::Aplicado) => t!("ajustes-grupo-estado-aplicado"),
        Some(EstadoDeCambio::Revertido) => t!("ajustes-grupo-estado-revertido"),
        Some(EstadoDeCambio::Fallido) => t!("ajustes-grupo-estado-fallido"),
        Some(EstadoDeCambio::RevertidoParcial) => t!("ajustes-grupo-estado-parcial"),
        Some(EstadoDeCambio::EnCola) => t!(
            "ajustes-grupo-estado-en-cola",
            puesto = a.estado.as_ref().and_then(|c| c.puesto).unwrap_or(1)
        ),
        Some(EstadoDeCambio::Cancelado) => t!("ajustes-grupo-estado-cancelado"),
    }
}

/// El aviso de los cambios en curso (o de los últimos), uno por línea: con
/// «Todo bien, terminar» mientras se vigila, «Quitar de la cola» mientras
/// espera y «Deshacer» cuando se ha aplicado; y el de un fallo.
#[component]
pub(crate) fn AvisoCambio() -> impl IntoView {
    let s = expect_context::<Seguimiento>();
    let preparando = RwSignal::new(false);
    // Deshacer un alta o una baja de usuarios del router pide la contraseña
    // del panel y se repite al darla.
    let (reauth, dialogo_reauth) = Reautenticacion::de_la_pagina();
    let deshacer_con = move |id: i64, elementos: Vec<String>, yo: Callback<()>| {
        preparando.set(true);
        spawn_local(async move {
            match deshacer_ajuste(id).await {
                Ok(g) => {
                    s.error.set(None);
                    s.dialogo.set(Some(Revision {
                        grupo: g,
                        elementos,
                        deshacer: true,
                        cerrar: false,
                        extra: None,
                    }));
                }
                Err(e) => reauth.tratar(&e, s.error, yo),
            }
            preparando.try_set(false);
        });
    };
    let deshacer = move |id: i64, elementos: Vec<String>| {
        if preparando.get_untracked() {
            return;
        }
        repetible(move |yo| deshacer_con(id, elementos.clone(), yo)).run(());
    };
    let linea = move |a: CambioActivo| {
        let aplicado = a
            .estado
            .as_ref()
            .is_some_and(|c| c.estado == EstadoDeCambio::Aplicado);
        let terminado = a.terminado();
        let motivo = a.estado.as_ref().and_then(|c| c.error.clone());
        let botones = a.estado.as_ref().map(|c| (c.id, c.terminable, c.quitable));
        let clave = a.clave;
        let elementos = a.elementos.clone();
        let id = a.id;
        view! {
            <div class="aviso-cambio" class:aviso-cambio-hecho=terminado>
                <p>
                    <strong>{a.texto.clone()}</strong>
                    " "
                    <span>{estado_texto(&a)}</span>
                    {motivo.map(|m| view! { " " <span>{m}</span> })}
                </p>
                {botones.map(|(id, terminable, quitable)| view! {
                    <BotonesEnCurso
                        id
                        terminable
                        quitable
                        al_cambiar=move |c: CambioVista| s.al_saber(id, c)
                        error=s.error
                    />
                })}
                {id.filter(|_| aplicado).map(|id| view! {
                    <button
                        type="button"
                        class="b-sec"
                        aria-disabled=move || preparando.get().then_some("true")
                        on:click=move |_| deshacer(id, elementos.clone())
                    >
                        {move || if preparando.get() { t!("ajustes-grupo-preparando") } else { t!("ajustes-grupo-deshacer") }}
                    </button>
                })}
                {terminado.then(|| view! {
                    <button
                        type="button"
                        class="b-txt"
                        aria-label=t!("ajustes-aviso-cerrar")
                        on:click=move |_| s.cambios.update(|l| l.retain(|x| x.clave != clave))
                    >
                        "✕"
                    </button>
                })}
            </div>
        }
    };
    // Solo se pintan de nuevo las líneas que cambian.
    let aviso = move || {
        view! {
            <For each=move || s.cambios.get() key=|a| format!("{a:?}") let:a>
                {linea(a)}
            </For>
        }
    };
    let error = move || {
        s.error.get().map(|m| {
            view! {
                <div class="aviso-cambio aviso-cambio-error">
                    <p>{m}</p>
                    <button
                        type="button"
                        class="b-txt"
                        aria-label=t!("ajustes-aviso-cerrar")
                        on:click=move |_| s.error.set(None)
                    >
                        "✕"
                    </button>
                </div>
            }
        })
    };
    view! {
        <div class="avisos-cambio">
            <div role="status" aria-live="polite">{aviso}</div>
            <div role="alert">{error}</div>
        </div>
        <DialogoDeLaPagina />
        {move || {
            (dialogo_reauth && reauth.abierto()).then(|| view! { <DialogoReautenticar /> })
        }}
    }
}
