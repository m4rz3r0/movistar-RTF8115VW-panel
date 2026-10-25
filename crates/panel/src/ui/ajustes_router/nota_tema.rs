//! Nota al pie de algunas páginas de Ajustes: en Puertos, que el router no tiene control parental.

use crate::t;
use leptos::prelude::*;

/// La nota del pie de la página del tema `tema`, si tiene: en Puertos,
/// que el router no tiene control parental y los dos caminos que quedan
/// (un DNS con filtro o un router propio), con un enlace a Mejoras.
pub(crate) fn nota_tema(tema: &str) -> Option<impl IntoView + use<>> {
    (tema == "puertos").then(|| {
        view! {
            <aside class="tema-nota">
                <p>{t!("ajustes-puertos-parental")}</p>
                <ul>
                    <li>{t!("ajustes-puertos-parental-dns")}</li>
                    <li>{t!("ajustes-puertos-parental-router")}</li>
                </ul>
                <a class="b-txt" href="/mejoras">{t!("ajustes-puertos-parental-enlace")}</a>
            </aside>
        }
    })
}
