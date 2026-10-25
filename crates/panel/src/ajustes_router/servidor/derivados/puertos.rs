//! Derivados de Puertos: el estado de la DMZ y su recomendación, que solo informa.

use super::*;

/// Ajuste de la DMZ.
const AJUSTE: &str = "puertos.dmz";
/// Su campo con el equipo que recibe lo que llega de internet.
const CAMPO: &str = "equipo";
/// Su campo que dice si está encendida.
const ACTIVA: &str = "activa";

/// Las instancias activas de la DMZ (`None` si no se pudo leer). El router
/// del usuario no tiene DMZ, así que el formato de una activa es una
/// suposición: cuenta toda instancia que no venga marcada como apagada
/// (una DMZ apagada pero guardada no es una DMZ activa).
fn activas<'a>(ctx: &Contexto<'a>) -> Option<Vec<&'a InstanciaVista>> {
    instancias(ctx, AJUSTE).map(|i| i.iter().filter(|i| !apagada(i)).collect())
}

/// Si la instancia viene marcada como apagada.
fn apagada(i: &InstanciaVista) -> bool {
    i.campos
        .iter()
        .any(|c| c.id == ACTIVA && c.valor == ValorLegible::Booleano(false))
}

/// `derivado:estado_dmz`: sin instancias activas, desactivada (en gris);
/// con alguna, activa (en ámbar). «Sin datos» si no se pudo leer.
pub(super) fn estado_dmz(ctx: &Contexto) -> (EstadoVisual, String) {
    segun(
        activas(ctx).map(|i| !i.is_empty()),
        (EstadoVisual::Atencion, "ajustes-estado-dmz-activa"),
        (EstadoVisual::Apagada, "ajustes-estado-dmz-desactivada"),
    )
}

/// IP del equipo de la DMZ, si la instancia la trae (0.0.0.0 es ninguno).
fn equipo(i: &InstanciaVista) -> Option<&str> {
    i.campos
        .iter()
        .find(|c| c.id == CAMPO)
        .and_then(|c| match &c.valor {
            ValorLegible::Texto(t) if !t.is_empty() && t != "0.0.0.0" => Some(t.as_str()),
            _ => None,
        })
}

/// Recomendación (solo informa: el panel no puede cambiar la DMZ) si está
/// activa: todo lo que llega de internet sin regla va a ese equipo. Si la
/// instancia no trae el equipo, el texto no lleva IP.
pub(super) fn dmz_activa(ctx: &Contexto) -> Option<RecomendacionVista> {
    let primera = *activas(ctx)?.first()?;
    let problema = match equipo(primera) {
        Some(ip) => t!("ajustes-problema-puertos-dmz", ip = ip.to_owned()),
        None => t!("ajustes-problema-puertos-dmz-sin-ip"),
    };
    Some(RecomendacionVista {
        problema,
        texto: t!("ajustes-puertos-dmz-recomendado"),
        riesgo: true,
        boton: String::new(),
        ajuste: AJUSTE.to_owned(),
        campo: CAMPO.to_owned(),
        valor: String::new(),
        lotes: Vec::new(),
        informativa: true,
    })
}
