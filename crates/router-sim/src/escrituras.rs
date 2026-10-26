//! Escrituras avanzadas reflejadas en sus lecturas pareadas.

use crate::estado::{Aplazada, Edicion, HangAfterWrite, Inner, ListaViva, Params, SimState};
use crate::http::param;
use crate::lecturas::{all_instances, aplicar_campos};
use crate::ofuscado;
use crate::xml::{failure, leaf, ok, response, result_node, set_child};
use catalogo::Lista;
use hgu::xml::XmlNode;
use std::{collections::BTreeMap, time::Instant};

pub(crate) fn write(
    s: &Inner,
    st: &mut SimState,
    name: &str,
    params: &[(String, String)],
) -> String {
    let key_ok = st
        .session_key
        .as_deref()
        .is_some_and(|k| param(params, "sessionKey") == Some(k));
    if !key_ok {
        return failure(name, "sessionKey no válido");
    }
    let clean: Params = params
        .iter()
        .filter(|(k, _)| k != "sessionKey")
        .cloned()
        .collect();
    // Validación previa (página WiFi): nunca aplica nada ni se registra;
    // puede consumir `fail_next`, pero no `ignore_next`.
    let validation_only = param(&clean, "validation_only") == Some("1");
    let mut fail_n = false;
    if !validation_only {
        st.real_writes += 1;
        let n = st.real_writes;
        fail_n = st.fail_write_at.remove(&n);
        if st.hang_at_write == Some(n) {
            st.hang_at_write = None;
            st.hang_after_write = HangAfterWrite::Armed;
        }
        st.writes.push((name.to_owned(), clean.clone()));
        if let Some(r) = st.fail_reads_after_write.take() {
            st.failing_reads.insert(r);
        }
        if st.hang_after_write == HangAfterWrite::Armed {
            st.hang_after_write = HangAfterWrite::Now;
        }
    }
    if let Some(err) = st.fail_next.take() {
        return failure(name, &err);
    }
    if fail_n {
        return failure(name, "escritura rechazada (simulado)");
    }
    if validation_only {
        return ok(name);
    }
    if std::mem::take(&mut st.ignore_next) {
        return ok(name);
    }
    let result = match name {
        "cbModifyPwd" => modify_pwd(s, st, &clean),
        n if crate::usuarios::es_suya(n) => {
            return crate::usuarios::escribir(&mut s.usuarios(), n, &clean);
        }
        "cbReboot" => {
            st.invalidate_sessions();
            st.rebooted = Some(Instant::now());
            return response(&result_node(
                name,
                vec![leaf("Msg", "Rebooting...")],
                "0",
                "",
            ));
        }
        _ => escritura_generica(s, st, name, &clean),
    };
    match result {
        Ok(()) => ok(name),
        Err(e) => failure(name, &e),
    }
}

/// Cambia la contraseña del usuario del sim (`rqUser`, `rqOPwd`, `rqNPwd`,
/// leídos con la copia propia del `mess_userpass` del router:
/// [`ofuscado::deshacer`]). Las sesiones abiertas siguen valiendo,
/// como en el router. Con `modify_pwd_con_retraso` o
/// `modify_pwd_tras_inicios`, la nueva queda pendiente hasta que toca.
fn modify_pwd(s: &Inner, st: &mut SimState, p: &[(String, String)]) -> Result<(), String> {
    let leer = |k: &str| param(p, k).map(ofuscado::deshacer);
    let nueva = leer("rqNPwd").ok_or("falta rqNPwd")?;
    let actual = s.password();
    if leer("rqUser").as_deref() != Some(s.user.as_str())
        || leer("rqOPwd").as_deref() != Some(actual.as_str())
    {
        return Err("usuario o contraseña actual incorrectos".to_owned());
    }
    if std::mem::take(&mut st.modify_pwd_sin_efecto) {
        return Ok(());
    }
    let aplazada = match (
        st.modify_pwd_retraso.take(),
        st.modify_pwd_tras_inicios.take(),
    ) {
        (_, Some(n)) => Some(Aplazada::TrasInicios(n)),
        (Some(retraso), None) => Some(Aplazada::Desde(Instant::now() + retraso)),
        (None, None) => None,
    };
    match aplazada {
        Some(a) => st.pwd_pendiente = Some((nueva, a)),
        None => s.fijar_password(nueva),
    }
    Ok(())
}

/// Refleja una escritura avanzada en su lectura pareada del catálogo. Una
/// función sin lectura pareada no cambia nada.
fn escritura_generica(
    s: &Inner,
    st: &mut SimState,
    name: &str,
    p: &[(String, String)],
) -> Result<(), String> {
    let Some(f) = s.catalogo.funcion(name) else {
        return Ok(());
    };
    let Some(pareada) = &f.lectura_pareada else {
        return Ok(());
    };
    let lectura = pareada.funcion.as_str();
    if let Some(lista) = &f.lista {
        if name == "cbDhcpV4SvrPoolSetStatic" {
            let p = validar_reserva(s, st, lectura, p)?;
            return escribir_lista(s, st, lectura, lista, &p);
        }
        return escribir_lista(s, st, lectura, lista, p);
    }
    // La instancia: los `{x}` de la lectura pareada, con el valor escrito.
    let mut ident = Params::new();
    for (_, v) in &pareada.params {
        if let Some(x) = v.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
            let valor = param(p, x).ok_or_else(|| format!("falta {x}"))?;
            ident.push((x.to_owned(), valor.to_owned()));
        }
    }
    let mut campos = campos_escritos(p, &[]);
    derivar_campos(name, &mut campos);
    match st
        .ediciones
        .iter_mut()
        .find(|e| e.lectura == lectura && e.ident == ident)
    {
        Some(e) => e.campos.extend(campos),
        None => st.ediciones.push(Edicion {
            lectura: lectura.to_owned(),
            ident,
            campos,
        }),
    }
    Ok(())
}

/// Lo que el router cambia por su cuenta al escribir: encender o apagar
/// una red WiFi (`adm_state`) hace que emita o deje de emitir
/// (`opr_status`).
fn derivar_campos(name: &str, campos: &mut BTreeMap<String, String>) {
    if name == "cbSetWifiAccPt"
        && let Some(v) = campos.get("adm_state").cloned()
    {
        campos.insert("opr_status".to_owned(), v);
    }
}

/// Reglas de una reserva DHCP que el router aplica y el catálogo no recoge:
/// un solo pool (el 0), id obligatorio (`-1` crea; otro que no existe, no),
/// MAC e IP al crear. Devuelve los parámetros con `adm_state` activado si se
/// crea sin él.
fn validar_reserva(
    s: &Inner,
    st: &mut SimState,
    lectura: &str,
    p: &[(String, String)],
) -> Result<Params, String> {
    if param(p, "rtm_cfg_dhcpv4_svr_pool_id") != Some("0") {
        return Err("pool desconocido".to_owned());
    }
    let id = param(p, "rtm_dhcpv4_svr_pool_static_id").ok_or("falta el id")?;
    let mut p = p.to_vec();
    if param(&p, "del") == Some("DEL") {
        return Ok(p);
    }
    if id == "-1" {
        for campo in ["mac_addr", "ipv4_addr"] {
            param(&p, campo).ok_or_else(|| format!("falta {campo}"))?;
        }
        if param(&p, "adm_state").is_none() {
            p.push(("adm_state".to_owned(), "1".to_owned()));
        }
    } else {
        materializar_lista(s, st, lectura)?;
        let existe = st.listas[lectura]
            .nodos
            .iter()
            .any(|n| n.text_of("rtm_dhcpv4_svr_pool_static_id") == Some(id));
        if !existe {
            return Err("no existe".to_owned());
        }
    }
    Ok(p)
}

/// Parámetros que llevan valor a la instancia (todos menos los de control y
/// los que ella excluye).
fn campos_escritos(p: &[(String, String)], excluir: &[&str]) -> BTreeMap<String, String> {
    p.iter()
        .filter(|(k, _)| k != "validation_only" && !excluir.contains(&k.as_str()))
        .cloned()
        .collect()
}

/// Crea, edita o borra una instancia de la lista de la lectura `lectura`.
pub(crate) fn escribir_lista(
    s: &Inner,
    st: &mut SimState,
    lectura: &str,
    lista: &Lista,
    p: &[(String, String)],
) -> Result<(), String> {
    materializar_lista(s, st, lectura)?;
    let create_twice = st.create_twice;
    let viva = st.listas.get_mut(lectura).ok_or("lista sin materializar")?;
    let id_param = lista.id_param.as_str();
    let pos = param(p, id_param).and_then(|id| {
        viva.nodos
            .iter()
            .position(|n| n.text_of(id_param) == Some(id))
    });
    if param(p, "del") == Some("DEL") {
        if !lista.del {
            return Err("esta lista no admite borrar".to_owned());
        }
        viva.nodos.remove(pos.ok_or("no existe")?);
    } else {
        let campos = campos_escritos(p, &[id_param, "del"]);
        match pos {
            Some(i) => aplicar_campos(&mut viva.nodos[i], &[], &campos),
            None => {
                let id = viva
                    .nodos
                    .iter()
                    .filter_map(|n| n.text_of(id_param)?.parse::<u64>().ok())
                    .max()
                    .map_or(0, |m| m + 1);
                let mut nuevo = viva.plantilla.clone();
                aplicar_campos(&mut nuevo, &[], &campos);
                let con_id = |mut n: XmlNode, id: u64| {
                    set_child(&mut n, id_param, &id.to_string());
                    // El índice de una instancia nueva es su id; el de las
                    // existentes no se toca (aunque se borren otras).
                    if let Some(base) = id_param.strip_suffix("_id") {
                        set_child(&mut n, &format!("{base}_0i"), &id.to_string());
                    }
                    n
                };
                if create_twice {
                    viva.nodos.push(con_id(nuevo.clone(), id));
                    viva.nodos.push(con_id(nuevo, id + 1));
                } else {
                    viva.nodos.push(con_id(nuevo, id));
                }
            }
        }
    }
    if pos.is_none() && param(p, "del") != Some("DEL") {
        st.create_twice = false;
    }
    Ok(())
}

/// Crea la lista viva de `lectura` (si no existe) a partir de la fixture `all=ALL` (o, sin
/// ella, la de sin parámetros o la suma de las fixtures por instancia).
fn materializar_lista(s: &Inner, st: &mut SimState, lectura: &str) -> Result<(), String> {
    if !st.listas.contains_key(lectura) {
        let viva = construir_lista(s, lectura)?;
        st.listas.insert(lectura.to_owned(), viva);
    }
    Ok(())
}

/// La lista de `lectura` tal como la dan las fixtures.
pub(crate) fn construir_lista(s: &Inner, lectura: &str) -> Result<ListaViva, String> {
    let todas = s
        .fixtures
        .iter()
        .filter(|((n, p), _)| n == lectura && param(p, "all") == Some("ALL"))
        .min_by_key(|((_, p), _)| p.clone())
        .map(|(_, raw)| raw)
        .or_else(|| s.fixtures.get(&(lectura.to_owned(), Vec::new())));
    // Una respuesta sin ninguna instancia (la del pool DHCP sin parámetros
    // solo trae el índice) no sirve de lista: se juntan las de las fixtures
    // por instancia.
    let node = match todas {
        Some(raw) => Some(hgu::xml::parse_response(lectura, raw).map_err(|e| e.to_string())?),
        None => None,
    }
    .filter(|n| n.children.iter().any(|c| !c.children.is_empty()))
    .or_else(|| all_instances(s, lectura))
    .ok_or("la lista no tiene fixture")?;
    let nodos: Vec<XmlNode> = node
        .children
        .into_iter()
        .filter(|c| c.tag != "Result" && c.tag != "ErrorString")
        .collect();
    let plantilla = nodos
        .first()
        .cloned()
        .ok_or("la lista no tiene ninguna instancia de molde")?;
    Ok(ListaViva { plantilla, nodos })
}
