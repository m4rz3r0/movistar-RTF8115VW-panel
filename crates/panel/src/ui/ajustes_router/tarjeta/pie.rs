//! Pie de la tarjeta: información viva y «Abrir».

use crate::ajustes_router::ElementoVista;
use crate::t;
use leptos::prelude::*;
use leptos_router::components::A;

/// La Biblioteca de la WiFi: ahí se ven todos los puntos de acceso.
const BIBLIOTECA_WIFI: &str = "/explorador/wifi";

/// Iniciales de un nombre de dispositivo («Móvil de Ana» → «MA»).
fn iniciales(nombre: &str) -> String {
    nombre
        .split_whitespace()
        .filter(|p| p.chars().next().is_some_and(char::is_alphanumeric))
        .filter(|p| p.chars().count() > 2 || p.chars().all(char::is_uppercase))
        .filter_map(|p| p.chars().next())
        .flat_map(char::to_uppercase)
        .take(2)
        .collect()
}

/// Lo que dice el pie de los dispositivos conectados: cuántos y sus
/// nombres («2 dispositivos conectados: Móvil de Ana, 1 sin nombre»).
/// `None` si no hay ninguno.
fn texto_dispositivos(dispositivos: &[Option<String>]) -> Option<String> {
    if dispositivos.is_empty() {
        return None;
    }
    let n = dispositivos.len();
    let mut nombres: Vec<String> = dispositivos.iter().flatten().cloned().collect();
    if nombres.is_empty() {
        return Some(t!("ajustes-pie-dispositivos", n = n));
    }
    let sin_nombre = n - nombres.len();
    if sin_nombre > 0 {
        nombres.push(t!("ajustes-pie-sin-nombre", n = sin_nombre));
    }
    Some(t!(
        "ajustes-pie-dispositivos-nombres",
        n = n,
        nombres = nombres.join(", ")
    ))
}

/// El pie de la tarjeta de `e`: los dispositivos conectados (cuántos, con
/// las iniciales y el nombre de los que lo tienen), desde cuándo está
/// apagada, la nota de una radio (con la Biblioteca, donde se ven todas sus
/// redes) y «Abrir».
pub(super) fn pie(e: &ElementoVista, nombre: &str, abrir: Callback<()>) -> impl IntoView + use<> {
    let dispositivos = texto_dispositivos(&e.pie.dispositivos).map(|texto| {
        let avatares = e
            .pie
            .dispositivos
            .iter()
            .flatten()
            .take(3)
            .map(|d| view! { <span class="tarjeta-av" aria-hidden="true">{iniciales(d)}</span> })
            .collect_view();
        view! {
            <div class="tarjeta-disp">
                <span class="tarjeta-avs">{avatares}</span>
                <span>{texto}</span>
            </div>
        }
    });
    let nota = e.pie.nota.clone().map(|n| {
        view! {
            <span class="tarjeta-nota">{n}</span>
            <A href=BIBLIOTECA_WIFI attr:class="b-txt">{t!("ajustes-wifi-radio-ocultas-enlace")}</A>
        }
    });
    let apagada = e.pie.apagada_desde.map(|ts| {
        view! {
            <span class="tarjeta-nota">
                {t!("ajustes-apagada-desde", fecha = crate::ui::historial::fecha_hora(ts, false))}
            </span>
        }
    });
    view! {
        <div class="tarjeta-pie">
            {dispositivos}
            {apagada}
            {nota}
            <span class="tarjeta-hueco"></span>
            <button
                type="button"
                class="b-sec"
                aria-label=t!("ajustes-abrir-nombre", nombre = nombre.to_owned())
                on:click=move |_| abrir.run(())
            >
                {t!("ajustes-abrir")}
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{iniciales, texto_dispositivos};

    #[test]
    fn iniciales_de_un_nombre() {
        assert_eq!(iniciales("Móvil de Ana"), "MA");
        assert_eq!(iniciales("portátil"), "P");
        assert_eq!(iniciales("TV"), "T");
        assert_eq!(iniciales(""), "");
    }

    /// El pie dice cuántos dispositivos hay conectados y el nombre de los
    /// que lo tienen; sin ninguno, nada.
    #[test]
    fn texto_de_los_dispositivos() {
        let ana = Some("Móvil de Ana".to_owned());
        assert_eq!(texto_dispositivos(&[]), None);
        assert_eq!(
            texto_dispositivos(&[ana.clone()]).as_deref(),
            Some("1 dispositivo conectado: Móvil de Ana")
        );
        assert_eq!(
            texto_dispositivos(&[ana, None, None]).as_deref(),
            Some("3 dispositivos conectados: Móvil de Ana, 2 sin nombre")
        );
        assert_eq!(
            texto_dispositivos(&[None, None]).as_deref(),
            Some("2 dispositivos conectados")
        );
    }

    /// «Apagada desde» dice que es desde el panel: no ve lo que se hizo
    /// con el botón del router o en su web (en los dos idiomas).
    #[test]
    fn apagada_desde_el_panel() {
        let mut args = ::i18n::FluentArgs::new();
        args.set("fecha", "22/10 10:00");
        let texto = |idioma| ::i18n::tr(idioma, "ajustes-apagada-desde", Some(&args));
        assert_eq!(
            texto(::i18n::Idioma::Es),
            "Apagada desde el panel el 22/10 10:00"
        );
        assert_eq!(
            texto(::i18n::Idioma::En),
            "Turned off from the panel on 22/10 10:00"
        );
    }
}
