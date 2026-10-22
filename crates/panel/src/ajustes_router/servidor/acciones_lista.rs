//! Acciones sobre las instancias marcadas de una lista: activar, desactivar o borrar.
//!
//! Cada instancia es un elemento (spec §5.1): un paso por instancia en un
//! único grupo del gestor, con un solo deshacer (que recrea lo borrado). Cada
//! paso pasa por la misma comprobación que el guardado de una regla o una IP
//! fija desde su detalle ([`comprobar_regla`], [`comprobar_reserva`]), con lo
//! que identifica la instancia tal como la tiene ahora el router
//! ([`identidad_de`]). Al borrar, de mayor a menor id; y el gestor comprueba,
//! justo antes de escribir cada paso, que su instancia sigue siendo la misma
//! (`ChangeRequest::identidad`): si el router la hubiera renumerado, ese paso
//! no se escribe y no se borra otra.

use super::acciones::vista_por_pasos;
use super::guardar::grupo_de;
use super::lectura::{instancias_con_id, lista};
use super::puertos::comprobar_regla;
use super::red_local::{comprobar_reserva, sin_leer};
use super::*;
use ajustes::{Accion, titulo_accion};

/// Orden de los pasos: de mayor a menor id (los ids que no son números,
/// detrás). Así, si el router renumerase su lista al borrar una instancia,
/// las que quedan por borrar (de id menor) no cambiarían de id.
fn de_mayor_a_menor(ids: &[String]) -> Vec<String> {
    let mut v = ids.to_vec();
    v.sort_by_key(|id| std::cmp::Reverse(id.parse::<i64>().unwrap_or(i64::MIN)));
    v
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
    let sin_id = || {
        tracing::warn!("ajustes: {}: instancias sin su id", aj.id);
        fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-lectura"))
    };
    let instancias = lista(&state.catalogo, aj, &prefijo_i18n(aj), &n).ok_or_else(sin_id)?;
    let leidas = instancias_con_id(&state.catalogo, aj, &n).ok_or_else(sin_id)?;
    let componente = aj.componente.map(|c| c.slug());
    let mut grupo = GrupoRequest {
        pasos: Vec::new(),
        titulo: Some(titulo_accion(&a.id)),
    };
    let mut nombres: Vec<String> = Vec::with_capacity(ids.len());
    // Al borrar, de mayor a menor id; y cada paso comprueba, justo antes de
    // escribirse, que su instancia sigue siendo la misma (`identidad`).
    let ids = if a.borrar {
        de_mayor_a_menor(ids)
    } else {
        ids.to_vec()
    };
    for id in &ids {
        let i = instancias
            .iter()
            .find(|i| i.id == *id)
            .ok_or_else(no_aplica)?;
        let p = PeticionAjuste {
            ajuste: aj.id.clone(),
            instancia: Some(id.clone()),
            borrar: a.borrar,
            valores: campo_valor.clone().into_iter().collect(),
            esperados: identidad_de(componente, &i.campos),
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
        let mut sub = grupo_de(aj, &p).map_err(|e| {
            tracing::warn!("ajustes: acción {} en {}: {e}", a.id, aj.id);
            fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
        })?;
        // La identidad en el formato del router: lo que el gestor relee.
        let crudos = leidas.iter().find(|(x, _)| x == id).map(|(_, l)| l);
        let identidad: BTreeMap<String, String> = identifican(componente)
            .iter()
            .filter_map(|k| {
                let lee = &aj.campo(k)?.lee;
                Some((lee.clone(), crudos?.get(lee)?.clone()))
            })
            .collect();
        for paso in &mut sub.pasos {
            paso.req.identidad = identidad.clone();
        }
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

#[cfg(test)]
mod tests {
    use super::de_mayor_a_menor;

    #[test]
    fn se_borra_de_mayor_a_menor_id() {
        let ids = ["2", "10", "x", "0"].map(String::from);
        assert_eq!(de_mayor_a_menor(&ids), ["10", "2", "0", "x"]);
    }
}
