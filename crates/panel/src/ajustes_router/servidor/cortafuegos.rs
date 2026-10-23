//! Lado del servidor del cortafuegos: «No responder al ping desde internet» (preparar y su resumen) y lo que dice su tarjeta.
//!
//! La regla la pone y la quita el gestor de cambios (ver
//! `changes::especiales::cortafuegos`): aquí se prepara el alta con la
//! carga exacta, se resume el cambio (también el de su deshacer) y se
//! calcula, con la lectura de la página, si el router responde al ping, la
//! recomendación (solo informa) y si la acción está disponible.

use super::derivados::Contexto;
use super::*;
use crate::ajustes_router::{AccionPropia, CambioLegible, PasoVista, RecomendacionVista};
use crate::changes::SIN_PING_YA_PUESTA;
use crate::ui::i18n::traducir;
use crate::validacion_real::{Funcion3, sin_validar};
use hgu::cortafuegos::{self as cf, ALTA, Filtro, Servicio};

/// El ajuste de las reglas.
pub(super) const AJUSTE: &str = "seguridad.cortafuegos";
/// Título de los cambios en el Historial: el del ajuste.
const TITULO: &str = "ajuste-seguridad-cortafuegos-titulo";
/// Id de la acción propia del elemento.
pub(crate) const SIN_PING: &str = "sin_ping";
/// El campo del resumen del cambio: la regla puesta o no.
const CAMPO: &str = "ajuste-seguridad-cortafuegos-sin-ping";

/// ¿Está ya la regla «sin ping» (en un filtro de entrada IPv4)?
pub(super) fn puesta(f: &[Filtro]) -> bool {
    f.iter()
        .filter(|x| x.tipo == "0" && x.version == "0")
        .flat_map(|x| &x.reglas)
        .any(cf::es_sin_ping)
}

/// ¿Responde el router al ping desde cualquier origen de internet (y no
/// tiene la regla del panel)?
pub(super) fn responde_al_ping(f: &[Filtro]) -> bool {
    !puesta(f)
        && cf::abiertos(f)
            .iter()
            .any(|a| a.servicio == Servicio::Ping && a.desde_cualquiera)
}

/// Los filtros de la lectura del cortafuegos de la página (`None` si no se
/// pudo leer).
pub(super) fn filtros(ctx: &Contexto) -> Option<Vec<Filtro>> {
    ctx.nodo(AJUSTE).map(cf::de_xml)
}

/// Si `e` es el elemento del cortafuegos, su recomendación (solo informa:
/// se aplica con la acción propia) y «No responder al ping desde internet»
/// con el motivo si no está disponible (sin leer, sin validar en el router
/// real o ya puesta).
pub(super) fn del_elemento(
    ctx: &Contexto,
    e: &ajustes::Elemento,
    leido: bool,
) -> Option<(Option<RecomendacionVista>, AccionPropia)> {
    if !e.ajustes.iter().any(|a| a == AJUSTE) {
        return None;
    }
    let accion = |motivo: Option<String>| AccionPropia {
        id: SIN_PING.to_owned(),
        motivo,
    };
    let Some(f) = filtros(ctx).filter(|_| leido) else {
        return Some((
            None,
            accion(Some(t!("ajuste-seguridad-cortafuegos-sin-leer"))),
        ));
    };
    let motivo = match sin_validar(Funcion3::SinPing) {
        Some(m) => Some(traducir(m, None)),
        None if puesta(&f) => Some(t!("ajuste-seguridad-cortafuegos-ya-puesta")),
        None => None,
    };
    let recomendacion = responde_al_ping(&f).then(|| RecomendacionVista {
        problema: t!("ajustes-problema-seguridad-cortafuegos-ping"),
        texto: t!("ajuste-seguridad-cortafuegos-sin-ping-recomendado"),
        riesgo: false,
        boton: String::new(),
        ajuste: AJUSTE.to_owned(),
        campo: String::new(),
        valor: String::new(),
        lotes: Vec::new(),
        informativa: true,
    });
    Some((recomendacion, accion(motivo)))
}

/// Resumen de un cambio de la regla «sin ping» (también el de su
/// deshacer): un paso que la pone o la quita. Quitarla no es «borrar» algo
/// de la página (`borra`, que cerraría el detalle del elemento): es volver
/// a como estaba.
pub(crate) fn vista(
    g: &ChangeManager,
    token: String,
    pasos: &[PasoPreparado],
    r: Riesgo,
) -> GrupoVista {
    GrupoVista {
        token,
        titulo: TITULO.into(),
        pasos: pasos
            .iter()
            .map(|p| {
                let pone = p.diff.campos.iter().any(|c| !c.despues.is_empty());
                PasoVista {
                    indice: p.indice,
                    sin_cambios: p.sin_cambios,
                    campos: vec![CambioLegible {
                        campo: CAMPO.into(),
                        antes: (!pone).to_string(),
                        despues: pone.to_string(),
                        secreto: false,
                        tipo: TipoCampo::Booleano,
                    }],
                    ..PasoVista::default()
                }
            })
            .collect(),
        riesgo: riesgo(r),
        efecto: None,
        vigilancia_s: g.tiempos().vigilancia(r).as_secs(),
        detalle: None,
        aviso: None,
        borra: false,
        proveedor_dns: None,
    }
}

/// Error 409 con el texto de `clave`.
fn conflicto(clave: &str) -> ServerFnError {
    fallo(StatusCode::CONFLICT, traducir(clave, None))
}

pub(in crate::ajustes_router) async fn preparar_sin_ping() -> Result<GrupoVista, ServerFnError> {
    let (state, _) = preparar().await?;
    if let Some(m) = sin_validar(Funcion3::SinPing) {
        return Err(conflicto(m));
    }
    let g = gestor(&state)?;
    libre(&g)?;
    let filtro = match g.filtro_de_internet().await {
        Ok(f) => f,
        Err(CambioError::Router(e)) => {
            tracing::warn!("cortafuegos: {e}");
            return Err(conflicto("ajuste-seguridad-cortafuegos-sin-leer"));
        }
        Err(CambioError::Invalida(e)) => {
            tracing::warn!("cortafuegos: {e}");
            return Err(conflicto("ajuste-seguridad-cortafuegos-sin-filtro"));
        }
        Err(e) => return Err(error_ajuste(e)),
    };
    if filtro.reglas.iter().any(cf::es_sin_ping) {
        return Err(conflicto(SIN_PING_YA_PUESTA));
    }
    let req = ChangeRequest {
        funcion: ALTA.into(),
        instancia: None,
        valores: cf::carga_sin_ping(&filtro)
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
        borrar: false,
        identidad: BTreeMap::new(),
    };
    let grupo = GrupoRequest {
        pasos: vec![Paso {
            req,
            etiqueta: Some(TITULO.into()),
        }],
        titulo: Some(TITULO.into()),
    };
    let prep = g.preparar_grupo(grupo).await.map_err(error_ajuste)?;
    Ok(vista(&g, prep.token, &prep.pasos, prep.riesgo))
}
