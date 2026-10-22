//! Derivados de Internet: la conexión, el relé DNS, el DNS dinámico y la franja de resumen.
//!
//! La conexión no es un ajuste (cambiarla puede cortar el acceso): su
//! estado sale de `cbGetWanIPIntf`, la lectura que usa también la página de
//! inicio, eligiendo la conexión de internet como la detección del router.

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

/// La conexión que lleva internet (`None` si no se pudo leer o no se sabe
/// cuál es).
fn conexion(ctx: &Contexto) -> Option<WanConn> {
    let conns = WanConn::parse_all(ctx.leida(&clave_conexiones())?);
    hgu::deteccion::elegir_wan(&conns, None).cloned()
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
