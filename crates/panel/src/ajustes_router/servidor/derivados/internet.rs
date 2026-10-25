//! Derivados de Internet: la conexión, el relé DNS, el DNS dinámico y la franja de resumen.
//!
//! La conexión es de solo lectura (cambiarla puede cortar el acceso): su
//! estado sale de `cbGetWanIPIntf`, la lectura que usa también la página de
//! inicio, eligiendo la conexión de internet como el gestor: la del panel
//! (elegida a mano o detectada) y, sin ella, la de la detección. Las demás
//! conexiones (voz, servicios) se marcan «del operador».

use super::*;
use hgu::models::{TipoConexion, WanConn};

/// Lectura de las conexiones a internet del router.
const CONEXIONES: &str = "cbGetWanIPIntf";

fn clave_conexiones() -> Clave {
    Clave::new::<&str, &str>(CONEXIONES, &[])
}

/// Lecturas extra del tema: las conexiones a internet.
pub(super) fn claves_extra() -> Vec<Clave> {
    vec![clave_conexiones()]
}

/// Las conexiones leídas en la tanda (ninguna si la lectura falló).
pub(in crate::ajustes_router::servidor) fn conexiones(
    leidas: &HashMap<Clave, Result<XmlNode, hgu::Error>>,
) -> Vec<WanConn> {
    leidas
        .get(&clave_conexiones())
        .and_then(|r| r.as_ref().ok())
        .map(WanConn::parse_all)
        .unwrap_or_default()
}

/// La conexión de internet entre `conns`, como el gestor y el cortafuegos:
/// la del panel (`wan`, la elegida a mano o la detectada, ver
/// `RedActual::wan_en`) si existe; si no, la que elige la
/// detección. `None` si no se sabe cuál es.
fn de_internet<'a>(conns: &'a [WanConn], wan: Option<&str>) -> Option<&'a WanConn> {
    wan.and_then(|n| conns.iter().find(|c| c.name == n))
        .or_else(|| hgu::deteccion::elegir_wan(conns, None))
}

/// La conexión que lleva internet (`None` si no se pudo leer o no se sabe
/// cuál es).
fn conexion(ctx: &Contexto) -> Option<WanConn> {
    let conns = WanConn::parse_all(ctx.leida(&clave_conexiones())?);
    de_internet(&conns, ctx.wan).cloned()
}

/// Clave de la nota de las conexiones que no son la de internet.
const DEL_OPERADOR: &str = "ajuste-internet-conexion-del-operador";

/// Clave de la nota de todas las conexiones cuando no se sabe cuál es la de
/// internet: ninguna se presenta como tal.
const SIN_IDENTIFICAR: &str = "ajuste-internet-conexion-sin-identificar";

/// Marca «del operador» las conexiones de la tarjeta que no son la de
/// internet (ver [`de_internet`], con `wan` la del panel) y pone esta
/// primero. Si no se sabe cuál es, las marca todas «sin identificar».
pub(in crate::ajustes_router::servidor) fn marcar_conexiones(
    vistas: &mut [AjusteVista],
    leidas: &HashMap<Clave, Result<XmlNode, hgu::Error>>,
    wan: Option<&str>,
) {
    let slug = Componente::Conexion.slug();
    let Some(v) = vistas
        .iter_mut()
        .find(|v| v.componente.as_deref() == Some(slug))
    else {
        return;
    };
    let conns = conexiones(leidas);
    let internet = de_internet(&conns, wan).map(|c| c.ip_intf_id.as_str());
    for i in &mut v.instancias {
        i.nota = match internet {
            Some(id) => (i.id != id).then(|| DEL_OPERADOR.to_owned()),
            None => Some(SIN_IDENTIFICAR.to_owned()),
        };
    }
    v.instancias.sort_by_key(|i| i.nota.is_some());
}

/// `derivado:estado_conexion`: «Conectada (PPPoE)» (o el tipo que sea) si la
/// conexión de internet está en marcha y con dirección; si no, «Sin
/// conexión». «Sin leer» si la lectura falló y «Sin datos» si se leyó pero
/// no se sabe cuál es la de internet.
pub(super) fn estado_conexion(ctx: &Contexto) -> (EstadoVisual, String) {
    let (estado, clave) = match conexion(ctx) {
        None if ctx.leida(&clave_conexiones()).is_none() => {
            (EstadoVisual::Atencion, "ajustes-estado-sin-leer")
        }
        None => (EstadoVisual::Atencion, "comun-sin-datos"),
        Some(c) if !(c.up && c.ipv4.is_some()) => {
            (EstadoVisual::Atencion, "ajustes-estado-conexion-sin")
        }
        Some(c) => (
            EstadoVisual::Emite,
            match c.tipo() {
                TipoConexion::Pppoe => "ajustes-estado-conexion-pppoe",
                TipoConexion::Dhcp => "ajustes-estado-conexion-dhcp",
                TipoConexion::Estatica => "ajustes-estado-conexion-fija",
                TipoConexion::Otro => "ajustes-estado-conexion-otro",
            },
        ),
    };
    (estado, clave.to_owned())
}

/// Cifra y explicación del dato «Conexión»: conectada (tiene dirección y
/// está en marcha) o no, y de qué tipo. Pura.
pub(super) fn cifra_conexion(c: Option<&WanConn>) -> (String, String) {
    let Some(c) = c else {
        return (sin_dato(), t!("ajustes-resumen-internet-conexion-sin-leer"));
    };
    let cifra = if c.up && c.ipv4.is_some() {
        t!("ajustes-resumen-internet-conectada")
    } else {
        t!("ajustes-resumen-internet-sin-conexion")
    };
    let tipo = match c.tipo() {
        TipoConexion::Pppoe => t!("ajustes-resumen-internet-tipo-pppoe"),
        TipoConexion::Dhcp => t!("ajustes-resumen-internet-tipo-dhcp"),
        TipoConexion::Estatica => t!("ajustes-resumen-internet-tipo-fija"),
        TipoConexion::Otro => t!("ajustes-resumen-internet-tipo-otro"),
    };
    (cifra, tipo)
}

/// `derivado:estado_dns`: el relé, activado o no.
pub(super) fn estado_dns(ctx: &Contexto) -> (EstadoVisual, String) {
    segun(
        si_no(ctx, "internet.dns", "rele"),
        (EstadoVisual::Emite, "ajustes-estado-dns-si"),
        (EstadoVisual::Apagada, "ajustes-estado-dns-no"),
    )
}

/// Nombres dados de alta en el DNS dinámico y si cada uno está activo.
fn nombres(ctx: &Contexto) -> Option<Vec<(String, bool)>> {
    let lista = instancias(ctx, "internet.ddns")?;
    Some(
        lista
            .iter()
            .map(|i| {
                let campo = |id: &str| i.campos.iter().find(|c| c.id == id).map(|c| &c.valor);
                let nombre = match campo("nombre") {
                    Some(ValorLegible::Texto(t)) => t.clone(),
                    _ => String::new(),
                };
                let activo = matches!(campo("activa"), Some(ValorLegible::Booleano(true)));
                (nombre, activo)
            })
            .collect(),
    )
}

/// `derivado:estado_ddns`: activo si alguno de sus nombres lo está; sin
/// ninguno, «Sin usar».
pub(super) fn estado_ddns(ctx: &Contexto) -> (EstadoVisual, String) {
    let (estado, clave) = match nombres(ctx) {
        None => (EstadoVisual::Atencion, "ajustes-estado-desconocido"),
        Some(n) if n.is_empty() => (EstadoVisual::Apagada, "ajustes-estado-ddns-sin-usar"),
        Some(n) if n.iter().any(|(_, activo)| *activo) => {
            (EstadoVisual::Emite, "ajustes-estado-ddns-si")
        }
        Some(_) => (EstadoVisual::Apagada, "ajustes-estado-ddns-no"),
    };
    (estado, clave.to_owned())
}

/// `derivado:nombres_ddns`: los nombres dados de alta, o «Ninguno».
pub(super) fn nombres_ddns(ctx: &Contexto) -> Option<ValorLegible> {
    let n = nombres(ctx)?;
    Some(ValorLegible::Texto(if n.is_empty() {
        t!("ajustes-dato-nombres_ddns-ninguno")
    } else {
        n.into_iter()
            .map(|(nombre, _)| nombre)
            .collect::<Vec<_>>()
            .join(", ")
    }))
}

/// Franja de resumen de Internet: la conexión, el relé DNS, los nombres del
/// DNS dinámico y las recomendaciones pendientes.
pub(super) fn resumen(
    ctx: &Contexto,
    elementos: &[(&Elemento, &ElementoVista)],
) -> Vec<DatoResumen> {
    let (cifra, tipo) = cifra_conexion(conexion(ctx).as_ref());
    let rele = "ajustes-resumen-internet-rele";
    let ddns = "ajustes-resumen-internet-ddns";
    vec![
        dato_resumen("ajustes-resumen-internet-conexion", cifra, tipo, false),
        dato_resumen(
            rele,
            cifra_si_no(si_no(ctx, "internet.dns", "rele")),
            detalle_fijo(rele),
            false,
        ),
        dato_resumen(
            ddns,
            nombres(ctx).map_or_else(sin_dato, |n| n.len().to_string()),
            detalle_fijo(ddns),
            false,
        ),
        recomendaciones(elementos),
    ]
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    fn conn(up: bool, ip: Option<&str>, wan_type: &str) -> WanConn {
        WanConn {
            name: "ip2".into(),
            ip_intf_id: "2".into(),
            up,
            enabled: true,
            wan_type: wan_type.into(),
            app_mask: 3,
            ipv4: ip.map(|i| i.parse().unwrap()),
        }
    }

    #[test]
    fn conexion_conectada_o_no_y_su_tipo() {
        let (cifra, tipo) = cifra_conexion(Some(&conn(true, Some("203.0.113.6"), "0")));
        assert_eq!(cifra, "Conectada");
        assert!(tipo.contains("PPPoE"), "{tipo}");
        let (cifra, _) = cifra_conexion(Some(&conn(false, Some("203.0.113.6"), "1")));
        assert_eq!(cifra, "Sin conexión");
        let (cifra, _) = cifra_conexion(Some(&conn(true, None, "2")));
        assert_eq!(cifra, "Sin conexión");
        let (cifra, _) = cifra_conexion(None);
        assert_eq!(cifra, "—");
    }
}
