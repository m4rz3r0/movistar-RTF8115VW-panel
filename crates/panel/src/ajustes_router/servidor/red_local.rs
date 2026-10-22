//! Red local: dispositivos, red de casa y reservas de IP fija.

use super::*;
use std::net::Ipv4Addr;

/// Lectura del router que falla al leer los dispositivos o la red: 502 con
/// el mensaje de siempre (el detalle va al registro).
pub(super) fn sin_leer(que: &str, e: hgu::Error) -> ServerFnError {
    tracing::warn!("ajustes: no se puede leer {que}: {e}");
    fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-lectura"))
}

/// Lectura de las estaciones asociadas a la radio `radio`.
fn clave_estaciones(radio: &str) -> Clave {
    Clave::new("cbListWifiAccPtAssociation", &[("dpm_cfg_wifi_id", radio)])
}

/// Lista de dispositivos del router.
pub(super) fn clave_hosts() -> Clave {
    Clave::new::<&str, &str>("cbGetLanHostList", &[])
}

/// Reservas del servidor DHCP.
fn clave_reservas() -> Clave {
    Clave::new("cbDhcpV4SvrPoolGetStatic", &hgu::read::POOL)
}

/// IP y máscara del router en la red de casa.
fn clave_red_casa() -> Clave {
    Clave::new::<&str, &str>("cbGetLanIPv4Addr", &[])
}

/// Lecturas de los dispositivos: la lista, las reservas y las
/// estaciones de cada radio.
fn claves_dispositivos() -> Vec<Clave> {
    let mut claves = vec![clave_hosts(), clave_reservas()];
    claves.extend(hgu::read::RADIOS_POR_DEFECTO.map(clave_estaciones));
    claves
}

/// Conexión de cada MAC asociada a la WiFi (en minúsculas), por los
/// puntos de acceso de cada radio: la radio 0 es la de 2,4 GHz y la 1
/// la de 5 GHz; en cada una, el punto de acceso con su mismo id es la
/// red principal y los demás son de invitados. `None` si no se pudo
/// leer alguna radio.
fn asociados(
    leidas: &HashMap<Clave, Result<XmlNode, hgu::Error>>,
) -> Option<HashMap<String, Conexion>> {
    let mut out = HashMap::new();
    for radio in hgu::read::RADIOS_POR_DEFECTO {
        let n = leidas.get(&clave_estaciones(radio))?.as_ref().ok()?;
        for st in hgu::models::WifiAssoc::parse_all(n) {
            let conexion = match (st.ap_id == radio, radio) {
                (false, _) => Conexion::Invitados,
                (true, "0") => Conexion::Wifi24,
                (true, _) => Conexion::Wifi5,
            };
            out.insert(mac_comparable(&st.mac), conexion);
        }
    }
    Some(out)
}

/// Orden de la lista: los conectados primero y, dentro, por IP (las que
/// no son IPv4, al final).
fn orden(d: &DispositivoVista) -> (bool, u32, String) {
    let ip = d.ip.parse::<Ipv4Addr>().map_or(u32::MAX, u32::from);
    (!d.activo, ip, d.ip.clone())
}

/// Lecturas de la red de casa que traen las páginas con listas (Puertos y
/// Red local), en la misma tanda que sus ajustes.
pub(super) fn claves_red() -> Vec<Clave> {
    let mut claves = claves_dispositivos();
    claves.push(clave_red_casa());
    claves
}

/// La red de casa con las lecturas de [`claves_red`]: cada parte, `None` si
/// su lectura falló.
pub(super) fn red_de(leidas: &HashMap<Clave, Result<XmlNode, hgu::Error>>) -> RedCasa {
    let nodo = |c: &Clave| leidas.get(c).and_then(|r| r.as_ref().ok());
    RedCasa {
        dispositivos: dispositivos_de(leidas),
        lan: nodo(&clave_red_casa()).and_then(hgu::read::lan_ipv4_de),
        ips_fijas: nodo(&clave_reservas()).map(|n| {
            hgu::models::StaticLease::parse_all(n)
                .into_iter()
                .map(|r| r.ip)
                .collect()
        }),
    }
}

/// Los dispositivos con las lecturas de [`claves_dispositivos`], los
/// conectados primero y por IP; `None` si no se pudo leer la lista o las
/// reservas (el fallo ya lo registró la caché de lecturas).
fn dispositivos_de(
    leidas: &HashMap<Clave, Result<XmlNode, hgu::Error>>,
) -> Option<Vec<DispositivoVista>> {
    let nodo = |c: &Clave| leidas.get(c).and_then(|r| r.as_ref().ok());
    let wifi = asociados(leidas);
    let hosts = hgu::models::LanHost::parse_all(nodo(&clave_hosts())?);
    let reservas = hgu::models::StaticLease::parse_all(nodo(&clave_reservas())?);
    let mut lista: Vec<DispositivoVista> = hosts
        .into_iter()
        .map(|h| {
            let conexion = match &wifi {
                _ if !h.active => Conexion::Desconocida,
                None => Conexion::Desconocida,
                Some(w) => w
                    .get(&mac_comparable(&h.mac))
                    .cloned()
                    .unwrap_or_else(|| Conexion::Cable(h.iface.clone())),
            };
            let fija = reservas
                .iter()
                .find(|r| mac_comparable(&r.mac) == mac_comparable(&h.mac));
            let (reserva, ip_fija) = (fija.map(|r| r.id.clone()), fija.map(|r| r.ip.clone()));
            DispositivoVista {
                nombre: h.hostname,
                ip: h.ip,
                mac: h.mac,
                conexion,
                activo: h.active,
                reserva,
                ip_fija,
            }
        })
        .collect();
    lista.sort_by_key(orden);
    Some(lista)
}

/// Nombre del dispositivo con la MAC `mac` en la lista del router, si lo
/// da (`None` también si no se puede leer: solo es para el resumen).
async fn nombre_del_dispositivo(state: &AppState, mac: &str) -> Option<String> {
    let hosts = hgu::read::lan_hosts(&state.hgu)
        .await
        .inspect_err(|e| tracing::warn!("ajustes: no se pueden leer los dispositivos: {e}"))
        .ok()?;
    let mac = mac_comparable(mac);
    hosts
        .into_iter()
        .find(|h| mac_comparable(&h.mac) == mac)
        .map(|h| h.hostname)
        .filter(|n| !n.trim().is_empty())
}

/// Comprueba una reserva de `p` con lo que tiene ahora el router y
/// devuelve el nombre de su dispositivo, si se conoce (para el resumen).
/// - Al editar o borrar, la reserva tiene que existir (400), la página
///   tiene que decir qué MAC espera (`esperados`; si no, 400) y tiene que
///   ser esa (si no, 409: otra sesión la cambió).
/// - Si hay que validarla ([`valida_la_reserva`]), como la página
///   ([`validar_reserva`]), con la red y las reservas del router; lo que
///   no se cambia es lo de la reserva. Una nueva necesita MAC e IP.
///   Error 400 junto al campo. Los formatos ya están validados.
pub(super) async fn comprobar_reserva(
    state: &AppState,
    a: &Ajuste,
    p: &PeticionAjuste,
) -> Result<Option<String>, ServerFnError> {
    let prefijo = prefijo_i18n(a);
    let reservas = hgu::read::static_leases(&state.hgu)
        .await
        .map_err(|e| sin_leer("las reservas", e))?;
    let actual = match &p.instancia {
        None => None,
        Some(id) => Some(
            reservas
                .iter()
                .find(|r| r.id == *id)
                .ok_or_else(|| fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida")))?,
        ),
    };
    // Al editar o borrar, la MAC que espera la página es obligatoria:
    // sin ella no se sabe si la reserva sigue siendo la misma.
    if let Some(r) = actual {
        let esperada = p.esperados.get("mac").ok_or_else(|| {
            tracing::warn!("ajustes: {}: editar o borrar sin la MAC esperada", a.id);
            fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
        })?;
        if mac_comparable(&r.mac) != mac_comparable(esperada) {
            return Err(fallo(
                StatusCode::CONFLICT,
                t!("ajuste-red_local-reservas-error-cambiada"),
            ));
        }
    }
    let valor = |campo: &str, de_la_reserva: Option<&str>| {
        p.valores
            .get(campo)
            .filter(|_| !p.borrar)
            .map(|v| v.trim().to_owned())
            .or_else(|| de_la_reserva.map(str::to_owned))
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                let c = a.campo(campo).expect("campo de la reserva");
                error_valor(&prefijo, c, &ErrorValor::Vacio)
            })
    };
    let mac = valor("mac", actual.map(|r| r.mac.as_str()))?;
    let detalle = nombre_del_dispositivo(state, &mac).await;
    if !valida_la_reserva(p) {
        return Ok(detalle);
    }
    let ip = valor("ip", actual.map(|r| r.ip.as_str()))?;
    let ip: Ipv4Addr = ip.parse().map_err(|_| {
        let c = a.campo("ip").expect("campo de la reserva");
        error_valor(&prefijo, c, &ErrorValor::Ip)
    })?;
    let lan = hgu::read::lan_ipv4(&state.hgu)
        .await
        .map_err(|e| sin_leer("la red de casa", e))?
        .ok_or_else(|| {
            tracing::warn!("ajustes: el router no da su IP en la red de casa");
            fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-lectura"))
        })?;
    let existentes: Vec<(String, String, String)> =
        reservas.into_iter().map(|r| (r.id, r.mac, r.ip)).collect();
    validar_reserva(&mac, ip, lan, &existentes, p.instancia.as_deref()).map_err(|e| {
        let etiqueta = crate::ui::i18n::traducir(&format!("{prefijo}-{}", e.campo()), None);
        fallo(
            StatusCode::BAD_REQUEST,
            format!("{etiqueta}: {}", e.texto(lan.0)),
        )
    })?;
    Ok(detalle)
}
