//! El cambio en curso de la rejilla: la insignia de lo que cambia y el aviso
//! con «Deshacer».
//!
//! Lo que se aplica desde una tarjeta, la barra o el detalle pasa por aquí:
//! mientras se confirma, la insignia de cada elemento afectado dice
//! «Aplicando…»; mientras el panel vigila el router, «Comprobando…». Al
//! terminar, la página vuelve a leer el tema y el aviso dice cómo acabó,
//! con «Deshacer» si se aplicó. Si falla (el router no lo hizo, la cola
//! está llena), la insignia vuelve a su estado y el aviso da el motivo.
//!
//! Mientras uno se aplica o se vigila, lo siguiente que se aplica espera en
//! la cola del gestor: el aviso los lista todos, por orden, cada uno con su
//! estado («En cola», «Comprobando…») y sus botones («Todo bien,
//! terminar», «Quitar de la cola», «Deshacer»).

use super::detalle::Borradores;
use super::seleccion::Seleccion;
use super::tras_borrar::{FocoTrasBorrar, antes_de_releer};
use crate::ajustes_router::{GrupoVista, confirmar_ajuste, deshacer_ajuste};
use crate::t;
use crate::ui::comun::EstadoVisual;
use crate::ui::explorador::{CambioVista, EstadoDeCambio, estado_cambio, texto_error};
use crate::ui::grupo::{DialogoGrupo, texto_valor};
use crate::ui::i18n::traducir;
use crate::ui::layout::RecargaMarco;
use leptos::{prelude::*, task::spawn_local};
use std::time::Duration;

mod consultas;
mod lista;
mod vista;

use lista::{CambioActivo, afecta, anadir, filas_de};

pub(crate) use vista::AvisoCambio;

/// Lo que añade a su diálogo una revisión con opciones (el proveedor de DNS
/// de un lote de Mejoras), para [`Seguimiento::revisar_con`].
#[derive(Clone)]
pub(crate) struct ExtraRevision {
    /// Se pinta encima de los pasos (el selector).
    pub antes_de_pasos: ViewFn,
    /// El grupo preparado de nuevo con otra opción: sustituye al revisado.
    pub nuevo_grupo: Signal<Option<GrupoVista>>,
    /// Se está preparando el grupo nuevo: el diálogo no se cierra ni aplica.
    pub ocupado: Signal<bool>,
}

/// Cada cuánto se pregunta por el estado del cambio.
const SONDEO: Duration = Duration::from_secs(2);

/// Un grupo preparado a la espera de que se revise en el diálogo de la página.
#[derive(Clone)]
struct Revision {
    grupo: GrupoVista,
    /// Los elementos que cambia (su insignia, y los que se cierran).
    elementos: Vec<String>,
    /// Deshace otro cambio (título y botón de deshacer).
    deshacer: bool,
    /// Al aplicarse, sus elementos se cierran (una instancia nueva, ya creada).
    cerrar: bool,
    /// Las opciones de la revisión, si las tiene.
    extra: Option<ExtraRevision>,
}

/// El cambio en curso de la página y el diálogo de revisión abierto. Vive
/// fuera de la lectura del tema (sobrevive a la relectura al terminar).
#[derive(Clone, Copy)]
pub(crate) struct Seguimiento {
    /// Los cambios que sigue, por orden: el que se aplica o se vigila, los
    /// que esperan en la cola y los ya terminados que aún se ven.
    cambios: RwSignal<Vec<CambioActivo>>,
    /// La clave del siguiente que se añada.
    claves: StoredValue<u64>,
    error: RwSignal<Option<String>>,
    /// Grupo preparado a la espera de que se revise en el diálogo.
    dialogo: RwSignal<Option<Revision>>,
    recargar: Callback<()>,
    /// El marco (el contador de Mejoras de la barra lateral), que vuelve a
    /// contar cuando termina un cambio.
    marco: Option<RecargaMarco>,
    /// Lo editado en cada detalle: el de lo que se aplica se olvida.
    borradores: Borradores,
    /// Elementos que se acaban de cerrar así: la página deja su dirección.
    cerrados: RwSignal<Vec<String>>,
    /// Lo marcado en la página: lo borrado deja de estarlo.
    seleccion: Seleccion,
    /// A dónde va el foco cuando la página se pinte tras un borrado.
    foco: RwSignal<Option<FocoTrasBorrar>>,
}

impl Seguimiento {
    /// Sigue los cambios de la página; `recargar` vuelve a leer el tema
    /// cuando uno termina. Pregunta por el estado cada [`SONDEO`] mientras
    /// no termine; al desmontarse, deja de preguntar.
    pub(crate) fn nuevo(
        recargar: Callback<()>,
        borradores: Borradores,
        seleccion: Seleccion,
    ) -> Seguimiento {
        let s = Seguimiento {
            seleccion,
            foco: RwSignal::new(None),
            cambios: RwSignal::new(Vec::new()),
            claves: StoredValue::new(0),
            error: RwSignal::new(None),
            dialogo: RwSignal::new(None),
            recargar,
            marco: use_context::<RecargaMarco>(),
            borradores,
            cerrados: RwSignal::new(Vec::new()),
        };
        // Se pregunta por todos los que no han terminado: el que se aplica
        // o se vigila y los que esperan en la cola (su puesto, y si se
        // cancelan).
        let vigilados = Memo::new(move |_| {
            s.cambios.with(|l| {
                l.iter()
                    .filter(|a| !a.terminado())
                    .filter_map(|a| a.id)
                    .collect::<Vec<_>>()
            })
        });
        Effect::new(move |_| {
            let ids = vigilados.get();
            if ids.is_empty() {
                return;
            }
            let consultar = move || {
                for id in ids.clone() {
                    spawn_local(async move {
                        match estado_cambio(id).await {
                            Ok(c) => s.al_saber(id, c),
                            Err(e) => {
                                s.error.try_set(Some(texto_error(&e)));
                            }
                        }
                    });
                }
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
        let mismo = self.cambios.try_update(|l| {
            let a = l.iter_mut().find(|a| a.id == Some(id))?;
            let ya = a.terminado();
            a.estado = Some(c);
            let cerrar = a.cerrar.then(|| (a.elementos.clone(), a.borra));
            // Lo que ya había terminado (y se atendió) no se vuelve a atender.
            (!ya).then(|| (a.campos.clone(), cerrar))
        });
        if let Some(Some((campos, cerrar))) = mismo
            && terminado
        {
            if aplicado {
                self.borradores.olvidar_campos(&campos);
                if let Some((elementos, borra)) = cerrar {
                    for e in &elementos {
                        self.borradores.olvidar(e);
                    }
                    // Lo borrado ya no existe: ni marcado (otra instancia
                    // podría heredar su id) ni con el foco.
                    if borra {
                        self.seleccion.quitar(&elementos);
                        self.foco.try_set(antes_de_releer(&elementos));
                    }
                    self.cerrados.try_set(elementos);
                }
            }
            self.recargar.run(());
            // Aplicado o deshecho, lo pendiente de Mejoras cambia: el
            // contador de la barra lateral vuelve a contar al momento.
            if let Some(RecargaMarco(r)) = self.marco {
                r.try_update(|n| *n += 1);
            }
        }
    }

    /// Antes de preparar un cambio sin diálogo (un interruptor): la
    /// insignia ya dice «Aplicando…» y el interruptor, su valor nuevo.
    pub(crate) fn empieza(self, elemento: &str, interruptor: Option<bool>, texto: String) {
        self.error.set(None);
        let clave = self.nueva_clave();
        self.cambios.update(|l| {
            anadir(
                l,
                CambioActivo {
                    clave,
                    id: None,
                    confirmando: false,
                    elementos: vec![elemento.to_owned()],
                    texto,
                    interruptor: interruptor.map(|v| (elemento.to_owned(), v)),
                    campos: Vec::new(),
                    estado: None,
                    cerrar: false,
                    borra: false,
                },
            )
        });
    }

    fn nueva_clave(self) -> u64 {
        self.claves.update_value(|c| *c += 1);
        self.claves.get_value()
    }

    /// No se pudo preparar: lo que se estaba preparando vuelve a como
    /// estaba (los cambios que ya se siguen, no) y el aviso da el motivo.
    pub(crate) fn falla(self, motivo: String) {
        self.cambios.update(|l| l.retain(|a| !a.preparandose()));
        self.error.set(Some(motivo));
    }

    /// Confirma el grupo preparado (`token`, sin los pasos `desmarcados`)
    /// que cambia `elementos`, y lo sigue.
    /// `cerrar`: al aplicarse, sus elementos se cierran (los que borra `g`
    /// siempre; `cerrar` lo añade para una instancia nueva). No es estado
    /// compartido: un interruptor de fila lo pasa `false`.
    pub(crate) fn aplicar(
        self,
        g: &GrupoVista,
        desmarcados: Vec<usize>,
        elementos: Vec<String>,
        cerrar: bool,
    ) {
        let interruptor = self.cambios.with_untracked(|l| {
            l.iter()
                .rev()
                .find(|a| a.preparandose())
                .and_then(|a| a.interruptor.clone())
        });
        self.error.set(None);
        let cerrar = g.borra || cerrar;
        let clave = self.nueva_clave();
        let nuevo = CambioActivo {
            clave,
            confirmando: true,
            cerrar,
            borra: g.borra,
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
        };
        self.cambios.update(|l| anadir(l, nuevo));
        let token = g.token.clone();
        spawn_local(async move {
            match confirmar_ajuste(token, desmarcados).await {
                Ok(id) => {
                    self.cambios.try_update(|l| {
                        if let Some(a) = l.iter_mut().find(|a| a.clave == clave) {
                            a.id = Some(id);
                            a.confirmando = false;
                        }
                    });
                }
                Err(e) => {
                    self.cambios.try_update(|l| l.retain(|a| a.clave != clave));
                    self.error.try_set(Some(texto_error(&e)));
                }
            }
        });
    }

    /// Abre el diálogo de revisión del grupo `g`, que cambia `elementos`.
    pub(crate) fn revisar(self, g: GrupoVista, elementos: Vec<String>) {
        self.abrir(g, elementos, false, None);
    }

    /// Como [`Seguimiento::revisar`], pero al aplicarse se cierran sus
    /// elementos (el detalle de una instancia nueva, ya creada).
    pub(crate) fn revisar_y_cerrar(self, g: GrupoVista, elementos: Vec<String>) {
        self.abrir(g, elementos, true, None);
    }

    /// Abre el diálogo de revisión del grupo `g` con sus opciones (`extra`):
    /// un lote de Mejoras, que no cambia ningún elemento de la página.
    pub(crate) fn revisar_con(self, g: GrupoVista, extra: ExtraRevision) {
        self.abrir(g, Vec::new(), false, Some(extra));
    }

    fn abrir(
        self,
        g: GrupoVista,
        elementos: Vec<String>,
        cerrar: bool,
        extra: Option<ExtraRevision>,
    ) {
        self.error.set(None);
        self.dialogo.set(Some(Revision {
            grupo: g,
            elementos,
            deshacer: false,
            cerrar,
            extra,
        }));
    }

    /// ¿Se acaba de cerrar `id` (borrado, o creado desde su detalle)? Reactivo.
    pub(crate) fn cerrado(self, id: &str) -> bool {
        self.cerrados.with(|c| c.iter().any(|x| x == id))
    }

    /// A dónde llevar el foco tras un borrado, si la página que se pinta
    /// (con los elementos `ids`) ya no tiene lo borrado. Se atiende una vez.
    pub(crate) fn tomar_foco(self, ids: &[String]) -> Option<FocoTrasBorrar> {
        let listo = self.foco.with_untracked(|f| {
            f.as_ref()
                .is_some_and(|f| !f.borrados.iter().any(|b| ids.contains(b)))
        });
        if listo {
            self.foco.try_update(Option::take).flatten()
        } else {
            None
        }
    }

    /// Los elementos cerrados, ya atendidos.
    pub(crate) fn olvidar_cerrados(self) {
        self.cerrados.set(Vec::new());
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
