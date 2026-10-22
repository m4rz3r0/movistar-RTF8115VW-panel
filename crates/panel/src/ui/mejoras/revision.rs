//! Revisión de un lote: el selector de proveedor de DNS dentro de su diálogo, que vuelve a preparar el lote.

use crate::ajustes_router::{GrupoVista, OpcionesLote, ProveedorVista, preparar_lote};
use crate::t;
use crate::ui::ajustes_router::ExtraRevision;
use crate::ui::explorador::texto_error;
use crate::ui::i18n::traducir;
use leptos::{prelude::*, task::spawn_local};

/// Clave del paso de un lote que escribe los DNS de la red de casa: si la
/// revisión lo lleva, lleva también el selector de proveedor.
const PASO_DNS: &str = "dns_casa";

/// El proveedor que se marca al abrir la revisión del grupo `g`: con el
/// que se preparó, si está entre `proveedores`; si no, el de por defecto.
fn marcado(g: &GrupoVista, proveedores: &[ProveedorVista]) -> Option<String> {
    let preparado = g
        .proveedor_dns
        .as_deref()
        .and_then(|id| proveedores.iter().find(|p| p.id == id));
    preparado
        .or_else(|| proveedores.iter().find(|p| p.defecto))
        .or(proveedores.first())
        .map(|p| p.id.clone())
}

/// El selector de proveedor de DNS de la revisión abierta. Vive en la
/// página y empieza de nuevo con cada revisión.
#[derive(Clone, Copy)]
pub(super) struct SelectorDns {
    /// El proveedor marcado.
    elegido: RwSignal<String>,
    /// El del grupo que enseña el diálogo: si preparar otro falla, se vuelve
    /// a marcar este.
    mostrado: StoredValue<String>,
    /// El lote preparado de nuevo con otro proveedor.
    nuevo: RwSignal<Option<GrupoVista>>,
    /// Se está preparando: el diálogo no se cierra ni aplica.
    ocupado: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    /// Cada preparación lleva su número: solo cuenta la respuesta de la
    /// última (y ninguna de una revisión anterior).
    vuelta: StoredValue<u32>,
}

impl SelectorDns {
    pub(super) fn nuevo() -> SelectorDns {
        SelectorDns {
            elegido: RwSignal::new(String::new()),
            mostrado: StoredValue::new(String::new()),
            nuevo: RwSignal::new(None),
            ocupado: RwSignal::new(false),
            error: RwSignal::new(None),
            vuelta: StoredValue::new(0),
        }
    }

    /// Las opciones de la revisión del grupo `g` del lote `lote`: el
    /// selector con `proveedores` si el grupo lleva el paso de los DNS,
    /// marcado en el proveedor con el que lo preparó el servidor (el que ya
    /// estaba puesto o, si no era ninguno, el de por defecto). `None` si no
    /// lo lleva.
    pub(super) fn extra(
        self,
        lote: String,
        g: &GrupoVista,
        proveedores: Vec<ProveedorVista>,
    ) -> Option<ExtraRevision> {
        if !g.pasos.iter().any(|p| p.clave.as_deref() == Some(PASO_DNS)) {
            return None;
        }
        let marcado = marcado(g, &proveedores)?;
        self.siguiente_vuelta();
        self.elegido.set(marcado.clone());
        self.mostrado.set_value(marcado);
        self.nuevo.set(None);
        self.ocupado.set(false);
        self.error.set(None);
        let antes_de_pasos = ViewFn::from(move || self.vista(&lote, &proveedores));
        Some(ExtraRevision {
            antes_de_pasos,
            nuevo_grupo: self.nuevo.into(),
            ocupado: self.ocupado.into(),
        })
    }

    fn siguiente_vuelta(self) -> u32 {
        self.vuelta.update_value(|v| *v = v.wrapping_add(1));
        self.vuelta.get_value()
    }

    /// Prepara de nuevo el lote `lote` con el proveedor `id`; el diálogo lo
    /// toma por `nuevo` y conserva lo marcado.
    fn elegir(self, lote: String, id: String) {
        self.elegido.set(id.clone());
        self.error.set(None);
        self.ocupado.set(true);
        let vuelta = self.siguiente_vuelta();
        spawn_local(async move {
            let opciones = OpcionesLote {
                proveedor_dns: Some(id.clone()),
            };
            let r = preparar_lote(lote, opciones).await;
            if self.vuelta.try_get_value() != Some(vuelta) {
                return;
            }
            match r {
                Ok(g) => {
                    self.mostrado.set_value(id);
                    self.nuevo.try_set(Some(g));
                }
                Err(e) => {
                    self.elegido.try_set(self.mostrado.get_value());
                    self.error.try_set(Some(texto_error(&e)));
                }
            }
            self.ocupado.try_set(false);
        });
    }

    /// Los proveedores, uno por radio con su nota.
    fn vista(self, lote: &str, proveedores: &[ProveedorVista]) -> AnyView {
        let opciones = proveedores
            .iter()
            .map(|p| {
                let id = format!("proveedor-dns-{}", p.id);
                let ayuda = format!("{id}-ayuda");
                let este = p.id.clone();
                let elegir = {
                    let (lote, este) = (lote.to_owned(), p.id.clone());
                    move |_| self.elegir(lote.clone(), este.clone())
                };
                view! {
                    <div class="opcion">
                        <input
                            id=id.clone()
                            type="radio"
                            name="proveedor-dns"
                            value=p.id.clone()
                            aria-describedby=ayuda.clone()
                            prop:checked=move || self.elegido.with(|e| *e == este)
                            on:change=elegir
                        />
                        <span class="opcion-texto">
                            <label class="opcion-nombre" for=id>
                                {traducir(&format!("mejoras-dns-{}", p.id), None)}
                            </label>
                            <span class="nota" id=ayuda>
                                {traducir(&format!("mejoras-dns-{}-desc", p.id), None)}
                            </span>
                        </span>
                    </div>
                }
            })
            .collect_view();
        view! {
            <fieldset class="opciones revision-dns">
                <legend class="revision-dns-titulo">{t!("mejoras-dns-elegir")}</legend>
                {opciones}
                <p class="error-formulario" role="alert">{move || self.error.get()}</p>
            </fieldset>
        }
        .into_any()
    }
}
