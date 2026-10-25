//! Derivados de Teléfono: la línea, el teléfono por internet, el paso directo y la franja de resumen.

use super::*;

/// `derivado:estado_linea`: la línea, activa o apagada.
pub(super) fn estado_linea(ctx: &Contexto) -> (EstadoVisual, String) {
    segun(
        si_no(ctx, "telefono.linea", "activa"),
        (EstadoVisual::Emite, "ajustes-estado-linea-si"),
        (EstadoVisual::Apagada, "ajustes-estado-linea-no"),
    )
}

/// `derivado:estado_voz`: el teléfono por internet apagado o, encendido,
/// si el router está registrado en el operador. Encendido y con un registro
/// que no se reconoce, solo «Encendido».
pub(super) fn estado_voz(ctx: &Contexto) -> (EstadoVisual, String) {
    let encendido = si_no(ctx, "telefono.sip_oculto", "activo");
    let si = match texto(ctx, "telefono.sip_oculto", "estado_registro") {
        Some("registrada") => (EstadoVisual::Emite, "ajustes-estado-voz-registrada"),
        Some("sin_registrar") => (EstadoVisual::Atencion, "ajustes-estado-voz-sin-registrar"),
        _ => (EstadoVisual::Emite, "ajustes-estado-voz-si"),
    };
    segun(
        encendido,
        si,
        (EstadoVisual::Apagada, "ajustes-estado-voz-no"),
    )
}

/// `derivado:estado_paso_directo`: el del fax, que es el interruptor de su
/// tarjeta (el de los módems va en sus datos).
pub(super) fn estado_paso_directo(ctx: &Contexto) -> (EstadoVisual, String) {
    segun(
        si_no(ctx, "telefono.fisico", "fax"),
        (EstadoVisual::Emite, "ajustes-estado-paso-directo-si"),
        (EstadoVisual::Apagada, "ajustes-estado-paso-directo-no"),
    )
}

/// Cifra de algo que se enciende: el texto de su estado o «—».
fn cifra(encendido: Option<bool>, si: &str, no: &str) -> String {
    match encendido {
        Some(true) => crate::ui::i18n::traducir(si, None),
        Some(false) => crate::ui::i18n::traducir(no, None),
        None => sin_dato(),
    }
}

/// Franja de resumen de Teléfono: la línea, el teléfono por internet, el
/// fax por internet y las recomendaciones pendientes.
pub(super) fn resumen(
    ctx: &Contexto,
    elementos: &[(&Elemento, &ElementoVista)],
) -> Vec<DatoResumen> {
    let [l, v, f] = [
        "ajustes-resumen-telefono-linea",
        "ajustes-resumen-telefono-voz",
        "ajustes-resumen-telefono-fax",
    ];
    vec![
        dato_resumen(
            l,
            cifra(
                si_no(ctx, "telefono.linea", "activa"),
                "ajustes-estado-linea-si",
                "ajustes-estado-linea-no",
            ),
            detalle_fijo(l),
            false,
        ),
        dato_resumen(
            v,
            cifra(
                si_no(ctx, "telefono.sip_oculto", "activo"),
                "ajustes-estado-voz-si",
                "ajustes-estado-voz-no",
            ),
            detalle_fijo(v),
            false,
        ),
        dato_resumen(
            f,
            cifra_si_no(si_no(ctx, "telefono.sip", "fax")),
            detalle_fijo(f),
            false,
        ),
        recomendaciones(elementos),
    ]
}
