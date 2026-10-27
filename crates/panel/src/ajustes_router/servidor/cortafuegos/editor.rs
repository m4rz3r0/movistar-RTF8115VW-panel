//! Lado del servidor del editor completo del cortafuegos: la lectura para la página, la preparación de cada cambio y su resumen (también el de su deshacer).
//!
//! Lo usan las funciones de servidor de `ajustes_router`
//! (`cortafuegos_editor`); el gestor de cambios hace el resto
//! (`changes::especiales::cortafuegos::editor`): las guardas, la carga de
//! la web, la comprobación y el deshacer. Lo que no está comprobado en el
//! router real (la lista blanca del catálogo) sale con su motivo y el
//! gestor no lo escribe.

use crate::ajustes_router::servidor::error_ajuste;
use crate::ajustes_router::{
    BloqueosCortafuegos, EditorCortafuegos, FiltroEditor, OpcionEditor, PeticionCortafuegos,
    ReglaEditor, ValoresCortafuegos,
};
use crate::ajustes_router::{CambioLegible, GrupoVista, PasoVista};
use crate::changes::{ChangeManager, ChangeRequest, GrupoRequest, Paso, PasoPreparado};
use crate::instalar::servidor::fallo;
use crate::panel_ajustes::servidor::preparar as sesion;
use crate::t;
use crate::ui::explorador::servidor::{gestor, riesgo};
use crate::ui::i18n::traducir;
use ajustes::TipoCampo;
use axum::http::StatusCode;
use catalogo::Riesgo;
use hgu::cortafuegos::{
    self as cf, ALTA_FILTRO, ALTA_REGLA, BAJA_FILTRO, MODIFICAR, MODIFICAR_FILTRO, TIPOS_ICMP,
    TIPOS_RECHAZO, es_de_filtro,
};
use leptos::prelude::ServerFnError;
use std::collections::BTreeMap;

/// Título de los cambios en el Historial: el del ajuste, como la regla
/// «sin ping» (así el deshacer encuentra su elemento).
const TITULO: &str = "ajuste-seguridad-cortafuegos-titulo";
/// Prefijo de las claves de los campos del editor.
const PREFIJO: &str = "ajuste-seguridad-cortafuegos-ed";

/// Motivo de `funcion` si no está comprobada en el router real.
fn bloqueo(state: &crate::state::AppState, funcion: &str) -> Option<String> {
    let comprobada = state
        .catalogo
        .funcion(funcion)
        .is_some_and(|f| f.comprobada());
    (!comprobada).then(|| traducir(crate::validacion_real::SIN_COMPROBAR, None))
}

/// El nombre legible de un tipo de la web (`port_unreachable` → «port
/// unreachable»).
fn nombre_de_tipo(t: &str) -> String {
    t.replace('_', " ")
}

/// Las opciones ICMP de la versión `version`: «cualquiera» y las suyas.
fn opciones_icmp(version: &str) -> Vec<OpcionEditor> {
    cf::icmp_de(version)
        .map(|i| OpcionEditor {
            valor: i.to_string(),
            nombre: if i == 1 {
                t!("ajuste-seguridad-cortafuegos-icmp-cualquiera")
            } else {
                nombre_de_tipo(TIPOS_ICMP[i])
            },
        })
        .collect()
}

/// Los tipos de rechazo de la versión `version`.
fn opciones_rechazo(version: &str) -> Vec<OpcionEditor> {
    cf::rechazo_de(version)
        .map(|i| OpcionEditor {
            valor: i.to_string(),
            nombre: nombre_de_tipo(TIPOS_RECHAZO[i]),
        })
        .collect()
}

/// Lo que lee el editor: los filtros del router y lo que se puede hacer.
pub(crate) async fn leer() -> Result<EditorCortafuegos, ServerFnError> {
    let (state, _) = sesion().await?;
    let filtros = cf::leer(&state.hgu).await.map_err(|e| {
        tracing::warn!("cortafuegos: no se puede leer para el editor: {e}");
        fallo(
            StatusCode::CONFLICT,
            t!("ajuste-seguridad-cortafuegos-editor-sin-leer"),
        )
    })?;
    let mut interfaces: Vec<(String, String)> = filtros
        .iter()
        .map(|f| (f.intf_id.clone(), f.interface.clone()))
        .collect();
    interfaces.sort();
    interfaces.dedup_by(|a, b| a.0 == b.0);
    let regla = bloqueo(&state, ALTA_REGLA);
    let bloqueos = BloqueosCortafuegos {
        regla_nueva: regla.clone(),
        regla_cambiar: bloqueo(&state, MODIFICAR),
        regla_borrar: regla,
        filtro_nuevo: bloqueo(&state, ALTA_FILTRO),
        filtro_cambiar: bloqueo(&state, MODIFICAR_FILTRO),
        filtro_borrar: bloqueo(&state, BAJA_FILTRO),
    };
    let filtros = filtros
        .iter()
        .map(|f| FiltroEditor {
            fid: f.fid.clone(),
            valores: cf::vista_filtro(f),
            interfaz: f.interface.clone(),
            reescribible: cf::filtro_reescribible(f),
            reglas: f
                .reglas
                .iter()
                .map(|r| ReglaEditor {
                    rid: r.rid.clone(),
                    valores: cf::vista_regla(f, r),
                    reescribible: cf::regla_reescribible(f, r),
                })
                .collect(),
        })
        .collect();
    Ok(EditorCortafuegos {
        filtros,
        interfaces,
        icmp: [opciones_icmp("0"), opciones_icmp("1")],
        rechazo: [opciones_rechazo("0"), opciones_rechazo("1")],
        bloqueos,
    })
}

/// Solo los campos de `params` de `v` (lo demás no es de la página).
fn solo(v: ValoresCortafuegos, params: &[&str]) -> BTreeMap<String, String> {
    v.into_iter()
        .filter(|(k, _)| params.contains(&k.as_str()))
        .collect()
}

/// La petición del gestor de `p`.
fn peticion(p: PeticionCortafuegos) -> ChangeRequest {
    use PeticionCortafuegos as P;
    let regla = |mut v: ValoresCortafuegos, filtro: &str| {
        v.insert("belong".into(), filtro.to_owned());
        v
    };
    let (funcion, instancia, valores, borrar, identidad) = match p {
        P::ReglaNueva { filtro, valores } => (
            ALTA_REGLA,
            None,
            regla(valores, &filtro),
            false,
            BTreeMap::new(),
        ),
        P::ReglaCambiada {
            filtro,
            regla: rid,
            valores,
            antes,
        } => (
            MODIFICAR,
            Some(rid),
            regla(valores, &filtro),
            false,
            regla(solo(antes, &cf::PARAMS_REGLA), &filtro),
        ),
        P::ReglaBorrada {
            filtro,
            regla: rid,
            antes,
        } => (
            ALTA_REGLA,
            Some(rid),
            BTreeMap::new(),
            true,
            regla(solo(antes, &cf::PARAMS_REGLA), &filtro),
        ),
        P::FiltroNuevo { valores } => (ALTA_FILTRO, None, valores, false, BTreeMap::new()),
        P::FiltroCambiado {
            filtro,
            valores,
            antes,
        } => (
            MODIFICAR_FILTRO,
            Some(filtro),
            valores,
            false,
            solo(antes, &cf::PARAMS_FILTRO),
        ),
        P::FiltroBorrado { filtro, antes } => (
            BAJA_FILTRO,
            Some(filtro),
            BTreeMap::new(),
            true,
            solo(antes, &cf::PARAMS_FILTRO),
        ),
    };
    ChangeRequest {
        funcion: funcion.into(),
        instancia,
        valores,
        borrar,
        identidad,
    }
}

/// 400 con el campo que no vale, si `req` trae una regla o un filtro que no
/// valen (el gestor lo rechazaría igual, pero sin decir cuál).
fn comprobar_campos(req: &ChangeRequest) -> Result<(), ServerFnError> {
    if req.borrar {
        return Ok(());
    }
    let r = if es_de_filtro(&req.funcion) {
        cf::normalizar_filtro(&req.valores).map(drop)
    } else {
        let version = req.valores.get("version").map_or("", String::as_str);
        cf::normalizar_regla(&req.valores, version).map(drop)
    };
    r.map_err(|cf::CampoMal(k)| {
        fallo(
            StatusCode::BAD_REQUEST,
            t!(
                "ajuste-seguridad-cortafuegos-error-campo",
                campo = traducir(&format!("{PREFIJO}-{k}"), None)
            ),
        )
    })
}

/// Prepara el cambio `p` y su resumen.
pub(crate) async fn preparar(p: PeticionCortafuegos) -> Result<GrupoVista, ServerFnError> {
    let (state, _) = sesion().await?;
    let g = gestor(&state)?;
    g.admite_cambio().map_err(error_ajuste)?;
    let req = peticion(p);
    comprobar_campos(&req)?;
    let grupo = GrupoRequest {
        pasos: vec![Paso {
            req,
            etiqueta: Some(TITULO.into()),
        }],
        titulo: Some(TITULO.into()),
    };
    let prep = g.preparar_grupo(grupo).await.map_err(error_ajuste)?;
    Ok(vista(
        &g,
        prep.token,
        &prep.pasos,
        (prep.riesgo, prep.vigila),
    ))
}

/// Clave, tipo y valor legible de un campo del editor con el valor `v`.
fn legible(param: &str, v: &str) -> (String, TipoCampo, String) {
    let opcion = |campo: &str, opciones: &[(&str, &str)]| {
        let valor = opciones
            .iter()
            .find(|(x, _)| *x == v)
            .map_or(v, |(_, clave)| clave);
        (
            format!("ajuste-seguridad-cortafuegos-{campo}"),
            TipoCampo::Opcion,
            valor.to_owned(),
        )
    };
    let por_indice = |tipos: &[&str]| {
        v.parse::<usize>()
            .ok()
            .and_then(|i| tipos.get(i))
            .map_or_else(|| v.to_owned(), |t| nombre_de_tipo(t))
    };
    match param {
        "version" => opcion("version", &[("0", "ipv4"), ("1", "ipv6")]),
        "enabled" => (
            "ajuste-seguridad-cortafuegos-regla_activa".into(),
            TipoCampo::Booleano,
            match v {
                "1" => "true".into(),
                "0" => "false".into(),
                otro => otro.to_owned(),
            },
        ),
        "protocol" => opcion("protocolo", &[("1", "icmp"), ("2", "tcp"), ("3", "udp")]),
        "specificAction" => opcion(
            "accion",
            &[("0", "descartar"), ("1", "rechazar"), ("2", "permitir")],
        ),
        "type" => opcion("ed-type", &[("0", "entrada"), ("1", "salida")]),
        "default" => opcion("ed-default", &[("0", "descartar"), ("1", "permitir")]),
        "icmpType" => (
            format!("{PREFIJO}-{param}"),
            TipoCampo::Texto,
            por_indice(&TIPOS_ICMP),
        ),
        "rejectType" => (
            format!("{PREFIJO}-{param}"),
            TipoCampo::Texto,
            por_indice(&TIPOS_RECHAZO),
        ),
        _ => (format!("{PREFIJO}-{param}"), TipoCampo::Texto, v.to_owned()),
    }
}

/// Un paso del editor: su título (qué hace) y lo que cambia (en un alta o
/// una baja, lo que tiene valor).
fn paso(p: &PasoPreparado) -> PasoVista {
    let filtro = es_de_filtro(&p.funcion);
    let crea = p.diff.campos.iter().all(|c| c.antes.is_empty());
    let quita = p.diff.campos.iter().all(|c| c.despues.is_empty());
    let que = match (crea, quita) {
        (true, _) => "nueva",
        (_, true) => "borrada",
        _ => "cambiada",
    };
    let entidad = if filtro { "filtro" } else { "regla" };
    let campos = p
        .diff
        .campos
        .iter()
        .filter(|c| c.antes != c.despues)
        .map(|c| {
            let (campo, tipo, antes) = legible(&c.nombre, &c.antes);
            let (_, _, despues) = legible(&c.nombre, &c.despues);
            CambioLegible {
                campo,
                antes,
                despues,
                secreto: false,
                tipo,
            }
        })
        .collect();
    PasoVista {
        indice: p.indice,
        sin_cambios: p.sin_cambios,
        campos,
        titulo: Some(format!("{PREFIJO}-paso-{entidad}-{que}")),
        ..PasoVista::default()
    }
}

/// Resumen de un cambio del editor (también el de su deshacer). Borrar una
/// regla o un filtro no es `borra` (cerraría el detalle del elemento del
/// cortafuegos, donde sigue el editor).
pub(crate) fn vista(
    g: &ChangeManager,
    token: String,
    pasos: &[PasoPreparado],
    (r, vigila): (Riesgo, bool),
) -> GrupoVista {
    GrupoVista {
        token,
        titulo: TITULO.into(),
        pasos: pasos.iter().map(paso).collect(),
        riesgo: riesgo(r),
        efecto: None,
        vigilancia_s: g.tiempos().plazo(r, vigila).as_secs(),
        detalle: None,
        aviso: None,
        borra: false,
        proveedor_dns: None,
    }
}

#[cfg(test)]
#[path = "editor/tests.rs"]
mod tests;
