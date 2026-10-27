//! Datos y estados que no son campos: se calculan de las lecturas del tema.
//!
//! Cada tema tiene su módulo con funciones puras. Los nombres que pide el
//! catálogo (`derivado:<nombre>`, ver [`ajustes::DERIVADOS_DATO`] y
//! [`ajustes::DERIVADOS_ESTADO`]) se resuelven aquí con un `match`: uno que
//! no está es un error del catálogo, que ya rechaza `ajustes::comprobar`.
//! Aquí van también las piezas que comparten: el valor de un campo según su
//! vista, el estado de algo que se enciende y se apaga y los datos comunes
//! de la franja de resumen.

mod internet;
mod puertos;
mod seguridad;
mod sistema;
mod telefono;
pub(super) mod wifi;

pub(super) use internet::{conexiones, marcar_conexiones};

use super::*;
use crate::ajustes_router::RecomendacionVista;
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
    /// La conexión a internet del panel entre las leídas (la elegida a mano
    /// o la detectada), si hay.
    pub wan: Option<&'a str>,
    /// Las funciones pendientes de validar en el router real que el servidor
    /// tiene desactivadas.
    pub desactivadas: crate::validacion_real::Desactivadas,
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
pub(super) fn claves_extra(tema: Tema, del_tema: &[&Ajuste]) -> Vec<Clave> {
    match tema {
        Tema::Wifi => wifi::claves_extra(del_tema),
        Tema::Internet => internet::claves_extra(),
        Tema::Puertos | Tema::RedLocal => super::red_local::claves_red(),
        _ => Vec::new(),
    }
}

/// Valor del campo `campo` del ajuste `ajuste` según su vista (`None` si
/// el ajuste no es del tema o no tiene ese campo).
pub(super) fn valor<'a>(ctx: &Contexto<'a>, ajuste: &str, campo: &str) -> Option<&'a ValorLegible> {
    ctx.vista(ajuste)?
        .campos
        .iter()
        .find(|c| c.id == campo)
        .map(|c| &c.valor)
}

/// Un campo sí/no según su vista.
pub(super) fn si_no(ctx: &Contexto, ajuste: &str, campo: &str) -> Option<bool> {
    match valor(ctx, ajuste, campo)? {
        ValorLegible::Booleano(b) => Some(*b),
        _ => None,
    }
}

/// La opción (su clave) o el texto de un campo según su vista.
pub(super) fn texto<'a>(ctx: &Contexto<'a>, ajuste: &str, campo: &str) -> Option<&'a str> {
    match valor(ctx, ajuste, campo)? {
        ValorLegible::Opcion(t) | ValorLegible::Texto(t) => Some(t.as_str()),
        _ => None,
    }
}

/// Instancias de un ajuste de tipo lista (`None` si no se pudo leer).
pub(super) fn instancias<'a>(ctx: &Contexto<'a>, ajuste: &str) -> Option<&'a [InstanciaVista]> {
    ctx.vista(ajuste)
        .filter(|v| v.estado == EstadoLectura::Leido)
        .map(|v| v.instancias.as_slice())
}

/// Estado de algo que se enciende y se apaga: `si` (estado y clave de su
/// texto) si está encendido, `no` si está apagado y «Sin datos» si la
/// lectura no lo dice.
pub(super) fn segun(
    encendido: Option<bool>,
    si: (EstadoVisual, &str),
    no: (EstadoVisual, &str),
) -> (EstadoVisual, String) {
    let (estado, clave) = match encendido {
        Some(true) => si,
        Some(false) => no,
        None => (EstadoVisual::Atencion, "ajustes-estado-desconocido"),
    };
    (estado, clave.to_owned())
}

/// `derivado:segun_lo_recomendado`: «Como se recomienda» si ninguno de sus
/// campos tiene una recomendación pendiente; si alguno la tiene, en ámbar.
fn segun_lo_recomendado(ctx: &Contexto, e: &Elemento) -> (EstadoVisual, String) {
    let pendiente = e
        .ajustes
        .iter()
        .filter_map(|id| ctx.vista(id))
        .flat_map(|v| v.campos.iter())
        .any(|c| c.solo_lectura.is_none() && super::pagina::pendiente(c));
    let (estado, clave) = if pendiente {
        (EstadoVisual::Atencion, "ajustes-estado-con-recomendacion")
    } else {
        (EstadoVisual::Emite, "ajustes-estado-como-se-recomienda")
    };
    (estado, clave.to_owned())
}

/// Un dato de la franja de resumen: su clave, la cifra y la explicación ya
/// traducidas, y si va en ámbar.
pub(super) fn dato_resumen(
    clave: &str,
    cifra: String,
    detalle: String,
    atencion: bool,
) -> DatoResumen {
    DatoResumen {
        clave: clave.to_owned(),
        cifra,
        detalle,
        atencion,
    }
}

/// La explicación fija de un dato del resumen (`<clave>-detalle`).
pub(super) fn detalle_fijo(clave: &str) -> String {
    crate::ui::i18n::traducir(&format!("{clave}-detalle"), None)
}

/// Cifra de un dato del resumen que la lectura no dice: «—».
pub(super) fn sin_dato() -> String {
    t!("ajustes-resumen-sin-dato")
}

/// «Sí» o «No» (o «—» si no se sabe), como cifra del resumen.
pub(super) fn cifra_si_no(b: Option<bool>) -> String {
    match b {
        Some(true) => t!("comun-si"),
        Some(false) => t!("comun-no"),
        None => sin_dato(),
    }
}

/// El último dato de la franja de todos los temas: cuántas recomendaciones
/// hay pendientes (en ámbar si alguna).
pub(super) fn recomendaciones(elementos: &[(&Elemento, &ElementoVista)]) -> DatoResumen {
    let pendientes = elementos
        .iter()
        .filter(|(_, v)| v.recomendacion.is_some())
        .count();
    let clave = "ajustes-resumen-recomendaciones";
    dato_resumen(
        clave,
        pendientes.to_string(),
        detalle_fijo(clave),
        pendientes > 0,
    )
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

/// Dispositivos conectados al elemento (si es una red WiFi), con su nombre
/// si lo tiene.
pub(super) fn dispositivos(ctx: &Contexto, e: &Elemento) -> Vec<Option<String>> {
    wifi::dispositivos(ctx, e)
}

/// Recomendación que no sale de un campo del elemento (las dos solo
/// informan): en la red de invitados, si alguna red de invitados encendida
/// no está separada de la red de casa; en la DMZ, si está activa.
pub(super) fn recomendacion_propia(ctx: &Contexto, e: &Elemento) -> Option<RecomendacionVista> {
    if e.papel.as_deref() == Some("invitados") {
        wifi::invitada_sin_separar(ctx)
    } else if e.estado.as_deref() == Some("derivado:estado_dmz") {
        puertos::dmz_activa(ctx)
    } else {
        None
    }
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
        Tema::Internet => internet::resumen(ctx, elementos),
        Tema::Seguridad => seguridad::resumen(ctx, elementos),
        Tema::Telefono => telefono::resumen(ctx, elementos),
        Tema::Sistema => sistema::resumen(ctx, elementos),
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
        "nombres_ddns" => Ok(internet::nombres_ddns(ctx)),
        "cuentas" => Ok(seguridad::cuentas(ctx)),
        "abierto_redes" => Ok(super::cortafuegos::abierto_redes(ctx)),
        "otras_invitadas" => Ok(wifi::otras_invitadas(ctx)),
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
        "estado_dns" => Ok(internet::estado_dns(ctx)),
        "estado_ddns" => Ok(internet::estado_ddns(ctx)),
        "estado_conexion" => Ok(internet::estado_conexion(ctx)),
        "estado_gestion_remota" => Ok(seguridad::estado_gestion_remota(ctx)),
        "estado_cortafuegos" => Ok(seguridad::estado_cortafuegos(ctx)),
        "estado_linea" => Ok(telefono::estado_linea(ctx)),
        "estado_voz" => Ok(telefono::estado_voz(ctx)),
        "estado_paso_directo" => Ok(telefono::estado_paso_directo(ctx)),
        "estado_hora" => Ok(sistema::estado_hora(ctx)),
        "estado_diagnostico" => Ok(sistema::estado_diagnostico(ctx)),
        "estado_dmz" => Ok(puertos::estado_dmz(ctx)),
        "segun_lo_recomendado" => Ok(segun_lo_recomendado(ctx, e)),
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
            wan: None,
            desactivadas: Default::default(),
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
