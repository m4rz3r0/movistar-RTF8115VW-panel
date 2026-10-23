//! Lado del servidor de Ajustes: lectura de la página, guardado y errores
//! comunes.

mod acciones;
mod acciones_lista;
mod derivados;
mod dns_propios;
mod foto;
mod guardar;
mod historial;
mod lectura;
mod mejoras;
mod pagina;
mod puertos;
mod red_local;
#[cfg(test)]
mod tests;

pub(super) use acciones::preparar_accion;
pub(super) use foto::{comparar_con_partida, preparar_volver_a_partida};
pub(super) use guardar::{confirmar_ajuste, preparar_ajuste, preparar_elemento};
pub(super) use historial::deshacer_ajuste;
pub(crate) use historial::etiquetas_de_campos;
pub(super) use mejoras::{contar_mejoras, leer_mejoras, preparar_lote};
pub(super) use pagina::leer_tema;

use super::*;
use crate::{
    changes::{
        BLOQUEO_SECRETO_TAPADO, BLOQUEO_SOLO_LECTURA, BLOQUEO_USUARIO_PROTEGIDO, CambioError,
        ChangeManager, ChangeRequest, Diff, GrupoRequest, Paso, PasoPreparado, USUARIO_YA_EXISTE,
        buscar,
    },
    instalar::servidor::fallo,
    instancia,
    lecturas::Clave,
    panel_ajustes::servidor::preparar,
    state::AppState,
    t,
    ui::explorador::{
        MASCARA,
        servidor::{error_cambio, gestor, riesgo},
    },
};
use ajustes::{
    Ajuste, Campo, Componente, ErrorValor, Tema, a_legible, a_router, falta_requerido, prefijo_i18n,
};
use ajustes::{TipoCampo, ValorLegible};
use axum::http::StatusCode;
use catalogo::{Catalogo, Riesgo};
use hgu::xml::XmlNode;
#[cfg(test)]
use lectura::vista;
use lectura::vista_leida;
use std::collections::BTreeMap;
use std::collections::HashMap;

/// Elementos de una lectura: nombre → valor.
type Leidos = BTreeMap<String, String>;

/// Lectura de un ajuste: la de su `leer` o, si busca su instancia por id,
/// la búsqueda.
pub(crate) fn clave(a: &Ajuste) -> Clave {
    match a.busqueda() {
        Some((posicion, ident)) => {
            Clave::buscando(&a.leer.funcion, &a.leer.params, posicion, &ident)
        }
        None => Clave::new(&a.leer.funcion, &a.leer.params),
    }
}

/// Error del gestor al guardar un ajuste, con el mensaje traducido y sin
/// nombres de la API (el detalle técnico va al registro).
pub(crate) fn error_ajuste(e: CambioError) -> ServerFnError {
    use CambioError as E;
    match e {
        E::SinCambios => fallo(StatusCode::CONFLICT, t!("ajustes-error-sin-cambios")),
        E::EnCurso => fallo(StatusCode::CONFLICT, t!("biblioteca-error-en-curso")),
        E::Bloqueada(m) if m == BLOQUEO_SOLO_LECTURA => {
            fallo(StatusCode::FORBIDDEN, t!("ajustes-solo-lectura"))
        }
        E::Bloqueada(m) if m == BLOQUEO_SECRETO_TAPADO => {
            fallo(StatusCode::FORBIDDEN, t!("cambio-error-secreto-tapado"))
        }
        E::Bloqueada(m) if m == BLOQUEO_USUARIO_PROTEGIDO => {
            fallo(StatusCode::FORBIDDEN, t!("usuarios-router-error-protegido"))
        }
        E::InstanciaCambiada(m) if m == USUARIO_YA_EXISTE => {
            fallo(StatusCode::CONFLICT, t!("usuarios-router-error-existe"))
        }
        E::Bloqueada(m) => {
            tracing::warn!("ajustes: escritura bloqueada: {m}");
            fallo(StatusCode::FORBIDDEN, t!("ajustes-error-bloqueada"))
        }
        E::Router(_) | E::Verificacion(_) => {
            tracing::warn!("ajustes: el router no hizo el cambio: {e}");
            fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-router"))
        }
        E::InstanciaCambiada(_) => {
            tracing::warn!("ajustes: no se escribe: {e}");
            fallo(StatusCode::CONFLICT, t!("ajustes-error-instancia-cambiada"))
        }
        E::Invalida(_) => {
            tracing::warn!("ajustes: petición no válida: {e}");
            fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
        }
        e => error_cambio(e),
    }
}

/// Mensaje traducido de un valor que no vale, con la etiqueta del campo.
fn error_valor(prefijo: &str, c: &Campo, e: &ErrorValor) -> ServerFnError {
    let etiqueta = crate::ui::i18n::traducir(&format!("{prefijo}-{}", c.id), None);
    fallo(
        StatusCode::BAD_REQUEST,
        format!("{etiqueta}: {}", texto_error_valor(e)),
    )
}

/// Mensaje traducido de un campo que hace falta y no viene (400), junto
/// a la etiqueta del campo que falta: «<falta>: hace falta para <pide>».
fn error_requiere(prefijo: &str, pide: &Campo, falta: &Campo) -> ServerFnError {
    let traducir = crate::ui::i18n::traducir;
    let etiqueta = |c: &Campo| traducir(&format!("{prefijo}-{}", c.id), None);
    let mut args = i18n::FluentArgs::new();
    args.set("campo", etiqueta(pide));
    let msg = traducir("ajustes-error-requiere", Some(&args));
    fallo(
        StatusCode::BAD_REQUEST,
        format!("{}: {msg}", etiqueta(falta)),
    )
}

/// Ajuste `id` del catálogo (404 si no existe).
fn ajuste_por_id<'a>(state: &'a AppState, id: &str) -> Result<&'a Ajuste, ServerFnError> {
    state
        .ajustes
        .ajuste(id)
        .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-ajuste")))
}

/// Un cambio a la vez: con otro aplicándose o vigilándose no se prepara
/// nada (409), tampoco en otro ajuste que escribe la misma instancia.
fn libre(g: &ChangeManager) -> Result<(), ServerFnError> {
    match g.activo() {
        Some(_) => Err(error_ajuste(CambioError::EnCurso)),
        None => Ok(()),
    }
}
