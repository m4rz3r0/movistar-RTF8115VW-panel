//! «Aparatos conectados» ([`Dispositivos`]): los aparatos que el router
//! conoce, con su nombre, su dirección, su MAC y por dónde se conectan, y
//! «Fijar esta IP», que prepara una reserva con la MAC y la IP actuales y la
//! guarda con el diálogo de grupo. Sin bloqueo ni control parental: el
//! catálogo no tiene función para ello (lo dice la ayuda).
//!
//! Los aparatos y la red de casa se leen una vez por página ([`DatosRed`]):
//! los usan también las IP fijas ([`super::reservas::Reservas`]).

use super::Foco;
use super::tarjeta::{ayuda, cabecera, detalles_tecnicos, dom};
use crate::ajustes_router::{
    AjusteVista, Conexion, DispositivoVista, GrupoVista, PeticionAjuste, leer_dispositivos,
    leer_red_casa, preparar_ajuste,
};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::grupo::DialogoGrupo;
use crate::ui::{Estado, IconoEstado};
use leptos::{prelude::*, task::spawn_local};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Lo que leen de la red de casa los componentes de Red local.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct RedLocal {
    pub dispositivos: Vec<DispositivoVista>,
    /// IP y máscara del router; `None` si no se pudo leer (entonces solo
    /// valida el servidor).
    pub lan: Option<(Ipv4Addr, Ipv4Addr)>,
}

/// Las lecturas de la red de casa de la página, compartidas por sus
/// tarjetas: los aparatos y la IP y máscara del router. Cada una es su
/// propio recurso, que llama a su función del servidor en cuanto empieza.
#[derive(Clone, Copy)]
pub(super) struct DatosRed {
    dispositivos: Resource<Result<Vec<DispositivoVista>, ServerFnError>>,
    red: Resource<Result<Option<(Ipv4Addr, Ipv4Addr)>, ServerFnError>>,
}

impl DatosRed {
    /// Se crea con cada lectura del tema: tras aplicar un cambio se vuelve
    /// a leer.
    pub(super) fn nuevo() -> Self {
        DatosRed {
            dispositivos: Resource::new(|| (), |()| leer_dispositivos()),
            red: Resource::new(|| (), |()| leer_red_casa()),
        }
    }

    /// Las dos lecturas. Si falla la de la red, solo valida el servidor.
    pub(super) async fn leer(self) -> Result<RedLocal, ServerFnError> {
        let lan = self.red.await.ok().flatten();
        let dispositivos = self.dispositivos.await?;
        Ok(RedLocal { dispositivos, lan })
    }
}

/// Nombre de un aparato para la página.
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

/// Se le puede fijar la IP: tiene una IPv4 de verdad.
fn ip_fijable(ip: &str) -> bool {
    ip.parse::<Ipv4Addr>()
        .is_ok_and(|a| !a.is_unspecified() && !a.is_broadcast())
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
    // MAC del aparato que se está preparando, y el error de su fila.
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

    let fila = move |d: DispositivoVista| {
        let activo = d.activo;
        let estado = if activo {
            conexion(&d.conexion)
        } else {
            t!("ajuste-red_local-dispositivos-desconectado")
        };
        let accion = match (&d.reserva, solo_lectura) {
            (Some(_), _) => view! {
                <button class="boton-sutil" type="button" disabled=true>
                    {t!("ajuste-red_local-dispositivos-ya-fija")}
                </button>
            }
            .into_any(),
            (None, true) => ().into_any(),
            (None, false) if !ip_fijable(&d.ip) => ().into_any(),
            (None, false) => {
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
        let mac_error = d.mac.clone();
        let error_fila = move || {
            error.with(|e| {
                e.as_ref()
                    .filter(|(m, _)| *m == mac_error)
                    .map(|(_, t)| t.clone())
            })
        };
        view! {
            <li class="aparato" class:aparato-apagado=!activo>
                <div class="aparato-quien">
                    <span class="aparato-nombre" class:ajuste-vacio=d.nombre.trim().is_empty()>
                        {nombre(&d.nombre)}
                    </span>
                    <span class="chip chip-conexion" class:chip-desconectado=!activo>{estado}</span>
                </div>
                <dl class="aparato-datos">
                    <div>
                        <dt>{t!("ajuste-red_local-dispositivos-col-ip")}</dt>
                        <dd class="mono aparato-ip">{d.ip.clone()}</dd>
                    </div>
                    <div>
                        <dt>{t!("ajuste-red_local-dispositivos-col-mac")}</dt>
                        <dd class="mono">{d.mac.clone()}</dd>
                    </div>
                </dl>
                <div class="aparato-accion">
                    {accion}
                    <p class="error-formulario" role="alert">{error_fila}</p>
                </div>
            </li>
        }
    };

    view! {
        {cabecera(&a, false)}
        {ayuda(&a)}
        <Transition fallback=|| view! { <p class="estado-espera">{t!("comun-cargando")}</p> }>
            {move || Suspend::new(async move {
                match datos.leer().await {
                    Ok(r) if r.dispositivos.is_empty() => {
                        view! { <p class="ayuda">{t!("ajuste-red_local-dispositivos-vacia")}</p> }
                            .into_any()
                    }
                    Ok(r) => {
                        let filas = r.dispositivos.into_iter().map(fila).collect_view();
                        // Cabecera de columnas solo a la vista (en pantallas
                        // anchas): cada fila ya nombra sus datos con `<dt>`.
                        view! {
                            <ul class="aparatos">
                                <li class="aparatos-cabecera" aria-hidden="true">
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
        {detalles_tecnicos(&a.clave, &a.tecnico)}
        {move || grupo.get().map(|g| view! { <DialogoGrupo grupo=g on_cerrar=cerrar /> })}
    }
}
