//! «Dispositivos conectados» ([`Dispositivos`]): los dispositivos que el router
//! conoce, con su nombre, su dirección, su MAC y por dónde se conectan, y
//! «Fijar esta IP», que prepara una reserva con la MAC y la IP actuales y la
//! guarda con el diálogo de grupo. Sin bloqueo ni control parental: el
//! catálogo no tiene función para ello (lo dice la ayuda).
//!
//! Los dispositivos y la red de casa se leen una vez por página ([`DatosRed`]):
//! los usan también las IP fijas ([`super::reservas::Reservas`]).

use super::Foco;
use super::tarjeta::{ayuda, cabecera, detalles_tecnicos, dom};
use crate::ajustes_router::{
    AjusteVista, Conexion, DispositivoVista, GrupoVista, PeticionAjuste, leer_dispositivos,
    leer_red_casa, preparar_ajuste, validar_reserva,
};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::grupo::DialogoGrupo;
use crate::ui::{Estado, IconoEstado};
use leptos::{prelude::*, task::spawn_local};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;

/// Lo que leen de la red de casa los componentes de Red local. Cada
/// lectura por su lado: si fallan los dispositivos, la red sigue sirviendo para
/// validar una IP fija en la página.
#[derive(Clone, Debug)]
pub(super) struct RedLocal {
    pub dispositivos: Result<Vec<DispositivoVista>, ServerFnError>,
    /// IP y máscara del router; `None` si no se pudo leer (entonces solo
    /// valida el servidor).
    pub lan: Option<(Ipv4Addr, Ipv4Addr)>,
}

/// Las lecturas de la red de casa de la página, compartidas por sus
/// tarjetas: los dispositivos y la IP y máscara del router. Cada una es su
/// propio recurso, que llama a su función del servidor en cuanto empieza.
#[derive(Clone, Copy)]
pub(super) struct DatosRed {
    dispositivos: Resource<Result<Vec<DispositivoVista>, ServerFnError>>,
    red: Resource<Result<Option<(Ipv4Addr, Ipv4Addr)>, ServerFnError>>,
    /// Las IP fijas del tema, `(id, mac, ip)`: con ellas se sabe si a un
    /// dispositivo se le puede fijar su IP.
    existentes: StoredValue<Vec<(String, String, String)>>,
}

impl DatosRed {
    /// Se crea con cada lectura del tema: tras aplicar un cambio se vuelve
    /// a leer.
    pub(super) fn nuevo(existentes: Vec<(String, String, String)>) -> Self {
        DatosRed {
            dispositivos: Resource::new(|| (), |()| leer_dispositivos()),
            red: Resource::new(|| (), |()| leer_red_casa()),
            existentes: StoredValue::new(existentes),
        }
    }

    /// Las dos lecturas. Si falla la de la red, solo valida el servidor.
    pub(super) async fn leer(self) -> RedLocal {
        let lan = self.red.await;
        let dispositivos = self.dispositivos.await;
        combinar(dispositivos, lan)
    }
}

fn combinar(
    dispositivos: Result<Vec<DispositivoVista>, ServerFnError>,
    lan: Result<Option<(Ipv4Addr, Ipv4Addr)>, ServerFnError>,
) -> RedLocal {
    RedLocal {
        dispositivos,
        lan: lan.ok().flatten(),
    }
}

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

    /// Si falla la lectura de los dispositivos, la red de casa no se pierde: la
    /// página sigue validando una IP fija antes de enviarla.
    #[test]
    fn sin_dispositivos_conserva_la_red() {
        let lan = (
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
        );
        let r = combinar(Err(ServerFnError::new("x")), Ok(Some(lan)));
        let RedLocal {
            dispositivos,
            lan: red,
        } = r;
        assert!(dispositivos.is_err());
        assert_eq!(red, Some(lan));
    }

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
pub(super) fn nombre(n: &str) -> String {
    if n.trim().is_empty() {
        t!("ajuste-red_local-dispositivos-sin-nombre")
    } else {
        n.to_owned()
    }
}

fn conexion(c: &Conexion) -> String {
    match c {
        Conexion::Cable(_) => t!("ajuste-red_local-dispositivos-cable"),
        Conexion::Wifi24 => t!("ajuste-red_local-dispositivos-wifi24"),
        Conexion::Wifi5 => t!("ajuste-red_local-dispositivos-wifi5"),
        Conexion::Invitados => t!("ajuste-red_local-dispositivos-invitados"),
        Conexion::Desconocida => t!("ajuste-red_local-dispositivos-desconocida"),
    }
}

/// Aviso de una lectura que ha fallado.
pub(super) fn aviso_error(m: String) -> AnyView {
    view! {
        <p class="aviso estado-error" role="status">
            <IconoEstado estado=Estado::Error />
            <span>{m}</span>
        </p>
    }
    .into_any()
}

#[component]
pub(crate) fn Dispositivos(
    a: AjusteVista,
    solo_lectura: bool,
    recargar: Callback<()>,
) -> impl IntoView {
    let datos = expect_context::<DatosRed>();
    let foco = expect_context::<Foco>();
    let id_dom = dom(&a);
    let grupo = RwSignal::new(None::<GrupoVista>);
    // MAC del dispositivo que se está preparando, y el error de su fila.
    let preparando = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<(String, String)>);

    let fijar = Callback::new(move |d: DispositivoVista| {
        if preparando.get_untracked().is_some() {
            return;
        }
        error.set(None);
        preparando.set(Some(d.mac.clone()));
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
                Ok(g) => grupo.set(Some(g)),
                Err(e) => error.set(Some((d.mac.clone(), texto_error(&e)))),
            }
            preparando.set(None);
        });
    });

    let titulo = format!("{id_dom}-titulo");
    let cerrar = Callback::new(move |aplicado: Option<i64>| {
        grupo.set(None);
        if aplicado.is_some() {
            foco.0.set(Some(titulo.clone()));
            recargar.run(());
        }
    });

    let fila = move |d: DispositivoVista, se_puede: Fijable| {
        let activo = d.activo;
        let estado = if activo {
            conexion(&d.conexion)
        } else {
            t!("ajuste-red_local-dispositivos-desconectado")
        };
        let nota = |texto: String| view! { <span class="nota-fijar">{texto}</span> }.into_any();
        let accion = match (se_puede, solo_lectura) {
            // Un dato, no una acción: una etiqueta, no un botón desactivado.
            (Fijable::YaFija(ip), _) => view! {
                <span class="chip chip-ya-fija">
                    {t!("ajuste-red_local-dispositivos-ya-fija", ip = ip)}
                </span>
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
                let mac = d.mac.clone();
                let d = d.clone();
                let ocupado =
                    Signal::derive(move || preparando.with(|p| p.as_deref() == Some(mac.as_str())));
                view! {
                    // Sin `disabled` al preparar: el botón conserva el foco y
                    // el diálogo se lo devuelve al cerrarse.
                    <button
                        class="boton-sutil"
                        type="button"
                        aria-label=etiqueta
                        aria-disabled=move || ocupado.get().then_some("true")
                        on:click=move |_| fijar.run(d.clone())
                    >
                        {move || {
                            if ocupado.get() {
                                t!("ajustes-grupo-preparando")
                            } else {
                                t!("ajuste-red_local-dispositivos-fijar")
                            }
                        }}
                    </button>
                }
                .into_any()
            }
        };
        // El error de la fila, a la vista junto a ella; lo anuncia la única
        // región de alerta de la tarjeta.
        let mac_error = d.mac.clone();
        let error_fila = move || {
            error.with(|e| {
                e.as_ref()
                    .filter(|(m, _)| *m == mac_error)
                    .map(|(_, t)| view! { <p class="error-formulario">{t.clone()}</p> })
            })
        };
        view! {
            <li class="dispositivo" class:dispositivo-apagado=!activo>
                <div class="dispositivo-quien">
                    <span class="dispositivo-nombre" class:ajuste-vacio=d.nombre.trim().is_empty()>
                        {nombre(&d.nombre)}
                    </span>
                    <span class="chip chip-conexion" class:chip-desconectado=!activo>{estado}</span>
                </div>
                <dl class="dispositivo-datos">
                    <div>
                        <dt>{t!("ajuste-red_local-dispositivos-col-ip")}</dt>
                        <dd class="mono dispositivo-ip">{d.ip.clone()}</dd>
                    </div>
                    <div>
                        <dt>{t!("ajuste-red_local-dispositivos-col-mac")}</dt>
                        <dd class="mono">{d.mac.clone()}</dd>
                    </div>
                </dl>
                <div class="dispositivo-accion">
                    {accion}
                    {error_fila}
                </div>
            </li>
        }
    };

    view! {
        {cabecera(&a, false)}
        {ayuda(&a)}
        <Transition fallback=|| view! { <p class="estado-espera">{t!("comun-cargando")}</p> }>
            {move || Suspend::new(async move {
                let RedLocal { dispositivos, lan } = datos.leer().await;
                match dispositivos {
                    Ok(lista) if lista.is_empty() => {
                        view! { <p class="ayuda">{t!("ajuste-red_local-dispositivos-vacia")}</p> }
                            .into_any()
                    }
                    Ok(lista) => {
                        let existentes = datos.existentes.get_value();
                        let filas = lista
                            .into_iter()
                            .map(|d| {
                                let se_puede = fijable(&d, lan, &existentes);
                                fila(d, se_puede)
                            })
                            .collect_view();
                        // Cabecera de columnas solo a la vista (en pantallas
                        // anchas): cada fila ya nombra sus datos con `<dt>`.
                        view! {
                            <ul class="dispositivos">
                                <li class="dispositivos-cabecera" aria-hidden="true">
                                    <span></span>
                                    <span>{t!("ajuste-red_local-dispositivos-col-ip")}</span>
                                    <span>{t!("ajuste-red_local-dispositivos-col-mac")}</span>
                                    <span></span>
                                </li>
                                {filas}
                            </ul>
                        }
                        .into_any()
                    }
                    Err(e) => aviso_error(texto_error(&e)),
                }
            })}
        </Transition>
        <p class="solo-lector" role="alert">
            {move || error.with(|e| e.as_ref().map(|(_, t)| t.clone()))}
        </p>
        {detalles_tecnicos(&a.clave, &a.tecnico)}
        {move || grupo.get().map(|g| view! { <DialogoGrupo grupo=g on_cerrar=cerrar /> })}
    }
}
