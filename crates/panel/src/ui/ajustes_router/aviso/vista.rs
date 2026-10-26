//! El aviso del cambio en curso (con «Deshacer») y el diálogo de revisión de la página.

use super::*;
use crate::ui::panel_ajustes::{DialogoReautenticar, Reautenticacion, repetible};

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
        Some(EstadoDeCambio::Vigilando) => t!("ajustes-insignia-comprobando"),
        Some(EstadoDeCambio::Aplicado) => t!("ajustes-grupo-estado-aplicado"),
        Some(EstadoDeCambio::Revertido) => t!("ajustes-grupo-estado-revertido"),
        Some(EstadoDeCambio::Fallido) => t!("ajustes-grupo-estado-fallido"),
        Some(EstadoDeCambio::RevertidoParcial) => t!("ajustes-grupo-estado-parcial"),
    }
}

/// El aviso del cambio en curso (o del último), con «Deshacer» cuando se
/// ha aplicado, y el de un fallo.
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
    let deshacer = move |_| {
        let Some((id, elementos)) = s
            .activo
            .with_untracked(|a| a.as_ref().and_then(|a| Some((a.id?, a.elementos.clone()))))
        else {
            return;
        };
        if preparando.get_untracked() {
            return;
        }
        repetible(move |yo| deshacer_con(id, elementos.clone(), yo)).run(());
    };
    let aviso = move || {
        s.activo.get().map(|a| {
            let aplicado = a
                .estado
                .as_ref()
                .is_some_and(|c| c.estado == EstadoDeCambio::Aplicado);
            let terminado = a.terminado();
            let motivo = a.estado.as_ref().and_then(|c| c.error.clone());
            view! {
                <div class="aviso-cambio" class:aviso-cambio-hecho=terminado>
                    <p>
                        <strong>{a.texto.clone()}</strong>
                        " "
                        <span>{estado_texto(&a)}</span>
                        {motivo.map(|m| view! { " " <span>{m}</span> })}
                    </p>
                    {aplicado.then(|| view! {
                        <button
                            type="button"
                            class="b-sec"
                            aria-disabled=move || preparando.get().then_some("true")
                            on:click=deshacer
                        >
                            {move || if preparando.get() { t!("ajustes-grupo-preparando") } else { t!("ajustes-grupo-deshacer") }}
                        </button>
                    })}
                    {terminado.then(|| view! {
                        <button
                            type="button"
                            class="b-txt"
                            aria-label=t!("ajustes-aviso-cerrar")
                            on:click=move |_| s.activo.set(None)
                        >
                            "✕"
                        </button>
                    })}
                </div>
            }
        })
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
