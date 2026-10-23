//! Lotes de mejoras en el servidor: el estado de cada paso con las lecturas del router, el contador y sus server fns.

mod calculo;
mod preparar;

pub(in crate::ajustes_router) use preparar::preparar_lote;

use super::*;
use ajustes::{
    CatalogoAjustes, ELECCION_DNS, Lote, OpcionEleccion, PasoExpandido, clave_paso,
    partir_referencia, pasos_del_lote,
};
use calculo::{Leido, estado_lote, estado_paso, opcion_leida, pendientes_unicos};

/// Lo leído para unos lotes: sus pasos expandidos y la vista de cada ajuste
/// que tocan.
struct Lecturas {
    expandidos: Vec<Vec<PasoExpandido>>,
    vistas: HashMap<String, AjusteVista>,
}

impl Lecturas {
    /// Lee una vez, en una tanda y de la caché corta si está (con
    /// `fresca`, no), cada ajuste que tocan los pasos de `lotes` o sus
    /// condiciones.
    async fn de(state: &AppState, lotes: &[&Lote], fresca: bool) -> Lecturas {
        let cat = &state.ajustes;
        let expandidos: Vec<Vec<PasoExpandido>> =
            lotes.iter().map(|l| pasos_del_lote(cat, l)).collect();
        let mut a_leer: Vec<&Ajuste> = Vec::new();
        for p in expandidos.iter().flatten() {
            let de_si =
                p.si.as_ref()
                    .and_then(|si| partir_referencia(&si.campo))
                    .map(|(a, _)| a);
            for id in p.destinos.iter().map(|d| d.ajuste.as_str()).chain(de_si) {
                if let Some(a) = cat.ajuste(id)
                    && !a_leer.iter().any(|x| x.id == a.id)
                {
                    a_leer.push(a);
                }
            }
        }
        let claves: Vec<Clave> = a_leer.iter().map(|a| clave(a)).collect();
        if fresca {
            state.lecturas.olvidar(&claves);
        }
        let leidas = state.lecturas.leer(&state.hgu, &claves).await;
        let vistas = a_leer
            .iter()
            .zip(&claves)
            .map(|(a, c)| (a.id.clone(), vista_leida(&state.catalogo, a, leidas.get(c))))
            .collect();
        Lecturas { expandidos, vistas }
    }

    /// El valor leído del campo `campo` del ajuste `ajuste`.
    fn leer(&self, ajuste: &str, campo: &str) -> Leido {
        match self.vistas.get(ajuste) {
            Some(v) if v.estado == EstadoLectura::Leido => v
                .campos
                .iter()
                .find(|c| c.id == campo)
                .map_or(Leido::Fallo, |c| Leido::Valor(c.valor.clone())),
            _ => Leido::Fallo,
        }
    }

    /// Los pasos de cada lote, con su estado.
    fn estados(&self, cat: &CatalogoAjustes) -> Vec<Vec<(PasoExpandido, EstadoPaso)>> {
        let leer = |ajuste: &str, campo: &str| self.leer(ajuste, campo);
        self.expandidos
            .iter()
            .map(|pasos| {
                pasos
                    .iter()
                    .map(|p| {
                        let e = p.elegir.as_deref().and_then(|id| cat.eleccion(id));
                        (p.clone(), estado_paso(p, &leer, e))
                    })
                    .collect()
            })
            .collect()
    }

    /// El proveedor de DNS que ya tiene el router, si es uno de la lista
    /// entero (ver [`opcion_leida`]).
    fn proveedor_dns<'c>(&self, cat: &'c CatalogoAjustes) -> Option<&'c OpcionEleccion> {
        let e = cat.eleccion(ELECCION_DNS)?;
        let p = self
            .expandidos
            .iter()
            .flatten()
            .find(|p| p.elegir.as_deref() == Some(ELECCION_DNS))?;
        opcion_leida(p, &|a, c| self.leer(a, c), e)
    }
}

/// Pasos expandidos de cada lote de `lotes`, con su estado (ver
/// [`Lecturas::de`]).
async fn estados(
    state: &AppState,
    lotes: &[&Lote],
    fresca: bool,
) -> Vec<Vec<(PasoExpandido, EstadoPaso)>> {
    Lecturas::de(state, lotes, fresca)
        .await
        .estados(&state.ajustes)
}

/// Clave del título del ajuste del elemento de un paso repetido por
/// elemento (`ajuste-wifi-red_24-titulo`).
fn detalle(cat: &CatalogoAjustes, p: &PasoExpandido) -> Option<String> {
    let id = p.elemento.as_deref()?;
    let e = cat.elementos_de(p.tema).find(|e| e.id == id)?;
    let a = cat.ajuste(e.ajustes.first()?)?;
    Some(format!("{}-titulo", prefijo_i18n(a)))
}

/// Los lotes del catálogo con el estado de cada paso, y los pasos
/// pendientes sin repetir de todos (el contador de la barra; `None` si no
/// se pudo leer nada).
pub(crate) async fn calcular(state: &AppState, fresca: bool) -> (Vec<LoteVista>, Option<u32>) {
    let lotes: Vec<&Lote> = state.ajustes.lotes().iter().collect();
    let estados = estados(state, &lotes, fresca).await;
    let pendientes = pendientes_unicos(&estados.concat());
    let vistas = lotes
        .iter()
        .zip(&estados)
        .map(|(l, pasos)| LoteVista {
            id: l.id.clone(),
            icono: l.icono.clone(),
            estado: estado_lote(pasos),
            pasos: pasos
                .iter()
                .map(|(p, e)| PasoLoteVista {
                    clave: p.clave(),
                    texto: clave_paso(&p.paso),
                    detalle: detalle(&state.ajustes, p),
                    estado: e.clone(),
                    opcional: p.opcional,
                })
                .collect(),
            avisos: l
                .avisos
                .iter()
                .map(|a| format!("mejoras-aviso-{a}"))
                .collect(),
            consejos: l
                .consejos
                .iter()
                .map(|c| format!("mejoras-consejo-{c}"))
                .collect(),
        })
        .collect();
    (vistas, pendientes)
}

/// Los proveedores de DNS del selector, en el orden del catálogo.
fn proveedores(cat: &CatalogoAjustes) -> Vec<ProveedorVista> {
    let Some(el) = cat.eleccion(ELECCION_DNS) else {
        return Vec::new();
    };
    let defecto = el.defecto().id.as_str();
    el.opciones
        .iter()
        .map(|o| ProveedorVista {
            id: o.id.clone(),
            servidores: ["principal", "secundario"]
                .map(|c| o.valores.get(c).cloned().unwrap_or_default()),
            defecto: o.id == defecto,
        })
        .collect()
}

pub(in crate::ajustes_router) async fn leer_mejoras(
    fresca: bool,
) -> Result<MejorasVista, ServerFnError> {
    // `preparar()` ya pone `no-store`.
    let (state, _) = preparar().await?;
    let (lotes, pendientes) = calcular(&state, fresca).await;
    Ok(MejorasVista {
        lotes,
        pendientes,
        solo_lectura: instancia::solo_lectura(&state.db).await,
        proveedores: proveedores(&state.ajustes),
    })
}

pub(in crate::ajustes_router) async fn contar_mejoras() -> Result<Option<u32>, ServerFnError> {
    let (state, _) = preparar().await?;
    Ok(calcular(&state, false).await.1)
}
