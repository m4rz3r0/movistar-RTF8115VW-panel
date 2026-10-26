//! La cabeza de cada columna del detalle: su título (por nivel, «Opciones» o el del propio ajuste), su descripción y su ayuda.

use super::super::piezas::ayuda;
use crate::ajustes_router::AjusteVista;
use crate::t;
use crate::ui::i18n::traducir;
use ajustes::Nivel;
use leptos::prelude::*;

/// Título de la columna de un ajuste, por su nivel.
fn titulo_columna(n: Nivel) -> String {
    match n {
        Nivel::Basico => t!("ajustes-detalle-basico"),
        Nivel::Avanzado => t!("ajustes-nivel-avanzado"),
        Nivel::Oculto => t!("ajustes-nivel-oculto"),
    }
}

/// Cómo se titula una columna (ver [`cabeza_columna`]).
#[derive(Debug, PartialEq, Eq)]
enum Titulo {
    /// «Opciones»: la única columna del elemento.
    Opciones,
    /// Su nivel: «Lo esencial», «Avanzado» u «Oculto».
    Nivel,
    /// El título del propio ajuste: la fibra, que acompaña a las conexiones
    /// a internet y va en su columna si estas no se leen.
    Propio,
}

/// Ver [`Titulo`]. `acompana`: `a` va con las conexiones de su elemento.
fn titulo(unica: bool, acompana: bool) -> Titulo {
    match (acompana, unica) {
        (true, _) => Titulo::Propio,
        (false, true) => Titulo::Opciones,
        (false, false) => Titulo::Nivel,
    }
}

/// Título, descripción y ayuda de la columna del ajuste `a` (`id`, el id
/// de su título). Lo oculto dice qué significa. La única columna de un
/// elemento se titula «Opciones» (su nivel solo dice algo junto a otras);
/// la de la fibra, con su título (ver [`Titulo`]).
pub(super) fn cabeza_columna(
    a: &AjusteVista,
    id: String,
    unica: bool,
    acompana: bool,
) -> impl IntoView + use<> {
    let titulo = match titulo(unica, acompana) {
        Titulo::Opciones => t!("ajustes-detalle-opciones"),
        Titulo::Nivel => titulo_columna(a.nivel),
        Titulo::Propio => traducir(&format!("{}-titulo", a.clave), None),
    };
    let oculto = (a.nivel == Nivel::Oculto)
        .then(|| view! { <p class="detalle-nota-oculto">{t!("ajustes-nivel-oculto-desc")}</p> });
    view! {
        <h3 id=id>{titulo}</h3>
        {oculto}
        <p class="ajuste-desc">{traducir(&format!("{}-desc", a.clave), None)}</p>
        {ayuda(a)}
    }
}

#[cfg(test)]
mod tests {
    use super::{Titulo, titulo};

    /// La fibra lleva su título también en su propia columna (sin las
    /// conexiones leídas), sea la única o no; las demás, «Opciones» o su
    /// nivel.
    #[test]
    fn la_fibra_con_su_titulo_en_su_columna() {
        assert_eq!(titulo(true, true), Titulo::Propio);
        assert_eq!(titulo(false, true), Titulo::Propio);
        assert_eq!(titulo(true, false), Titulo::Opciones);
        assert_eq!(titulo(false, false), Titulo::Nivel);
    }
}
