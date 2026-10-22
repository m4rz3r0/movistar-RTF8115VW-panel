//! Preparación de un lote de mejoras como un solo grupo del gestor, con los pasos agrupados por pestaña.

use super::super::acciones::{subgrupo, vista_por_pasos};
use super::*;
use ajustes::{OpcionEleccion, titulo_lote};

/// Opción del proveedor de DNS `id`; 400 si no existe. Sin `id`, `None`:
/// se decide con lo leído (ver [`preparar_lote`]).
fn proveedor<'a>(
    cat: &'a CatalogoAjustes,
    id: Option<&str>,
) -> Result<Option<&'a OpcionEleccion>, ServerFnError> {
    let Some(id) = id else {
        return Ok(None);
    };
    cat.eleccion(ELECCION_DNS)
        .and_then(|e| e.opciones.iter().find(|o| o.id == id))
        .map(Some)
        .ok_or_else(|| fallo(StatusCode::BAD_REQUEST, t!("mejoras-error-proveedor")))
}

/// Valores legibles por id de campo, de cada ajuste que escribe un paso.
type PorAjuste<'a> = Vec<(&'a Ajuste, BTreeMap<String, String>)>;

/// Lo que escribe el paso `p`, por ajuste: el valor de cada destino o, si
/// sale de una elección, el de la opción elegida (`dns`, para la del DNS;
/// para otra, la de por defecto).
fn por_ajuste<'a>(
    state: &'a AppState,
    p: &PasoExpandido,
    dns: Option<&OpcionEleccion>,
) -> Result<PorAjuste<'a>, ServerFnError> {
    let elegida = match p.elegir.as_deref() {
        Some(ELECCION_DNS) => dns,
        Some(id) => state.ajustes.eleccion(id).map(|e| e.defecto()),
        None => None,
    };
    let mut out: PorAjuste = Vec::new();
    for d in &p.destinos {
        let valor = d
            .valor
            .as_ref()
            .or_else(|| elegida.and_then(|o| o.valores.get(&d.campo)))
            .cloned()
            .ok_or_else(|| {
                tracing::warn!("mejoras: {}: sin valor para {}", p.clave(), d.campo);
                fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
            })?;
        let a = ajuste_por_id(state, &d.ajuste)?;
        match out.iter_mut().find(|(x, _)| x.id == a.id) {
            Some((_, valores)) => {
                valores.insert(d.campo.clone(), valor);
            }
            None => out.push((a, [(d.campo.clone(), valor)].into())),
        }
    }
    Ok(out)
}

pub(in crate::ajustes_router) async fn preparar_lote(
    id: String,
    opciones: OpcionesLote,
) -> Result<GrupoVista, ServerFnError> {
    let (state, _) = preparar().await?;
    // Antes de leer nada: sin permiso de escritura no hay revisión.
    if instancia::solo_lectura(&state.db).await {
        return Err(fallo(StatusCode::FORBIDDEN, t!("ajustes-solo-lectura")));
    }
    let lote = state
        .ajustes
        .lote(&id)
        .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("mejoras-error-lote")))?;
    let elegido = proveedor(&state.ajustes, opciones.proveedor_dns.as_deref())?;
    let g = gestor(&state)?;
    libre(&g)?;
    let lecturas = Lecturas::de(&state, &[lote], true).await;
    // Sin proveedor elegido, el que ya está puesto (el paso sale sin
    // cambios) y, si no es ninguno de la lista, el de por defecto.
    let dns = elegido.or_else(|| {
        lecturas
            .proveedor_dns(&state.ajustes)
            .or_else(|| state.ajustes.eleccion(ELECCION_DNS).map(|e| e.defecto()))
    });
    // Lo que no aplica o no se pudo leer no se revisa: el aviso lo dice.
    let pasos = lecturas.estados(&state.ajustes).pop().unwrap_or_default();
    let sin_leer = pasos
        .iter()
        .filter(|(_, e)| matches!(e, EstadoPaso::SinLeer { .. }))
        .count();
    let mut aplicables: Vec<PasoExpandido> = pasos
        .into_iter()
        .filter(|(_, e)| matches!(e, EstadoPaso::Hecho | EstadoPaso::Pendiente))
        .map(|(p, _)| p)
        .collect();
    // Agrupados por pestaña, en el orden de las pestañas.
    aplicables.sort_by_key(|p| Tema::TODOS.iter().position(|t| *t == p.tema));
    let mut grupo = GrupoRequest {
        pasos: Vec::new(),
        titulo: Some(titulo_lote(&lote.id)),
    };
    // El ajuste y el paso del lote de cada paso del grupo.
    let mut de_cada_paso: Vec<&Ajuste> = Vec::new();
    let mut origen: Vec<&PasoExpandido> = Vec::new();
    for p in &aplicables {
        for (a, valores) in por_ajuste(&state, p, dns)? {
            let sub = subgrupo(&state, a, valores)?;
            de_cada_paso.extend(std::iter::repeat_n(a, sub.pasos.len()));
            origen.extend(std::iter::repeat_n(p, sub.pasos.len()));
            grupo.pasos.extend(sub.pasos);
        }
    }
    if grupo.pasos.is_empty() {
        // Nada que revisar. Si es porque no se pudo leer, no se sabe si ya
        // está aplicado.
        return Err(if sin_leer > 0 {
            fallo(StatusCode::BAD_GATEWAY, t!("mejoras-error-lectura"))
        } else {
            fallo(StatusCode::CONFLICT, t!("mejoras-error-ya-aplicado"))
        });
    }
    let pedidos: Vec<BTreeMap<String, String>> =
        grupo.pasos.iter().map(|p| p.req.valores.clone()).collect();
    let prep = g.preparar_grupo(grupo).await.map_err(|e| match e {
        // Lo leído ya está, pero lo no leído no se da por hecho.
        CambioError::SinCambios if sin_leer > 0 => fallo(
            StatusCode::CONFLICT,
            t!("mejoras-error-sin-leer", n = sin_leer),
        ),
        CambioError::SinCambios => fallo(StatusCode::CONFLICT, t!("mejoras-error-ya-aplicado")),
        e => error_ajuste(e),
    })?;
    let mut vista = vista_por_pasos(
        &g,
        titulo_lote(&lote.id),
        &de_cada_paso,
        prep.token,
        &prep.pasos,
        prep.riesgo,
        &|i, param| buscar(&pedidos[i], param).map(str::to_owned),
    );
    for paso in &mut vista.pasos {
        let Some(p) = origen.get(paso.indice) else {
            continue;
        };
        paso.pestana = Some(format!("ajustes-tema-{}", p.tema.slug()));
        paso.nota = Some(clave_paso(&p.paso));
        paso.clave = Some(p.clave());
        paso.desmarcado = p.opcional;
    }
    vista.aviso = (sin_leer > 0).then(|| t!("mejoras-revision-sin-leer", n = sin_leer));
    // El selector de la revisión marca el proveedor con el que se preparó.
    if origen
        .iter()
        .any(|p| p.elegir.as_deref() == Some(ELECCION_DNS))
    {
        vista.proveedor_dns = dns.map(|o| o.id.clone());
    }
    Ok(vista)
}
