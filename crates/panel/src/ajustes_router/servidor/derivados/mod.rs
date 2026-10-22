//! Datos y estados que no son campos: se calculan de las lecturas del tema.
//!
//! Cada tema tiene su módulo con funciones puras. Los nombres que pide el
//! catálogo (`derivado:<nombre>`, ver [`ajustes::DERIVADOS_DATO`] y
//! [`ajustes::DERIVADOS_ESTADO`]) se resuelven aquí con un `match`: uno que
//! no está es un error del catálogo, que ya rechaza `ajustes::comprobar`.

pub(super) mod wifi;

use super::*;
use crate::ui::comun::EstadoVisual;
use ajustes::Elemento;

/// Las lecturas de una página y lo que ya se ha montado con ellas.
pub(super) struct Contexto<'a> {
    /// Ajustes del tema, en el orden del catálogo.
    pub del_tema: &'a [&'a Ajuste],
    /// La lectura de cada uno (`None` si falló), en el mismo orden.
    pub nodos: &'a [Option<&'a XmlNode>],
    /// Su vista, en el mismo orden.
    pub vistas: &'a [AjusteVista],
    /// Todas las lecturas de la tanda, también las que no son de un ajuste.
    pub leidas: &'a HashMap<Clave, Result<XmlNode, hgu::Error>>,
    /// Desde cuándo está apagado cada punto de acceso que se apagó desde
    /// el panel (su id → segundos Unix), del Historial.
    pub apagadas: &'a HashMap<String, i64>,
}

impl<'a> Contexto<'a> {
    fn indice(&self, id: &str) -> Option<usize> {
        self.del_tema.iter().position(|a| a.id == id)
    }

    /// Ajuste `id` del tema.
    pub fn ajuste(&self, id: &str) -> Option<&'a Ajuste> {
        self.indice(id).map(|i| self.del_tema[i])
    }

    /// Lectura del ajuste `id` (`None` si falló o no es del tema).
    pub fn nodo(&self, id: &str) -> Option<&'a XmlNode> {
        self.indice(id).and_then(|i| self.nodos[i])
    }

    /// Vista del ajuste `id`.
    pub fn vista(&self, id: &str) -> Option<&'a AjusteVista> {
        self.indice(id).map(|i| &self.vistas[i])
    }

    /// Respuesta de una lectura extra de la tanda (`None` si falló).
    pub fn leida(&self, c: &Clave) -> Option<&'a XmlNode> {
        self.leidas.get(c).and_then(|r| r.as_ref().ok())
    }
}

/// Nombre que el catálogo no conoce.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Desconocido(pub String);

/// Lecturas que hacen falta además de las de los ajustes del tema.
pub(super) fn claves_extra(del_tema: &[&Ajuste]) -> Vec<Clave> {
    wifi::claves_extra(del_tema)
}

/// Marca en las vistas de los ajustes lo que la interfaz de tarjetas
/// sueltas aún pinta por ajuste: si emite cada red y la nota de su radio.
pub(super) fn marcar(
    del_tema: &[&Ajuste],
    vistas: &mut [AjusteVista],
    nodos: &[Option<&XmlNode>],
    leidas: &HashMap<Clave, Result<XmlNode, hgu::Error>>,
) {
    let listas: Vec<(String, Option<&XmlNode>)> = wifi::claves_de_listas(del_tema)
        .into_iter()
        .map(|(radio, c)| (radio, leidas.get(&c).and_then(|r| r.as_ref().ok())))
        .collect();
    wifi::marcar_emision(del_tema, vistas, nodos, &listas);
}

/// Nombre de la red, si el elemento es una red WiFi.
pub(super) fn ssid(ctx: &Contexto, e: &Elemento) -> Option<String> {
    wifi::ssid(ctx, e)
}

/// Desde cuándo está apagado el elemento, si se apagó desde el panel.
pub(super) fn apagada_desde(ctx: &Contexto, e: &Elemento) -> Option<i64> {
    wifi::apagada_desde(ctx, e)
}

/// Nota del pie de un elemento, ya traducida.
pub(super) fn nota(ctx: &Contexto, e: &Elemento) -> Option<String> {
    wifi::nota(ctx, e)
}

/// Franja de resumen del tema (vacía en los que aún no la tienen).
pub(super) fn resumen(
    tema: Tema,
    ctx: &Contexto,
    elementos: &[(&Elemento, &ElementoVista)],
) -> Vec<DatoResumen> {
    match tema {
        Tema::Wifi => wifi::resumen(ctx, elementos),
        _ => Vec::new(),
    }
}

/// Dato `derivado:<nombre>` de un elemento: `Ok(None)` si las lecturas no
/// lo dicen.
pub(super) fn dato(
    nombre: &str,
    ctx: &Contexto,
    e: &Elemento,
) -> Result<Option<ValorLegible>, Desconocido> {
    match nombre {
        "canal_y_ancho" => Ok(wifi::canal_y_ancho(ctx, e)),
        "conectados" => Ok(wifi::conectados(ctx, e).map(|n| ValorLegible::Texto(n.to_string()))),
        otro => Err(Desconocido(otro.to_owned())),
    }
}

/// Estado `derivado:<nombre>` de un elemento y la clave de traducción de su
/// texto.
pub(super) fn estado(
    nombre: &str,
    ctx: &Contexto,
    e: &Elemento,
) -> Result<(EstadoVisual, String), Desconocido> {
    match nombre {
        "estado_red" => Ok(wifi::estado(ctx, e)),
        otro => Err(Desconocido(otro.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ajustes::{CatalogoAjustes, DERIVADOS_DATO, DERIVADOS_ESTADO};

    /// Cada nombre que admite el catálogo se resuelve; uno que no, es un
    /// error.
    #[test]
    fn cada_derivado_del_catalogo_se_resuelve() {
        let cat = CatalogoAjustes::load().unwrap();
        let e = cat.elementos()[0].clone();
        let leidas = HashMap::new();
        let apagadas = HashMap::new();
        let ctx = Contexto {
            del_tema: &[],
            nodos: &[],
            vistas: &[],
            leidas: &leidas,
            apagadas: &apagadas,
        };
        for n in DERIVADOS_DATO {
            assert!(dato(n, &ctx, &e).is_ok(), "{n}");
        }
        for n in DERIVADOS_ESTADO {
            assert!(estado(n, &ctx, &e).is_ok(), "{n}");
        }
        assert_eq!(dato("nada", &ctx, &e), Err(Desconocido("nada".into())));
        assert_eq!(estado("nada", &ctx, &e), Err(Desconocido("nada".into())));
    }
}
