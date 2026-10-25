//! Nota al pie de algunas páginas de Ajustes: en Puertos, que el router no tiene control parental.

use crate::t;
use leptos::prelude::*;

/// La nota del pie de la página del tema `tema`, si tiene: en Puertos,
/// que el router no tiene control parental y los dos caminos que quedan
/// (un DNS con filtro o un router propio), con un enlace a la opción de
/// DNS familiar de Mejoras. Una sección con nombre: su encabezado.
pub(crate) fn nota_tema(tema: &str) -> Option<impl IntoView + use<>> {
    (tema == "puertos").then(|| {
        view! {
            <section class="tema-nota" aria-labelledby="tema-nota-titulo">
                <h2 id="tema-nota-titulo">{t!("ajustes-puertos-parental-titulo")}</h2>
                <p>{t!("ajustes-puertos-parental")}</p>
                <ul>
                    <li>{t!("ajustes-puertos-parental-dns")}</li>
                    <li>{t!("ajustes-puertos-parental-router")}</li>
                </ul>
                <a class="b-txt" href="/mejoras">{t!("ajustes-puertos-parental-enlace")}</a>
            </section>
        }
    })
}
