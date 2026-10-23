//! Cortafuegos del router simulado: los filtros de la página de la lista, sus dos páginas y el alta y la baja de reglas.
//!
//! Como el router real: `scbidirectionalflt_ls.asp` trae `var sessKey` y
//! `var filter` (JSON) y `scbidirectionalflt_ed.asp`, solo `var sessKey`
//! (ninguna trae `sessionKey`). Se escribe por GET en
//! `scbidirectionalflt.cmd`: `action=add&target=rule&belong=<FID>&…` añade
//! una regla al final del filtro con el RID siguiente al mayor;
//! `action=remove&item=<FID>|<RID>` la quita, e `item=<FID>` (sin `|`)
//! quita el filtro entero.

use crate::estado::SimState;
use crate::http::param;
use hgu::cortafuegos::{Filtro, PAGINA_EDITOR, PAGINA_LISTA, Regla, analizar};

/// Fichero de las escrituras (sin la consulta).
pub(crate) const ORDEN: &str = "scbidirectionalflt.cmd";

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
    match param(params, "action") {
        Some("add") if param(params, "target") == Some("rule") => alta(st, params),
        Some("remove") => baja(st, param(params, "item").unwrap_or_default()),
        _ => {}
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
    let (cualquiera, todo) = if v6 {
        ("::", "128")
    } else {
        ("0.0.0.0", "255.255.255.255")
    };
    let o = |k: &str, defecto: &str| param(params, k).unwrap_or(defecto).to_owned();
    f.reglas.push(Regla {
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
    });
}

/// `item` = `<FID>|<RID>` quita la regla; `<FID>` solo, el filtro entero.
fn baja(st: &mut SimState, item: &str) {
    match item.split_once('|') {
        Some((fid, rid)) => {
            if let Some(f) = st.cortafuegos.iter_mut().find(|f| f.fid == fid) {
                f.reglas.retain(|r| r.rid != rid);
            }
        }
        None => st.cortafuegos.retain(|f| f.fid != item),
    }
}
