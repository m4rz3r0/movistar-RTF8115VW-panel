//! Lado del servidor de Ajustes: lectura, guardado y errores comunes.

mod foto;
mod guardar;
mod historial;
mod lectura;
mod puertos;
mod red_local;
#[cfg(test)]
mod tests;

pub(super) use foto::{comparar_con_partida, preparar_volver_a_partida};
pub(super) use guardar::{confirmar_ajuste, preparar_ajuste};
pub(super) use historial::deshacer_ajuste;
pub(crate) use historial::etiquetas_de_campos;
pub(super) use red_local::{leer_dispositivos, leer_red_casa, leer_reservas};

use super::*;
use crate::{
    changes::{
        BLOQUEO_SECRETO_TAPADO, BLOQUEO_SOLO_LECTURA, CambioError, ChangeManager, ChangeRequest,
        Diff, GrupoRequest, Paso, PasoPreparado, buscar,
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
use lectura::vista;
use std::collections::BTreeMap;
use std::collections::HashMap;

/// Elementos de una lectura: nombre → valor.
type Leidos = BTreeMap<String, String>;

/// Lectura de un ajuste.
fn clave(a: &Ajuste) -> Clave {
    Clave::new(&a.leer.funcion, &a.leer.params)
}

pub(super) async fn leer_tema(tema: String) -> Result<TemaVista, ServerFnError> {
    let (state, _) = preparar().await?;
    let tema = Tema::from_slug(&tema)
        .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-tema")))?;
    let del_tema: Vec<&Ajuste> = state.ajustes.por_tema(tema).collect();
    let claves: Vec<Clave> = del_tema.iter().map(|a| clave(a)).collect();
    let leidas = state.lecturas.leer(&state.hgu, &claves).await;
    let ajustes = del_tema
        .iter()
        .zip(&claves)
        .map(|(a, c)| {
            let nodo = leidas.get(c).and_then(|r| r.as_ref().ok());
            vista(&state.catalogo, a, nodo)
        })
        .collect();
    Ok(TemaVista {
        tema: tema.slug().to_owned(),
        solo_lectura: instancia::solo_lectura(&state.db).await,
        foto_de_partida: crate::fotos::hay(&state.db, crate::fotos::TipoFoto::Partida).await,
        ajustes,
    })
}

/// Error del gestor al guardar un ajuste, con el mensaje traducido y sin
/// nombres de la API (el detalle técnico va al registro).
fn error_ajuste(e: CambioError) -> ServerFnError {
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
        E::Bloqueada(m) => {
            tracing::warn!("ajustes: escritura bloqueada: {m}");
            fallo(StatusCode::FORBIDDEN, t!("ajustes-error-bloqueada"))
        }
        E::Router(_) | E::Verificacion(_) => {
            tracing::warn!("ajustes: el router no hizo el cambio: {e}");
            fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-router"))
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
