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

/// Campo del id de la instancia en las respuestas de la posición
/// `posicion` (`X_0i` → `X_id`).
fn campo_id(posicion: &str) -> Option<String> {
    posicion
        .strip_suffix("_0i")
        .map(|base| format!("{base}_id"))
}

fn valor(i: &XmlNode, k: &str) -> Option<String> {
    i.text_of(k).map(|v| v.trim().to_owned())
}

/// ¿Es `i` la instancia de `ident`?
fn casa(i: &XmlNode, ident: &[(&str, &str)]) -> bool {
    ident
        .iter()
        .all(|(k, v)| valor(i, k).as_deref() == Some(v.trim()))
}

/// ¿Trae `n` la instancia de `ident`? Una respuesta en la que ninguna
/// instancia trae `fin` (el id de la posición) es el final de la lista.
fn leida(n: &XmlNode, ident: &[(&str, &str)], fin: &str) -> Leida {
    let instancias = instancias(n);
    if !instancias.iter().any(|i| valor(i, fin).is_some()) {
        Leida::Fin
    } else if instancias.iter().any(|i| casa(i, ident)) {
        Leida::Esa
    } else {
        Leida::Otra
    }
}

/// La respuesta `n` con solo la instancia de `ident` (y sus hojas sueltas,
/// como `Result`): quien la use no puede tomar otra.
fn solo_esa(mut n: XmlNode, ident: &[(&str, &str)]) -> XmlNode {
    n.children
        .retain(|c| c.children.is_empty() || casa(c, ident));
    n
}

/// ¿Es `e` el error del final de la lista (`BAD_PARAM`)?
fn fin_de_lista(e: &Error) -> bool {
    matches!(e, Error::CallFailed { error, .. } if error.contains("BAD_PARAM"))
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
/// la instancia de `ident`, y devuelve la respuesta con solo esa instancia.
/// La primera prueba (la del id) puede no existir sin que se acabe la
/// búsqueda; en las demás, la acaba el final de la lista (`BAD_PARAM`, una
/// respuesta sin instancia) o un id ya visto. Cualquier otro error se
/// devuelve al momento. Sin la instancia, [`Error::SinInstancia`].
async fn buscar(
    l: &mut impl Llamar,
    name: &str,
    params: &[(&str, &str)],
    posicion: &str,
    ident: &[(&str, &str)],
) -> Result<XmlNode, Error> {
    let orden = orden(posicion, ident);
    let adivinada = orden.first().copied().filter(|i| *i != 0);
    let fin_campo = campo_id(posicion)
        .or_else(|| ident.first().map(|(k, _)| (*k).to_owned()))
        .unwrap_or_default();
    let mut vistos: Vec<String> = Vec::new();
    for i in orden {
        let pos = i.to_string();
        let mut p: Vec<(&str, &str)> = params
            .iter()
            .filter(|(k, _)| *k != posicion)
            .copied()
            .collect();
        p.push((posicion, &pos));
        let fin = match l.llamar(name, &p).await {
            Ok(n) => match leida(&n, ident, &fin_campo) {
                Leida::Esa => return Ok(solo_esa(n, ident)),
                Leida::Otra => {
                    // Un id ya visto: la lista vuelve a empezar o repite.
                    let ids: Vec<String> = instancias(&n)
                        .into_iter()
                        .filter_map(|x| valor(x, &fin_campo))
                        .collect();
                    let repetido = ids.iter().any(|id| vistos.contains(id));
                    vistos.extend(ids);
                    repetido
                }
                Leida::Fin => true,
            },
            Err(e) if fin_de_lista(&e) => true,
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

    /// Router de mentira: da las respuestas en orden y, acabadas, el
    /// `BAD_PARAM` del final de la lista. Apunta las posiciones pedidas.
    struct Guion {
        respuestas: std::collections::VecDeque<Result<XmlNode, Error>>,
        pedidas: Vec<String>,
    }

    impl Llamar for Guion {
        async fn llamar(&mut self, name: &str, params: &[(&str, &str)]) -> Result<XmlNode, Error> {
            self.pedidas.extend(
                params
                    .iter()
                    .filter(|(k, _)| *k == "x_0i")
                    .map(|(_, v)| (*v).to_owned()),
            );
            self.respuestas.pop_front().unwrap_or_else(|| {
                Err(Error::CallFailed {
                    name: name.to_owned(),
                    result: "1".into(),
                    error: "BAD_PARAM".into(),
                })
            })
        }
    }

    fn guion(respuestas: Vec<Result<XmlNode, Error>>) -> Guion {
        Guion {
            respuestas: respuestas.into(),
            pedidas: Vec::new(),
        }
    }

    fn instancia(hojas: &[(&str, &str)]) -> XmlNode {
        XmlNode {
            tag: "x".into(),
            text: String::new(),
            children: hojas.iter().map(|(k, v)| hoja(k, v)).collect(),
        }
    }

    fn respuesta(instancias: Vec<XmlNode>) -> XmlNode {
        let mut children = instancias;
        children.push(hoja("Result", "0"));
        XmlNode {
            tag: "cbGetX".into(),
            text: String::new(),
            children,
        }
    }

    fn buscar_en(g: &mut Guion, ident: &[(&str, &str)]) -> Result<XmlNode, Error> {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(buscar(g, "cbGetX", &[], "x_0i", ident))
    }

    /// Una respuesta con varias instancias (un router que no hiciera caso
    /// de la posición): sale solo la del id, aunque sea la segunda.
    #[test]
    fn devuelve_solo_la_instancia_del_id() {
        let otra = instancia(&[("x_id", "1"), ("v", "otra")]);
        let esa = instancia(&[("x_id", "0"), ("v", "esa")]);
        let mut g = guion(vec![Ok(respuesta(vec![otra, esa.clone()]))]);
        let n = buscar_en(&mut g, &[("x_id", "0")]).unwrap();
        let con_hijos: Vec<&XmlNode> = n
            .children
            .iter()
            .filter(|c| !c.children.is_empty())
            .collect();
        assert_eq!(con_hijos, vec![&esa]);
        assert_eq!(n.text_of("Result"), Some("0"));
    }

    /// Solo el `BAD_PARAM` o una respuesta sin instancia acaban la lista:
    /// otro error del router a mitad se devuelve.
    #[test]
    fn un_error_a_mitad_no_es_el_final() {
        let fallo = Error::CallFailed {
            name: "cbGetX".into(),
            result: "1".into(),
            error: "otro fallo".into(),
        };
        let mut g = guion(vec![
            Ok(respuesta(vec![instancia(&[("x_id", "1")])])),
            Err(fallo),
        ]);
        let r = buscar_en(&mut g, &[("x_id", "0")]);
        assert!(
            matches!(&r, Err(Error::CallFailed { error, .. }) if error == "otro fallo"),
            "{r:?}"
        );
        let mut g = guion(vec![Ok(respuesta(vec![instancia(&[("x_id", "1")])]))]);
        assert!(matches!(
            buscar_en(&mut g, &[("x_id", "0")]),
            Err(Error::SinInstancia(_))
        ));
    }

    /// El final se decide por el id de la posición (`x_id`), no por el
    /// primer campo de `ident`.
    #[test]
    fn el_final_lo_dice_el_id_de_la_posicion() {
        let mut g = guion(vec![
            Ok(respuesta(vec![instancia(&[("x_id", "1")])])),
            Ok(respuesta(vec![instancia(&[("x_id", "0"), ("aaa", "z")])])),
        ]);
        assert!(buscar_en(&mut g, &[("aaa", "z"), ("x_id", "0")]).is_ok());
    }

    /// Un id que se repite (la lista vuelve a empezar o repite la última)
    /// acaba la búsqueda.
    #[test]
    fn un_id_repetido_acaba_la_busqueda() {
        let uno = || Ok(respuesta(vec![instancia(&[("x_id", "1")])]));
        let mut g = guion(vec![uno(), uno(), uno(), uno()]);
        assert!(matches!(
            buscar_en(&mut g, &[("x_id", "0")]),
            Err(Error::SinInstancia(_))
        ));
        assert_eq!(g.pedidas, ["0", "1"]);
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
        assert_eq!(leida(&pool("0"), &id, "x_id"), Leida::Esa);
        assert_eq!(leida(&pool("1"), &id, "x_id"), Leida::Otra);
        // Solo el índice (la respuesta sin instancia): el final.
        let vacia = XmlNode {
            tag: "cbGetX".into(),
            text: String::new(),
            children: vec![hoja("x_0i", "0"), hoja("Result", "0")],
        };
        assert_eq!(leida(&vacia, &id, "x_id"), Leida::Fin);
        // Valores sueltos en la propia respuesta.
        let suelta = XmlNode {
            tag: "cbGetX".into(),
            text: String::new(),
            children: vec![hoja("x_id", " 0 "), hoja("Result", "0")],
        };
        assert_eq!(leida(&suelta, &id, "x_id"), Leida::Esa);
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
