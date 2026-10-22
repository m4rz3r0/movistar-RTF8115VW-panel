//! Las filas de una lista: su nombre, una celda por columna, su estado y el orden.
//!
//! Las columnas las declara el catálogo (`columnas` del ajuste): un campo de
//! cada instancia, un rango (`externo`: de `externo_desde` a
//! `externo_hasta`) o, en los dispositivos, un dato de la red de casa. Un
//! campo sí/no que se puede escribir es la columna del interruptor. Cada
//! celda sabe cómo se ordena (un número o un texto), y cada fila, el texto
//! con el que se filtra.

use super::super::dispositivos::{conexion, nombre};
use super::super::puertos::{RedPuertos, rango};
use super::super::{DISPOSITIVOS, PUERTOS, RESERVAS};
use crate::ajustes_router::{
    AjusteVista, CampoVista, DispositivoVista, InstanciaVista, RedCasa, mac_comparable, regla_de,
    valor_de,
};
use crate::t;
use crate::ui::comun::EstadoVisual;
use crate::ui::i18n::traducir;
use ajustes::{TipoCampo, ValorLegible};
use std::net::Ipv4Addr;

/// Qué lista es: cambia el nombre, el estado y el detalle de cada fila.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tipo {
    Reglas,
    Reservas,
    Dispositivos,
}

impl Tipo {
    /// La lista del ajuste `a`, si lo es (por su componente).
    pub(crate) fn de(a: &AjusteVista) -> Option<Tipo> {
        match a.componente.as_deref()? {
            PUERTOS => Some(Tipo::Reglas),
            RESERVAS => Some(Tipo::Reservas),
            DISPOSITIVOS => Some(Tipo::Dispositivos),
            _ => None,
        }
    }

    /// Se crean instancias nuevas desde la página (la fila «nueva»).
    pub(crate) fn admite_nuevas(self) -> bool {
        self != Tipo::Dispositivos
    }
}

/// Cómo se ordena una celda: los números antes que los textos.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Orden {
    Numero(u64),
    Texto(String),
}

/// Una celda de una fila.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Celda {
    pub texto: String,
    /// Una segunda línea más tenue (la IP bajo el nombre del dispositivo).
    pub nota: Option<String>,
    /// En letra de ancho fijo (una IP, una MAC, un puerto).
    pub mono: bool,
    /// En un chip (el protocolo, la IP fija).
    pub chip: bool,
    /// Es la columna del interruptor: su valor.
    pub interruptor: Option<bool>,
    pub orden: Orden,
}

impl Celda {
    fn texto(texto: String) -> Celda {
        let orden = Orden::Texto(texto.to_lowercase());
        Celda {
            texto,
            nota: None,
            mono: false,
            chip: false,
            interruptor: None,
            orden,
        }
    }

    fn mono(texto: String, orden: Orden) -> Celda {
        Celda {
            mono: true,
            orden,
            ..Celda::texto(texto)
        }
    }
}

/// Una fila de la lista.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Fila {
    /// Id en la dirección (`/ajustes/<tema>/<id>`): el de la instancia en el
    /// router o, en un dispositivo, su MAC con guiones.
    pub id: String,
    pub nombre: String,
    /// Bajo el nombre (en letra de ancho fijo si `sub_mono`).
    pub sub: Option<String>,
    pub sub_mono: bool,
    /// Una por columna, en el orden del catálogo.
    pub celdas: Vec<Celda>,
    pub estado: EstadoVisual,
    pub texto_estado: String,
    /// Valor del interruptor, si la lista lo tiene.
    pub activa: Option<bool>,
}

/// Orden de una IP (las que no son IPv4, al final).
fn orden_ip(ip: &str) -> Orden {
    Orden::Numero(
        ip.trim()
            .parse::<Ipv4Addr>()
            .map_or(u64::MAX, |ip| u64::from(u32::from(ip))),
    )
}

/// Id en la dirección de un dispositivo: su MAC con guiones.
pub(crate) fn id_de_mac(mac: &str) -> String {
    mac_comparable(mac).replace(':', "-")
}

/// Texto de la cabecera de la columna `col` de la lista `a`.
pub(crate) fn cabecera(a: &AjusteVista, col: &str) -> String {
    traducir(&format!("{}-col-{col}", a.clave), None)
}

/// ¿Es `c` el campo del interruptor (un sí/no que se puede escribir)?
fn es_interruptor(c: &CampoVista) -> bool {
    c.tipo == TipoCampo::Booleano && c.solo_lectura.is_none()
}

/// Celda de la columna `col` de una instancia (un campo o un rango).
fn celda_de_campo(i: &InstanciaVista, col: &str) -> Celda {
    let v = |id: &str| valor_de(&i.campos, id);
    let Some(c) = i.campos.iter().find(|c| c.id == col) else {
        // Un rango: `<col>_desde` y `<col>_hasta`.
        let (desde, hasta) = (v(&format!("{col}_desde")), v(&format!("{col}_hasta")));
        let orden = Orden::Numero(desde.parse().unwrap_or(u64::MAX));
        return match (desde.parse::<u16>(), hasta.parse::<u16>()) {
            (Ok(d), Ok(h)) => Celda::mono(rango((d, h)), orden),
            _ => Celda::mono(desde, orden),
        };
    };
    match (&c.valor, c.tipo) {
        (ValorLegible::Booleano(b), _) if es_interruptor(c) => Celda {
            interruptor: Some(*b),
            orden: Orden::Numero(u64::from(*b)),
            ..Celda::texto(super::super::campo::texto_legible(c, &b.to_string()))
        },
        (_, TipoCampo::Ip) => Celda::mono(v(col), orden_ip(&v(col))),
        (_, TipoCampo::Mac) => Celda::mono(v(col), Orden::Texto(mac_comparable(&v(col)))),
        (ValorLegible::Opcion(k), TipoCampo::Opcion) => Celda {
            chip: true,
            ..Celda::texto(traducir(&format!("{}-{k}", c.clave), None))
        },
        (ValorLegible::Desconocido(x), _) => {
            Celda::texto(t!("ajustes-valor-desconocido", valor = x.clone()))
        }
        _ => Celda::texto(super::super::campo::texto_legible(c, &v(col))),
    }
}

/// Las filas de la lista `a` (de tipo `tipo`) con la red de casa `red`.
pub(crate) fn filas(tipo: Tipo, a: &AjusteVista, red: &RedCasa) -> Vec<Fila> {
    let dispositivos: &[DispositivoVista] = red.dispositivos.as_deref().unwrap_or_default();
    match tipo {
        Tipo::Dispositivos => dispositivos
            .iter()
            .map(|d| fila_de_dispositivo(a, d))
            .collect(),
        Tipo::Reglas => a
            .instancias
            .iter()
            .map(|i| fila_de_regla(a, i, red))
            .collect(),
        Tipo::Reservas => a
            .instancias
            .iter()
            .map(|i| fila_de_reserva(a, i, dispositivos))
            .collect(),
    }
}

/// Las celdas de una instancia, una por columna.
fn celdas(a: &AjusteVista, i: &InstanciaVista) -> Vec<Celda> {
    a.columnas
        .iter()
        .map(|col| celda_de_campo(i, col))
        .collect()
}

/// El interruptor de una instancia, si la lista lo tiene.
fn activa(celdas: &[Celda]) -> Option<bool> {
    celdas.iter().find_map(|c| c.interruptor)
}

/// Una regla: su nombre (o su puerto), el protocolo y el puerto debajo, el
/// dispositivo de destino con su nombre si se conoce, y si está abierta,
/// cerrada o sin destino.
fn fila_de_regla(a: &AjusteVista, i: &InstanciaVista, red: &RedCasa) -> Fila {
    let v = |id: &str| valor_de(&i.campos, id);
    let r = regla_de(i);
    let mut celdas = celdas(a, i);
    let destino = v("destino");
    let red = RedPuertos::de(red);
    let quien = red.nombre_de(&destino);
    for (col, celda) in a.columnas.iter().zip(celdas.iter_mut()) {
        match col.as_str() {
            "destino" if quien.is_some() => {
                celda.nota = Some(destino.clone());
                celda.texto = quien.clone().unwrap_or_default();
                celda.mono = false;
            }
            "externo" => {
                celda.nota = r
                    .as_ref()
                    .filter(|r| r.interno != r.externo)
                    .map(|r| t!("ajuste-puertos-reglas-a-ip", puerto = rango(r.interno)));
            }
            _ => {}
        }
    }
    let perdido = red.destino_perdido(&destino);
    let encendida = activa(&celdas);
    let (estado, clave) = match (perdido, encendida) {
        (true, _) => (
            EstadoVisual::Atencion,
            "ajuste-puertos-reglas-estado-sin-destino",
        ),
        (false, Some(false)) => (
            EstadoVisual::Apagada,
            "ajuste-puertos-reglas-estado-cerrada",
        ),
        (false, _) => (EstadoVisual::Emite, "ajuste-puertos-reglas-estado-abierta"),
    };
    let puerto = r
        .as_ref()
        .map_or_else(|| v("externo_desde"), |r| rango(r.externo));
    Fila {
        id: i.id.clone(),
        nombre: Some(v("nombre"))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| puerto.clone()),
        sub: Some(t!(
            "ajuste-puertos-reglas-hacia",
            puerto = puerto,
            ip = destino
        )),
        sub_mono: false,
        activa: encendida,
        celdas,
        estado,
        texto_estado: traducir(clave, None),
    }
}

/// Una IP fija: el dispositivo (por su MAC) y su IP debajo, y si está
/// desactivada o si el dispositivo está conectado ahora.
fn fila_de_reserva(a: &AjusteVista, i: &InstanciaVista, dispositivos: &[DispositivoVista]) -> Fila {
    let mac = valor_de(&i.campos, "mac");
    let d = dispositivos
        .iter()
        .find(|d| mac_comparable(&d.mac) == mac_comparable(&mac));
    let celdas = celdas(a, i);
    let encendida = activa(&celdas);
    let (estado, clave) = match (encendida, d.is_some_and(|d| d.activo)) {
        (Some(false), _) => (
            EstadoVisual::Apagada,
            "ajuste-red_local-reservas-estado-desactivada",
        ),
        (_, true) => (
            EstadoVisual::Emite,
            "ajuste-red_local-reservas-estado-conectado",
        ),
        (_, false) => (
            EstadoVisual::Apagada,
            "ajuste-red_local-dispositivos-desconectado",
        ),
    };
    Fila {
        id: i.id.clone(),
        nombre: nombre(d.map_or("", |d| d.nombre.as_str())),
        sub: Some(valor_de(&i.campos, "ip")).filter(|ip| !ip.is_empty()),
        sub_mono: true,
        activa: encendida,
        celdas,
        estado,
        texto_estado: traducir(clave, None),
    }
}

/// Un dispositivo: su nombre y su IP debajo, sus columnas y si está
/// conectado. Sin interruptor: el router no tiene nada que encender o
/// apagar en un dispositivo.
fn fila_de_dispositivo(a: &AjusteVista, d: &DispositivoVista) -> Fila {
    let celdas = a
        .columnas
        .iter()
        .map(|col| match col.as_str() {
            "ip" => Celda::mono(d.ip.clone(), orden_ip(&d.ip)),
            "mac" => Celda::mono(d.mac.clone(), Orden::Texto(mac_comparable(&d.mac))),
            "conexion" if !d.activo => {
                Celda::texto(t!("ajuste-red_local-dispositivos-desconectado"))
            }
            "conexion" => Celda::texto(conexion(&d.conexion)),
            "ip_fija" => match &d.ip_fija {
                Some(ip) => Celda {
                    chip: true,
                    ..Celda::mono(ip.clone(), orden_ip(ip))
                },
                None => Celda {
                    orden: Orden::Numero(u64::MAX),
                    ..Celda::texto(t!("ajuste-red_local-dispositivos-sin-ip-fija"))
                },
            },
            _ => Celda::texto(String::new()),
        })
        .collect();
    let (estado, clave) = if d.activo {
        (
            EstadoVisual::Emite,
            "ajuste-red_local-reservas-estado-conectado",
        )
    } else {
        (
            EstadoVisual::Apagada,
            "ajuste-red_local-dispositivos-desconectado",
        )
    };
    Fila {
        id: id_de_mac(&d.mac),
        nombre: nombre(&d.nombre),
        sub: Some(d.ip.clone()).filter(|ip| !ip.trim().is_empty()),
        sub_mono: true,
        activa: None,
        celdas,
        estado,
        texto_estado: traducir(clave, None),
    }
}

/// Por qué se ordena la lista: el nombre o una columna (su posición), y en
/// qué sentido.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Criterio {
    /// `None`: el nombre.
    pub columna: Option<usize>,
    pub ascendente: bool,
}

impl Criterio {
    /// Pulsar la cabecera `columna`: la primera vez, de menor a mayor; otra
    /// vez la misma, al revés.
    pub(crate) fn al_pulsar(self, columna: Option<usize>) -> Criterio {
        Criterio {
            columna,
            ascendente: self.columna != columna || !self.ascendente,
        }
    }
}

/// Ordena `filas` según `c` (sin criterio, como las da el router).
pub(crate) fn ordenar(filas: &mut [Fila], c: Option<Criterio>) {
    let Some(c) = c else {
        return;
    };
    let clave = |f: &Fila| match c.columna {
        None => Orden::Texto(f.nombre.to_lowercase()),
        Some(i) => f
            .celdas
            .get(i)
            .map_or(Orden::Numero(u64::MAX), |x| x.orden.clone()),
    };
    filas.sort_by(|a, b| {
        let o = clave(a).cmp(&clave(b));
        if c.ascendente { o } else { o.reverse() }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fila(nombre: &str, puerto: u64) -> Fila {
        Fila {
            id: nombre.into(),
            nombre: nombre.into(),
            sub: None,
            sub_mono: false,
            celdas: vec![Celda::mono(puerto.to_string(), Orden::Numero(puerto))],
            estado: EstadoVisual::Emite,
            texto_estado: String::new(),
            activa: None,
        }
    }

    fn nombres(f: &[Fila]) -> Vec<&str> {
        f.iter().map(|f| f.nombre.as_str()).collect()
    }

    /// Por número (el 9000 después del 800), por nombre, y al revés al
    /// pulsar otra vez la misma cabecera.
    #[test]
    fn ordenar_por_columna() {
        let mut f = vec![fila("b", 9000), fila("c", 800), fila("a", 27015)];
        ordenar(&mut f, None);
        assert_eq!(nombres(&f), ["b", "c", "a"], "sin criterio, como vienen");
        let c = Criterio {
            columna: None,
            ascendente: true,
        }
        .al_pulsar(Some(0));
        assert!(c.ascendente);
        ordenar(&mut f, Some(c));
        assert_eq!(nombres(&f), ["c", "b", "a"]);
        let c = c.al_pulsar(Some(0));
        assert!(!c.ascendente);
        ordenar(&mut f, Some(c));
        assert_eq!(nombres(&f), ["a", "b", "c"]);
        let c = c.al_pulsar(None);
        assert!(c.ascendente);
        ordenar(&mut f, Some(c));
        assert_eq!(nombres(&f), ["a", "b", "c"]);
    }

    #[test]
    fn id_de_un_dispositivo() {
        assert_eq!(id_de_mac("02:00:00:00:00:0A"), "02-00-00-00-00-0a");
    }
}
