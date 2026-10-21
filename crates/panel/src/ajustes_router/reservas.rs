//! Validación de las reservas de IP fija.

use super::vistas::PeticionAjuste;
use crate::t;
use std::net::Ipv4Addr;

/// Por qué no vale una reserva (IP fija).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorReserva {
    /// La IP no es de la red de casa.
    FueraDeLaRed,
    /// La IP es la del router.
    IpDelRouter,
    /// Otra reserva ya tiene esa IP.
    IpRepetida,
    /// El dispositivo (su MAC) ya tiene otra reserva.
    MacRepetida,
    /// La primera o la última dirección de la red, que son de toda la red.
    Difusion,
}

impl ErrorReserva {
    /// Campo de la reserva junto al que va el mensaje.
    pub fn campo(self) -> &'static str {
        match self {
            ErrorReserva::MacRepetida => "mac",
            _ => "ip",
        }
    }

    /// Mensaje traducido; `router` es la IP del router (la cita al decir
    /// que una IP no es de la red).
    pub fn texto(self, router: Ipv4Addr) -> String {
        match self {
            ErrorReserva::FueraDeLaRed => t!(
                "ajuste-red_local-reservas-error-fuera",
                router = router.to_string()
            ),
            ErrorReserva::IpDelRouter => t!("ajuste-red_local-reservas-error-router"),
            ErrorReserva::IpRepetida => t!("ajuste-red_local-reservas-error-ip-repetida"),
            ErrorReserva::MacRepetida => t!("ajuste-red_local-reservas-error-mac-repetida"),
            ErrorReserva::Difusion => t!("ajuste-red_local-reservas-error-difusion"),
        }
    }
}

/// MAC comparable: normalizada si es válida; si no, en minúsculas.
pub(crate) fn mac_comparable(mac: &str) -> String {
    ajustes::normalizar_mac(mac.trim()).unwrap_or_else(|| mac.trim().to_ascii_lowercase())
}

/// ¿Vale la reserva de `mac` a `ip` en la red `lan` (IP y máscara del
/// router), con las reservas `existentes` (`(id, mac, ip)`)? Al editar la
/// reserva `editando`, la suya no cuenta como repetida. La MAC y la IP ya
/// tienen buen formato (lo comprueba `validar_legible`).
pub fn validar_reserva(
    mac: &str,
    ip: Ipv4Addr,
    lan: (Ipv4Addr, Ipv4Addr),
    existentes: &[(String, String, String)],
    editando: Option<&str>,
) -> Result<(), ErrorReserva> {
    let (router, mascara) = lan;
    let (m, n) = (u32::from(mascara), u32::from(ip));
    if n & m != u32::from(router) & m {
        return Err(ErrorReserva::FueraDeLaRed);
    }
    if ip == router {
        return Err(ErrorReserva::IpDelRouter);
    }
    let anfitrion = n & !m;
    if anfitrion == 0 || anfitrion == !m {
        return Err(ErrorReserva::Difusion);
    }
    let otras = || {
        existentes
            .iter()
            .filter(|(id, _, _)| Some(id.as_str()) != editando)
    };
    if otras().any(|(_, _, otra)| otra.trim().parse::<Ipv4Addr>().ok() == Some(ip)) {
        return Err(ErrorReserva::IpRepetida);
    }
    let mac = mac_comparable(mac);
    if otras().any(|(_, otra, _)| mac_comparable(otra) == mac) {
        return Err(ErrorReserva::MacRepetida);
    }
    Ok(())
}

/// ¿Hay que validar la reserva que pide `p` ([`validar_reserva`])? Una
/// nueva, o una que cambia de MAC o de IP o se activa. Desactivarla o
/// borrarla, no: así se puede apagar una reserva que ya no valdría (su IP
/// quedó fuera de la red, o la comparte con otra) sin corregirla antes.
pub fn valida_la_reserva(p: &PeticionAjuste) -> bool {
    if p.borrar {
        return false;
    }
    p.instancia.is_none()
        || p.valores.contains_key("mac")
        || p.valores.contains_key("ip")
        || p.valores.get("activa").map(|v| v.trim()) == Some("true")
}
