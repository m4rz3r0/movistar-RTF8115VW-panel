//! Selección de elementos: la marca redonda, lo marcado y la barra de acciones.
//!
//! Marcar sirve para aplicar una acción a varios elementos a la vez: solo
//! se pueden marcar los que comparten alguna acción de la pantalla con otro
//! ([`marcables`]). Sin eso no hay marca, ni tecla de marcar, ni barra; lo
//! que vale para un solo elemento ya lo ofrece su tarjeta (su recomendación,
//! su interruptor o su detalle).
//!
//! La marca es un círculo en la esquina de la tarjeta (nunca una casilla
//! cuadrada): solo se ve al pasar el ratón, con el foco o cuando ya hay
//! alguna marcada. La barra aparece al marcar algo, dice cuántas, ofrece
//! solo las acciones que valen para todo lo marcado y «Desmarcar todo». Lo
//! que no se ha podido leer no entra en la acción, y se dice.

use super::ID_SOLO_LECTURA;
use crate::ajustes_router::{AccionVista, ElementoVista, EstadoLectura};
use crate::t;
use crate::ui::i18n::traducir;
use leptos::prelude::*;
use std::collections::BTreeSet;

/// Lo marcado en la página (ids de elementos). Vive fuera de la lectura del
/// tema: sobrevive a las relecturas tras un cambio.
#[derive(Clone, Copy)]
pub(crate) struct Seleccion(RwSignal<BTreeSet<String>>);

impl Seleccion {
    pub(crate) fn nueva() -> Seleccion {
        Seleccion(RwSignal::new(BTreeSet::new()))
    }

    /// ¿Está marcado `id`? Reactivo.
    pub(crate) fn marcada(self, id: &str) -> bool {
        self.0.with(|s| s.contains(id))
    }

    /// Marca o desmarca `id`.
    pub(crate) fn alternar(self, id: &str) {
        self.0.update(|s| {
            if !s.remove(id) {
                s.insert(id.to_owned());
            }
        });
    }

    /// Desmarca `ids` (lo que ya no existe).
    pub(crate) fn quitar(self, ids: &[String]) {
        self.0.update(|s| s.retain(|id| !ids.contains(id)));
    }

    pub(crate) fn vaciar(self) {
        self.0.update(BTreeSet::clear);
    }

    /// Deja marcados solo los de `marcables` (tras una relectura, lo que ya
    /// no se puede marcar no sigue contando en la barra).
    pub(crate) fn conservar(self, marcables: &BTreeSet<String>) {
        if self
            .0
            .with_untracked(|s| s.iter().any(|id| !marcables.contains(id)))
        {
            self.0.update(|s| s.retain(|id| marcables.contains(id)));
        }
    }

    /// Cuántos hay marcados. Reactivo.
    pub(crate) fn cuantas(self) -> usize {
        self.0.with(BTreeSet::len)
    }

    /// Los marcados. Reactivo.
    pub(crate) fn ids(self) -> BTreeSet<String> {
        self.0.get()
    }
}

/// Los elementos de la pantalla que se pueden marcar: los que tienen alguna
/// acción de `acciones` que vale también para otro elemento. Un elemento sin
/// lectura no tiene acciones (no entra). Vacío: la pantalla no ofrece
/// selección.
pub(crate) fn marcables(acciones: &[AccionVista], elementos: &[ElementoVista]) -> BTreeSet<String> {
    let cuantos = |a: &AccionVista| {
        elementos
            .iter()
            .filter(|e| e.acciones.contains(&a.id))
            .count()
    };
    let compartidas: Vec<&String> = acciones
        .iter()
        .filter(|a| cuantos(a) >= 2)
        .map(|a| &a.id)
        .collect();
    elementos
        .iter()
        .filter(|e| e.acciones.iter().any(|a| compartidas.contains(&a)))
        .map(|e| e.id.clone())
        .collect()
}

/// Lo que ofrece la barra para lo marcado: las acciones que se pueden
/// aplicar a todos los marcados que se han leído (en el orden del tema),
/// los ids de esos (en el orden de la página) y los marcados que no se han
/// podido leer, que no entran.
pub(crate) fn aplicables<'a>(
    acciones: &'a [AccionVista],
    elementos: &'a [ElementoVista],
    marcados: &BTreeSet<String>,
) -> (Vec<&'a AccionVista>, Vec<String>, Vec<&'a ElementoVista>) {
    let (leidos, sin_leer): (Vec<&ElementoVista>, Vec<&ElementoVista>) = elementos
        .iter()
        .filter(|e| marcados.contains(&e.id))
        .partition(|e| e.lectura == EstadoLectura::Leido);
    if leidos.is_empty() {
        return (Vec::new(), Vec::new(), sin_leer);
    }
    let valen = acciones
        .iter()
        .filter(|a| leidos.iter().all(|e| e.acciones.contains(&a.id)))
        .collect();
    let ids = leidos.iter().map(|e| e.id.clone()).collect();
    (valen, ids, sin_leer)
}

/// Nombre de un elemento: el título de su primer ajuste.
pub(crate) fn nombre(e: &ElementoVista) -> String {
    traducir(&format!("{}-titulo", e.clave), None)
}

/// La marca redonda de la tarjeta del elemento `id` (`nombre`, para su
/// nombre accesible). `aria-pressed` dice si está marcada.
#[component]
pub(crate) fn Marca(id: String, nombre: String) -> impl IntoView {
    let sel = expect_context::<Seleccion>();
    let marcada = {
        let id = id.clone();
        move || sel.marcada(&id).to_string()
    };
    view! {
        <button
            type="button"
            class="marcar"
            aria-pressed=marcada
            aria-label=t!("ajustes-marcar", nombre = nombre)
            on:click=move |ev| {
                ev.stop_propagation();
                sel.alternar(&id);
            }
        >
            <svg viewBox="0 0 12 12" aria-hidden="true" focusable="false">
                <path d="M2.5 6.2 5 8.5 9.5 3.5" />
            </svg>
        </button>
    }
}

/// La barra contextual: «N marcadas», las acciones que valen para todo lo
/// marcado y «Desmarcar todo». Siempre en la página (oculta sin nada
/// marcado). En solo lectura, desactivada y con el porqué. Al pulsar una
/// acción llama a `al_pulsar` con su id y los elementos a los que se
/// aplica.
#[component]
pub(crate) fn BarraSeleccion(
    acciones: Vec<AccionVista>,
    elementos: Vec<ElementoVista>,
    solo_lectura: bool,
    #[prop(into)] al_pulsar: Callback<(String, Vec<String>)>,
) -> impl IntoView {
    let sel = expect_context::<Seleccion>();
    let datos = StoredValue::new((acciones, elementos));
    let botones = move || {
        let marcados = sel.ids();
        datos.with_value(|(acciones, elementos)| {
            let (valen, ids, sin_leer) = aplicables(acciones, elementos, &marcados);
            let nota = (!sin_leer.is_empty()).then(|| {
                let nombres: Vec<String> = sin_leer.iter().map(|e| nombre(e)).collect();
                view! {
                    <span class="barra-acciones-nota">
                        {t!("ajustes-barra-sin-leer", nombres = nombres.join(", "))}
                    </span>
                }
            });
            let ninguna = (valen.is_empty() && !ids.is_empty()).then(
                || view! { <span class="barra-acciones-nota">{t!("ajustes-barra-ninguna")}</span> },
            );
            let botones = valen
                .into_iter()
                .map(|a| {
                    let id = a.id.clone();
                    let ids = ids.clone();
                    let clase = if a.peligro { "b-pel" } else { "b-sec" };
                    view! {
                        <button
                            type="button"
                            class=clase
                            aria-disabled=solo_lectura.then_some("true")
                            on:click=move |_| {
                                if !solo_lectura {
                                    al_pulsar.run((id.clone(), ids.clone()));
                                }
                            }
                        >
                            {traducir(&a.clave, None)}
                        </button>
                    }
                })
                .collect_view();
            view! { {botones} {ninguna} {nota} }
        })
    };
    view! {
        <div
            class="barra-acciones"
            role="toolbar"
            aria-label=t!("ajustes-barra")
            aria-disabled=solo_lectura.then_some("true")
            aria-describedby=solo_lectura.then_some(ID_SOLO_LECTURA)
            hidden=move || sel.cuantas() == 0
        >
            <span class="barra-acciones-n">{move || t!("ajustes-marcadas", n = sel.cuantas())}</span>
            {botones}
            <span class="barra-acciones-hueco"></span>
            <button type="button" class="b-txt" on:click=move |_| sel.vaciar()>
                {t!("ajustes-desmarcar-todo")}
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ajustes_router::PieVista;
    use crate::ui::comun::EstadoVisual;

    fn elemento(id: &str, acciones: &[&str], leido: bool) -> ElementoVista {
        ElementoVista {
            id: id.into(),
            clave: format!("ajuste-wifi-{id}"),
            papel: None,
            ssid: None,
            icono: None,
            estado: EstadoVisual::Apagada,
            texto_estado: String::new(),
            interruptor: None,
            datos: Vec::new(),
            recomendacion: None,
            pie: PieVista::default(),
            ajustes: Vec::new(),
            lectura: if leido {
                EstadoLectura::Leido
            } else {
                EstadoLectura::SinLeer("no".into())
            },
            acciones: acciones.iter().map(|a| (*a).to_owned()).collect(),
            accion_propia: None,
            desactivadas: Default::default(),
        }
    }

    fn accion(id: &str) -> AccionVista {
        AccionVista {
            id: id.into(),
            clave: format!("ajustes-accion-{id}"),
            peligro: false,
        }
    }

    /// Solo se marca lo que comparte una acción con otro elemento: una
    /// pantalla cuyas acciones valen cada una para un solo elemento (o para
    /// ninguno) no ofrece selección.
    #[test]
    fn solo_se_marca_lo_que_comparte_una_accion() {
        let acciones = [
            accion("encender"),
            accion("aislar"),
            accion("apagar_gestion"),
        ];
        let elementos = [
            elemento("red-24", &["encender", "aislar"], true),
            elemento("red-5", &["encender"], true),
            elemento("radio-24", &[], true),
            elemento("invitados", &[], false),
        ];
        let ids = |s: BTreeSet<String>| s.into_iter().collect::<Vec<_>>();
        assert_eq!(ids(marcables(&acciones, &elementos)), ["red-24", "red-5"]);

        // Como en Seguridad: una acción, y para un solo elemento.
        let solas = [
            elemento("gestion", &["apagar_gestion"], true),
            elemento("contrasena", &[], true),
            elemento("cortafuegos", &[], true),
        ];
        assert!(marcables(&acciones, &solas).is_empty());
        // Sin acciones en la pantalla, nada.
        assert!(marcables(&[], &elementos).is_empty());
        // Una lista con una sola fila tampoco: no hay «varios».
        assert!(marcables(&acciones, &elementos[1..2]).is_empty());
    }

    /// La barra ofrece solo lo que vale para todo lo marcado que se ha
    /// leído; lo que no se ha podido leer no entra y se nombra.
    #[test]
    fn acciones_aplicables_segun_lo_marcado() {
        let acciones = [accion("encender"), accion("quitar_wps"), accion("aislar")];
        let elementos = [
            elemento("red-24", &["encender", "quitar_wps", "aislar"], true),
            elemento("red-5", &["encender", "quitar_wps"], true),
            elemento("radio-24", &[], true),
            elemento("invitados", &[], false),
        ];
        let marcar = |ids: &[&str]| ids.iter().map(|i| (*i).to_owned()).collect();
        let ids_de = |v: Vec<&AccionVista>| v.into_iter().map(|a| a.id.clone()).collect::<Vec<_>>();

        let (valen, ids, sin_leer) =
            aplicables(&acciones, &elementos, &marcar(&["red-5", "red-24"]));
        assert_eq!(ids_de(valen), ["encender", "quitar_wps"]);
        assert_eq!(ids, ["red-24", "red-5"], "en el orden de la página");
        assert!(sin_leer.is_empty());

        // Con una radio marcada, ninguna de las redes vale.
        let (valen, _, _) = aplicables(&acciones, &elementos, &marcar(&["red-24", "radio-24"]));
        assert!(valen.is_empty());

        // Lo que no se ha leído no entra, y se dice.
        let (valen, ids, sin_leer) =
            aplicables(&acciones, &elementos, &marcar(&["red-24", "invitados"]));
        assert_eq!(ids_de(valen), ["encender", "quitar_wps", "aislar"]);
        assert_eq!(ids, ["red-24"]);
        assert_eq!(sin_leer.len(), 1);
        assert_eq!(sin_leer[0].id, "invitados");

        let (valen, ids, _) = aplicables(&acciones, &elementos, &marcar(&["invitados"]));
        assert!(valen.is_empty() && ids.is_empty());
    }
}
