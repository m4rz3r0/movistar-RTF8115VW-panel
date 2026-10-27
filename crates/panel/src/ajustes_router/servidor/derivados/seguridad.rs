//! Derivados de Seguridad: la gestión remota, el cortafuegos, las cuentas del router y la franja de resumen.

use super::*;

/// La gestión remota: encendida (`Some(true)`), apagada o no se sabe.
fn gestion_remota(ctx: &Contexto) -> Option<bool> {
    match texto(ctx, "seguridad.gestion_remota", "activa")? {
        "encendida" => Some(true),
        "apagada" => Some(false),
        _ => None,
    }
}

/// `derivado:estado_gestion_remota`: encendida es una puerta de entrada
/// desde internet (en ámbar); apagada, en gris.
pub(super) fn estado_gestion_remota(ctx: &Contexto) -> (EstadoVisual, String) {
    segun(
        gestion_remota(ctx),
        (EstadoVisual::Atencion, "ajustes-estado-gestion-remota-si"),
        (EstadoVisual::Apagada, "ajustes-estado-gestion-remota-no"),
    )
}

/// `derivado:estado_cortafuegos`: si el router responde al ping desde
/// internet, por versión (en ámbar), si solo está la regla del panel (de
/// IPv4, sin afirmar su efecto) o si nada lo abre.
pub(super) fn estado_cortafuegos(ctx: &Contexto) -> (EstadoVisual, String) {
    use super::super::cortafuegos as cf;
    match cf::filtros(ctx) {
        Some(f) => {
            let (estado, clave) = cf::estado(cf::ping(&f));
            (estado, clave.to_owned())
        }
        None => (
            EstadoVisual::Atencion,
            "ajustes-estado-desconocido".to_owned(),
        ),
    }
}

/// Cuántas cuentas tiene la web del router.
fn numero_de_cuentas(ctx: &Contexto) -> Option<usize> {
    instancias(ctx, "seguridad.usuarios_router").map(<[_]>::len)
}

/// `derivado:cuentas`: cuántas cuentas tiene la web del router.
pub(super) fn cuentas(ctx: &Contexto) -> Option<ValorLegible> {
    numero_de_cuentas(ctx).map(|n| ValorLegible::Texto(n.to_string()))
}

/// Cifra del aviso periódico: «No» si está apagado; si no, cada cuánto.
fn cifra_aviso(ctx: &Contexto) -> String {
    let ajuste = "seguridad.gestion_remota_avanzado";
    match si_no(ctx, ajuste, "aviso") {
        Some(false) => t!("comun-no"),
        // El texto de su opción («Cada semana»).
        Some(true) => match (ctx.vista(ajuste), texto(ctx, ajuste, "intervalo")) {
            (Some(v), Some(i)) => {
                crate::ui::i18n::traducir(&format!("{}-intervalo-{i}", v.clave), None)
            }
            _ => sin_dato(),
        },
        None => sin_dato(),
    }
}

/// Franja de resumen de Seguridad: la gestión remota (en ámbar si está
/// encendida), su aviso periódico, las cuentas del router y las
/// recomendaciones pendientes.
pub(super) fn resumen(
    ctx: &Contexto,
    elementos: &[(&Elemento, &ElementoVista)],
) -> Vec<DatoResumen> {
    let remota = gestion_remota(ctx);
    let cifra_remota = match remota {
        Some(true) => t!("ajustes-estado-gestion-remota-si"),
        Some(false) => t!("ajustes-estado-gestion-remota-no"),
        None => sin_dato(),
    };
    let [r, a, c] = [
        "ajustes-resumen-seguridad-remota",
        "ajustes-resumen-seguridad-aviso",
        "ajustes-resumen-seguridad-cuentas",
    ];
    vec![
        dato_resumen(r, cifra_remota, detalle_fijo(r), remota == Some(true)),
        dato_resumen(a, cifra_aviso(ctx), detalle_fijo(a), false),
        dato_resumen(
            c,
            numero_de_cuentas(ctx).map_or_else(sin_dato, |n| n.to_string()),
            detalle_fijo(c),
            false,
        ),
        recomendaciones(elementos),
    ]
}
