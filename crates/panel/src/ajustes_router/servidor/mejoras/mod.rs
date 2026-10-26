//! Lotes de mejoras en el servidor: el estado de cada paso con las lecturas del router, el contador y sus server fns.

mod calculo;
mod dns;
mod preparar;

pub(in crate::ajustes_router) use preparar::preparar_lote;

use super::*;
use crate::ajustes_router::MANTENER_DNS;
use ajustes::{
    CatalogoAjustes, ELECCION_DNS, Lote, OpcionEleccion, PasoExpandido, clave_paso,
    partir_referencia, pasos_del_lote,
};
use calculo::{Leido, estado_lote, estado_paso, pendientes_unicos};
use dns::{estado_dns, opcion_leida};
use std::net::Ipv4Addr;

/// Texto del paso de los DNS hecho con DNS propios, en vez de «Hecho».
const NOTA_DNS_PROPIOS: &str = "mejoras-paso-estado-dns-propios";
/// Texto del paso de los DNS hecho con solo el principal de un proveedor,
/// con su nombre (`$proveedor`).
const NOTA_DNS_SOLO_PRINCIPAL: &str = "mejoras-paso-estado-dns-solo-principal";

/// Lo leído para unos lotes: sus pasos expandidos, la vista de cada ajuste
/// que tocan y la IP del router en casa (para saber si los DNS son propios).
struct Lecturas {
    expandidos: Vec<Vec<PasoExpandido>>,
    vistas: HashMap<String, AjusteVista>,
    ip_router: Option<Ipv4Addr>,
}

impl Lecturas {
    /// Lee una vez, en una tanda y de la caché corta si está (con
    /// `fresca`, no), cada ajuste que tocan los pasos de `lotes` o sus
    /// condiciones.
    async fn de(state: &AppState, lotes: &[&Lote], fresca: bool) -> Lecturas {
        let cat = &state.ajustes;
        let expandidos: Vec<Vec<PasoExpandido>> =
            lotes.iter().map(|l| pasos_que_escribe(state, l)).collect();
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
        let todas: Vec<Clave> = claves
            .iter()
            .cloned()
            .chain(std::iter::once(super::red_local::clave_red_casa()))
            .collect();
        if fresca {
            state.lecturas.olvidar(&todas);
        }
        let leidas = state.lecturas.leer(&state.hgu, &todas).await;
        let vistas = a_leer
            .iter()
            .zip(&claves)
            .map(|(a, c)| (a.id.clone(), vista_leida(&state.catalogo, a, leidas.get(c))))
            .collect();
        Lecturas {
            expandidos,
            vistas,
            ip_router: super::red_local::ip_router(&leidas),
        }
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
                        let estado = match e {
                            Some(e) if e.id == ELECCION_DNS => {
                                estado_dns(cat, p, &leer, e, self.ip_router)
                            }
                            _ => estado_paso(p, &leer, e),
                        };
                        (p.clone(), estado)
                    })
                    .collect()
            })
            .collect()
    }

    /// El paso de los DNS de estos lotes, si lo tienen.
    fn paso_dns(&self) -> Option<&PasoExpandido> {
        self.expandidos
            .iter()
            .flatten()
            .find(|p| p.elegir.as_deref() == Some(ELECCION_DNS))
    }

    /// El proveedor de DNS que ya tiene el router, si es uno de la lista
    /// entero (ver [`opcion_leida`]).
    fn proveedor_dns<'c>(&self, cat: &'c CatalogoAjustes) -> Option<&'c OpcionEleccion> {
        let e = cat.eleccion(ELECCION_DNS)?;
        opcion_leida(self.paso_dns()?, &|a, c| self.leer(a, c), e)
    }

    /// ¿Tiene el router DNS propios, que no son de ningún proveedor de la
    /// lista ni del router o del operador (ver [`dns::propios`])?
    fn dns_propios(&self, cat: &CatalogoAjustes) -> bool {
        let Some(p) = self.paso_dns() else {
            return false;
        };
        self.proveedor_dns(cat).is_none()
            && dns::propios(cat, p, &|a, c| self.leer(a, c), self.ip_router) == Some(true)
    }

    /// El proveedor de la lista del que el router tiene solo el DNS
    /// principal (ver [`dns::solo_principal`]).
    fn dns_solo_principal<'c>(&self, cat: &'c CatalogoAjustes) -> Option<&'c OpcionEleccion> {
        let e = cat.eleccion(ELECCION_DNS)?;
        dns::solo_principal(cat, self.paso_dns()?, &|a, c| self.leer(a, c), e)
    }

    /// Los valores que tiene ahora el router en los campos del paso de los
    /// DNS, como una opción más: «Mantener los actuales». `None` si alguno
    /// no se leyó.
    fn dns_actuales(&self) -> Option<OpcionEleccion> {
        let p = self.paso_dns()?;
        let mut valores = BTreeMap::new();
        for d in &p.destinos {
            let Leido::Valor(ValorLegible::Texto(t)) = self.leer(&d.ajuste, &d.campo) else {
                return None;
            };
            valores.insert(d.campo.clone(), t);
        }
        Some(OpcionEleccion {
            id: MANTENER_DNS.to_owned(),
            defecto: false,
            valores,
            fuentes: Vec::new(),
            respaldo: None,
        })
    }
}

/// Los pasos de `l` que el panel escribe: sin los que tocan un campo cuya
/// función no está comprobada en el router real (la lista blanca del
/// catálogo), que no salen en Mejoras.
fn pasos_que_escribe(state: &AppState, l: &Lote) -> Vec<PasoExpandido> {
    let comprobado = |d: &ajustes::Destino| {
        state
            .ajustes
            .ajuste(&d.ajuste)
            .and_then(|a| a.campo(&d.campo))
            .and_then(|c| c.escribe.as_ref())
            .and_then(|e| state.catalogo.funcion(&e.funcion))
            .is_some_and(|f| f.comprobada())
    };
    pasos_del_lote(&state.ajustes, l)
        .into_iter()
        .filter(|p| p.destinos.iter().all(comprobado))
        .collect()
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
    // Un lote sin ningún paso que el panel escriba no sale.
    let lotes: Vec<&Lote> = state
        .ajustes
        .lotes()
        .iter()
        .filter(|l| !pasos_que_escribe(state, l).is_empty())
        .collect();
    let lecturas = Lecturas::de(state, &lotes, fresca).await;
    let estados = lecturas.estados(&state.ajustes);
    // El paso de los DNS hecho con el principal de un proveedor (sin su
    // secundario) o con DNS propios lo dice: el proveedor o «DNS propios».
    let solo_principal = lecturas.dns_solo_principal(&state.ajustes);
    let propios = lecturas.dns_propios(&state.ajustes);
    let nota = |p: &PasoExpandido, e: &EstadoPaso| -> (Option<String>, Option<String>) {
        if *e != EstadoPaso::Hecho || p.elegir.as_deref() != Some(ELECCION_DNS) {
            return (None, None);
        }
        match solo_principal {
            Some(o) => (Some(NOTA_DNS_SOLO_PRINCIPAL.to_owned()), Some(o.id.clone())),
            None if propios => (Some(NOTA_DNS_PROPIOS.to_owned()), None),
            None => (None, None),
        }
    };
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
                .map(|(p, e)| {
                    let (nota, nota_proveedor) = nota(p, e);
                    let (fuentes, respaldo) = super::fuentes::del_paso(&state.ajustes, p);
                    PasoLoteVista {
                        clave: p.clave(),
                        texto: clave_paso(&p.paso),
                        detalle: detalle(&state.ajustes, p),
                        estado: e.clone(),
                        nota,
                        nota_proveedor,
                        opcional: p.opcional,
                        fuentes,
                        respaldo,
                    }
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

/// Los proveedores de DNS del selector: primero «Mantener los actuales»
/// ([`MANTENER_DNS`]) y después los del catálogo, en su orden.
fn proveedores(cat: &CatalogoAjustes) -> Vec<ProveedorVista> {
    let Some(el) = cat.eleccion(ELECCION_DNS) else {
        return Vec::new();
    };
    let defecto = el.defecto().id.as_str();
    let mantener = ProveedorVista {
        id: MANTENER_DNS.to_owned(),
        servidores: [String::new(), String::new()],
        defecto: false,
    };
    std::iter::once(mantener)
        .chain(el.opciones.iter().map(|o| ProveedorVista {
            id: o.id.clone(),
            servidores:
                ["principal", "secundario"].map(|c| o.valores.get(c).cloned().unwrap_or_default()),
            defecto: o.id == defecto,
        }))
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
