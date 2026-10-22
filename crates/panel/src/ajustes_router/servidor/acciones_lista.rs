//! Acciones sobre las instancias marcadas de una lista: activar, desactivar o borrar.
//!
//! Cada instancia es un elemento (spec §5.1): un paso por instancia en un
//! único grupo del gestor, con un solo deshacer (que recrea lo borrado). Cada
//! paso pasa por la misma comprobación que el guardado de una regla o una IP
//! fija desde su detalle ([`comprobar_regla`], [`comprobar_reserva`]), con lo
//! que identifica la instancia tal como la tiene ahora el router.

use super::acciones::vista_por_pasos;
use super::guardar::grupo_de;
use super::lectura::lista;
use super::puertos::comprobar_regla;
use super::red_local::{comprobar_reserva, sin_leer};
use super::*;
use ajustes::{Accion, titulo_accion};

/// Lo que identifica la instancia `i` del ajuste `a` al editarla o
/// borrarla (lo que la página enviaría desde su detalle).
fn esperados(a: &Ajuste, i: &InstanciaVista) -> BTreeMap<String, String> {
    let campos: &[&str] = match a.componente {
        Some(Componente::Puertos) => &IDENTIFICAN_REGLA,
        Some(Componente::Reservas) => &["mac"],
        _ => &[],
    };
    campos
        .iter()
        .map(|k| ((*k).to_owned(), valor_de(&i.campos, k)))
        .collect()
}

/// Prepara la acción `a` (de una lista) sobre las instancias `ids` (ya sin
/// repetir, como mucho [`MAX_ELEMENTOS_ACCION`]). 400 si alguna no existe o
/// la acción no es de una lista; 409 si ninguna cambia.
pub(super) async fn preparar_en_lista(
    state: &AppState,
    a: &Accion,
    ids: &[String],
) -> Result<GrupoVista, ServerFnError> {
    let no_aplica = || fallo(StatusCode::BAD_REQUEST, t!("ajustes-accion-no-aplica"));
    let aj = a
        .lista
        .as_deref()
        .and_then(|id| state.ajustes.ajuste(id))
        .filter(|aj| aj.lista)
        .ok_or_else(no_aplica)?;
    let campo_valor = match (&a.campo, &a.valor) {
        _ if a.borrar => None,
        (Some(c), Some(v)) => Some((c.clone(), v.clone())),
        _ => return Err(no_aplica()),
    };
    let g = gestor(state)?;
    libre(&g)?;
    // Las instancias de ahora: las marcadas tienen que seguir ahí.
    let params: Vec<(&str, &str)> = aj
        .leer
        .params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let n = state
        .hgu
        .call(&aj.leer.funcion, &params)
        .await
        .map_err(|e| sin_leer("la lista", e))?;
    let instancias = lista(&state.catalogo, aj, &prefijo_i18n(aj), &n).ok_or_else(|| {
        tracing::warn!("ajustes: {}: instancias sin su id", aj.id);
        fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-lectura"))
    })?;
    let mut grupo = GrupoRequest {
        pasos: Vec::new(),
        titulo: Some(titulo_accion(&a.id)),
    };
    let mut nombres: Vec<String> = Vec::with_capacity(ids.len());
    for id in ids {
        let i = instancias
            .iter()
            .find(|i| i.id == *id)
            .ok_or_else(no_aplica)?;
        let p = PeticionAjuste {
            ajuste: aj.id.clone(),
            instancia: Some(id.clone()),
            borrar: a.borrar,
            valores: campo_valor.clone().into_iter().collect(),
            esperados: esperados(aj, i),
        };
        let nombre = match aj.componente {
            Some(Componente::Puertos) => comprobar_regla(state, aj, &p).await?,
            Some(Componente::Reservas) => comprobar_reserva(state, aj, &p).await?,
            _ => None,
        };
        // Sin nombre (una IP fija de un dispositivo que el router no
        // nombra), su MAC; sin nada, su id.
        nombres.push(
            nombre
                .or_else(|| Some(valor_de(&i.campos, "mac")).filter(|m| !m.is_empty()))
                .unwrap_or_else(|| id.clone()),
        );
        let sub = grupo_de(aj, &p).map_err(|e| {
            tracing::warn!("ajustes: acción {} en {}: {e}", a.id, aj.id);
            fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
        })?;
        grupo.pasos.extend(sub.pasos);
    }
    let pedidos: Vec<BTreeMap<String, String>> =
        grupo.pasos.iter().map(|p| p.req.valores.clone()).collect();
    let prep = g.preparar_grupo(grupo).await.map_err(|e| match e {
        CambioError::SinCambios => fallo(StatusCode::CONFLICT, t!("ajustes-accion-ya-estaba")),
        e => error_ajuste(e),
    })?;
    let de_cada_paso = vec![aj; prep.pasos.len()];
    let vista = vista_por_pasos(
        &g,
        titulo_accion(&a.id),
        &de_cada_paso,
        prep.token,
        &prep.pasos,
        prep.riesgo,
        &|i, param| buscar(&pedidos[i], param).map(str::to_owned),
    );
    Ok(GrupoVista {
        detalle: Some(nombres.join(", ")),
        borra: a.borrar,
        ..vista
    })
}
