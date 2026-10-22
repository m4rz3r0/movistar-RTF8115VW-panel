//! «Puertos abiertos» ([`Puertos`]): las reglas de apertura de puertos en
//! una lista (apilada en pantallas estrechas) con su nombre, el puerto de
//! internet y su protocolo, el dispositivo al que van, si está activa,
//! «Editar» y «Borrar». Una regla cuyo dispositivo de destino ya no está en
//! la red lleva un aviso ([`destino_inexistente`]).
//!
//! «Abrir un puerto» es un formulario corto: nombre, protocolo (también
//! «TCP y UDP», que son dos reglas en un solo cambio), el puerto o el rango
//! de internet, el dispositivo (de la lista, u otra IP) y, plegado, el
//! puerto del dispositivo si no es el mismo. Antes de enviar, la página
//! comprueba la regla como el servidor ([`validar_regla`]) y dice el error
//! junto al campo. Todo se guarda con el diálogo de grupo.

mod formulario;
mod lista;

pub(crate) use lista::Puertos;

use super::Foco;
use super::campo::{Campo, DESCONOCIDO, validar};
use super::dispositivos::{aviso_error, nombre};
use super::tarjeta::{ayuda, cabecera, detalles_tecnicos, dom, enfocar};
use crate::ajustes_router::{
    AMBOS, AjusteVista, CampoVista, DispositivoVista, EstadoLectura, GrupoVista, IDENTIFICAN_REGLA,
    InstanciaVista, OpcionVista, PeticionAjuste, Protocolo, ReglaPuertos, destino_inexistente,
    leer_dispositivos, leer_red_casa, leer_reservas, preparar_ajuste, regla_de, validar_regla,
    valor_de,
};
use crate::t;
use crate::ui::comun::{EstadoVisual, Insignia};
use crate::ui::explorador::texto_error;
use crate::ui::grupo::DialogoGrupo;
use crate::ui::i18n::traducir;
use ajustes::{TipoCampo, ValorLegible};
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;

/// Prefijo de los textos propios del componente.
const CLAVE: &str = "ajuste-puertos-reglas";

/// Valor del `<select>` de dispositivos que pide escribir la IP.
const OTRA_IP: &str = "otra";

/// Las IP de las IP fijas.
type IpFijas = Vec<String>;

/// Lo que lee de la red de casa la tarjeta de Puertos: los dispositivos
/// (para elegir el destino y ponerle nombre), las IP fijas (para saber si
/// el destino sigue en la red) y la red (para validar antes de enviar). Cada
/// lectura es su propio recurso: ninguna función del servidor va detrás de
/// otra dentro de un mismo recurso.
#[derive(Clone, Copy)]
pub(super) struct DatosPuertos {
    dispositivos: Resource<Result<Vec<DispositivoVista>, ServerFnError>>,
    reservas: Resource<Result<IpFijas, ServerFnError>>,
    red: Resource<Result<Option<(Ipv4Addr, Ipv4Addr)>, ServerFnError>>,
}

/// Las tres lecturas; `None` en la que falló.
#[derive(Clone, Debug, Default)]
struct RedPuertos {
    dispositivos: Option<Vec<DispositivoVista>>,
    reservas: Option<IpFijas>,
    lan: Option<(Ipv4Addr, Ipv4Addr)>,
}

impl RedPuertos {
    /// ¿Hay que avisar de que el destino `ip` ya no está en la red? Solo si
    /// se pudieron leer los dispositivos y las IP fijas: sin ellos no se sabe.
    fn destino_perdido(&self, ip: &str) -> bool {
        match (&self.dispositivos, &self.reservas) {
            (Some(d), Some(r)) => {
                let fijas: Vec<(String, String, String)> = r
                    .iter()
                    .map(|ip| (String::new(), String::new(), ip.clone()))
                    .collect();
                destino_inexistente(ip, d, &fijas)
            }
            _ => false,
        }
    }

    /// Nombre del dispositivo con la IP `ip`, si se conoce.
    fn nombre_de(&self, ip: &str) -> Option<String> {
        self.dispositivos
            .as_ref()?
            .iter()
            .find(|d| d.ip.trim() == ip.trim())
            .map(|d| nombre(&d.nombre))
    }
}

impl DatosPuertos {
    /// Se crea con cada lectura del tema: tras aplicar un cambio se vuelve
    /// a leer.
    pub(super) fn nuevo() -> Self {
        DatosPuertos {
            dispositivos: Resource::new(|| (), |()| leer_dispositivos()),
            reservas: Resource::new(|| (), |()| leer_reservas()),
            red: Resource::new(|| (), |()| leer_red_casa()),
        }
    }

    async fn leer(self) -> RedPuertos {
        let lan = self.red.await.ok().flatten();
        let reservas = self.reservas.await.ok();
        let dispositivos = self.dispositivos.await.ok();
        RedPuertos {
            dispositivos,
            reservas,
            lan,
        }
    }
}

/// Puerto o rango escrito por la persona: `8080` o `27015-27030` (con
/// guion o raya, con o sin espacios). `None` si no son números de 0 a
/// 65535 (el 0 y el orden los dice [`validar_regla`]).
fn leer_puertos(texto: &str) -> Option<(u16, u16)> {
    let texto = texto.trim();
    let (desde, hasta) = match texto.split_once(['-', '–']) {
        Some((d, h)) => (d.trim(), h.trim()),
        None => (texto, texto),
    };
    Some((desde.parse().ok()?, hasta.parse().ok()?))
}

/// Puerto o rango para leerlo: `8080` o `27015–27030`.
fn rango(r: (u16, u16)) -> String {
    if r.0 == r.1 {
        r.0.to_string()
    } else {
        format!("{}–{}", r.0, r.1)
    }
}

/// Puerto o rango para editarlo: `8080` o `27015-27030`.
fn rango_editable(r: (u16, u16)) -> String {
    if r.0 == r.1 {
        r.0.to_string()
    } else {
        format!("{}-{}", r.0, r.1)
    }
}

/// Puertos del dispositivo: los mismos que los de internet o, con
/// `primero`, desde él y del mismo tamaño. `None` si se sale de 65535.
fn puertos_internos(externo: (u16, u16), primero: Option<u16>) -> Option<(u16, u16)> {
    match primero {
        None => Some(externo),
        Some(p) => Some((p, p.checked_add(externo.1.saturating_sub(externo.0))?)),
    }
}

/// Campo del formulario que no es del catálogo (el puerto de internet o el
/// del dispositivo), con sus textos propios.
fn campo_propio(id: &str, tipo: TipoCampo, opcional: bool) -> CampoVista {
    CampoVista {
        id: id.into(),
        clave: format!("{CLAVE}-{id}"),
        tipo,
        valor: ValorLegible::Texto(String::new()),
        opciones: Vec::new(),
        min: None,
        max: None,
        ascii: false,
        opcional,
        solo_lectura: None,
        tapado: false,
        valor_secreto: None,
        recomendado: None,
        motivo_recomendado: None,
        requiere: None,
        sin_eleccion: false,
    }
}

/// Qué se está editando: una regla nueva o la de ese id.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Modo {
    Nueva,
    Editar(String),
}

/// Valores con que empieza el formulario.
#[derive(Clone, Debug, Default)]
struct Inicial {
    nombre: String,
    protocolo: String,
    puerto: String,
    destino: String,
    interno: String,
}

impl Inicial {
    /// De la regla que se edita (`r`, leída de `i`); sin ella, una nueva.
    /// Un protocolo que el panel no conoce empieza «sin elegir»
    /// ([`DESCONOCIDO`]): hay que elegir TCP o UDP para guardarla.
    fn de(r: Option<&ReglaPuertos>, i: Option<&InstanciaVista>) -> Inicial {
        let Some(r) = r else {
            return Inicial {
                protocolo: "tcp".into(),
                ..Inicial::default()
            };
        };
        let protocolo = i
            .map(|i| valor_de(&i.campos, "protocolo"))
            .filter(|p| {
                matches!(
                    Protocolo::de_clave(p),
                    Some(Protocolo::Tcp | Protocolo::Udp)
                )
            })
            .unwrap_or_else(|| DESCONOCIDO.to_owned());
        Inicial {
            nombre: r.nombre.clone(),
            protocolo,
            puerto: rango_editable(r.externo),
            destino: r.ip.to_string(),
            interno: if r.interno == r.externo {
                String::new()
            } else {
                r.interno.0.to_string()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puerto_o_rango_escrito() {
        assert_eq!(leer_puertos("8080"), Some((8080, 8080)));
        assert_eq!(leer_puertos(" 27015-27030 "), Some((27015, 27030)));
        assert_eq!(leer_puertos("27015 – 27030"), Some((27015, 27030)));
        assert_eq!(leer_puertos("0"), Some((0, 0)));
        for mal in ["", "http", "70000", "80-", "-80", "1-2-3", "8080,8081"] {
            assert_eq!(leer_puertos(mal), None, "{mal}");
        }
    }

    #[test]
    fn puertos_del_dispositivo_del_mismo_tamano() {
        assert_eq!(puertos_internos((8080, 8080), None), Some((8080, 8080)));
        assert_eq!(puertos_internos((9000, 9010), Some(80)), Some((80, 90)));
        assert_eq!(puertos_internos((9000, 9010), Some(65530)), None);
    }

    #[test]
    fn rango_para_leer_y_para_editar() {
        assert_eq!(rango((8080, 8080)), "8080");
        assert_eq!(rango((1, 2)), "1–2");
        assert_eq!(rango_editable((1, 2)), "1-2");
    }

    fn red(d: Option<Vec<DispositivoVista>>, r: Option<IpFijas>) -> RedPuertos {
        RedPuertos {
            dispositivos: d,
            reservas: r,
            lan: None,
        }
    }

    /// Sin los dispositivos o sin las IP fijas no se sabe si el destino
    /// sigue: no se avisa.
    #[test]
    fn sin_leer_no_avisa() {
        assert!(red(Some(vec![]), Some(vec![])).destino_perdido("192.168.1.75"));
        assert!(!red(None, Some(vec![])).destino_perdido("192.168.1.75"));
        assert!(!red(Some(vec![]), None).destino_perdido("192.168.1.75"));
    }
}
