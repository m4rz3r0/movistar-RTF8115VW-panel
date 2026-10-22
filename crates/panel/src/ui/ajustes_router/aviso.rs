//! El cambio en curso de la rejilla: la insignia de lo que cambia y el aviso
//! con «Deshacer».
//!
//! Lo que se aplica desde una tarjeta, la barra o el detalle pasa por aquí:
//! mientras se confirma, la insignia de cada elemento afectado dice
//! «Aplicando…»; mientras el panel vigila el router, «Comprobando…». Al
//! terminar, la página vuelve a leer el tema y el aviso dice cómo acabó,
//! con «Deshacer» si se aplicó. Si falla (otro cambio en curso, el router
//! no lo hizo), la insignia vuelve a su estado y el aviso da el motivo.

use super::detalle::Borradores;
use crate::ajustes_router::{GrupoVista, confirmar_ajuste, deshacer_ajuste};
use crate::t;
use crate::ui::comun::EstadoVisual;
use crate::ui::explorador::{CambioVista, EstadoDeCambio, estado_cambio, texto_error};
use crate::ui::grupo::{DialogoGrupo, texto_valor};
use crate::ui::i18n::traducir;
use leptos::{prelude::*, task::spawn_local};
use std::time::Duration;

/// Cada cuánto se pregunta por el estado del cambio.
const SONDEO: Duration = Duration::from_secs(2);

/// El cambio que sigue la página.
#[derive(Clone, Debug, PartialEq)]
struct CambioActivo {
    /// `None` mientras se confirma.
    id: Option<i64>,
    /// Elementos que cambian (su insignia lo dice).
    elementos: Vec<String>,
    /// Lo que cambia, en una línea («Red WiFi 5 GHz: Red encendida,
    /// Apagada → Encendida»).
    texto: String,
    /// El interruptor pulsado y su valor nuevo, que se ve mientras tanto.
    interruptor: Option<(String, bool)>,
    /// Claves de los campos que escribe (`ajuste-wifi-red_24-nombre`).
    campos: Vec<String>,
    estado: Option<CambioVista>,
}

impl CambioActivo {
    fn terminado(&self) -> bool {
        self.estado.as_ref().is_some_and(|c| c.estado.final_())
    }
}

/// El cambio en curso de la página y el diálogo de revisión abierto. Vive
/// fuera de la lectura del tema (sobrevive a la relectura al terminar).
#[derive(Clone, Copy)]
pub(crate) struct Seguimiento {
    activo: RwSignal<Option<CambioActivo>>,
    error: RwSignal<Option<String>>,
    /// Grupo preparado a la espera de que se revise, con los elementos que
    /// cambia y si deshace otro cambio.
    dialogo: RwSignal<Option<(GrupoVista, Vec<String>, bool)>>,
    recargar: Callback<()>,
    /// Lo editado en cada detalle: el de lo que se aplica se olvida.
    borradores: Borradores,
}

impl Seguimiento {
    /// Sigue los cambios de la página; `recargar` vuelve a leer el tema
    /// cuando uno termina. Pregunta por el estado cada [`SONDEO`] mientras
    /// no termine; al desmontarse, deja de preguntar.
    pub(crate) fn nuevo(recargar: Callback<()>, borradores: Borradores) -> Seguimiento {
        let s = Seguimiento {
            activo: RwSignal::new(None),
            error: RwSignal::new(None),
            dialogo: RwSignal::new(None),
            recargar,
            borradores,
        };
        let vigilado = Memo::new(move |_| {
            s.activo
                .with(|a| a.as_ref().filter(|a| !a.terminado()).and_then(|a| a.id))
        });
        Effect::new(move |_| {
            let Some(id) = vigilado.get() else {
                return;
            };
            let consultar = move || {
                spawn_local(async move {
                    match estado_cambio(id).await {
                        Ok(c) => s.al_saber(id, c),
                        Err(e) => {
                            s.error.try_set(Some(texto_error(&e)));
                        }
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
        s
    }

    /// Llega el estado `c` del cambio `id`; si ha terminado, la página lo
    /// vuelve a leer. Si se aplicó, lo editado en los campos que escribió ya
    /// está en el router: se olvida (una clave que el router no deja leer
    /// seguiría escrita, como un cambio sin guardar). Lo demás editado
    /// sigue.
    fn al_saber(self, id: i64, c: CambioVista) {
        let terminado = c.estado.final_();
        let aplicado = c.estado == EstadoDeCambio::Aplicado;
        let mismo = self.activo.try_update(|a| match a {
            Some(a) if a.id == Some(id) => {
                a.estado = Some(c);
                Some(a.campos.clone())
            }
            _ => None,
        });
        if let Some(Some(campos)) = mismo
            && terminado
        {
            if aplicado {
                self.borradores.olvidar_campos(&campos);
            }
            self.recargar.run(());
        }
    }

    /// Antes de preparar un cambio sin diálogo (un interruptor): la
    /// insignia ya dice «Aplicando…» y el interruptor, su valor nuevo.
    pub(crate) fn empieza(self, elemento: &str, interruptor: Option<bool>, texto: String) {
        self.error.set(None);
        self.activo.set(Some(CambioActivo {
            id: None,
            elementos: vec![elemento.to_owned()],
            texto,
            interruptor: interruptor.map(|v| (elemento.to_owned(), v)),
            campos: Vec::new(),
            estado: None,
        }));
    }

    /// ¿Hay un cambio aplicado que aún no ha terminado? Entonces otro no
    /// lo sustituye: el servidor lo rechaza (409) y el aviso lo dice.
    pub(crate) fn ocupado(self) -> bool {
        self.activo
            .with_untracked(|a| a.as_ref().is_some_and(|a| a.id.is_some() && !a.terminado()))
    }

    /// No se pudo: lo que se estaba confirmando vuelve a como estaba (el
    /// cambio que ya se sigue, no) y el aviso da el motivo.
    pub(crate) fn falla(self, motivo: String) {
        self.activo.update(|a| {
            if a.as_ref().is_some_and(|a| a.id.is_none()) {
                *a = None;
            }
        });
        self.error.set(Some(motivo));
    }

    /// Confirma el grupo preparado (`token`, sin los pasos `desmarcados`)
    /// que cambia `elementos`, y lo sigue.
    pub(crate) fn aplicar(self, g: &GrupoVista, desmarcados: Vec<usize>, elementos: Vec<String>) {
        let interruptor = self.activo.with_untracked(|a| {
            a.as_ref()
                .filter(|a| a.id.is_none())
                .and_then(|a| a.interruptor.clone())
        });
        self.error.set(None);
        self.activo.set(Some(CambioActivo {
            id: None,
            elementos,
            texto: resumen(g),
            interruptor,
            campos: g
                .pasos
                .iter()
                .filter(|p| !p.sin_cambios && !desmarcados.contains(&p.indice))
                .flat_map(|p| p.campos.iter().map(|c| c.campo.clone()))
                .collect(),
            estado: None,
        }));
        let token = g.token.clone();
        spawn_local(async move {
            match confirmar_ajuste(token, desmarcados).await {
                Ok(id) => {
                    self.activo.try_update(|a| {
                        if let Some(a) = a {
                            a.id = Some(id);
                        }
                    });
                }
                Err(e) => self.falla(texto_error(&e)),
            }
        });
    }

    /// Abre el diálogo de revisión del grupo `g`, que cambia `elementos`.
    pub(crate) fn revisar(self, g: GrupoVista, elementos: Vec<String>) {
        self.error.set(None);
        self.dialogo.set(Some((g, elementos, false)));
    }

    /// Insignia de `elemento` mientras cambia. Reactivo.
    pub(crate) fn insignia(self, elemento: &str) -> Option<(EstadoVisual, String)> {
        self.activo.with(|a| {
            let a = a
                .as_ref()
                .filter(|a| a.elementos.iter().any(|e| e == elemento))?;
            match a.estado.as_ref().map(|c| c.estado) {
                None | Some(EstadoDeCambio::Preparado | EstadoDeCambio::Aplicando) => {
                    Some((EstadoVisual::Proceso, t!("ajustes-insignia-aplicando")))
                }
                Some(EstadoDeCambio::Vigilando) => {
                    Some((EstadoVisual::Proceso, t!("ajustes-insignia-comprobando")))
                }
                Some(_) => None,
            }
        })
    }

    /// Valor del interruptor de `elemento` mientras cambia. Reactivo.
    pub(crate) fn interruptor(self, elemento: &str) -> Option<bool> {
        self.activo.with(|a| {
            let a = a.as_ref().filter(|a| !a.terminado())?;
            let (e, v) = a.interruptor.as_ref()?;
            (e == elemento).then_some(*v)
        })
    }
}

/// Lo que cambia un grupo, en una línea: el campo con su antes y después si
/// es uno solo; si no, cuántos pasos cambian.
pub(crate) fn resumen(g: &GrupoVista) -> String {
    let titulo = traducir(&g.titulo, None);
    let pasos: Vec<_> = g.pasos.iter().filter(|p| !p.sin_cambios).collect();
    match pasos.as_slice() {
        [p] if p.campos.len() == 1 && !p.campos[0].secreto => {
            let c = &p.campos[0];
            t!(
                "ajustes-aviso-cambio",
                titulo = titulo,
                campo = traducir(&c.campo, None),
                antes = texto_valor(c, &c.antes),
                despues = texto_valor(c, &c.despues)
            )
        }
        _ => t!("ajustes-aviso-varios", titulo = titulo, n = pasos.len()),
    }
}

/// El diálogo de revisión abierto, que solo revisa: «Aplicar» lo cierra y
/// la página sigue el cambio.
#[component]
pub(crate) fn DialogoDeLaPagina() -> impl IntoView {
    let s = expect_context::<Seguimiento>();
    move || {
        s.dialogo.get().map(|(g, elementos, deshacer)| {
            let grupo = g.clone();
            view! {
                <DialogoGrupo
                    grupo=g
                    deshacer
                    on_cerrar=move |_| s.dialogo.set(None)
                    al_aplicar=move |(_, desmarcados): (String, Vec<usize>)| {
                        s.aplicar(&grupo, desmarcados, elementos.clone())
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
        preparando.set(true);
        spawn_local(async move {
            match deshacer_ajuste(id).await {
                Ok(g) => {
                    s.error.set(None);
                    s.dialogo.set(Some((g, elementos, true)));
                }
                Err(e) => s.error.set(Some(texto_error(&e))),
            }
            preparando.try_set(false);
        });
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
    }
}
