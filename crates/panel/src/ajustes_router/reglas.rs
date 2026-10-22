//! Reglas de puertos: tipos y validación.

use super::vistas::{CampoVista, DispositivoVista, InstanciaVista, PeticionAjuste};
use crate::t;
use ajustes::ValorLegible;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Valor del campo «protocolo» de una regla nueva que abre el puerto en TCP
/// y en UDP: el router solo admite uno por regla, así que el servidor crea
/// dos (una de cada) en un único grupo de cambios.
pub const AMBOS: &str = "ambos";

/// Protocolo de una regla de apertura de puertos. `Tcp` y `Udp` son las
/// opciones `tcp` y `udp` del catálogo (su valor del router sale de ahí);
/// `Ambos` solo existe al crear: son dos reglas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocolo {
    Tcp,
    Udp,
    Ambos,
}

impl Protocolo {
    /// Del valor legible del campo (`tcp`, `udp` o [`AMBOS`]).
    pub fn de_clave(clave: &str) -> Option<Protocolo> {
        match clave.trim() {
            "tcp" => Some(Protocolo::Tcp),
            "udp" => Some(Protocolo::Udp),
            AMBOS => Some(Protocolo::Ambos),
            _ => None,
        }
    }

    /// Las opciones del catálogo que cubre.
    pub fn claves(self) -> &'static [&'static str] {
        match self {
            Protocolo::Tcp => &["tcp"],
            Protocolo::Udp => &["udp"],
            Protocolo::Ambos => &["tcp", "udp"],
        }
    }

    /// ¿Comparten algún protocolo?
    fn comparte(self, otro: Protocolo) -> bool {
        self.claves().iter().any(|c| otro.claves().contains(c))
    }
}

/// Una regla de apertura de puertos: lo que llega de internet por los
/// puertos `externo` (desde, hasta) va a los `interno` del dispositivo `ip`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglaPuertos {
    /// Id en el router; `None` si es nueva.
    pub id: Option<String>,
    pub nombre: String,
    pub protocolo: Protocolo,
    pub externo: (u16, u16),
    pub interno: (u16, u16),
    pub ip: Ipv4Addr,
    pub activa: bool,
}

/// Por qué no vale una regla de apertura de puertos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorRegla {
    /// El último puerto de un rango es menor que el primero.
    RangoInvertido,
    /// El rango del dispositivo no tiene el mismo tamaño que el de internet.
    RangosDistintos,
    /// Otra regla ya abre alguno de esos puertos con el mismo protocolo.
    Solapa,
    /// La IP no es la de un dispositivo de la red de casa.
    FueraDeLaRed,
    /// El puerto 0 no existe.
    PuertoCero,
}

impl ErrorRegla {
    /// Campo de la regla junto al que va el mensaje: el puerto de internet
    /// (`externo_desde`) o el dispositivo (`destino`).
    pub fn campo(self) -> &'static str {
        match self {
            ErrorRegla::FueraDeLaRed => "destino",
            _ => "externo_desde",
        }
    }

    /// Mensaje traducido; `router` es la IP del router.
    pub fn texto(self, router: Ipv4Addr) -> String {
        match self {
            ErrorRegla::RangoInvertido => t!("ajuste-puertos-reglas-error-invertido"),
            ErrorRegla::RangosDistintos => t!("ajuste-puertos-reglas-error-distintos"),
            ErrorRegla::Solapa => t!("ajuste-puertos-reglas-error-solapa"),
            ErrorRegla::FueraDeLaRed => t!(
                "ajuste-puertos-reglas-error-fuera",
                router = router.to_string()
            ),
            ErrorRegla::PuertoCero => t!("ajuste-puertos-reglas-error-cero"),
        }
    }
}

/// ¿Vale la regla `r` con las reglas `existentes` en la red `lan` (IP y
/// máscara del router)? Por orden: ningún puerto es 0; cada rango va de
/// menor a mayor; los dos rangos tienen el mismo tamaño; la IP es la de un
/// dispositivo de la red (ni la del router ni la primera o la última de la
/// red); ninguna otra regla (la propia, por su id, no cuenta) abre alguno
/// de esos puertos de internet con el mismo protocolo.
pub fn validar_regla(
    r: &ReglaPuertos,
    existentes: &[ReglaPuertos],
    lan: (Ipv4Addr, Ipv4Addr),
) -> Result<(), ErrorRegla> {
    let (ext, int) = (r.externo, r.interno);
    if [ext.0, ext.1, int.0, int.1].contains(&0) {
        return Err(ErrorRegla::PuertoCero);
    }
    if ext.0 > ext.1 || int.0 > int.1 {
        return Err(ErrorRegla::RangoInvertido);
    }
    if ext.1 - ext.0 != int.1 - int.0 {
        return Err(ErrorRegla::RangosDistintos);
    }
    let (router, mascara) = lan;
    let (m, n) = (u32::from(mascara), u32::from(r.ip));
    let anfitrion = n & !m;
    if n & m != u32::from(router) & m || r.ip == router || anfitrion == 0 || anfitrion == !m {
        return Err(ErrorRegla::FueraDeLaRed);
    }
    let solapa = existentes
        .iter()
        .filter(|o| r.id.is_none() || o.id != r.id)
        .any(|o| o.protocolo.comparte(r.protocolo) && o.externo.0 <= ext.1 && ext.0 <= o.externo.1);
    if solapa {
        return Err(ErrorRegla::Solapa);
    }
    Ok(())
}

/// ¿El dispositivo de destino `ip` ya no está en la red? Cierto si no es la
/// IP de ningún dispositivo que conozca el router (conectado o no) ni la de
/// ninguna IP fija (`reservas`: `(id, mac, ip)`).
pub fn destino_inexistente(
    ip: &str,
    dispositivos: &[DispositivoVista],
    reservas: &[(String, String, String)],
) -> bool {
    let ip = ip.trim();
    !dispositivos.iter().any(|d| d.ip.trim() == ip)
        && !reservas.iter().any(|(_, _, r)| r.trim() == ip)
}

/// Valor legible del campo `id` de una instancia (vacío si falta).
pub fn valor_de(campos: &[CampoVista], id: &str) -> String {
    match campos.iter().find(|c| c.id == id).map(|c| &c.valor) {
        Some(ValorLegible::Texto(t) | ValorLegible::Opcion(t) | ValorLegible::Desconocido(t)) => {
            t.trim().to_owned()
        }
        Some(ValorLegible::Booleano(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// La regla de una instancia leída de `puertos.reglas`; `None` si un puerto
/// o la IP no lo son. Un protocolo que no es TCP ni UDP queda como
/// [`Protocolo::Ambos`]: no se sabe con cuál choca.
pub fn regla_de(i: &InstanciaVista) -> Option<ReglaPuertos> {
    let v = |id: &str| valor_de(&i.campos, id);
    let puerto = |id: &str| v(id).parse::<u16>().ok();
    Some(ReglaPuertos {
        id: Some(i.id.clone()),
        nombre: v("nombre"),
        // Un protocolo que el panel no conoce puede ser cualquiera: cuenta
        // como los dos (al comprobar si otra regla choca con ella).
        protocolo: Protocolo::de_clave(&v("protocolo"))
            .filter(|p| *p != Protocolo::Ambos)
            .unwrap_or(Protocolo::Ambos),
        externo: (puerto("externo_desde")?, puerto("externo_hasta")?),
        interno: (puerto("interno_desde")?, puerto("interno_hasta")?),
        ip: v("destino").parse().ok()?,
        activa: v("activa") == "true",
    })
}

/// Campos de una regla que la identifican: la página los envía como
/// `esperados` al editarla o borrarla, y el servidor comprueba que el
/// router aún los tiene (si no, 409).
pub const IDENTIFICAN_REGLA: [&str; 3] = ["protocolo", "externo_desde", "destino"];

/// Campos que identifican una instancia de la lista con el componente
/// `componente` (`puertos`, `reservas`): una regla por lo que abre y hacia
/// dónde; una IP fija, por su MAC. Ninguno en las demás listas.
pub fn identifican(componente: Option<&str>) -> &'static [&'static str] {
    match componente {
        Some("puertos") => &IDENTIFICAN_REGLA,
        Some("reservas") => &["mac"],
        _ => &[],
    }
}

/// Lo que identifica una instancia (`campo → valor legible`), tal como la
/// leyó la página: lo que se envía como `esperados` al editarla o borrarla
/// (desde la página o desde una acción sobre varias, en el servidor).
pub fn identidad_de(
    componente: Option<&str>,
    campos: &[CampoVista],
) -> std::collections::BTreeMap<String, String> {
    identifican(componente)
        .iter()
        .map(|k| ((*k).to_owned(), valor_de(campos, k)))
        .collect()
}

/// ¿Hay que validar la regla que pide `p` ([`validar_regla`])? Una nueva, o
/// una que cambia algo más que el nombre o se activa. Desactivarla, cambiarle
/// el nombre o borrarla, no: así se puede apagar una regla que ya no
/// valdría sin corregirla antes.
pub fn valida_la_regla(p: &PeticionAjuste) -> bool {
    if p.borrar {
        return false;
    }
    p.instancia.is_none()
        || p.valores.iter().any(|(k, v)| match k.as_str() {
            "nombre" => false,
            "activa" => v.trim() == "true",
            _ => true,
        })
}

/// Texto traducido de un valor que no vale, con sus límites: el mismo en el
/// servidor y en la página.
pub fn texto_error_valor(e: &ajustes::ErrorValor) -> String {
    let mut args = i18n::FluentArgs::new();
    let (min, max) = e.limites();
    if let Some(min) = min {
        args.set("min", min);
    }
    if let Some(max) = max {
        args.set("max", max);
    }
    crate::ui::i18n::traducir(e.clave_i18n(), Some(&args))
}
