//! Cortafuegos de la web oculta del router: la lista de filtros de `scbidirectionalflt_ls.asp`, la regla «sin ping» del panel y lo abierto desde internet.
//!
//! La página trae todos los filtros en `var filter = [...]` (JSON). Cada
//! filtro (`FID`, `Name`…) lleva sus reglas (`rule`, con su `RID`). Las
//! reglas no tienen nombre: la del panel se reconoce por su huella
//! ([`es_sin_ping`]): ICMP, descartar, `echo_request` y origen y destino
//! «cualquiera».
//!
//! Se escribe por GET con la carga del editor (`scbidirectionalflt_ed.asp`,
//! `btnApplyRule`): el alta, [`carga_sin_ping`] en [`ALTA`]; la baja,
//! `item=<FID>|<RID>` ([`item`]) en [`BAJA`], como `removeClick`.

use crate::client::Client;
use crate::error::Error;
use crate::xml::XmlNode;
use serde::{Deserialize, Serialize};

/// Página con la lista de filtros (`var sessKey` y `var filter`).
pub const PAGINA_LISTA: &str = "scbidirectionalflt_ls.asp";
/// Página del editor (solo `var sessKey`).
pub const PAGINA_EDITOR: &str = "scbidirectionalflt_ed.asp";
/// Alta de una regla (o de un filtro).
pub const ALTA: &str = "scbidirectionalflt.cmd?action=add";
/// Baja de una regla (`item=<FID>|<RID>`) o de un filtro entero (`item=<FID>`).
pub const BAJA: &str = "scbidirectionalflt.cmd?action=remove";

/// Un filtro del cortafuegos, como lo da la página.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filtro {
    #[serde(rename = "FID")]
    pub fid: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Interface")]
    pub interface: String,
    #[serde(rename = "IntfID")]
    pub intf_id: String,
    /// 0 entrada, 1 salida.
    #[serde(rename = "Type")]
    pub tipo: String,
    /// 0 IPv4, 1 IPv6.
    #[serde(rename = "Version")]
    pub version: String,
    /// Acción por defecto: 0 descartar, 1 permitir.
    #[serde(rename = "Default")]
    pub default: String,
    #[serde(rename = "rule", default)]
    pub reglas: Vec<Regla>,
}

/// Una regla de un filtro.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Regla {
    #[serde(rename = "RID")]
    pub rid: String,
    pub enable: String,
    /// 0 ninguno («NONE»: no filtra por protocolo), 1 ICMP, 2 TCP, 3 UDP.
    pub protocol: String,
    /// 0 descartar, 1 rechazar, 2 permitir.
    pub action: String,
    pub reject_type: String,
    pub icmp_type: String,
    pub orig_addr: String,
    pub orig_mask: String,
    pub orig_start_port: String,
    pub orig_end_port: String,
    pub dest_addr: String,
    pub dest_mask: String,
    pub dest_start_port: String,
    pub dest_end_port: String,
}

/// Lo que se puede abrir desde internet con una regla de entrada.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Servicio {
    /// Responder al ping (ICMP `echo_request` o cualquiera).
    Ping,
    /// La web del router (80 o 443).
    Web,
    /// SSH (22).
    Ssh,
    /// La gestión remota del operador (7547).
    GestionOperador,
}

/// Un servicio abierto desde internet y si lo está desde cualquier origen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abierto {
    pub servicio: Servicio,
    pub desde_cualquiera: bool,
}

/// Datos del filtro que lleva cada `<regla>` de [`a_xml`].
const CAMPOS_FILTRO: [&str; 7] = [
    "FID",
    "Name",
    "Interface",
    "IntfID",
    "Type",
    "Version",
    "Default",
];
/// Datos de la regla en cada `<regla>` de [`a_xml`].
const CAMPOS_REGLA: [&str; 14] = [
    "RID",
    "Enable",
    "Protocol",
    "Action",
    "RejectType",
    "IcmpType",
    "OrigAddr",
    "OrigMask",
    "OrigStartPort",
    "OrigEndPort",
    "DestAddr",
    "DestMask",
    "DestStartPort",
    "DestEndPort",
];
/// Etiqueta de cada regla en [`a_xml`].
const REGLA: &str = "regla";
/// Origen o destino «cualquiera».
const CUALQUIERA: [&str; 3] = ["", "0.0.0.0", "::"];
/// `IcmpType` que responden al ping: cualquiera (1) y `echo_request` de
/// IPv4 (25) y de IPv6 (55).
const ICMP_PING: [&str; 3] = ["1", "25", "55"];

/// Los filtros de la página de la lista (`var filter = [...]`). El error
/// nunca lleva el HTML ni lo que traía el JSON.
pub fn analizar(html: &str) -> Result<Vec<Filtro>, Error> {
    static VAR_FILTER: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"var\s+filter\s*=\s*").expect("regex válida")
    });
    let Some(m) = VAR_FILTER.find(html) else {
        return Err(Error::Unexpected(
            "cortafuegos: la página no trae la lista de filtros".into(),
        ));
    };
    let resto = &html[m.end()..];
    serde_json::Deserializer::from_str(resto)
        .into_iter::<Vec<Filtro>>()
        .next()
        .unwrap_or_else(|| Err(serde::de::Error::custom("vacío")))
        .map_err(|e| {
            Error::Unexpected(format!(
                "cortafuegos: la lista de filtros no es válida (línea {}, columna {})",
                e.line(),
                e.column()
            ))
        })
}

/// Las hojas `campos` de un objeto JSON (los valores, como texto).
fn hojas(v: &serde_json::Value, campos: &[&str]) -> Vec<XmlNode> {
    campos
        .iter()
        .map(|k| XmlNode {
            tag: (*k).to_owned(),
            text: v
                .get(*k)
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_owned(),
            children: Vec::new(),
        })
        .collect()
}

/// Los filtros como un nodo `<scbidirectionalflt_ls.asp>` (el nombre de la
/// lectura, como las respuestas `cb*`): un hijo `<regla>` por regla, con
/// los datos de su filtro (`FID`, `Name`, `Interface`, `IntfID`, `Type`,
/// `Version`, `Default`) y los suyos. Un filtro sin reglas no sale.
pub fn a_xml(f: &[Filtro]) -> XmlNode {
    let mut children = Vec::new();
    for filtro in f {
        let fv = serde_json::to_value(filtro).unwrap_or_default();
        for r in &filtro.reglas {
            let rv = serde_json::to_value(r).unwrap_or_default();
            let mut hijos = hojas(&fv, &CAMPOS_FILTRO);
            hijos.extend(hojas(&rv, &CAMPOS_REGLA));
            children.push(XmlNode {
                tag: REGLA.to_owned(),
                text: String::new(),
                children: hijos,
            });
        }
    }
    XmlNode {
        tag: PAGINA_LISTA.into(),
        text: String::new(),
        children,
    }
}

/// Los filtros de un nodo de [`a_xml`] (lo que guarda la caché de
/// lecturas), en el mismo orden. Una regla incompleta no cuenta.
pub fn de_xml(n: &XmlNode) -> Vec<Filtro> {
    let objeto = |r: &XmlNode, campos: &[&str]| {
        let m: serde_json::Map<String, serde_json::Value> = campos
            .iter()
            .filter_map(|k| r.text_of(k).map(|v| ((*k).to_owned(), v.into())))
            .collect();
        serde_json::Value::Object(m)
    };
    let mut out: Vec<Filtro> = Vec::new();
    for r in n.all(REGLA) {
        let Ok(regla) = serde_json::from_value::<Regla>(objeto(r, &CAMPOS_REGLA)) else {
            continue;
        };
        match out.last_mut() {
            Some(f) if r.text_of("FID") == Some(f.fid.as_str()) => f.reglas.push(regla),
            _ => {
                if let Ok(mut f) = serde_json::from_value::<Filtro>(objeto(r, &CAMPOS_FILTRO)) {
                    f.reglas.push(regla);
                    out.push(f);
                }
            }
        }
    }
    out
}

/// El filtro de entrada (`Type` 0) IPv4 (`Version` 0) de la conexión de
/// `ip_intf_id` (`IntfID` = `0.<ip_intf_id>`).
pub fn filtro_de_entrada_v4<'a>(f: &'a [Filtro], ip_intf_id: &str) -> Option<&'a Filtro> {
    let intf = format!("0.{ip_intf_id}");
    f.iter()
        .find(|x| x.tipo == "0" && x.version == "0" && x.intf_id == intf)
}

/// Carga del alta de la regla «sin ping» en `filtro`, la de `btnApplyRule`
/// del editor y en su orden: activa, ICMP, descartar y `echo_request`, sin
/// id, puertos ni direcciones (con ICMP no se envían).
pub fn carga_sin_ping(filtro: &Filtro) -> Vec<(&'static str, String)> {
    [
        ("target", "rule"),
        ("belong", filtro.fid.as_str()),
        ("version", "0"),
        ("enabled", "1"),
        ("protocol", "1"),
        ("specificAction", "0"),
        ("icmpType", "25"),
    ]
    .into_iter()
    .map(|(k, v)| (k, v.to_owned()))
    .collect()
}

/// ¿Tiene `r` la huella de la regla «sin ping» del panel? ICMP (1),
/// descartar (0), `echo_request` (25) y origen y destino «cualquiera».
pub fn es_sin_ping(r: &Regla) -> bool {
    r.protocol == "1"
        && r.action == "0"
        && r.icmp_type == "25"
        && CUALQUIERA.contains(&r.orig_addr.as_str())
        && CUALQUIERA.contains(&r.dest_addr.as_str())
}

/// `item` de la baja de la regla `rid` del filtro `fid`: `<FID>|<RID>`
/// (sin la regla, el router borraría el filtro entero).
pub fn item(fid: &str, rid: &str) -> String {
    format!("{fid}|{rid}")
}

/// ¿Abre la regla TCP `r` el puerto `p`? Un puerto inicial 0 es
/// «cualquiera», como lo pinta el router (`dsport==="0"` sale vacío en
/// `scbidirectionalflt_ls.asp`): abre todos.
fn abre(r: &Regla, p: u32) -> bool {
    let desde = r.dest_start_port.parse::<u32>().unwrap_or(0);
    let hasta = r.dest_end_port.parse::<u32>().unwrap_or(0).max(desde);
    desde == 0 || (desde..=hasta).contains(&p)
}

/// Los servicios TCP que abre `r`.
fn servicios_tcp(r: &Regla) -> impl Iterator<Item = Servicio> + '_ {
    [
        (Servicio::Web, &[80, 443][..]),
        (Servicio::Ssh, &[22][..]),
        (Servicio::GestionOperador, &[7547][..]),
    ]
    .into_iter()
    .filter(|(_, puertos)| puertos.iter().any(|p| abre(r, *p)))
    .map(|(s, _)| s)
}

/// Lo que abre una regla de entrada que permite el paso. El protocolo 0
/// («NONE», la primera entrada de `PROTOCOL` en la página; el editor no lo
/// ofrece) no filtra por protocolo: cuenta como todos, el ping incluido
/// (mejor avisar de más que callar algo abierto).
fn servicios_de(r: &Regla) -> Vec<Servicio> {
    match r.protocol.as_str() {
        "0" => std::iter::once(Servicio::Ping)
            .chain(servicios_tcp(r))
            .collect(),
        "1" if ICMP_PING.contains(&r.icmp_type.as_str()) => vec![Servicio::Ping],
        "2" => servicios_tcp(r).collect(),
        _ => Vec::new(),
    }
}

/// Servicios abiertos desde internet por las reglas activas que permiten el
/// paso (`Action` 2) en los filtros de entrada, cada uno una vez y en el
/// orden de [`Servicio`]: desde cualquier origen si alguna lo abre así.
pub fn abiertos(f: &[Filtro]) -> Vec<Abierto> {
    let mut out: Vec<Abierto> = Vec::new();
    let reglas = f
        .iter()
        .filter(|x| x.tipo == "0")
        .flat_map(|x| &x.reglas)
        .filter(|r| r.enable == "1" && r.action == "2");
    for r in reglas {
        let cualquiera = CUALQUIERA.contains(&r.orig_addr.as_str());
        for s in servicios_de(r) {
            match out.iter_mut().find(|a| a.servicio == s) {
                Some(a) => a.desde_cualquiera |= cualquiera,
                None => out.push(Abierto {
                    servicio: s,
                    desde_cualquiera: cualquiera,
                }),
            }
        }
    }
    let orden = [
        Servicio::Ping,
        Servicio::Web,
        Servicio::Ssh,
        Servicio::GestionOperador,
    ];
    out.sort_by_key(|a| orden.iter().position(|s| *s == a.servicio));
    out
}

/// Lee los filtros del router.
pub async fn leer(c: &Client) -> Result<Vec<Filtro>, Error> {
    analizar(&c.basic_page(PAGINA_LISTA).await?)
}

#[cfg(test)]
mod tests;
