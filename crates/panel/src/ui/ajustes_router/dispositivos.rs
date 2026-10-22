//! «Dispositivos conectados»: el detalle de un dispositivo de la lista
//! ([`DetalleDispositivo`]), con su nombre, su dirección, su MAC, por dónde
//! se conecta y «Fijar esta IP», que prepara una reserva con la MAC y la IP
//! actuales y la guarda con el diálogo de grupo. Sin bloqueo ni control
//! parental ni tráfico por dispositivo: el router no da al panel nada de
//! eso (lo dice la ayuda). La lista misma es la genérica
//! ([`super::lista`]).

use crate::ajustes_router::{
    Conexion, DispositivoVista, GrupoVista, PeticionAjuste, preparar_ajuste, validar_reserva,
};
use crate::t;
use crate::ui::explorador::texto_error;
use leptos::{prelude::*, task::spawn_local};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;

/// Qué se puede hacer con la IP de un dispositivo.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Fijable {
    Si,
    /// Ya tiene IP fija: esta (puede no ser la que tiene ahora).
    YaFija(String),
    /// No está conectado: su última IP puede tenerla ya otro dispositivo.
    Desconectado,
    /// El servidor la rechazaría siempre (no es de la red de casa, es la del
    /// router o la de otra IP fija…).
    NoSePuede,
}

/// ¿Se le puede fijar la IP al dispositivo `d`, con la red `lan` (si se leyó)
/// y las IP fijas `existentes`?
fn fijable(
    d: &DispositivoVista,
    lan: Option<(Ipv4Addr, Ipv4Addr)>,
    existentes: &[(String, String, String)],
) -> Fijable {
    if d.reserva.is_some() {
        return Fijable::YaFija(d.ip_fija.clone().unwrap_or_else(|| d.ip.clone()));
    }
    if !d.activo {
        return Fijable::Desconectado;
    }
    let Ok(ip) = d.ip.trim().parse::<Ipv4Addr>() else {
        return Fijable::NoSePuede;
    };
    if ip.is_unspecified() {
        return Fijable::NoSePuede;
    }
    match lan {
        Some(lan) if validar_reserva(&d.mac, ip, lan, existentes, None).is_err() => {
            Fijable::NoSePuede
        }
        _ => Fijable::Si,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dispositivo(ip: &str, activo: bool, reserva: Option<(&str, &str)>) -> DispositivoVista {
        DispositivoVista {
            nombre: "host-2".into(),
            ip: ip.into(),
            mac: "02:00:00:00:00:03".into(),
            conexion: Conexion::Desconocida,
            activo,
            reserva: reserva.map(|(id, _)| id.to_owned()),
            ip_fija: reserva.map(|(_, ip)| ip.to_owned()),
        }
    }

    /// Solo se ofrece «Fijar esta IP» si el servidor podría aceptarla.
    #[test]
    fn cuando_se_puede_fijar() {
        let lan = Some((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
        ));
        let otra = [(
            "0".to_owned(),
            "02:00:00:00:00:04".to_owned(),
            "192.168.1.68".to_owned(),
        )];
        let f = |d: DispositivoVista| fijable(&d, lan, &otra);
        assert_eq!(f(dispositivo("192.168.1.62", true, None)), Fijable::Si);
        assert_eq!(
            f(dispositivo(
                "192.168.1.60",
                true,
                Some(("1", "192.168.1.69"))
            )),
            Fijable::YaFija("192.168.1.69".into())
        );
        assert_eq!(
            f(dispositivo("192.168.1.62", false, None)),
            Fijable::Desconectado
        );
        for no in [
            "198.51.100.7",
            "192.168.1.1",
            "192.168.1.0",
            "192.168.1.68",
            "0.0.0.0",
            "",
        ] {
            assert_eq!(f(dispositivo(no, true, None)), Fijable::NoSePuede, "{no}");
        }
        // Sin la red leída, decide el servidor.
        let d = dispositivo("198.51.100.7", true, None);
        assert_eq!(fijable(&d, None, &otra), Fijable::Si);
    }
}

/// Nombre de un dispositivo para la página.
pub(crate) fn nombre(n: &str) -> String {
    if n.trim().is_empty() {
        t!("ajuste-red_local-dispositivos-sin-nombre")
    } else {
        n.to_owned()
    }
}

/// Por dónde se conecta, para la página.
pub(crate) fn conexion(c: &Conexion) -> String {
    match c {
        Conexion::Cable(_) => t!("ajuste-red_local-dispositivos-cable"),
        Conexion::Wifi24 => t!("ajuste-red_local-dispositivos-wifi24"),
        Conexion::Wifi5 => t!("ajuste-red_local-dispositivos-wifi5"),
        Conexion::Invitados => t!("ajuste-red_local-dispositivos-invitados"),
        Conexion::Desconocida => t!("ajuste-red_local-dispositivos-desconocida"),
    }
}

/// El detalle del dispositivo `d`: sus datos y qué se puede hacer con su
/// IP (fijarla, o por qué no). `lan` y `existentes` (las IP fijas, `(id,
/// mac, ip)`) dicen si se puede fijar; en solo lectura, solo los datos.
#[component]
pub(crate) fn DetalleDispositivo(
    d: DispositivoVista,
    lan: Option<(Ipv4Addr, Ipv4Addr)>,
    existentes: Vec<(String, String, String)>,
    solo_lectura: bool,
    error: RwSignal<Option<String>>,
    preparado: Callback<GrupoVista>,
) -> impl IntoView {
    let preparando = RwSignal::new(false);
    let se_puede = fijable(&d, lan, &existentes);
    let fijar = {
        let d = d.clone();
        move |_| {
            if preparando.get_untracked() {
                return;
            }
            error.set(None);
            preparando.set(true);
            let p = PeticionAjuste {
                ajuste: "red_local.reservas".into(),
                instancia: None,
                borrar: false,
                valores: [
                    ("mac".to_owned(), d.mac.clone()),
                    ("ip".to_owned(), d.ip.clone()),
                    ("activa".to_owned(), "true".to_owned()),
                ]
                .into(),
                esperados: BTreeMap::new(),
            };
            spawn_local(async move {
                match preparar_ajuste(p).await {
                    Ok(g) => preparado.run(g),
                    Err(e) => error.set(Some(texto_error(&e))),
                }
                preparando.try_set(false);
            });
        }
    };
    let nota = |texto: String| view! { <p class="nota-fijar">{texto}</p> }.into_any();
    let accion = match (se_puede, solo_lectura) {
        // Un dato, no una acción: una etiqueta, no un botón desactivado.
        (Fijable::YaFija(ip), _) => view! {
            <p>
                <span class="chip chip-ya-fija">
                    {t!("ajuste-red_local-dispositivos-ya-fija", ip = ip)}
                </span>
            </p>
        }
        .into_any(),
        (_, true) => ().into_any(),
        (Fijable::Desconectado, false) => {
            nota(t!("ajuste-red_local-dispositivos-fijar-desconectado"))
        }
        (Fijable::NoSePuede, false) => nota(t!("ajuste-red_local-dispositivos-no-fijable")),
        (Fijable::Si, false) => {
            // El nombre accesible empieza por lo que se ve (WCAG 2.5.3).
            let etiqueta = t!(
                "ajuste-red_local-dispositivos-fijar-de",
                ip = d.ip.clone(),
                equipo = nombre(&d.nombre)
            );
            view! {
                <div class="detalle-pie">
                    // Sin `disabled` al preparar: el botón conserva el foco y
                    // el diálogo se lo devuelve al cerrarse.
                    <button
                        class="b-pri"
                        type="button"
                        aria-label=etiqueta
                        aria-disabled=move || preparando.get().then_some("true")
                        on:click=fijar
                    >
                        {move || {
                            if preparando.get() {
                                t!("ajustes-grupo-preparando")
                            } else {
                                t!("ajuste-red_local-dispositivos-fijar")
                            }
                        }}
                    </button>
                </div>
            }
            .into_any()
        }
    };
    let estado = if d.activo {
        conexion(&d.conexion)
    } else {
        t!("ajuste-red_local-dispositivos-desconectado")
    };
    let con_fija = d.ip_fija.is_some();
    let fija = d
        .ip_fija
        .clone()
        .unwrap_or_else(|| t!("ajuste-red_local-dispositivos-sin-ip-fija"));
    view! {
        <dl class="ajuste-valores">
            <div class="ajuste-fila">
                <dt>{t!("ajuste-red_local-dispositivos-col-ip")}</dt>
                <dd class="mono">{d.ip.clone()}</dd>
            </div>
            <div class="ajuste-fila">
                <dt>{t!("ajuste-red_local-dispositivos-col-mac")}</dt>
                <dd class="mono">{d.mac.clone()}</dd>
            </div>
            <div class="ajuste-fila">
                <dt>{t!("ajuste-red_local-dispositivos-col-conexion")}</dt>
                <dd>{estado}</dd>
            </div>
            <div class="ajuste-fila">
                <dt>{t!("ajuste-red_local-dispositivos-col-ip_fija")}</dt>
                <dd class:mono=con_fija>{fija}</dd>
            </div>
        </dl>
        {accion}
    }
}
