//! La página de un tema de una sola lectura: elementos, resumen y acciones.

use super::derivados::{self, Contexto};
use super::*;
use crate::ui::comun::EstadoVisual;
use crate::ui::i18n::traducir;
use ajustes::{
    CatalogoAjustes, Elemento, PREFIJO_DERIVADO, clave_accion, clave_papel, partir_referencia,
};

pub(in crate::ajustes_router) async fn leer_tema(
    tema: String,
) -> Result<PaginaTema, ServerFnError> {
    let (state, _) = preparar().await?;
    let tema = Tema::from_slug(&tema)
        .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-tema")))?;
    let del_tema: Vec<&Ajuste> = state.ajustes.por_tema(tema).collect();
    let claves: Vec<Clave> = del_tema.iter().map(|a| clave(a)).collect();
    // Una sola tanda: las de los ajustes y las que piden los derivados.
    let todas: Vec<Clave> = claves
        .iter()
        .cloned()
        .chain(derivados::claves_extra(&del_tema))
        .collect();
    let leidas = state.lecturas.leer(&state.hgu, &todas).await;
    let nodos: Vec<Option<&XmlNode>> = claves
        .iter()
        .map(|c| leidas.get(c).and_then(|r| r.as_ref().ok()))
        .collect();
    let mut vistas: Vec<AjusteVista> = del_tema
        .iter()
        .zip(&nodos)
        .map(|(a, nodo)| vista(&state.catalogo, a, *nodo))
        .collect();
    derivados::marcar(&del_tema, &mut vistas, &nodos, &leidas);
    let ctx = Contexto {
        del_tema: &del_tema,
        nodos: &nodos,
        vistas: &vistas,
        leidas: &leidas,
    };
    let (resumen, elementos, acciones) = montar(&state.catalogo, &state.ajustes, tema, &ctx);
    Ok(PaginaTema {
        tema: tema.slug().to_owned(),
        solo_lectura: instancia::solo_lectura(&state.db).await,
        foto_de_partida: crate::fotos::hay(&state.db, crate::fotos::TipoFoto::Partida).await,
        resumen,
        elementos,
        acciones,
    })
}

/// Id en la URL del elemento implícito de un ajuste suelto
/// (`puertos.upnp` → `upnp`).
fn id_implicito(a: &Ajuste) -> String {
    a.id.split_once('.')
        .map_or(a.id.as_str(), |(_, n)| n)
        .replace('_', "-")
}

/// Resumen, elementos y acciones del tema con las lecturas de `ctx`. Cada
/// ajuste que no está en un elemento declarado es uno por sí solo, detrás
/// de los declarados.
pub(super) fn montar(
    f: &Catalogo,
    cat: &CatalogoAjustes,
    tema: Tema,
    ctx: &Contexto,
) -> (Vec<DatoResumen>, Vec<ElementoVista>, Vec<AccionVista>) {
    let mut del_catalogo: Vec<Elemento> = cat.elementos_de(tema).cloned().collect();
    let sueltos: Vec<Elemento> = ctx
        .del_tema
        .iter()
        .filter(|a| !del_catalogo.iter().any(|e| e.ajustes.contains(&a.id)))
        .map(|a| Elemento {
            id: id_implicito(a),
            tema,
            ajustes: vec![a.id.clone()],
            papel: None,
            interruptor: None,
            datos: Vec::new(),
            estado: None,
        })
        .collect();
    del_catalogo.extend(sueltos);
    let acciones: Vec<&ajustes::Accion> = cat.acciones_de(tema).collect();
    let vistas: Vec<ElementoVista> = del_catalogo
        .iter()
        .map(|e| elemento(f, cat, &acciones, ctx, e))
        .collect();
    let pares: Vec<(&Elemento, &ElementoVista)> = del_catalogo.iter().zip(&vistas).collect();
    let resumen = derivados::resumen(tema, ctx, &pares);
    let acciones = acciones
        .iter()
        .map(|a| AccionVista {
            id: a.id.clone(),
            clave: clave_accion(&a.id),
            peligro: a.peligro,
        })
        .collect();
    (resumen, vistas, acciones)
}

/// Valor legible de un campo como se escribe (`true`, la clave de una
/// opción, el texto); `None` si es un secreto o el panel no lo reconoce.
fn legible(v: &ValorLegible) -> Option<String> {
    match v {
        ValorLegible::Texto(t) | ValorLegible::Opcion(t) => Some(t.clone()),
        ValorLegible::Booleano(b) => Some(b.to_string()),
        ValorLegible::Desconocido(_) | ValorLegible::Secreto => None,
    }
}

/// El campo tiene recomendación y no el valor recomendado.
fn pendiente(c: &CampoVista) -> bool {
    c.tipo != TipoCampo::Secreto
        && c.recomendado
            .as_ref()
            .is_some_and(|r| legible(&c.valor).as_ref() != Some(r))
}

fn elemento(
    f: &Catalogo,
    cat: &CatalogoAjustes,
    acciones: &[&ajustes::Accion],
    ctx: &Contexto,
    e: &Elemento,
) -> ElementoVista {
    let ajustes: Vec<AjusteVista> = e
        .ajustes
        .iter()
        .filter_map(|id| ctx.vista(id))
        .cloned()
        .collect();
    let lectura = ajustes
        .iter()
        .map(|a| &a.estado)
        .find(|s| matches!(s, EstadoLectura::SinLeer(_)))
        .cloned()
        .unwrap_or(EstadoLectura::Leido);
    let leido = lectura == EstadoLectura::Leido;
    let campo = |r: &str| {
        let (a, c) = partir_referencia(r)?;
        let v = ajustes.iter().find(|v| v.id == a)?;
        Some((v, v.campos.iter().find(|x| x.id == c)?))
    };
    let (estado, texto_estado) = if leido {
        estado(ctx, e)
    } else {
        (EstadoVisual::Peligro, t!("ajustes-estado-sin-leer"))
    };
    let interruptor = e
        .interruptor
        .as_deref()
        .filter(|_| leido)
        .and_then(campo)
        .and_then(|(v, c)| match c.valor {
            ValorLegible::Booleano(encendido) => Some(InterruptorVista {
                ajuste: v.id.clone(),
                campo: c.id.clone(),
                encendido,
                efecto: v.efecto.clone(),
            }),
            _ => None,
        });
    let datos = if leido {
        e.datos
            .iter()
            .filter_map(|d| dato(ctx, e, d, campo))
            .collect()
    } else {
        Vec::new()
    };
    let recomendacion = leido
        .then(|| recomendacion(f, cat, acciones, &ajustes))
        .flatten();
    let aplicables = acciones
        .iter()
        .filter(|a| {
            leido
                && a.campo.as_ref().is_some_and(|id| {
                    ajustes
                        .iter()
                        .flat_map(|v| v.campos.iter())
                        .any(|c| c.id == *id && c.solo_lectura.is_none())
                })
        })
        .map(|a| a.id.clone())
        .collect();
    ElementoVista {
        id: e.id.clone(),
        clave: ajustes
            .first()
            .map_or_else(String::new, |a| a.clave.clone()),
        papel: e.papel.as_deref().map(clave_papel),
        ssid: derivados::ssid(ctx, e),
        estado,
        texto_estado,
        interruptor,
        datos,
        recomendacion,
        pie: PieVista {
            nota: derivados::nota(ctx, e),
            ..PieVista::default()
        },
        ajustes,
        lectura,
        acciones: aplicables,
    }
}

/// Estado de un elemento leído y el texto de su insignia: su regla del
/// catálogo o, sin ella, «Leído».
fn estado(ctx: &Contexto, e: &Elemento) -> (EstadoVisual, String) {
    let regla = e
        .estado
        .as_deref()
        .and_then(|r| r.strip_prefix(PREFIJO_DERIVADO));
    match regla.map(|n| derivados::estado(n, ctx, e)) {
        Some(Ok((estado, clave))) => (estado, traducir(&clave, None)),
        Some(Err(d)) => {
            tracing::error!("ajustes: {}: regla de estado desconocida {}", e.id, d.0);
            (EstadoVisual::Emite, t!("ajustes-estado-leido"))
        }
        None => (EstadoVisual::Emite, t!("ajustes-estado-leido")),
    }
}

/// Un dato de la tarjeta: un campo con su valor o un derivado.
fn dato<'v>(
    ctx: &Contexto,
    e: &Elemento,
    d: &str,
    campo: impl Fn(&str) -> Option<(&'v AjusteVista, &'v CampoVista)>,
) -> Option<DatoVista> {
    let Some(nombre) = d.strip_prefix(PREFIJO_DERIVADO) else {
        let (_, c) = campo(d)?;
        return Some(DatoVista {
            clave: c.clave.clone(),
            valor: c.valor.clone(),
            atencion: pendiente(c),
        });
    };
    match derivados::dato(nombre, ctx, e) {
        Ok(valor) => valor.map(|valor| DatoVista {
            clave: format!("ajustes-dato-{nombre}"),
            valor,
            atencion: false,
        }),
        Err(d) => {
            tracing::error!("ajustes: {}: dato derivado desconocido {}", e.id, d.0);
            None
        }
    }
}

/// La recomendación pendiente más importante de los ajustes de un
/// elemento: primero las de seguridad o de una escritura de riesgo alto;
/// después, el resto en el orden del catálogo. Solo de campos editables.
fn recomendacion(
    f: &Catalogo,
    cat: &CatalogoAjustes,
    acciones: &[&ajustes::Accion],
    ajustes: &[AjusteVista],
) -> Option<RecomendacionVista> {
    let candidatas: Vec<(&AjusteVista, &CampoVista, bool)> = ajustes
        .iter()
        .flat_map(|v| v.campos.iter().map(move |c| (v, c)))
        .filter(|(_, c)| c.solo_lectura.is_none() && pendiente(c))
        .map(|(v, c)| {
            let del_catalogo = cat.ajuste(&v.id).and_then(|a| a.campo(&c.id));
            let alto = del_catalogo
                .and_then(Campo::escribible)
                .and_then(|w| f.funcion(&w.funcion))
                .is_some_and(|func| func.riesgo == Riesgo::Alto);
            (v, c, alto || del_catalogo.is_some_and(|x| x.seguridad))
        })
        .collect();
    let (v, c, riesgo) = candidatas
        .iter()
        .find(|(_, _, riesgo)| *riesgo)
        .or_else(|| candidatas.first())?;
    let valor = c.recomendado.clone()?;
    let boton = acciones
        .iter()
        .find(|a| a.campo.as_ref() == Some(&c.id) && a.valor.as_ref() == Some(&valor))
        .map_or_else(
            || t!("ajustes-recomendacion-aplicar"),
            |a| traducir(&clave_accion(&a.id), None),
        );
    Some(RecomendacionVista {
        texto: c
            .motivo_recomendado
            .as_deref()
            .map(|k| traducir(k, None))
            .unwrap_or_default(),
        riesgo: *riesgo,
        boton,
        ajuste: v.id.clone(),
        campo: c.id.clone(),
        valor,
    })
}
