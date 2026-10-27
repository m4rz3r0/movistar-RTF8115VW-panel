//! Las escrituras del cortafuegos como las manda el editor de la web (`scbidirectionalflt_ed.asp`): reglas y filtros, la forma de cada campo y sus cargas.
//!
//! Lo capturado de la web oficial (todas por GET a `scbidirectionalflt.cmd`,
//! el router responde 301 a la lista):
//!
//! - alta de regla: `action=add&escape=true&target=rule&belong&version&enabled&protocol&specificAction&icmpType&srcaddr&srcmask&dstaddr&dstmask`;
//! - edición de regla: `action=modify&id=<RID>&target=rule&…` (sin `escape`);
//! - alta de filtro: `action=add&target=entry&name&interface&type&version&default`;
//! - edición de filtro: `action=modify&id=<FID>&target=entry&…`;
//! - bajas: `action=remove&item=<FID>|<RID>&escape=true` (una regla) o
//!   `item=<FID>` (el filtro entero, con sus reglas).
//!
//! Lo que el editor no rellena no viaja: el tipo de rechazo solo con la
//! acción «rechazar», el tipo ICMP solo con ICMP, las direcciones solo si
//! se escriben y los puertos solo con TCP o UDP. Supuesto sin comprobar en
//! el router real: el sitio de los puertos (tras la máscara de su lado),
//! que la web no los mandó en la captura.
//!
//! Los valores van con los nombres de la carga ([`PARAMS_REGLA`],
//! [`PARAMS_FILTRO`]) y en «forma de formulario» ([`vista_regla`]): lo que
//! el router da como «cualquiera» (`0.0.0.0`, `::`, máscara entera, puerto
//! 0) o lo que no aplica, vacío.

use super::{CUALQUIERA, Filtro, Regla};
use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Valores de una regla o un filtro, por el nombre de su parámetro.
pub type Valores = BTreeMap<String, String>;

/// Alta de una regla con el editor completo en el catálogo (se manda a
/// [`super::ALTA`], con `escape=true`).
pub const ALTA_REGLA: &str = "scbidirectionalflt.cmd?action=add.regla";
/// Edición de una regla (`id=<RID>`) o de un filtro (`id=<FID>`).
pub const MODIFICAR: &str = "scbidirectionalflt.cmd?action=modify";
/// Alta de un filtro en el catálogo (se manda a [`super::ALTA`]).
pub const ALTA_FILTRO: &str = "scbidirectionalflt.cmd?action=add.filtro";
/// Edición de un filtro en el catálogo (se manda a [`MODIFICAR`]).
pub const MODIFICAR_FILTRO: &str = "scbidirectionalflt.cmd?action=modify.filtro";
/// Baja de un filtro vacío en el catálogo (se manda a [`super::BAJA`]).
pub const BAJA_FILTRO: &str = "scbidirectionalflt.cmd?action=remove.filtro";

/// ¿Es `funcion` una de las del catálogo que escriben filtros (no reglas)?
pub fn es_de_filtro(funcion: &str) -> bool {
    matches!(funcion, ALTA_FILTRO | MODIFICAR_FILTRO | BAJA_FILTRO)
}

/// Campos de una regla, con el nombre de la carga, en su orden.
pub const PARAMS_REGLA: [&str; 15] = [
    "belong",
    "version",
    "enabled",
    "protocol",
    "specificAction",
    "rejectType",
    "icmpType",
    "srcaddr",
    "srcmask",
    "srcportstart",
    "srcportend",
    "dstaddr",
    "dstmask",
    "dstportstart",
    "dstportend",
];

/// Campos de un filtro, con el nombre de la carga, en su orden.
pub const PARAMS_FILTRO: [&str; 5] = ["name", "interface", "type", "version", "default"];

/// Lo más largo del nombre de un filtro (el que se capturó llegó cortado a
/// 16 caracteres).
pub const MAX_NOMBRE_FILTRO: usize = 16;

/// Los tipos ICMP de la web (`ICMPTYPE`), por su índice: 0 ninguno, 1
/// cualquiera, 2-37 los de IPv4 y 38-76 los de IPv6.
pub const TIPOS_ICMP: [&str; 77] = [
    "none",
    "any",
    "echo_reply",
    "destination_unreachable",
    "network_unreachable",
    "host_unreachable",
    "protocol_unreachable",
    "port_unreachable",
    "fragmentation_needed",
    "source_route_failed",
    "network_unknown",
    "host_unknown",
    "network_prohibited",
    "host_prohibited",
    "tos_network_unreachable",
    "tos_host_unreachable",
    "communication_prohibited",
    "host_precedence_violation",
    "precedence_cutoff",
    "source_quench",
    "redirect",
    "network_redirect",
    "host_redirect",
    "tos_network_redirect",
    "tos_host_redirect",
    "echo_request",
    "router_advertisement",
    "router_solicitation",
    "time_exceeded",
    "ttl_zero_during_transit",
    "ttl_zero_during_reassembly",
    "parameter_problem",
    "ip_header_bad",
    "required_option_missing",
    "timestamp_request",
    "timestamp_reply",
    "address_mask_request",
    "address_mask_reply",
    "destination_unreachable",
    "no_route_to_destination",
    "administratively_prohibited",
    "beyond_scope_of_source_address",
    "address_unreachable",
    "port_unreachable",
    "source_address_failed",
    "reject_route_to_destination",
    "error_source_routing_header",
    "packet_too_big",
    "time_exceeded",
    "hop_limit_exceeded",
    "fragment_reassembly_time_exceeded",
    "parameter_problem",
    "erroneous_header_field",
    "unrecognized_next_header_type",
    "unrecognized_ipv6_option",
    "echo_request",
    "echo_reply",
    "multicast_listener_query",
    "multicast_listener_report",
    "multicast_listener_done",
    "router_solicitation",
    "router_advertisement",
    "neighbor_solicitation",
    "neighbor_advertisement",
    "redirect_message",
    "router_renumbering",
    "router_renumbering_command",
    "router_renumbering_result",
    "sequence_number_reset",
    "node_information_query",
    "node_information_response",
    "inverse_neighbor_discovery_solicitation",
    "inverse_neighbor_discovery_advertisement",
    "version2_multicast_listener_report",
    "multicast_router_advertisement",
    "multicast_router_solicitation",
    "multicast_router_termination",
];

/// Los tipos de rechazo de la web (`REJECTTYPE`), por su índice: 1-6 de
/// IPv4, 7 `tcp_reset` y 8-13 de IPv6.
pub const TIPOS_RECHAZO: [&str; 14] = [
    "none",
    "icmpv4_net_unreachable",
    "icmpv4_host_unreachable",
    "icmpv4_proto_unreachable",
    "icmpv4_port_unreachable",
    "icmpv4_net_prohibited",
    "icmpv4_host_prohibited",
    "tcp_reset",
    "icmpv6_no_route_to_destination",
    "icmpv6_administratively_prohibited",
    "icmpv6_address_unreachable",
    "icmpv6_port_unreachable",
    "icmpv6_source_address_failed",
    "icmpv6_reject_route_to_destination",
];

/// Los índices de [`TIPOS_ICMP`] que valen en un filtro de la versión
/// `version` (`"1"` IPv6): «cualquiera» y los de su versión.
pub fn icmp_de(version: &str) -> impl Iterator<Item = usize> {
    let resto = if version == "1" { 38..=76 } else { 2..=37 };
    std::iter::once(1).chain(resto)
}

/// Los índices de [`TIPOS_RECHAZO`] que valen en un filtro de la versión
/// `version`: los de su versión y `tcp_reset`.
pub fn rechazo_de(version: &str) -> impl Iterator<Item = usize> {
    let (a, b) = if version == "1" {
        (7..=7, 8..=13)
    } else {
        (1..=6, 7..=7)
    };
    a.chain(b)
}

/// ¿Usa la regla puertos (TCP o UDP)?
fn con_puertos(protocolo: &str) -> bool {
    matches!(protocolo, "2" | "3")
}

/// Una regla de `f` en forma de formulario (ver el módulo), con su filtro
/// (`belong`) y su versión.
pub fn vista_regla(f: &Filtro, r: &Regla) -> Valores {
    let puertos = con_puertos(&r.protocol);
    let lado = |dir: &str, mascara: &str| {
        if CUALQUIERA.contains(&dir) {
            (String::new(), String::new())
        } else {
            (dir.to_owned(), mascara.to_owned())
        }
    };
    let rango = |desde: &str, hasta: &str| {
        if puertos && desde != "0" && !desde.is_empty() {
            (desde.to_owned(), hasta.to_owned())
        } else {
            (String::new(), String::new())
        }
    };
    let (srcaddr, srcmask) = lado(&r.orig_addr, &r.orig_mask);
    let (dstaddr, dstmask) = lado(&r.dest_addr, &r.dest_mask);
    let (srcportstart, srcportend) = rango(&r.orig_start_port, &r.orig_end_port);
    let (dstportstart, dstportend) = rango(&r.dest_start_port, &r.dest_end_port);
    let si = |c: bool, v: &str| if c { v.to_owned() } else { String::new() };
    [
        ("belong", f.fid.clone()),
        ("version", f.version.clone()),
        ("enabled", r.enable.clone()),
        ("protocol", r.protocol.clone()),
        ("specificAction", r.action.clone()),
        ("rejectType", si(r.action == "1", &r.reject_type)),
        ("icmpType", si(r.protocol == "1", &r.icmp_type)),
        ("srcaddr", srcaddr),
        ("srcmask", srcmask),
        ("srcportstart", srcportstart),
        ("srcportend", srcportend),
        ("dstaddr", dstaddr),
        ("dstmask", dstmask),
        ("dstportstart", dstportstart),
        ("dstportend", dstportend),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect()
}

/// Un filtro en forma de formulario: `interface` es su `IntfID`.
pub fn vista_filtro(f: &Filtro) -> Valores {
    [
        ("name", &f.name),
        ("interface", &f.intf_id),
        ("type", &f.tipo),
        ("version", &f.version),
        ("default", &f.default),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.clone()))
    .collect()
}

/// Un campo que no vale: su parámetro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CampoMal(pub &'static str);

impl std::fmt::Display for CampoMal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: valor no válido", self.0)
    }
}

/// `k` de `v`, sin espacios alrededor (vacío si no está).
fn de<'a>(v: &'a Valores, k: &str) -> &'a str {
    v.get(k).map_or("", |s| s.trim())
}

fn numero_en(s: &str, valido: impl IntoIterator<Item = usize>) -> bool {
    s.parse::<usize>()
        .is_ok_and(|n| valido.into_iter().any(|x| x == n))
}

fn es_entero(s: &str) -> bool {
    !s.is_empty() && s.len() <= 9 && s.bytes().all(|b| b.is_ascii_digit())
}

/// Máscara IPv4 de bits seguidos.
fn mascara_v4(s: &str) -> bool {
    s.parse::<Ipv4Addr>().is_ok_and(|m| {
        let n = u32::from(m);
        n.leading_ones() + n.trailing_zeros() == 32
    })
}

/// Dirección y máscara de un lado (`addr`, `mask`) en `out`, ya
/// comprobadas para la versión `v6`. Sin dirección, ni la máscara; sin
/// máscara, la de un solo equipo.
fn lado(
    v: &Valores,
    (addr, mask): (&'static str, &'static str),
    v6: bool,
    out: &mut Valores,
) -> Result<(), CampoMal> {
    let (a, m) = (de(v, addr), de(v, mask));
    if a.is_empty() {
        if !m.is_empty() {
            return Err(CampoMal(mask));
        }
        out.insert(addr.into(), String::new());
        out.insert(mask.into(), String::new());
        return Ok(());
    }
    let dir_ok = match a.parse::<IpAddr>() {
        Ok(IpAddr::V4(x)) => !v6 && !x.is_unspecified(),
        Ok(IpAddr::V6(x)) => v6 && x != Ipv6Addr::UNSPECIFIED,
        Err(_) => false,
    };
    if !dir_ok {
        return Err(CampoMal(addr));
    }
    let m = match (m, v6) {
        ("", false) => "255.255.255.255".to_owned(),
        ("", true) => "128".to_owned(),
        (m, false) if mascara_v4(m) => m.to_owned(),
        (m, true) if m.parse::<u8>().is_ok_and(|p| (1..=128).contains(&p)) => {
            m.parse::<u8>().map(|p| p.to_string()).unwrap_or_default()
        }
        _ => return Err(CampoMal(mask)),
    };
    out.insert(addr.into(), a.to_owned());
    out.insert(mask.into(), m);
    Ok(())
}

/// Puertos de un lado (`start`, `end`) en `out`: con TCP o UDP, vacíos
/// («cualquiera») o de 1 a 65535 y en orden (sin el final, el inicial);
/// con otro protocolo, nada.
fn puertos(
    v: &Valores,
    (start, end): (&'static str, &'static str),
    con: bool,
    out: &mut Valores,
) -> Result<(), CampoMal> {
    let (s, e) = (de(v, start), de(v, end));
    let puerto = |x: &str| x.parse::<u16>().ok().filter(|p| *p >= 1);
    let (s, e) = match (con, s, e) {
        (_, "", "") => (String::new(), String::new()),
        (_, "", _) | (false, _, _) => return Err(CampoMal(start)),
        (true, s, e) => {
            let a = puerto(s).ok_or(CampoMal(start))?;
            let b = if e.is_empty() {
                a
            } else {
                puerto(e).ok_or(CampoMal(end))?
            };
            if b < a {
                return Err(CampoMal(end));
            }
            (a.to_string(), b.to_string())
        }
    };
    out.insert(start.into(), s);
    out.insert(end.into(), e);
    Ok(())
}

/// Comprueba los valores de una regla del filtro de versión `version` y
/// los devuelve completos y en forma de formulario (todos los
/// [`PARAMS_REGLA`]): un campo que no aplica tiene que venir vacío (con
/// valor, [`CampoMal`]); una máscara sin escribir es la de un solo equipo y
/// un puerto final sin escribir, el inicial. `version`, si viene, tiene
/// que ser la del filtro. Un parámetro que no es de regla, [`CampoMal`]
/// con su nombre si es conocido o con `"?"`.
pub fn normalizar_regla(v: &Valores, version: &str) -> Result<Valores, CampoMal> {
    if let Some(k) = v.keys().find(|k| !PARAMS_REGLA.contains(&k.as_str())) {
        return Err(CampoMal(
            PARAMS_FILTRO
                .iter()
                .find(|p| **p == k.as_str())
                .copied()
                .unwrap_or("?"),
        ));
    }
    let mut out = Valores::new();
    let mut fijo = |k: &'static str, ok: bool, valor: &str| {
        if ok {
            out.insert(k.into(), valor.to_owned());
            Ok(())
        } else {
            Err(CampoMal(k))
        }
    };
    let belong = de(v, "belong");
    fijo("belong", es_entero(belong), belong)?;
    let ver = de(v, "version");
    fijo("version", ver.is_empty() || ver == version, version)?;
    let enabled = de(v, "enabled");
    fijo("enabled", matches!(enabled, "0" | "1"), enabled)?;
    let protocol = de(v, "protocol");
    fijo("protocol", matches!(protocol, "1" | "2" | "3"), protocol)?;
    let action = de(v, "specificAction");
    fijo("specificAction", matches!(action, "0" | "1" | "2"), action)?;
    let rechazo = de(v, "rejectType");
    let rechazo_ok = if action == "1" {
        numero_en(rechazo, rechazo_de(version))
    } else {
        rechazo.is_empty()
    };
    fijo("rejectType", rechazo_ok, rechazo)?;
    let icmp = de(v, "icmpType");
    let icmp_ok = if protocol == "1" {
        numero_en(icmp, icmp_de(version))
    } else {
        icmp.is_empty()
    };
    fijo("icmpType", icmp_ok, icmp)?;
    let v6 = version == "1";
    let con = con_puertos(protocol);
    lado(v, ("srcaddr", "srcmask"), v6, &mut out)?;
    puertos(v, ("srcportstart", "srcportend"), con, &mut out)?;
    lado(v, ("dstaddr", "dstmask"), v6, &mut out)?;
    puertos(v, ("dstportstart", "dstportend"), con, &mut out)?;
    Ok(out)
}

/// Caracteres que valen en el nombre de un filtro: la lista de la web lo
/// pinta en el HTML y lo separa con `|` en `auxiliary`.
fn nombre_valido(s: &str) -> bool {
    !s.trim().is_empty()
        && s.chars().count() <= MAX_NOMBRE_FILTRO
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-' | '.'))
}

/// `IntfID` de una interfaz: `<tipo>.<id>` (`0.2`, `3.-1`).
fn interfaz_valida(s: &str) -> bool {
    let Some((a, b)) = s.split_once('.') else {
        return false;
    };
    es_entero(a) && (es_entero(b) || b.strip_prefix('-').is_some_and(es_entero))
}

/// Comprueba los valores de un filtro y los devuelve completos (todos los
/// [`PARAMS_FILTRO`]).
pub fn normalizar_filtro(v: &Valores) -> Result<Valores, CampoMal> {
    if v.keys().any(|k| !PARAMS_FILTRO.contains(&k.as_str())) {
        return Err(CampoMal("?"));
    }
    let comprobar = |k: &'static str, ok: fn(&str) -> bool| {
        let x = v.get(k).map_or("", String::as_str);
        if ok(x) {
            Ok(x.to_owned())
        } else {
            Err(CampoMal(k))
        }
    };
    let binario = |s: &str| matches!(s, "0" | "1");
    Ok([
        ("name", comprobar("name", nombre_valido)?),
        ("interface", comprobar("interface", interfaz_valida)?),
        ("type", comprobar("type", binario)?),
        ("version", comprobar("version", binario)?),
        ("default", comprobar("default", binario)?),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect())
}

/// La carga de una regla (`v`, ya normalizada con [`normalizar_regla`]),
/// en el orden de la web: el alta (`id` `None`) empieza con
/// `escape=true`; la edición, con el `id` de la regla. Lo vacío no viaja.
pub fn carga_regla(v: &Valores, id: Option<&str>) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = match id {
        None => vec![("escape", "true".into())],
        Some(id) => vec![("id", id.to_owned())],
    };
    out.push(("target", "rule".into()));
    let pares = [
        ("srcaddr", "srcmask"),
        ("srcportstart", "srcportend"),
        ("dstaddr", "dstmask"),
        ("dstportstart", "dstportend"),
    ];
    for k in PARAMS_REGLA {
        let valor = de(v, k);
        // Un par va entero si su primero tiene valor.
        let par = pares.iter().find(|(a, b)| *a == k || *b == k);
        let va = match par {
            Some((a, _)) => !de(v, a).is_empty(),
            None => !valor.is_empty() || !matches!(k, "rejectType" | "icmpType"),
        };
        if va {
            out.push((k, valor.to_owned()));
        }
    }
    out
}

/// La carga de un filtro (`v`, ya normalizado con [`normalizar_filtro`]),
/// en el orden de la web: el alta sin `escape`; la edición, con el `id`
/// del filtro.
pub fn carga_filtro(v: &Valores, id: Option<&str>) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> =
        id.map(|id| ("id", id.to_owned())).into_iter().collect();
    out.push(("target", "entry".into()));
    out.extend(PARAMS_FILTRO.iter().map(|k| (*k, de(v, k).to_owned())));
    out
}

/// ¿Coincide `leido` (forma de formulario) con `esperado` en los campos
/// `campos`? Las direcciones IP se comparan como direcciones (el router
/// puede escribirlas de otra forma); lo demás, sin espacios ni mayúsculas.
pub fn coincide(leido: &Valores, esperado: &Valores, campos: &[&str]) -> bool {
    campos.iter().all(|k| {
        let (a, b) = (de(leido, k), de(esperado, k));
        match (a.parse::<IpAddr>(), b.parse::<IpAddr>()) {
            (Ok(x), Ok(y)) => x == y,
            _ => a.eq_ignore_ascii_case(b),
        }
    })
}

#[cfg(test)]
mod tests;
