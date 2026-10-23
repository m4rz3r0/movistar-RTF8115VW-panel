//! Lectura de una instancia de lista por su id, recorriendo posiciones.
//!
//! Las lecturas de lista del router eligen la instancia por su **posición**
//! (`X_0i`), y las escrituras, por su **id** (`X_id`). No tienen por qué
//! coincidir: en el router real, la posición 0 del pool DHCP es el pool de
//! id 1 y la 1, el de id 0. [`Client::call_por_id`] y [`Turno::call_por_id`]
//! leen la posición que coincide con el id y, si no es esa instancia, las
//! demás desde la 0, hasta dar con la que tiene el id o con el final de la
//! lista (el router responde con error o sin instancia).

use super::{Client, Turno};
use crate::error::Error;
use crate::xml::XmlNode;

/// Posiciones que se recorren como mucho buscando una instancia.
pub const TOPE_POSICIONES: usize = 32;

/// Lo que da una lectura por posición: la instancia buscada, otra o el final
/// de la lista.
#[derive(Debug, PartialEq, Eq)]
enum Leida {
    Esa,
    Otra,
    Fin,
}

/// Instancias de una respuesta: sus elementos con hijos o, si trae los
/// valores sueltos, la propia respuesta.
fn instancias(n: &XmlNode) -> Vec<&XmlNode> {
    let con_hijos: Vec<&XmlNode> = n
        .children
        .iter()
        .filter(|c| !c.children.is_empty())
        .collect();
    if con_hijos.is_empty() {
        vec![n]
    } else {
        con_hijos
    }
}

/// ¿Es `n` la instancia de `ident`? Una respuesta sin el primer campo de
/// `ident` en ninguna instancia es el final de la lista.
fn leida(n: &XmlNode, ident: &[(&str, &str)]) -> Leida {
    let valor = |i: &XmlNode, k: &str| i.text_of(k).map(str::trim).map(str::to_owned);
    let instancias = instancias(n);
    let Some((primero, _)) = ident.first() else {
        return Leida::Fin;
    };
    if !instancias.iter().any(|i| valor(i, primero).is_some()) {
        return Leida::Fin;
    }
    if instancias.iter().any(|i| {
        ident
            .iter()
            .all(|(k, v)| valor(i, k).as_deref() == Some(v.trim()))
    }) {
        Leida::Esa
    } else {
        Leida::Otra
    }
}

/// Posiciones en el orden en que se prueban: primero la que coincide con el
/// id de la instancia (`X_id` de `ident`, con `X_0i` = `posicion`), que es
/// lo normal; después, las demás desde la 0.
fn orden(posicion: &str, ident: &[(&str, &str)]) -> Vec<usize> {
    let id = posicion
        .strip_suffix("_0i")
        .map(|base| format!("{base}_id"))
        .and_then(|k| ident.iter().find(|(c, _)| *c == k))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .filter(|i| *i < TOPE_POSICIONES);
    id.into_iter()
        .chain((0..TOPE_POSICIONES).filter(|i| Some(*i) != id))
        .collect()
}

/// Quien hace las lecturas: el cliente o un turno.
trait Llamar {
    async fn llamar(&mut self, name: &str, params: &[(&str, &str)]) -> Result<XmlNode, Error>;
}

impl Llamar for &Client {
    async fn llamar(&mut self, name: &str, params: &[(&str, &str)]) -> Result<XmlNode, Error> {
        self.call(name, params).await
    }
}

impl Llamar for Turno<'_> {
    async fn llamar(&mut self, name: &str, params: &[(&str, &str)]) -> Result<XmlNode, Error> {
        self.call(name, params).await
    }
}

/// Lee `name` con `params` en cada posición (`posicion`) de [`orden`] hasta
/// la instancia de `ident`. La primera prueba (la del id) puede fallar o no
/// existir sin que se acabe la búsqueda; en las demás, el final de la lista
/// la acaba. Un error que no es del propio router (red, sesión) se devuelve
/// al momento. Sin la instancia, [`Error::SinInstancia`].
async fn buscar(
    l: &mut impl Llamar,
    name: &str,
    params: &[(&str, &str)],
    posicion: &str,
    ident: &[(&str, &str)],
) -> Result<XmlNode, Error> {
    let orden = orden(posicion, ident);
    let adivinada = orden.first().copied().filter(|i| *i != 0);
    for i in orden {
        let pos = i.to_string();
        let mut p: Vec<(&str, &str)> = params
            .iter()
            .filter(|(k, _)| *k != posicion)
            .copied()
            .collect();
        p.push((posicion, &pos));
        let fin = match l.llamar(name, &p).await {
            Ok(n) => match leida(&n, ident) {
                Leida::Esa => return Ok(n),
                Leida::Otra => false,
                Leida::Fin => true,
            },
            Err(Error::CallFailed { .. }) => true,
            Err(e) => return Err(e),
        };
        if fin && Some(i) != adivinada {
            break;
        }
    }
    let ident: Vec<String> = ident.iter().map(|(k, v)| format!("{k}={v}")).collect();
    Err(Error::SinInstancia(format!(
        "{name}: ninguna posición tiene la instancia {}",
        ident.join(", ")
    )))
}

impl Client {
    /// La instancia de la lista `name` cuyos campos valen `ident` (su id,
    /// `X_id`), buscándola por la posición `posicion` (`X_0i`): ver el
    /// módulo. `params`, los demás parámetros de la lectura.
    pub async fn call_por_id(
        &self,
        name: &str,
        params: &[(&str, &str)],
        posicion: &str,
        ident: &[(&str, &str)],
    ) -> Result<XmlNode, Error> {
        let mut c = self;
        buscar(&mut c, name, params, posicion, ident).await
    }
}

impl Turno<'_> {
    /// Como [`Client::call_por_id`], dentro del turno.
    pub async fn call_por_id(
        &mut self,
        name: &str,
        params: &[(&str, &str)],
        posicion: &str,
        ident: &[(&str, &str)],
    ) -> Result<XmlNode, Error> {
        buscar(self, name, params, posicion, ident).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hoja(tag: &str, text: &str) -> XmlNode {
        XmlNode {
            tag: tag.into(),
            text: text.into(),
            children: Vec::new(),
        }
    }

    fn pool(id: &str) -> XmlNode {
        XmlNode {
            tag: "cbGetX".into(),
            text: String::new(),
            children: vec![
                XmlNode {
                    tag: "x".into(),
                    text: String::new(),
                    children: vec![hoja("x_0i", "0"), hoja("x_id", id)],
                },
                hoja("Result", "0"),
            ],
        }
    }

    #[test]
    fn primero_la_posicion_del_id() {
        let o = orden("x_0i", &[("x_id", "1")]);
        assert_eq!(&o[..3], &[1, 0, 2]);
        assert_eq!(o.len(), TOPE_POSICIONES);
        let o = orden("x_0i", &[("x_id", "0")]);
        assert_eq!(&o[..2], &[0, 1]);
        assert_eq!(o.len(), TOPE_POSICIONES);
        // Un id que no es una posición: desde la 0.
        assert_eq!(orden("x_0i", &[("x_id", "-1")])[0], 0);
        assert_eq!(orden("x_0i", &[("otro", "3")])[0], 0);
    }

    #[test]
    fn la_instancia_otra_o_el_final() {
        let id = [("x_id", "0")];
        assert_eq!(leida(&pool("0"), &id), Leida::Esa);
        assert_eq!(leida(&pool("1"), &id), Leida::Otra);
        // Solo el índice (la respuesta sin instancia): el final.
        let vacia = XmlNode {
            tag: "cbGetX".into(),
            text: String::new(),
            children: vec![hoja("x_0i", "0"), hoja("Result", "0")],
        };
        assert_eq!(leida(&vacia, &id), Leida::Fin);
        // Valores sueltos en la propia respuesta.
        let suelta = XmlNode {
            tag: "cbGetX".into(),
            text: String::new(),
            children: vec![hoja("x_id", " 0 "), hoja("Result", "0")],
        };
        assert_eq!(leida(&suelta, &id), Leida::Esa);
    }

    /// Con los dos pools de las fixtures (posición 0 = id 1, posición 1 =
    /// id 0), cada id da su pool; uno que no está, [`Error::SinInstancia`].
    #[tokio::test]
    async fn busca_el_pool_por_su_id_en_el_simulador() {
        use crate::ClientConfig;
        use router_sim::{Sim, SimConfig};
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
        let sim = Sim::start(SimConfig::new("u", "clave-de-prueba", dir)).await;
        let mut cfg = ClientConfig::new(&sim.base_url, "u", "clave-de-prueba");
        cfg.min_gap = std::time::Duration::from_millis(1);
        let c = Client::new(cfg);
        let pos = "rtm_cfg_dhcpv4_svr_pool_0i";
        let inicio = |n: &XmlNode| {
            n.get("rtm_cfg_dhcpv4_svr_pool")
                .and_then(|i| i.text_of("ipv4_addr_min"))
                .map(str::to_owned)
        };
        let lee = async |id: &str| {
            c.call_por_id(
                "cbGetDhcpv4ServerPool",
                &[],
                pos,
                &[("rtm_cfg_dhcpv4_svr_pool_id", id)],
            )
            .await
        };
        assert_eq!(
            inicio(&lee("0").await.unwrap()).as_deref(),
            Some("192.168.1.100")
        );
        assert_eq!(
            inicio(&lee("1").await.unwrap()).as_deref(),
            Some("192.168.1.230")
        );
        assert!(matches!(lee("2").await, Err(Error::SinInstancia(_))));
        let mut t = c.turno().await;
        let n = t
            .call_por_id(
                "cbGetDhcpv4ServerPool",
                &[],
                pos,
                &[("rtm_cfg_dhcpv4_svr_pool_id", "0")],
            )
            .await
            .unwrap();
        assert_eq!(inicio(&n).as_deref(), Some("192.168.1.100"));
    }
}
