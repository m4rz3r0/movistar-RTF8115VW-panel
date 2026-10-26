//! Lado del servidor de los usuarios del router: preparar crear o borrar, el resumen del cambio y el flujo propio del cambio de otro usuario.

use super::*;
use crate::ajustes_router::servidor::error_ajuste;
use crate::ajustes_router::{CambioLegible, PasoVista};
use crate::changes::{
    CambioError, ChangeManager, ChangeRequest, GrupoRequest, Nota, Paso, PasoPreparado,
};
use crate::contrasena_forma::{ErrorNueva, validar_forma_hasta};
use crate::contrasena_router::servidor::anotar;
use crate::instalar::servidor::{fallo, interno};
use crate::instancia::{self, CredencialesRouter};
use crate::panel_ajustes::servidor::preparar_delicada;
use crate::state::AppState;
use crate::t;
use crate::ui::explorador::MASCARA;
use crate::ui::explorador::servidor::{gestor, riesgo};
use crate::validacion_real::{Funcion3, sin_validar};
use ::i18n::Idioma;
use ajustes::TipoCampo;
use axum::http::StatusCode;
use catalogo::Riesgo;
use hgu::fmt::{
    ALTA_USUARIO, CAMBIO_OTRO, PAGINA_USUARIOS, parametros_cambio_otro, usuario_valido,
};
use std::collections::BTreeMap;

/// Título de los cambios en el Historial: el del ajuste.
pub(crate) const TITULO: &str = "ajuste-seguridad-usuarios_router-titulo";
/// Prefijo de las claves de los campos del ajuste.
const PREFIJO: &str = "ajuste-seguridad-usuarios_router";

/// 409 si la validación real aún no ha confirmado `f`, con su motivo.
fn exigir_validada(f: Funcion3) -> Result<(), ServerFnError> {
    match sin_validar(f) {
        Some(motivo) => Err(fallo(
            StatusCode::CONFLICT,
            crate::ui::i18n::traducir(motivo, None),
        )),
        None => Ok(()),
    }
}

pub(super) async fn preparar(op: OperacionUsuario) -> Result<GrupoVista, ServerFnError> {
    // El resumen lleva el nombre en claro.
    crate::ui::explorador::servidor::sin_guardar();
    // Crear o borrar un usuario del router es delicado: reautenticación.
    let (state, _) = preparar_delicada().await?;
    exigir_validada(Funcion3::Usuarios)?;
    if let OperacionUsuario::Crear {
        usuario,
        contrasena,
        ..
    } = &op
    {
        if !usuario_valido(usuario) {
            return Err(fallo(
                StatusCode::BAD_REQUEST,
                t!("usuarios-router-error-nombre"),
            ));
        }
        validar_forma_hasta(contrasena, MAX_OTRO)
            .map_err(|e| fallo(StatusCode::BAD_REQUEST, texto_error_otro(e)))?;
    }
    let g = gestor(&state)?;
    if g.activo().is_some() {
        return Err(error_ajuste(CambioError::EnCurso));
    }
    let borra = matches!(op, OperacionUsuario::Borrar { .. });
    let usuario = op.usuario().to_owned();
    let req = match op {
        OperacionUsuario::Crear {
            usuario,
            contrasena,
            grupo,
        } => ChangeRequest {
            funcion: ALTA_USUARIO.into(),
            instancia: None,
            valores: [
                ("rqUser", usuario),
                ("rqPwd", contrasena),
                ("rqGroup", grupo.del_router().to_owned()),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
            borrar: false,
            identidad: BTreeMap::new(),
        },
        OperacionUsuario::Borrar { usuario } => ChangeRequest {
            funcion: ALTA_USUARIO.into(),
            instancia: Some(usuario),
            valores: BTreeMap::new(),
            borrar: true,
            identidad: BTreeMap::new(),
        },
    };
    let grupo = GrupoRequest {
        pasos: vec![Paso {
            req,
            etiqueta: Some(TITULO.into()),
        }],
        titulo: Some(TITULO.into()),
    };
    let prep = g.preparar_grupo(grupo).await.map_err(error_ajuste)?;
    let v = vista(&g, prep.token, &prep.pasos, prep.riesgo);
    Ok(GrupoVista {
        detalle: Some(usuario),
        // El deshacer no puede devolverle su contraseña: se avisa antes.
        aviso: borra.then(|| t!("usuarios-router-aviso-borrar")),
        borra,
        ..v
    })
}

/// Un campo del diff del gestor como cambio legible del ajuste: el nombre,
/// la contraseña (siempre tapada) y el tipo de cuenta.
fn legible(d: &crate::changes::CampoDiff) -> Option<CambioLegible> {
    let opcion = |v: &str| match v {
        "system" => "operador".to_owned(),
        "admin" => "administrador".to_owned(),
        "user" => "usuario".to_owned(),
        otro => otro.to_owned(),
    };
    Some(match d.nombre.as_str() {
        "rqUser" => CambioLegible {
            campo: format!("{PREFIJO}-nombre"),
            antes: d.antes.clone(),
            despues: d.despues.clone(),
            secreto: d.secreto,
            tipo: TipoCampo::Texto,
        },
        "rqPwd" => CambioLegible {
            campo: "usuarios-router-contrasena".into(),
            antes: String::new(),
            despues: MASCARA.to_owned(),
            secreto: true,
            tipo: TipoCampo::Secreto,
        },
        "rqGroup" => CambioLegible {
            campo: format!("{PREFIJO}-papel"),
            antes: opcion(&d.antes),
            despues: opcion(&d.despues),
            secreto: false,
            tipo: TipoCampo::Opcion,
        },
        _ => return None,
    })
}

/// Resumen de un cambio de usuarios preparado (también el de su deshacer):
/// un paso con el nombre, la contraseña tapada y el tipo de cuenta.
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
            .map(|p| PasoVista {
                indice: p.indice,
                sin_cambios: p.sin_cambios,
                campos: p.diff.campos.iter().filter_map(legible).collect(),
                ..PasoVista::default()
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

/// Cómo acaba un cambio de otro usuario que no se hace.
#[derive(Debug)]
enum Fallo {
    EnCurso,
    SinRouter,
    SoloLectura,
    Protegido,
    NoExiste,
    Nueva(ErrorNueva),
    /// El router rechazó el cambio: sigue como estaba.
    Rechazado,
    /// El router respondió, pero el usuario no entra con la nueva.
    NoEntra,
    /// Entra con la nueva, pero el router no muestra el grupo pedido.
    Grupo,
    Interno(String),
}

/// Un [`Fallo`] como clave de traducción (para la respuesta y el
/// Historial, que lo pinta en el idioma de quien lo mira).
fn nota(f: &Fallo) -> Nota {
    match f {
        Fallo::EnCurso => Nota::clave("usuarios-router-error-en-curso"),
        Fallo::SinRouter => Nota::clave("contrasena-router-error-sin-router"),
        Fallo::SoloLectura => Nota::clave("panel-solo-lectura"),
        Fallo::Protegido => Nota::clave("usuarios-router-error-protegido"),
        Fallo::NoExiste => Nota::clave("usuarios-router-error-no-existe"),
        Fallo::Nueva(e) => Nota::de_contrasena(*e, MAX_OTRO),
        Fallo::Rechazado => Nota::clave("usuarios-router-error-rechazado"),
        Fallo::NoEntra => Nota::clave("usuarios-router-error-no-entra"),
        Fallo::Grupo => Nota::clave("usuarios-router-error-grupo"),
        Fallo::Interno(_) => Nota::clave("comun-error-interno"),
    }
}

fn a_error(f: Fallo, idioma: Idioma) -> ServerFnError {
    let status = match &f {
        Fallo::EnCurso => StatusCode::CONFLICT,
        Fallo::SinRouter | Fallo::NoExiste | Fallo::Nueva(_) => StatusCode::BAD_REQUEST,
        Fallo::SoloLectura | Fallo::Protegido => StatusCode::FORBIDDEN,
        Fallo::Rechazado | Fallo::NoEntra | Fallo::Grupo => StatusCode::BAD_GATEWAY,
        Fallo::Interno(m) => return interno(m),
    };
    fallo(status, nota(&f).en(idioma))
}

pub(super) async fn cambiar(
    usuario: String,
    nueva: String,
    grupo: GrupoUsuario,
) -> Result<(), ServerFnError> {
    let (state, yo) = preparar_delicada().await?;
    exigir_validada(Funcion3::ContrasenaOtro)?;
    let idioma = crate::ui::i18n::idioma_actual();
    // En su propia tarea: si quien lo pidió se va, el cambio acaba igual
    // (y se apunta).
    let tarea = tokio::spawn(async move {
        cambiar_y_anotar(&state, &yo.username, &usuario, &nueva, grupo).await
    });
    match tarea.await {
        Ok(r) => r.map_err(|f| a_error(f, idioma)),
        Err(e) => Err(interno(format!(
            "el cambio de la contraseña de otro usuario se interrumpió: {e}"
        ))),
    }
}

/// Las guardas, la escritura y la comprobación; lo que acabe se apunta en
/// el Historial (salvo lo que no llega a tocar el router).
async fn cambiar_y_anotar(
    state: &AppState,
    yo: &str,
    usuario: &str,
    nueva: &str,
    grupo: GrupoUsuario,
) -> Result<(), Fallo> {
    // Mientras viva, el gestor no empieza ningún cambio.
    let _reserva = state
        .cambios()
        .map(|m| m.excluir_cambios())
        .transpose()
        .map_err(|_| Fallo::EnCurso)?;
    let panel = instancia::leer_credenciales(&state.db, &state.history_key)
        .await
        .map_err(Fallo::Interno)?
        .ok_or(Fallo::SinRouter)?;
    if instancia::solo_lectura(&state.db).await {
        return Err(Fallo::SoloLectura);
    }
    let papel = papel_de(state, usuario).await?;
    if usuario == panel.usuario || papel.as_deref() == Some("system") {
        return Err(Fallo::Protegido);
    }
    if papel.is_none() {
        return Err(Fallo::NoExiste);
    }
    validar_forma_hasta(nueva, MAX_OTRO).map_err(Fallo::Nueva)?;
    let r = en_el_router(state, &panel, usuario, nueva, grupo).await;
    let resultado = match &r {
        Ok(()) => Ok(Nota::clave("usuarios-router-historial-nota")),
        Err(f) => Err(nota(f)),
    };
    anotar(state, yo, CAMBIO_OTRO, TITULO, resultado).await;
    state.lecturas.invalidar();
    r
}

/// Papel de `usuario` en el router ahora (`None` si no está).
async fn papel_de(state: &AppState, usuario: &str) -> Result<Option<String>, Fallo> {
    let n = state
        .hgu
        .call(crate::changes::LECTURA_USUARIOS, &[])
        .await
        .map_err(|e| Fallo::Interno(format!("no se puede leer la lista de usuarios: {e}")))?;
    Ok(crate::changes::usuarios_de(&n)
        .into_iter()
        .find(|u| u.get("rqUser").map(String::as_str) == Some(usuario))
        .and_then(|u| u.get("rqGroup").cloned()))
}

/// Escribe el cambio por la sesión del panel y comprueba, con un cliente
/// aparte, que `usuario` entra con `nueva`; luego, que tiene el grupo.
async fn en_el_router(
    state: &AppState,
    panel: &CredencialesRouter,
    usuario: &str,
    nueva: &str,
    grupo: GrupoUsuario,
) -> Result<(), Fallo> {
    let params = parametros_cambio_otro(usuario, nueva, grupo.del_router());
    let params: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    match state.hgu.write(PAGINA_USUARIOS, CAMBIO_OTRO, &params).await {
        Ok(_) => {}
        // El texto del router no se anota: no se sabe qué repite.
        Err(hgu::Error::CallFailed { result, .. }) => {
            tracing::warn!("el router rechazó el cambio de otro usuario (Result={result})");
            return Err(Fallo::Rechazado);
        }
        // Sin respuesta: pudo llegar; lo dice la prueba de acceso.
        Err(_) => tracing::warn!("cambio de otro usuario sin respuesta válida del router"),
    }
    let otro = CredencialesRouter {
        usuario: usuario.to_owned(),
        contrasena: nueva.to_owned(),
        ..panel.clone()
    };
    if !entra(state, &otro).await {
        tracing::warn!("el usuario cambiado no entra en el router con la contraseña nueva");
        return Err(Fallo::NoEntra);
    }
    if papel_de(state, usuario).await?.as_deref() != Some(grupo.del_router()) {
        tracing::warn!("el router no muestra el grupo pedido para el usuario cambiado");
        return Err(Fallo::Grupo);
    }
    tracing::info!("contraseña o grupo de otro usuario del router cambiados");
    Ok(())
}

/// ¿Entra `c` en el router? Solo los dos inicios de sesión (otro usuario
/// quizá no puede leer nada más), con un cliente y una sesión aparte que
/// se cierra; si tarda más de [`AppState::espera_prueba_router`], no.
async fn entra(state: &AppState, c: &CredencialesRouter) -> bool {
    let cliente = hgu::Client::new(c.client_config(&state.cfg));
    let r = tokio::time::timeout(state.espera_prueba_router, cliente.entrar()).await;
    cliente.logout().await;
    matches!(r, Ok(Ok(())))
}
