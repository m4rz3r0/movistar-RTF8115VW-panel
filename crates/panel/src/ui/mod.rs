//! Interfaz: páginas, marco común y componentes.
//!
//! Aquí viven también las piezas mínimas que comparten tablas, indicadores y
//! páginas: el [`Estado`] (ok, aviso, error, neutro) con su icono. Los
//! componentes del sistema visual (insignia, interruptor, icono de emisión)
//! están en [`comun`]. Los colores de estado se reservan para estados y van
//! siempre con texto, nunca solos.

pub mod ajustes_router;
pub mod charts;
pub mod comun;
pub mod contrasena_campo;
pub mod estado;
pub mod explorador;
pub mod grupo;
pub mod historial;
pub mod i18n;
pub mod instalar;
pub mod layout;
pub mod login;
pub mod mejoras;
pub mod panel_ajustes;
pub mod recomendaciones;
pub mod router;
pub mod secreto;
pub mod table;
pub mod tema;
pub mod vista;

use crate::t;
use leptos::prelude::*;

/// Texto de un error que trae una server function: el código fijo de sesión
/// caducada ([`crate::auth::ERROR_SESION`]) se traduce aquí; el resto ya
/// viene traducido del servidor.
pub(crate) fn mensaje_del_servidor(msg: &str) -> String {
    if msg == crate::auth::ERROR_SESION {
        t!("comun-error-sesion")
    } else if msg == crate::auth::ERROR_REAUTENTICACION {
        t!("panel-reautenticar-texto")
    } else {
        msg.to_owned()
    }
}

/// ¿El error es por no tener sesión (o haber caducado)? Por el código fijo,
/// no por el texto, que depende del idioma.
pub(crate) fn es_error_de_sesion(e: &ServerFnError) -> bool {
    matches!(e, ServerFnError::ServerError(m) if m == crate::auth::ERROR_SESION)
}

/// ¿Falta volver a escribir la contraseña del panel? Por el código fijo.
pub(crate) fn es_error_de_reautenticacion(e: &ServerFnError) -> bool {
    matches!(e, ServerFnError::ServerError(m) if m == crate::auth::ERROR_REAUTENTICACION)
}

/// Estado con significado (bien, aviso, error) o neutro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    Ok,
    Aviso,
    Error,
    Neutro,
}

impl Estado {
    /// Clase CSS del estado (color del icono y del borde).
    pub fn clase(self) -> &'static str {
        match self {
            Estado::Ok => "estado-ok",
            Estado::Aviso => "estado-aviso",
            Estado::Error => "estado-error",
            Estado::Neutro => "estado-neutro",
        }
    }
}

impl From<Estado> for comun::EstadoVisual {
    /// Color de la insignia de un estado de tabla o indicador: lo neutro
    /// (desconectado, apagado) va en gris, como lo apagado.
    fn from(e: Estado) -> Self {
        match e {
            Estado::Ok => comun::EstadoVisual::Emite,
            Estado::Aviso => comun::EstadoVisual::Atencion,
            Estado::Error => comun::EstadoVisual::Peligro,
            Estado::Neutro => comun::EstadoVisual::Apagada,
        }
    }
}

/// Oculta los iconos de estado ([`IconoEstado`]) de lo que se pinta debajo
/// mientras la señal es cierta: en el modo de visualización «texto» la página
/// de estado no lleva ningún gráfico (el texto que acompaña al icono ya dice
/// el estado).
#[derive(Clone, Copy)]
pub(crate) struct IconosOcultos(pub Signal<bool>);

/// Icono de 16 px del estado (círculo con marca, triángulo, círculo con
/// aspa, círculo con «i»). Decorativo: el texto que lo acompaña da el estado.
/// Debajo de [`IconosOcultos`], solo mientras no estén ocultos.
#[component]
pub fn IconoEstado(estado: Estado) -> impl IntoView {
    match use_context::<IconosOcultos>() {
        None => dibujo_estado(estado).into_any(),
        Some(IconosOcultos(ocultos)) => {
            (move || (!ocultos.get()).then(|| dibujo_estado(estado))).into_any()
        }
    }
}

/// SVG de [`IconoEstado`].
fn dibujo_estado(estado: Estado) -> impl IntoView {
    let trazos = match estado {
        Estado::Ok => view! {
            <circle cx="8" cy="8" r="6.25" />
            <path d="M5.2 8.3l1.9 1.9 3.8-4" />
        }
        .into_any(),
        Estado::Aviso => view! {
            <path d="M8 1.9l6.4 11.3H1.6z" />
            <path d="M8 6.4v3.1M8 11.6v.1" />
        }
        .into_any(),
        Estado::Error => view! {
            <circle cx="8" cy="8" r="6.25" />
            <path d="M5.8 5.8l4.4 4.4M10.2 5.8l-4.4 4.4" />
        }
        .into_any(),
        Estado::Neutro => view! {
            <circle cx="8" cy="8" r="6.25" />
            <path d="M8 7.3v3.8M8 4.9v.1" />
        }
        .into_any(),
    };
    view! {
        <svg
            class=format!("icono icono-estado {}", estado.clase())
            viewBox="0 0 16 16"
            width="16"
            height="16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"
        >
            {trazos}
        </svg>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_de_sesion_por_codigo_y_traducido() {
        let e = ServerFnError::ServerError(crate::auth::ERROR_SESION.into());
        assert!(es_error_de_sesion(&e));
        assert_eq!(
            mensaje_del_servidor(crate::auth::ERROR_SESION),
            t!("comun-error-sesion")
        );
        let otro = ServerFnError::ServerError(t!("comun-error-sesion"));
        assert!(!es_error_de_sesion(&otro), "el texto no cuenta");
        assert_eq!(mensaje_del_servidor("otro error"), "otro error");
    }
}
