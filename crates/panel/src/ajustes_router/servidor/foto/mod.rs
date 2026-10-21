//! Foto de partida: lo que ha cambiado desde la lectura del asistente de
//! instalación ([`comparar_con_partida`]) y volver un ajuste a como estaba
//! ([`preparar_volver_a_partida`]).
//!
//! La foto guarda la respuesta de cada función de lectura leída sin
//! parámetros (o con `all=ALL`): en una función con varias instancias (los
//! puntos de acceso WiFi, las radios) trae todas, y la de un ajuste se busca
//! por los parámetros de su `leer` y sus `fijos` ([`valor_en_foto`]).

use super::lectura::{campo, hojas, instancias_con_id, valor_secreto};
use super::*;
use crate::fotos::{self, TipoFoto};
use crate::ui::explorador::servidor::sin_guardar;

/// Valor de `nombre` en `leidos`, sin distinguir mayúsculas (como
/// `ajustes::a_legible`).
fn leido<'a>(leidos: &'a Leidos, nombre: &str) -> Option<&'a str> {
    leidos
        .get(nombre)
        .or_else(|| {
            leidos
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nombre))
                .map(|(_, v)| v)
        })
        .map(String::as_str)
}

/// La respuesta de la lectura de `a` guardada en la foto `xml` (el nodo de
/// la función, como lo guarda `XmlNode::to_xml`).
fn nodo(a: &Ajuste, xml: &str) -> Option<XmlNode> {
    hgu::xml::parse_response(&a.leer.funcion, &format!("<Response>{xml}</Response>")).ok()
}

/// Parámetros que eligen la instancia de `a`: los de su lectura y sus
/// `fijos`, sin `all`.
fn filtros(a: &Ajuste) -> Vec<(&str, &str)> {
    a.leer
        .params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .chain(a.fijos.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .filter(|(k, _)| *k != "all")
        .collect()
}

/// ¿La instancia `leidos` es la del parámetro `k = v`? El router da cada
/// instancia con su índice (`X_0i`) y su id (`X_id`), y leída por id
/// devuelve `X_0i = -1`: vale cualquiera de los dos.
fn casa(leidos: &Leidos, k: &str, v: &str) -> bool {
    let otro = k
        .strip_suffix("_0i")
        .map(|base| format!("{base}_id"))
        .or_else(|| k.strip_suffix("_id").map(|base| format!("{base}_0i")));
    std::iter::once(k)
        .chain(otro.as_deref())
        .any(|k| leido(leidos, k) == Some(v))
}

/// Elementos de la instancia de `a` (que no es lista) en la respuesta `n` de
/// la foto: la que casa con sus [`filtros`] o, sin filtros, todos (como al
/// leerlo ahora). `None` si ninguna casa o no trae ninguno de sus campos.
fn instancia_en_foto(a: &Ajuste, n: &XmlNode) -> Option<Leidos> {
    let filtros = filtros(a);
    let todas = |n: &XmlNode| {
        let mut l = Leidos::new();
        hojas(n, &mut l);
        l
    };
    let leidos = if filtros.is_empty() {
        todas(n)
    } else {
        let mut candidatas: Vec<Leidos> = n
            .children
            .iter()
            .filter(|c| !c.children.is_empty())
            .map(todas)
            .collect();
        candidatas.push(todas(n));
        candidatas
            .into_iter()
            .find(|l| filtros.iter().all(|(k, v)| casa(l, k, v)))?
    };
    a.campos
        .iter()
        .any(|c| leido(&leidos, &c.lee).is_some())
        .then_some(leidos)
}

/// Valor del router (tal cual) del campo `campo` de `a` en la foto `xml`
/// de su lectura: en una lista, el de la instancia `instancia`; si no, el
/// de la instancia que eligen su `leer` y sus `fijos`. `None` si no está.
pub fn valor_en_foto(
    f: &Catalogo,
    a: &Ajuste,
    campo: &Campo,
    xml: &str,
    instancia: Option<&str>,
) -> Option<String> {
    let n = nodo(a, xml)?;
    let leidos = if a.lista {
        let id = instancia?;
        instancias_con_id(f, a, &n)?
            .into_iter()
            .find(|(i, _)| i == id)?
            .1
    } else {
        instancia_en_foto(a, &n)?
    };
    leido(&leidos, &campo.lee).map(str::to_owned)
}

/// Lo que cambia en el campo `c` de la foto (`antes`) a ahora (`ahora`), o
/// `None` si nada (o la foto no lo trae). Un secreto se compara por su valor
/// del router, que solo sale en el campo con el ojo de cada lado.
fn diferencia_de_campo(
    f: &Catalogo,
    prefijo: &str,
    c: &Campo,
    antes: &Leidos,
    ahora: &Leidos,
    en_lista: bool,
) -> Option<Diferencia> {
    let crudo_antes = leido(antes, &c.lee)?;
    let (l_antes, l_ahora) = (a_legible(c, antes), a_legible(c, ahora));
    let igual = if c.tipo == TipoCampo::Secreto {
        Some(crudo_antes) == leido(ahora, &c.lee)
    } else {
        l_antes == l_ahora
    };
    if igual {
        return None;
    }
    let secreto_antes = valor_secreto(f, c, antes);
    let restaurable = !en_lista
        && c.escribible().is_some()
        && match &l_antes {
            ValorLegible::Desconocido(_) => false,
            ValorLegible::Secreto => secreto_antes.as_deref().is_some_and(|v| !v.is_empty()),
            _ => true,
        };
    Some(Diferencia::Valor {
        partida: Box::new(campo(f, prefijo, c, l_antes, secreto_antes)),
        actual: Box::new(campo(f, prefijo, c, l_ahora, valor_secreto(f, c, ahora))),
        restaurable,
    })
}

/// Campos de una instancia de una lista para describirla, sin los secretos.
fn descripcion(f: &Catalogo, a: &Ajuste, prefijo: &str, leidos: &Leidos) -> Vec<CampoVista> {
    a.campos
        .iter()
        .filter(|c| c.tipo != TipoCampo::Secreto)
        .map(|c| campo(f, prefijo, c, a_legible(c, leidos), None))
        .collect()
}

/// Diferencias del ajuste `a` entre su respuesta en la foto y la de ahora;
/// `None` si la foto no tiene su dato.
fn diferencias(
    f: &Catalogo,
    a: &Ajuste,
    foto: &XmlNode,
    ahora: &XmlNode,
) -> Option<Vec<DiferenciaVista>> {
    let prefijo = prefijo_i18n(a);
    let fila = |instancia: Option<&str>, cambio| DiferenciaVista {
        tema: a.tema.slug().to_owned(),
        ajuste: a.id.clone(),
        clave: prefijo.clone(),
        instancia: instancia.map(str::to_owned),
        cambio,
    };
    if !a.lista {
        let antes = instancia_en_foto(a, foto)?;
        let mut ahora_l = Leidos::new();
        hojas(ahora, &mut ahora_l);
        return Some(
            a.campos
                .iter()
                .filter_map(|c| diferencia_de_campo(f, &prefijo, c, &antes, &ahora_l, false))
                .map(|d| fila(None, d))
                .collect(),
        );
    }
    let antes = instancias_con_id(f, a, foto)?;
    let ahora_i = instancias_con_id(f, a, ahora)?;
    let mut out = Vec::new();
    for (id, l) in &antes {
        match ahora_i.iter().find(|(i, _)| i == id) {
            None => out.push(fila(
                Some(id),
                Diferencia::Baja(descripcion(f, a, &prefijo, l)),
            )),
            Some((_, l_ahora)) => out.extend(
                a.campos
                    .iter()
                    .filter_map(|c| diferencia_de_campo(f, &prefijo, c, l, l_ahora, true))
                    .map(|d| fila(Some(id), d)),
            ),
        }
    }
    for (id, l) in &ahora_i {
        if !antes.iter().any(|(i, _)| i == id) {
            out.push(fila(
                Some(id),
                Diferencia::Alta(descripcion(f, a, &prefijo, l)),
            ));
        }
    }
    Some(out)
}

/// Ajustes que se comparan: los que tienen campos (no los dispositivos
/// conectados ni el enlace a la contraseña del router), por temas.
fn comparables(state: &AppState) -> Vec<&Ajuste> {
    Tema::TODOS
        .into_iter()
        .flat_map(|t| state.ajustes.por_tema(t))
        .filter(|a| !a.campos.is_empty())
        .collect()
}

pub(in crate::ajustes_router) async fn comparar_con_partida()
-> Result<ComparacionVista, ServerFnError> {
    let (state, _) = preparar().await?;
    sin_guardar();
    let solo_lectura = instancia::solo_lectura(&state.db).await;
    let Some((id, foto)) = fotos::ultima(&state.db, &state.history_key, TipoFoto::Partida).await
    else {
        return Ok(ComparacionVista {
            momento: None,
            solo_lectura,
            diferencias: Vec::new(),
            sin_dato: Vec::new(),
            sin_leer: Vec::new(),
        });
    };
    let momento = fotos::momento(&state.db, id).await.ok_or_else(|| {
        fallo(
            StatusCode::INTERNAL_SERVER_ERROR,
            t!("ajustes-foto-error-leer"),
        )
    })?;
    let ajustes = comparables(&state);
    let claves: Vec<Clave> = ajustes.iter().map(|a| clave(a)).collect();
    let leidas = state.lecturas.leer(&state.hgu, &claves).await;
    let mut c = ComparacionVista {
        momento: Some(momento),
        solo_lectura,
        diferencias: Vec::new(),
        sin_dato: Vec::new(),
        sin_leer: Vec::new(),
    };
    for (a, k) in ajustes.iter().zip(&claves) {
        let en_foto = foto.get(&a.leer.funcion).and_then(|xml| nodo(a, xml));
        let ahora = leidas.get(k).and_then(|r| r.as_ref().ok());
        match (en_foto, ahora) {
            (None, _) => c.sin_dato.push(prefijo_i18n(a)),
            (Some(_), None) => c.sin_leer.push(prefijo_i18n(a)),
            (Some(n), Some(ahora)) => match diferencias(&state.catalogo, a, &n, ahora) {
                Some(d) => c.diferencias.extend(d),
                None => c.sin_dato.push(prefijo_i18n(a)),
            },
        }
    }
    Ok(c)
}

/// Error 400 de volver a la foto, con su mensaje.
fn no_se_puede(clave: &str) -> ServerFnError {
    fallo(
        StatusCode::BAD_REQUEST,
        crate::ui::i18n::traducir(clave, None),
    )
}

/// Valor legible de la foto para guardarlo con el campo `c` (el texto que
/// enviaría la página), o la clave del motivo por el que no se puede.
fn valor_para_volver(f: &Catalogo, c: &Campo, leidos: &Leidos) -> Result<String, &'static str> {
    if c.escribible().is_none() {
        return Err("ajustes-error-solo-lectura");
    }
    match a_legible(c, leidos) {
        ValorLegible::Texto(t) | ValorLegible::Opcion(t) => Ok(t),
        ValorLegible::Booleano(b) => Ok(b.to_string()),
        ValorLegible::Desconocido(_) => Err("ajustes-foto-error-desconocido"),
        // Nunca un secreto vacío, enmascarado o tapado: borraría la clave.
        ValorLegible::Secreto => valor_secreto(f, c, leidos)
            .filter(|v| !v.is_empty())
            .ok_or("ajustes-foto-error-secreto"),
    }
}

pub(in crate::ajustes_router) async fn preparar_volver_a_partida(
    ajuste: String,
    campo: Option<String>,
    instancia: Option<String>,
) -> Result<GrupoVista, ServerFnError> {
    let (state, _) = preparar().await?;
    let a = ajuste_por_id(&state, &ajuste)?;
    if a.lista || instancia.is_some() {
        return Err(no_se_puede("ajustes-foto-error-lista"));
    }
    let sin_dato = || fallo(StatusCode::NOT_FOUND, t!("ajustes-foto-error-sin-dato"));
    let (_, foto) = fotos::ultima(&state.db, &state.history_key, TipoFoto::Partida)
        .await
        .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-foto-error-sin-foto")))?;
    let leidos = foto
        .get(&a.leer.funcion)
        .and_then(|xml| nodo(a, xml))
        .and_then(|n| instancia_en_foto(a, &n))
        .ok_or_else(sin_dato)?;
    let mut valores = BTreeMap::new();
    match campo {
        Some(id) => {
            let c = a
                .campo(&id)
                .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-campo")))?;
            if leido(&leidos, &c.lee).is_none() {
                return Err(sin_dato());
            }
            let v = valor_para_volver(&state.catalogo, c, &leidos).map_err(no_se_puede)?;
            valores.insert(c.id.clone(), v);
        }
        // Todo el ajuste: los campos que se pueden volver a poner.
        None => {
            for c in &a.campos {
                if leido(&leidos, &c.lee).is_some()
                    && let Ok(v) = valor_para_volver(&state.catalogo, c, &leidos)
                {
                    valores.insert(c.id.clone(), v);
                }
            }
        }
    }
    if valores.is_empty() {
        return Err(error_ajuste(CambioError::SinCambios));
    }
    super::preparar_ajuste(PeticionAjuste {
        ajuste,
        instancia: None,
        borrar: false,
        valores,
        esperados: BTreeMap::new(),
    })
    .await
}

#[cfg(test)]
mod tests;
