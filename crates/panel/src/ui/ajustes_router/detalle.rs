//! Detalle de un elemento: sus campos por columnas, «Descartar» y «Guardar cambios».
//!
//! Se pinta con lo que ya trajo la página (sin otra petición): una columna
//! por ajuste del elemento (lo esencial, lo avanzado y lo oculto), cada una
//! con su ayuda, sus campos, su efecto y sus detalles técnicos plegados. Una
//! lista sin componente propio se ve como tabla, y la contraseña del router
//! con el enlace a donde se cambia: se consultan, no se editan aquí.
//! «Guardar cambios» prepara un solo cambio con lo de todas las columnas y
//! lo abre en el diálogo de revisión. Se cierra con ✕ o con Esc; en el
//! móvil ocupa la pantalla y vuelve con «‹ WiFi».

use super::aviso::Seguimiento;
use super::campo::Campo;
use super::consulta;
use super::edicion::Edicion;
use super::piezas::{ayuda, detalles_tecnicos, dom, enfocar, fila, motivos};
use super::rejilla::Pagina;
use super::seleccion::nombre;
use crate::ajustes_router::{
    AjusteVista, ElementoVista, EstadoLectura, PeticionAjuste, preparar_elemento,
};
use crate::t;
use crate::ui::comun::Insignia;
use crate::ui::explorador::texto_error;
use crate::ui::i18n::traducir;
use crate::ui::{Estado, IconoEstado};
use ajustes::Nivel;
use leptos::{ev, prelude::*, task::spawn_local};
use std::collections::{BTreeMap, HashMap};

/// Id del título del detalle (recibe el foco al abrirlo).
pub(crate) const ID_TITULO_DETALLE: &str = "detalle-titulo";

/// Título de la columna de un ajuste, por su nivel.
fn titulo_columna(n: Nivel) -> String {
    match n {
        Nivel::Basico => t!("ajustes-detalle-basico"),
        Nivel::Avanzado => t!("ajustes-nivel-avanzado"),
        Nivel::Oculto => t!("ajustes-nivel-oculto"),
    }
}

/// Título, descripción y ayuda de la columna del ajuste `a` (`id`, el id
/// de su título). Lo oculto dice qué significa.
fn cabeza_columna(a: &AjusteVista, id: String) -> impl IntoView + use<> {
    let oculto = (a.nivel == Nivel::Oculto)
        .then(|| view! { <p class="detalle-nota-oculto">{t!("ajustes-nivel-oculto-desc")}</p> });
    view! {
        <h3 id=id>{titulo_columna(a.nivel)}</h3>
        {oculto}
        <p class="ajuste-desc">{traducir(&format!("{}-desc", a.clave), None)}</p>
        {ayuda(a)}
    }
}

/// ¿Se editan en el detalle los campos de `a`? Ni una lista (sus campos son
/// la plantilla de una instancia nueva) ni un ajuste con componente propio.
fn se_edita(a: &AjusteVista) -> bool {
    !a.lista && a.componente.is_none()
}

/// La columna de un ajuste que solo se consulta: una lista en una tabla o
/// el enlace a donde se cambia la contraseña del router.
fn columna_de_consulta(a: &AjusteVista) -> impl IntoView + use<> {
    let col = format!("{}-columna", dom(a));
    let titulo = col.clone();
    let cuerpo = if a.componente.as_deref() == Some(super::CONTRASENA_ROUTER) {
        consulta::enlace_contrasena_router().into_any()
    } else if a.lista {
        consulta::lista(a).into_any()
    } else {
        view! { <p class="nota-hueco">{t!("ajustes-hueco")}</p> }.into_any()
    };
    view! {
        <section class="detalle-columna" aria-labelledby=col>
            {cabeza_columna(a, titulo)}
            {cuerpo}
            {detalles_tecnicos(&a.clave, &a.tecnico)}
        </section>
    }
}

/// La columna del ajuste `a`, con sus campos editables (si `editable`) o
/// de consulta.
fn columna(
    a: &AjusteVista,
    ed: Edicion,
    editable: bool,
    bloqueado: Signal<bool>,
) -> impl IntoView + use<> {
    let prefijo = dom(a);
    let col = format!("{prefijo}-columna");
    let col_titulo = col.clone();
    let campos = a
        .campos
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if !editable || c.solo_lectura.is_some() {
                view! { <dl class="ajuste-valores">{fila(c)}</dl> }.into_any()
            } else {
                view! {
                    <Campo
                        c=c.clone()
                        dom=format!("{prefijo}-c{i}")
                        valor=ed.valor(i)
                        error=ed.error(i)
                        bloqueado
                    />
                }
                .into_any()
            }
        })
        .collect_view();
    let efecto = a.efecto.clone().filter(|_| editable).map(|e| {
        view! {
            <p class="ajuste-efecto">
                <strong>{t!("ajustes-efecto")}</strong>
                ": "
                {traducir(&e, None)}
            </p>
        }
    });
    view! {
        <section class="detalle-columna" aria-labelledby=col>
            {cabeza_columna(a, col_titulo)}
            <div class="ajuste-campos">{campos}</div>
            {motivos(&a.campos)}
            {efecto}
            {detalles_tecnicos(&a.clave, &a.tecnico)}
        </section>
    }
}

/// Lo editado y sin guardar en el detalle de cada elemento (id del
/// elemento → (ajuste, campo) → valor en edición). Vive fuera de la lectura
/// del tema: cuando la página vuelve a leerlo (tras un cambio o con «Volver
/// a leer»), el detalle se pinta de nuevo con lo editado.
#[derive(Clone, Copy)]
pub(crate) struct Borradores(RwSignal<HashMap<String, Borrador>>);

/// Lo editado en un elemento: (ajuste, campo) → valor en edición.
type Borrador = BTreeMap<(String, String), String>;

impl Borradores {
    pub(crate) fn nuevos() -> Borradores {
        Borradores(RwSignal::new(HashMap::new()))
    }

    /// Olvida lo editado en todos los elementos (al cambiar de tema).
    pub(crate) fn vaciar(self) {
        self.0.update(HashMap::clear);
    }

    /// Olvida lo editado en el elemento `id` (se descartó).
    pub(crate) fn olvidar(self, id: &str) {
        self.0.update(|b| {
            b.remove(id);
        });
    }

    /// Olvida lo editado en los campos de `claves` (las de traducción,
    /// `ajuste-wifi-red_24-nombre`), en todos los elementos: ya están en el
    /// router.
    pub(crate) fn olvidar_campos(self, claves: &[String]) {
        if claves.is_empty() {
            return;
        }
        self.0.update(|b| {
            for cambios in b.values_mut() {
                cambios.retain(|(ajuste, campo), _| {
                    let clave = format!("ajuste-{}-{campo}", ajuste.replace('.', "-"));
                    !claves.contains(&clave)
                });
            }
            b.retain(|_, cambios| !cambios.is_empty());
        });
    }

    /// Guarda lo editado en `id` (sin nada, lo olvida).
    fn guardar(self, id: &str, cambios: Borrador) {
        self.0.update(|b| {
            if cambios.is_empty() {
                b.remove(id);
            } else {
                b.insert(id.to_owned(), cambios);
            }
        });
    }

    /// Lo editado en `id`.
    fn de(self, id: &str) -> Borrador {
        self.0
            .with_untracked(|b| b.get(id).cloned().unwrap_or_default())
    }
}

/// Lo que se edita en el detalle abierto: el id de su elemento y la edición
/// de cada columna. La rejilla lo mira (al cambiar de elemento, cerrar o
/// usar «Atrás») para no perder cambios sin guardar.
pub(crate) type Registro = RwSignal<Option<(String, Vec<Edicion>)>>;

/// ¿Hay cambios sin guardar en el detalle de `abierto`?
pub(crate) fn sin_guardar(registro: Registro, abierto: Option<&str>) -> bool {
    registro.with_untracked(|r| {
        r.as_ref().is_some_and(|(id, eds)| {
            Some(id.as_str()) == abierto && eds.iter().any(|e| e.cambiada())
        })
    })
}

/// El detalle de `e`. Se apunta en `registro` al crearse.
#[component]
pub(crate) fn Detalle(e: ElementoVista, registro: Registro) -> impl IntoView {
    let pagina = expect_context::<Pagina>();
    let s = expect_context::<Seguimiento>();
    let leido = e.lectura == EstadoLectura::Leido;
    let preparando = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let leidos: Vec<&AjusteVista> = e
        .ajustes
        .iter()
        .filter(|a| a.estado == EstadoLectura::Leido)
        .collect();
    // Sin nada que se pueda cambiar (la contraseña del router, una lista de
    // consulta), sin «Guardar cambios».
    let editable = leido
        && !pagina.solo_lectura
        && leidos
            .iter()
            .any(|a| se_edita(a) && a.campos.iter().any(|c| c.solo_lectura.is_none()));
    let ediciones: Vec<(String, String, Edicion)> = leidos
        .iter()
        .filter(|a| se_edita(a))
        .map(|a| (a.id.clone(), dom(a), Edicion::nueva(&a.campos)))
        .collect();
    let columnas = leidos
        .iter()
        .map(|a| match ediciones.iter().find(|(id, _, _)| *id == a.id) {
            Some((_, _, ed)) => columna(a, *ed, editable, preparando.into()).into_any(),
            None => columna_de_consulta(a).into_any(),
        })
        .collect_view();
    registro.set(Some((
        e.id.clone(),
        ediciones.iter().map(|(_, _, ed)| *ed).collect(),
    )));
    // Lo editado antes de que la página volviera a leer el tema, de vuelta;
    // y, desde ahora, guardado mientras se edita.
    let borradores = expect_context::<Borradores>();
    for ((ajuste, campo), v) in borradores.de(&e.id) {
        if let Some((_, _, ed)) = ediciones.iter().find(|(a, _, _)| *a == ajuste) {
            ed.poner(&campo, v);
        }
    }
    let ediciones = StoredValue::new(ediciones);
    let id_borrador = e.id.clone();
    Effect::new(move |_| {
        let cambios: Borrador = ediciones.with_value(|es| {
            es.iter()
                .flat_map(|(ajuste, _, ed)| {
                    ed.cambios()
                        .into_iter()
                        .map(move |(campo, v)| ((ajuste.clone(), campo), v))
                })
                .collect()
        });
        borradores.guardar(&id_borrador, cambios);
    });
    let id = e.id.clone();
    let guardar = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        if !editable || preparando.get_untracked() {
            return;
        }
        error.set(None);
        let mut peticiones = Vec::new();
        for (ajuste, prefijo, ed) in ediciones.get_value() {
            match ed.pedir() {
                Err(i) => {
                    enfocar(&format!("{prefijo}-c{i}-control"));
                    return;
                }
                Ok(valores) if valores.is_empty() => {}
                Ok(valores) => peticiones.push(PeticionAjuste {
                    ajuste,
                    instancia: None,
                    borrar: false,
                    valores,
                    esperados: BTreeMap::new(),
                }),
            }
        }
        if peticiones.is_empty() {
            error.set(Some(t!("ajustes-error-sin-cambios")));
            return;
        }
        preparando.set(true);
        let id = id.clone();
        spawn_local(async move {
            match preparar_elemento(peticiones).await {
                Ok(g) => s.revisar(g, vec![id]),
                Err(e) => {
                    error.try_set(Some(texto_error(&e)));
                }
            }
            preparando.try_set(false);
        });
    };
    let descartar = move |_| {
        ediciones.with_value(|es| es.iter().for_each(|(_, _, ed)| ed.reiniciar()));
        error.set(None);
    };
    let cerrar = move |_| pagina.cerrar.run(());
    let teclas = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Escape" && !ev.default_prevented() {
            ev.prevent_default();
            pagina.cerrar.run(());
        }
    };
    let id_insignia = e.id.clone();
    let (estado, texto) = (e.estado, e.texto_estado.clone());
    let insignia = move || {
        let (estado, texto) = s.insignia(&id_insignia).unwrap_or((estado, texto.clone()));
        view! { <Insignia estado texto /> }
    };
    let sin_leer = match &e.lectura {
        EstadoLectura::SinLeer(m) => Some(view! {
            <p class="aviso estado-error">
                <IconoEstado estado=Estado::Error />
                <span>{m.clone()}</span>
            </p>
        }),
        EstadoLectura::Leido => None,
    };
    let tema = pagina
        .tema
        .with_value(|t| traducir(&format!("ajustes-tema-{t}"), None));
    // Recién abierto, el foco a su título en cuanto está en la página.
    let titulo = NodeRef::<leptos::html::H2>::new();
    Effect::new(move |_| {
        if let Some(h) = titulo.get()
            && pagina.enfocar_detalle.get_untracked()
        {
            pagina.enfocar_detalle.set(false);
            let _ = h.focus();
        }
    });
    view! {
        <section class="detalle" aria-labelledby=ID_TITULO_DETALLE on:keydown=teclas>
            <header class="detalle-cabecera">
                <button type="button" class="b-txt detalle-volver" on:click=cerrar>
                    {t!("ajustes-detalle-volver", tema = tema)}
                </button>
                <h2 id=ID_TITULO_DETALLE tabindex="-1" node_ref=titulo>
                    {nombre(&e)}
                </h2>
                <span class="detalle-insignia">{insignia}</span>
                <button
                    type="button"
                    class="b-txt detalle-cerrar"
                    aria-label=t!("ajustes-detalle-cerrar")
                    on:click=cerrar
                >
                    "✕"
                </button>
            </header>
            {sin_leer}
            <form class="detalle-formulario" novalidate=true on:submit=guardar>
                <div class="detalle-columnas">{columnas}</div>
                <p class="error-formulario" role="alert">{move || error.get()}</p>
                {editable.then(|| view! {
                    <div class="detalle-pie">
                        <button type="button" class="b-sec" on:click=descartar>
                            {t!("ajustes-descartar")}
                        </button>
                        // Sin `disabled`: conserva el foco, y el diálogo se lo
                        // devuelve al cerrarse.
                        <button
                            type="submit"
                            class="b-pri"
                            aria-disabled=move || preparando.get().then_some("true")
                        >
                            {move || if preparando.get() { t!("ajustes-grupo-preparando") } else { t!("ajustes-guardar-cambios") }}
                        </button>
                    </div>
                })}
            </form>
        </section>
    }
}
