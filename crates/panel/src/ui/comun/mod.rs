//! Componentes comunes del sistema visual: [`Insignia`], [`Interruptor`],
//! [`IconoEmision`], [`Glifo`] y [`ProsContras`], y el [`EstadoVisual`] que comparten.
//!
//! Cada color tiene un único significado (spec del rediseño, §3.1): verde,
//! encendida y emite; gris, apagada; ámbar, requiere atención; rojo, errores
//! y lo destructivo. El estado va siempre también en texto.

mod emision;
mod glifo;
mod insignia;
mod interruptor;
mod pros_contras;

pub use emision::IconoEmision;
pub use glifo::Glifo;
pub use insignia::Insignia;
pub use interruptor::Interruptor;
pub use pros_contras::ProsContras;

use serde::{Deserialize, Serialize};

/// Estado de un elemento tal como se ve: decide el color de la insignia y
/// del icono de emisión. Lo calcula el servidor y viaja con los datos.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EstadoVisual {
    /// Encendida y emite, o todo bien (verde).
    Emite,
    /// Apagada (gris).
    Apagada,
    /// Requiere atención (ámbar).
    Atencion,
    /// Error o algo destructivo (rojo).
    Peligro,
    /// Algo en curso: aplicando, comprobando (neutro).
    Proceso,
}

impl EstadoVisual {
    /// Sufijo de las clases CSS del estado (`ins-…`, `emision-…`).
    pub fn sufijo(self) -> &'static str {
        match self {
            EstadoVisual::Emite => "ok",
            EstadoVisual::Apagada => "off",
            EstadoVisual::Atencion => "aviso",
            EstadoVisual::Peligro => "peligro",
            EstadoVisual::Proceso => "proceso",
        }
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use leptos::prelude::*;

    fn html(vista: impl Fn() -> AnyView) -> String {
        Owner::new().with(|| vista().to_html())
    }

    #[test]
    fn insignia_con_el_color_del_estado_y_su_texto() {
        let texto = crate::t!("ajustes-wifi-red-apagada");
        let t = texto.clone();
        let h = html(move || {
            view! { <Insignia estado=EstadoVisual::Apagada texto=t.clone() /> }.into_any()
        });
        assert!(h.contains(r#"class="ins ins-off""#), "{h}");
        assert!(h.contains(&format!(">{texto}</span>")), "{h}");
        assert!(!h.contains("<svg"), "solo el punto, sin icono: {h}");
    }

    #[test]
    fn interruptor_accesible() {
        let h = html(|| {
            view! {
                <Interruptor
                    encendido=true
                    etiqueta="Red WiFi 5 GHz encendida".to_owned()
                    al_cambiar=Callback::new(|_: bool| {})
                />
            }
            .into_any()
        });
        assert!(h.contains(r#"role="switch""#), "{h}");
        assert!(h.contains(r#"aria-checked="true""#), "{h}");
        assert!(
            h.contains(r#"aria-label="Red WiFi 5 GHz encendida""#),
            "{h}"
        );
        assert!(!h.contains("disabled"), "{h}");
        let h = html(|| {
            view! {
                <Interruptor
                    encendido=false
                    etiqueta="Red WiFi 5 GHz apagada".to_owned()
                    desactivado=true
                    al_cambiar=Callback::new(|_: bool| {})
                />
            }
            .into_any()
        });
        assert!(h.contains(r#"aria-checked="false""#), "{h}");
        // Desactivado sin perder el foco: aria-disabled, no disabled.
        assert!(h.contains(r#"aria-disabled="true""#), "{h}");
        assert!(!h.contains(" disabled"), "{h}");
    }

    #[test]
    fn icono_de_emision_con_tres_arcos() {
        let h = html(|| view! { <IconoEmision estado=EstadoVisual::Atencion /> }.into_any());
        assert!(h.contains("emision emision-aviso"), "{h}");
        assert!(h.contains(r#"aria-hidden="true""#), "{h}");
        assert_eq!(h.matches("<path").count(), 3, "{h}");
    }

    #[test]
    fn estado_visual_viaja_en_minusculas() {
        assert_eq!(
            serde_json::to_string(&EstadoVisual::Atencion).unwrap(),
            "\"atencion\""
        );
    }
}
