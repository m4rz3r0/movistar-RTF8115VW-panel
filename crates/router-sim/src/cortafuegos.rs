//! Cortafuegos del router simulado: los filtros de la página de la lista, sus dos páginas y el alta, la edición y la baja de reglas y filtros.
//!
//! Como el router real: `scbidirectionalflt_ls.asp` trae `var sessKey` y
//! `var filter` (JSON) y `scbidirectionalflt_ed.asp`, solo `var sessKey`
//! (ninguna trae `sessionKey`). Se escribe por GET en
//! `scbidirectionalflt.cmd` y cada escritura responde 301 a la lista
//! ([`REDIRECCION`]):
//!
//! - `action=add&target=rule&belong=<FID>&…` añade una regla al final del
//!   filtro con el RID siguiente al mayor; `action=modify&id=<RID>&target=rule&…`
//!   la cambia entera (lo que no viaja queda en «cualquiera»);
//! - `action=add&target=entry&…` añade un filtro vacío con el FID
//!   siguiente al mayor; `action=modify&id=<FID>&target=entry&…` lo cambia;
//! - `action=remove&item=<FID>|<RID>` quita una regla, e `item=<FID>` (sin
//!   `|`) el filtro entero.

use crate::estado::SimState;
use crate::http::param;
use hgu::cortafuegos::{Filtro, PAGINA_EDITOR, PAGINA_LISTA, Regla, analizar};

/// Fichero de las escrituras (sin la consulta).
pub(crate) const ORDEN: &str = "scbidirectionalflt.cmd";
/// A dónde manda el router tras cada escritura (301).
pub(crate) const REDIRECCION: &str = "/scbidirectionalflt_ls.asp";

/// Página real saneada: el estado de partida.
const PAGINA_DE_PARTIDA: &str =
    include_str!("../../hgu/fixtures/paginas/scbidirectionalflt_ls.asp");

/// Los filtros de partida (los de la copia saneada de la página real).
pub(crate) fn de_partida() -> Vec<Filtro> {
    analizar(PAGINA_DE_PARTIDA).unwrap_or_default()
}

/// ¿Es `pagina` una de las dos del cortafuegos?
pub(crate) fn es_suya(pagina: &str) -> bool {
    pagina == PAGINA_LISTA || pagina == PAGINA_EDITOR
}

/// Cuerpo de la página `pagina` del cortafuegos con el `sessKey` `clave`.
/// Con `ilegible`, la lista sale sin `var filter`.
pub(crate) fn pagina(st: &SimState, pagina: &str, clave: &str) -> String {
    let mut vars = format!("var sessKey = '{clave}';\r\n");
    if pagina == PAGINA_LISTA && !st.cortafuegos_ilegible {
        let json = serde_json::to_string(&st.cortafuegos).unwrap_or_else(|_| "[]".into());
        vars.push_str(&format!("var filter = {json};\r\n"));
    }
    format!("<html><head><script>\r\n{vars}</script></head><body>{pagina}</body></html>")
}

/// Aplica una escritura de `scbidirectionalflt.cmd` (sin el `sessionKey`).
pub(crate) fn escribir(st: &mut SimState, params: &[(String, String)]) {
    let id = param(params, "id").unwrap_or_default();
    match (param(params, "action"), param(params, "target")) {
        (Some("add"), Some("rule")) => alta(st, params),
        (Some("modify"), Some("rule")) => edicion(st, id, params),
        (Some("add"), Some("entry")) => alta_filtro(st, params),
        (Some("modify"), Some("entry")) => edicion_filtro(st, id, params),
        (Some("remove"), _) => baja(st, param(params, "item").unwrap_or_default()),
        _ => {}
    }
}

/// FID siguiente al mayor.
fn fid_nuevo(filtros: &[Filtro]) -> String {
    let mayor = filtros
        .iter()
        .filter_map(|f| f.fid.parse::<u32>().ok())
        .max();
    mayor.map_or(0, |m| m + 1).to_string()
}

/// Los datos de un filtro que vienen en `params`, sobre `f`. El nombre de
/// la interfaz, el de otro filtro con el mismo `IntfID` (o el `IntfID`).
fn datos_de_filtro(otros: &[Filtro], f: &mut Filtro, params: &[(String, String)]) {
    let o = |k: &str, actual: &str| param(params, k).unwrap_or(actual).to_owned();
    f.name = o("name", &f.name);
    f.intf_id = o("interface", &f.intf_id);
    f.tipo = o("type", &f.tipo);
    f.version = o("version", &f.version);
    f.default = o("default", &f.default);
    f.interface = otros
        .iter()
        .find(|x| x.intf_id == f.intf_id && x.fid != f.fid)
        .map_or_else(|| f.intf_id.clone(), |x| x.interface.clone());
}

fn alta_filtro(st: &mut SimState, params: &[(String, String)]) {
    let mut f = Filtro {
        fid: fid_nuevo(&st.cortafuegos),
        name: String::new(),
        interface: String::new(),
        intf_id: String::new(),
        tipo: "0".into(),
        version: "0".into(),
        default: "0".into(),
        reglas: Vec::new(),
    };
    datos_de_filtro(&st.cortafuegos, &mut f, params);
    st.cortafuegos.push(f);
}

fn edicion_filtro(st: &mut SimState, fid: &str, params: &[(String, String)]) {
    let otros = st.cortafuegos.clone();
    if let Some(f) = st.cortafuegos.iter_mut().find(|f| f.fid == fid) {
        datos_de_filtro(&otros, f, params);
    }
}

/// La regla `rid` con lo que traen `params` (lo que no viaja, por defecto).
fn regla(rid: String, v6: bool, params: &[(String, String)]) -> Regla {
    let (cualquiera, todo) = if v6 {
        ("::", "128")
    } else {
        ("0.0.0.0", "255.255.255.255")
    };
    let o = |k: &str, defecto: &str| param(params, k).unwrap_or(defecto).to_owned();
    Regla {
        rid,
        enable: o("enabled", "1"),
        protocol: o("protocol", "0"),
        action: o("specificAction", "0"),
        reject_type: o("rejectType", "0"),
        icmp_type: o("icmpType", "0"),
        orig_addr: o("srcaddr", cualquiera),
        orig_mask: o("srcmask", todo),
        orig_start_port: o("srcportstart", "0"),
        orig_end_port: o("srcportend", "0"),
        dest_addr: o("dstaddr", cualquiera),
        dest_mask: o("dstmask", todo),
        dest_start_port: o("dstportstart", "0"),
        dest_end_port: o("dstportend", "0"),
    }
}

fn edicion(st: &mut SimState, rid: &str, params: &[(String, String)]) {
    if let Some(f) = st
        .cortafuegos
        .iter_mut()
        .find(|f| f.reglas.iter().any(|r| r.rid == rid))
    {
        let v6 = f.version == "1";
        if let Some(r) = f.reglas.iter_mut().find(|r| r.rid == rid) {
            *r = regla(rid.to_owned(), v6, params);
        }
    }
}

/// RID siguiente al mayor de todos los filtros.
fn rid_nuevo(filtros: &[Filtro]) -> String {
    let mayor = filtros
        .iter()
        .flat_map(|f| &f.reglas)
        .filter_map(|r| r.rid.parse::<u32>().ok())
        .max();
    mayor.map_or(0, |m| m + 1).to_string()
}

fn alta(st: &mut SimState, params: &[(String, String)]) {
    let rid = rid_nuevo(&st.cortafuegos);
    let Some(f) = st
        .cortafuegos
        .iter_mut()
        .find(|f| param(params, "belong") == Some(f.fid.as_str()))
    else {
        return;
    };
    let v6 = f.version == "1";
    f.reglas.push(regla(rid, v6, params));
}

/// `item` = `<FID>|<RID>` quita la regla; `<FID>` solo, el filtro entero.
/// Con `cortafuegos_baja_de_mas`, la de una regla borra además el filtro o
/// otra regla.
fn baja(st: &mut SimState, item: &str) {
    match item.split_once('|') {
        Some((fid, rid)) => match st.cortafuegos_baja_de_mas.take() {
            Some(None) => st.cortafuegos.retain(|f| f.fid != fid),
            otra => {
                let otra = otra.flatten();
                if let Some(f) = st.cortafuegos.iter_mut().find(|f| f.fid == fid) {
                    f.reglas
                        .retain(|r| r.rid != rid && Some(&r.rid) != otra.as_ref());
                }
            }
        },
        None => st.cortafuegos.retain(|f| f.fid != item),
    }
}
