//! Lecturas del router para pintar páginas (Ajustes): una caché corta por
//! lectura y la tanda que lee lo que falta en un solo turno del cliente.
//!
//! - **Solo para mostrar.** Lo que decide qué se escribe (el «antes» de
//!   `preparar_ajuste`, el gestor de cambios, el sondeo) lee siempre el
//!   router con [`hgu::Client::call`], nunca de aquí.
//! - **Caducidad corta** ([`CADUCIDAD_LECTURAS`]): un cambio hecho fuera del
//!   panel (la app del operador, un reinicio) se ve como mucho tan tarde.
//! - **Invalidación inmediata.** Cada entrada recuerda la generación con que
//!   empezó su lectura: la del propio caché ([`CacheLecturas::invalidar`],
//!   que llama el gestor al verificar y al terminar un cambio) y el contador
//!   de escrituras del cliente con que se lee ([`hgu::Client::escrituras`],
//!   que cambia con cualquier escritura o reconfiguración). Si cualquiera de las dos cambió,
//!   la entrada no vale (ni se guarda una lectura que empezó antes).
//! - **Con los secretos, solo en memoria.** Desde la Tarea 11 las páginas
//!   de Ajustes muestran el valor de sus secretos (censurado, con un ojo):
//!   la respuesta se guarda tal cual la da el router, en memoria del
//!   proceso y como mucho [`CADUCIDAD_LECTURAS`]. Nunca va a disco ni al
//!   registro; quien la usa decide qué enseña (los secretos, solo en su
//!   campo).

use hgu::xml::XmlNode;
use std::{
    collections::HashMap,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub use crate::ajustes_router::CADUCIDAD_LECTURAS;

/// Error de las lecturas de una tanda que no se intentan porque el router
/// no respondió a una anterior (ver [`CacheLecturas::leer`]).
pub const NO_INTENTADA: &str = "no se intentó: el router no respondió a la lectura anterior";

/// Una lectura: función y parámetros, ordenados por nombre (el orden no
/// cambia lo que se lee).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Clave {
    pub funcion: String,
    pub params: Vec<(String, String)>,
}

impl Clave {
    pub fn new<K: AsRef<str>, V: AsRef<str>>(funcion: &str, params: &[(K, V)]) -> Clave {
        let mut params: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (k.as_ref().to_owned(), v.as_ref().to_owned()))
            .collect();
        params.sort();
        Clave {
            funcion: funcion.to_owned(),
            params,
        }
    }
}

/// Generación de lo leído: la del caché y el contador de escrituras del
/// cliente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Generacion {
    propia: u64,
    cliente: u64,
}

struct Entrada {
    leida: Instant,
    generacion: Generacion,
    nodo: XmlNode,
}

/// Caché corta de lecturas para mostrar. Una por proceso, en
/// [`AppState::lecturas`](crate::state::AppState::lecturas).
pub struct CacheLecturas {
    caducidad: Duration,
    propia: AtomicU64,
    mapa: Mutex<HashMap<Clave, Entrada>>,
}

impl CacheLecturas {
    /// Caché de lecturas que valen `caducidad`.
    pub fn new(caducidad: Duration) -> CacheLecturas {
        CacheLecturas {
            caducidad,
            propia: AtomicU64::new(0),
            mapa: Mutex::new(HashMap::new()),
        }
    }

    /// Olvida todo lo guardado y lo que se esté leyendo ahora.
    pub fn invalidar(&self) {
        self.propia.fetch_add(1, Ordering::SeqCst);
        self.mapa().clear();
    }

    fn generacion(&self, hgu: &hgu::Client) -> Generacion {
        Generacion {
            propia: self.propia.load(Ordering::SeqCst),
            cliente: hgu.escrituras(),
        }
    }

    fn mapa(&self) -> MutexGuard<'_, HashMap<Clave, Entrada>> {
        self.mapa.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// La lectura `clave` si está guardada y aún vale.
    fn buscar(&self, hgu: &hgu::Client, clave: &Clave) -> Option<XmlNode> {
        let actual = self.generacion(hgu);
        let mut mapa = self.mapa();
        let vale = mapa
            .get(clave)
            .is_some_and(|e| e.generacion == actual && e.leida.elapsed() < self.caducidad);
        if vale {
            mapa.get(clave).map(|e| e.nodo.clone())
        } else {
            mapa.remove(clave);
            None
        }
    }

    /// Guarda `nodo` si nada ha cambiado desde `generacion` (cuando empezó
    /// su lectura). También retira lo que ya no vale.
    fn guardar(&self, hgu: &hgu::Client, generacion: Generacion, clave: Clave, nodo: XmlNode) {
        if generacion != self.generacion(hgu) {
            return;
        }
        let mut mapa = self.mapa();
        mapa.retain(|_, e| e.generacion == generacion && e.leida.elapsed() < self.caducidad);
        mapa.insert(
            clave,
            Entrada {
                leida: Instant::now(),
                generacion,
                nodo,
            },
        );
    }

    /// Las lecturas `claves` para mostrar, tal cual: las guardadas
    /// que aún valen y, en una sola tanda (un turno de `hgu`: nadie llama al
    /// router entre ellas), las que faltan. Las que fallan se registran (una
    /// vez cada una, aquí) y no se guardan.
    ///
    /// Si una lectura falla porque el router no responde (error de red o
    /// tiempo agotado), las que quedan no se intentan: fallan al momento
    /// ([`NO_INTENTADA`]) y la tanda suelta el turno, para que el gestor de
    /// cambios (su vigilancia, una reversión) y el sondeo no esperen un
    /// plazo por cada una. Un error de la propia función (el router responde
    /// con `Result` distinto de 0) no corta la tanda.
    pub async fn leer(
        &self,
        hgu: &hgu::Client,
        claves: &[Clave],
    ) -> HashMap<Clave, Result<XmlNode, hgu::Error>> {
        let mut out = HashMap::new();
        let mut faltan = Vec::new();
        for c in claves {
            if out.contains_key(c) || faltan.contains(c) {
                continue;
            }
            match self.buscar(hgu, c) {
                Some(n) => {
                    out.insert(c.clone(), Ok(n));
                }
                None => faltan.push(c.clone()),
            }
        }
        if faltan.is_empty() {
            return out;
        }
        let generacion = self.generacion(hgu);
        let mut turno = hgu.turno().await;
        let mut sin_router = false;
        for c in faltan {
            // Otra tanda que tenía el turno antes pudo leerla ya.
            if let Some(n) = self.buscar(hgu, &c) {
                out.insert(c, Ok(n));
                continue;
            }
            if sin_router {
                tracing::warn!("no se lee {}: {NO_INTENTADA}", c.funcion);
                out.insert(c, Err(hgu::Error::Network(NO_INTENTADA.to_owned())));
                continue;
            }
            let params: Vec<(&str, &str)> = c
                .params
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let r = turno
                .call(&c.funcion, &params)
                .await
                .inspect(|n| self.guardar(hgu, generacion, c.clone(), n.clone()));
            if let Err(e) = &r {
                tracing::warn!("no se puede leer {}: {e}", c.funcion);
                sin_router = matches!(e, hgu::Error::Network(_));
            }
            out.insert(c, r);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodo(texto: &str) -> XmlNode {
        XmlNode {
            tag: "cbGetX".into(),
            text: texto.into(),
            children: Vec::new(),
        }
    }

    #[test]
    fn la_clave_no_depende_del_orden_de_los_parametros() {
        assert_eq!(
            Clave::new("cbGetX", &[("b", "2"), ("a", "1")]),
            Clave::new("cbGetX", &[("a", "1"), ("b", "2")])
        );
        assert_ne!(
            Clave::new("cbGetX", &[("a", "1")]),
            Clave::new("cbGetX", &[("a", "2")])
        );
    }

    #[test]
    fn guarda_hasta_que_caduca() {
        let h = hgu::Client::sin_configurar();
        let c = CacheLecturas::new(Duration::from_millis(50));
        let k = Clave::new::<&str, &str>("cbGetX", &[]);
        c.guardar(&h, c.generacion(&h), k.clone(), nodo("1"));
        assert_eq!(c.buscar(&h, &k), Some(nodo("1")));
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(c.buscar(&h, &k), None);
    }

    #[test]
    fn invalidar_olvida_lo_guardado_y_lo_que_se_esta_leyendo() {
        let h = hgu::Client::sin_configurar();
        let c = CacheLecturas::new(CADUCIDAD_LECTURAS);
        let k = Clave::new::<&str, &str>("cbGetX", &[]);
        c.guardar(&h, c.generacion(&h), k.clone(), nodo("1"));
        let en_curso = c.generacion(&h);
        c.invalidar();
        assert_eq!(c.buscar(&h, &k), None);
        // Una lectura que empezó antes de invalidar no se guarda.
        c.guardar(&h, en_curso, k.clone(), nodo("viejo"));
        assert_eq!(c.buscar(&h, &k), None);
        c.guardar(&h, c.generacion(&h), k.clone(), nodo("2"));
        assert_eq!(c.buscar(&h, &k), Some(nodo("2")));
    }
}
