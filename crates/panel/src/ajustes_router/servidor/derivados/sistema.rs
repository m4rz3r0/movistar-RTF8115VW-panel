//! Derivados de Sistema: la hora, la copia del tráfico y la franja de resumen.

use super::*;

/// `derivado:estado_hora`: el router se pone en hora por internet o no.
pub(super) fn estado_hora(ctx: &Contexto) -> (EstadoVisual, String) {
    segun(
        si_no(ctx, "sistema.hora", "activa"),
        (EstadoVisual::Emite, "ajustes-estado-hora-si"),
        (EstadoVisual::Apagada, "ajustes-estado-hora-no"),
    )
}

/// `derivado:estado_diagnostico`: la copia del tráfico encendida enseña
/// todo el tráfico a quien se enchufe a esa toma (en ámbar).
pub(super) fn estado_diagnostico(ctx: &Contexto) -> (EstadoVisual, String) {
    segun(
        si_no(ctx, "sistema.diagnostico", "espejo"),
        (EstadoVisual::Atencion, "ajustes-estado-diagnostico-si"),
        (EstadoVisual::Apagada, "ajustes-estado-diagnostico-no"),
    )
}

/// Los servidores de hora puestos (los que no están vacíos), en orden.
fn servidores<'a>(ctx: &Contexto<'a>) -> Option<Vec<&'a str>> {
    ctx.vista("sistema.hora")
        .filter(|v| v.estado == EstadoLectura::Leido)?;
    Some(
        ["servidor_1", "servidor_2"]
            .into_iter()
            .filter_map(|c| texto(ctx, "sistema.hora", c))
            .filter(|s| !s.trim().is_empty())
            .collect(),
    )
}

/// Franja de resumen de Sistema: la hora por internet, sus servidores, la
/// copia del tráfico (en ámbar si está encendida) y las recomendaciones
/// pendientes.
pub(super) fn resumen(
    ctx: &Contexto,
    elementos: &[(&Elemento, &ElementoVista)],
) -> Vec<DatoResumen> {
    let [h, s, d] = [
        "ajustes-resumen-sistema-hora",
        "ajustes-resumen-sistema-servidores",
        "ajustes-resumen-sistema-copia",
    ];
    let (cifra_servidores, detalle_servidores) = match servidores(ctx) {
        Some(lista) => (
            lista.len().to_string(),
            match lista.first() {
                Some(primero) => t!(
                    "ajustes-resumen-sistema-servidores-primero",
                    servidor = (*primero).to_owned()
                ),
                None => t!("ajustes-resumen-sistema-servidores-ninguno"),
            },
        ),
        None => (sin_dato(), detalle_fijo(s)),
    };
    let copia = si_no(ctx, "sistema.diagnostico", "espejo");
    let cifra_copia = match copia {
        Some(true) => t!("ajustes-estado-diagnostico-si"),
        Some(false) => t!("ajustes-estado-diagnostico-no"),
        None => sin_dato(),
    };
    vec![
        dato_resumen(
            h,
            cifra_si_no(si_no(ctx, "sistema.hora", "activa")),
            detalle_fijo(h),
            false,
        ),
        dato_resumen(s, cifra_servidores, detalle_servidores, false),
        dato_resumen(d, cifra_copia, detalle_fijo(d), copia == Some(true)),
        recomendaciones(elementos),
    ]
}
