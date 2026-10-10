//! Planificación de cambios, sin E/S: carga a enviar, diff, inversa,
//! comparación con lo que muestra el router, validación y enmascarado.

use super::{CambioError, CampoDiff, ChangeRequest, Diff};
use catalogo::{
    Funcion, Parametro, Tipo, TipoParam,
    mask::{MASCARA, Mascara},
};
use hgu::{fmt::split_mac, xml::XmlNode};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::Ipv4Addr,
};

/// Palabra que hay que escribir para reiniciar el router.
pub const PALABRA_REINICIO: &str = "REINICIAR";
/// Palabra para las demás acciones.
pub const PALABRA_ACCION: &str = "CONFIRMAR";
/// Interfaz de la conexión PPP principal.
pub(super) const WAN: &str = "ip2";
pub(super) const REBOOT: &str = "cbReboot";

// ------------------------------------------------------------- planes

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Operacion {
    Editar,
    Crear,
    Borrar,
    Accion,
}

impl Operacion {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Operacion::Editar => "editar",
            Operacion::Crear => "crear",
            Operacion::Borrar => "borrar",
            Operacion::Accion => "accion",
        }
    }

    pub(super) fn parse(s: &str) -> Option<Operacion> {
        [
            Operacion::Editar,
            Operacion::Crear,
            Operacion::Borrar,
            Operacion::Accion,
        ]
        .into_iter()
        .find(|o| o.as_str() == s)
    }
}

/// Dónde leer lo que escribe una función.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Lectura {
    /// Llamada `cb*` de la UI avanzada. `clave_id`: parámetro de la lectura
    /// que lleva el id de la instancia (se quita para listarlas todas).
    Xml {
        funcion: String,
        params: Vec<(String, String)>,
        clave_id: Option<String>,
    },
    /// Variables `var x='…'` de una página de la UI básica.
    Pagina(String),
    /// Sin lectura comprobable.
    Ninguna,
}

/// Un campo que cambia, con los valores en claro (solo en memoria).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Campo {
    pub(super) nombre: String,
    pub(super) antes: String,
    pub(super) despues: String,
    pub(super) secreto: bool,
    /// El router mostraba el campo: `antes` es su valor real.
    pub(super) leido: bool,
}

/// Cambio listo para escribir y comprobar.
#[derive(Clone, Debug)]
pub(super) struct Plan {
    pub(super) funcion: Funcion,
    pub(super) operacion: Operacion,
    pub(super) instancia: Option<String>,
    /// Valores de la petición, con los secretos sin tocar sustituidos por el
    /// valor actual (o quitados si no se conoce) y el id de la instancia.
    pub(super) valores: BTreeMap<String, String>,
    /// Lo que se envía (sin `sessionKey`).
    pub(super) payload: Vec<(String, String)>,
    pub(super) campos: Vec<Campo>,
    pub(super) lectura: Lectura,
    /// Al crear: ids de la lista antes de escribir. La instancia creada es la
    /// que aparece nueva (no la primera con esos valores: puede haber una
    /// igual de antes). `None` si no se leyó (cambios anteriores a v0.1.1).
    pub(super) ids_antes: Option<BTreeSet<String>>,
}

/// Valor de `nombre` en `m`: primero exacto, luego sin distinguir mayúsculas.
pub(crate) fn buscar<'a>(m: &'a BTreeMap<String, String>, nombre: &str) -> Option<&'a str> {
    m.get(nombre)
        .or_else(|| {
            m.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nombre))
                .map(|(_, v)| v)
        })
        .map(String::as_str)
}

/// Iguales una vez normalizados: sin espacios en los extremos; IP y MAC sin
/// distinguir mayúsculas.
pub(super) fn iguales(p: Option<&Parametro>, a: &str, b: &str) -> bool {
    let (a, b) = (a.trim(), b.trim());
    match p.map(|p| p.tipo) {
        Some(TipoParam::Ip | TipoParam::Mac) => a.eq_ignore_ascii_case(b),
        _ => a == b,
    }
}

pub(super) fn es_secreto_param(f: &Funcion, m: &Mascara, nombre: &str) -> bool {
    f.parametro(nombre)
        .is_some_and(|p| p.tipo == TipoParam::Secret)
        || m.es_secreto(nombre)
}

/// Comprueba el valor nuevo de un parámetro.
pub(super) fn validar(p: &Parametro, v: &str) -> Result<(), String> {
    let n = &p.nombre;
    let ok = match p.tipo {
        TipoParam::Ip => v.trim().parse::<Ipv4Addr>().is_ok(),
        TipoParam::Mac => split_mac(v.trim()).is_some_and(|parts| {
            parts
                .iter()
                .all(|x| x.len() == 2 && x.bytes().all(|b| b.is_ascii_hexdigit()))
        }),
        TipoParam::Bool => matches!(v.trim(), "0" | "1"),
        TipoParam::Enum => p.opciones.iter().any(|(o, _)| o == v.trim()),
        TipoParam::Int => v.trim().parse::<i64>().is_ok(),
        TipoParam::Text | TipoParam::Secret => !v.chars().any(char::is_control),
    };
    if ok {
        Ok(())
    } else if p.tipo == TipoParam::Secret {
        Err(format!("{n}: contiene caracteres no válidos"))
    } else {
        Err(format!("{n}: «{v}» no es un valor válido"))
    }
}

/// Parámetros que identifican la instancia: los que usa la lectura pareada
/// (`{param}`) y el id de la lista.
pub(crate) fn identidad(f: &Funcion) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = f
        .lectura_pareada
        .iter()
        .flat_map(|lp| &lp.params)
        .filter_map(|(_, v)| v.strip_prefix('{').and_then(|v| v.strip_suffix('}')))
        .map(str::to_owned)
        .collect();
    if let Some(l) = &f.lista {
        out.insert(l.id_param.clone());
    }
    out
}

/// Lectura pareada con los `{param}` sustituidos por `valores` (o la
/// instancia, para el id de la lista).
pub(super) fn lectura_de(
    f: &Funcion,
    valores: &BTreeMap<String, String>,
    instancia: Option<&str>,
) -> Result<Lectura, CambioError> {
    let Some(lp) = &f.lectura_pareada else {
        return Ok(Lectura::Ninguna);
    };
    if lp.funcion.ends_with(".asp") {
        return Ok(Lectura::Pagina(lp.funcion.clone()));
    }
    if !lp.funcion.starts_with("cb") {
        // Lecturas `.cmd` de la UI básica (texto sin formato conocido).
        return Ok(Lectura::Ninguna);
    }
    let id_param = f.lista.as_ref().map(|l| l.id_param.as_str());
    let mut params = Vec::new();
    let mut clave_id = None;
    for (k, v) in &lp.params {
        let valor = match v.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
            Some(p) => {
                if Some(p) == id_param {
                    clave_id = Some(k.clone());
                }
                valores
                    .get(p)
                    .map(String::as_str)
                    .or(if Some(p) == id_param { instancia } else { None })
                    .ok_or_else(|| CambioError::Invalida(format!("falta el parámetro {p}")))?
                    .to_owned()
            }
            None if Some(k.as_str()) == id_param => {
                clave_id = Some(k.clone());
                instancia.unwrap_or(v).to_owned()
            }
            None => v.clone(),
        };
        params.push((k.clone(), valor));
    }
    Ok(Lectura::Xml {
        funcion: lp.funcion.clone(),
        params,
        clave_id,
    })
}

/// Campos hoja de un elemento.
pub(super) fn hojas(n: &XmlNode) -> BTreeMap<String, String> {
    n.children
        .iter()
        .filter(|c| c.children.is_empty() && c.tag != "Result" && c.tag != "ErrorString")
        .map(|c| (c.tag.clone(), c.text.clone()))
        .collect()
}

/// Elementos (hijos con hijos) de una respuesta; si no hay, la propia
/// respuesta.
pub(super) fn elementos(n: &XmlNode) -> Vec<BTreeMap<String, String>> {
    let v: Vec<_> = n
        .children
        .iter()
        .filter(|c| !c.children.is_empty())
        .map(hojas)
        .collect();
    if v.is_empty() { vec![hojas(n)] } else { v }
}

/// Resultado de [`componer`].
pub(super) struct Compuesto {
    /// Valores de la petición con los secretos sin tocar resueltos.
    pub(super) efectivos: BTreeMap<String, String>,
    /// Carga a enviar.
    pub(super) payload: Vec<(String, String)>,
    /// Campos que cambian.
    pub(super) campos: Vec<Campo>,
}

/// Une valores de la petición y valores actuales. No hace peticiones.
///
/// `explicito`: los valores son exactos (reversión o deshacer, con valores
/// anteriores reales): un secreto vacío se escribe vacío en vez de
/// entenderse como "sin tocar".
pub(super) fn componer(
    f: &Funcion,
    m: &Mascara,
    op: Operacion,
    actual: &BTreeMap<String, String>,
    valores: &BTreeMap<String, String>,
    explicito: bool,
) -> Compuesto {
    let ident = identidad(f);
    // Sin tocar (un secreto vacío o cualquier campo que llega enmascarado):
    // valor actual, o fuera si no se conoce. `MASCARA` nunca se escribe.
    let mut efectivos = BTreeMap::new();
    for (k, v) in valores {
        let p = f.parametro(k);
        let vacio = !explicito && es_secreto_param(f, m, k) && v.trim().is_empty();
        if v == MASCARA || vacio {
            if let Some(cur) = buscar(actual, k) {
                efectivos.insert(k.clone(), cur.to_owned());
            }
            continue;
        }
        let v = match p.map(|p| p.tipo) {
            Some(TipoParam::Text | TipoParam::Secret) | None => v.clone(),
            Some(_) => v.trim().to_owned(),
        };
        efectivos.insert(k.clone(), v);
    }
    let mut payload = Vec::new();
    for p in &f.parametros {
        if let Some(v) = efectivos.get(&p.nombre) {
            payload.push((p.nombre.clone(), v.clone()));
        } else if op == Operacion::Editar
            && let Some(cur) = buscar(actual, &p.nombre)
        {
            payload.push((p.nombre.clone(), cur.to_owned()));
        }
    }
    let secreto =
        |k: &str, a: &str, b: &str| es_secreto_param(f, m, k) || m.oculta(k, a) || m.oculta(k, b);
    let mut campos = Vec::new();
    if op == Operacion::Borrar {
        payload.push(("del".into(), "DEL".into()));
        for p in &f.parametros {
            if ident.contains(&p.nombre) || efectivos.contains_key(&p.nombre) {
                continue;
            }
            if let Some(cur) = buscar(actual, &p.nombre) {
                campos.push(Campo {
                    nombre: p.nombre.clone(),
                    antes: cur.to_owned(),
                    despues: String::new(),
                    secreto: secreto(&p.nombre, cur, ""),
                    leido: true,
                });
            }
        }
    } else {
        for p in &f.parametros {
            let Some(v) = efectivos.get(&p.nombre) else {
                continue;
            };
            if ident.contains(&p.nombre) && op != Operacion::Accion {
                continue;
            }
            let cur = buscar(actual, &p.nombre);
            let antes = cur.unwrap_or_default();
            if op != Operacion::Accion && iguales(Some(p), antes, v) {
                continue;
            }
            campos.push(Campo {
                nombre: p.nombre.clone(),
                antes: antes.to_owned(),
                despues: v.clone(),
                secreto: secreto(&p.nombre, antes, v),
                leido: cur.is_some(),
            });
        }
    }
    Compuesto {
        efectivos,
        payload,
        campos,
    }
}

/// Petición que deshace `plan` (`None` si no se puede: acciones, o sin
/// valores anteriores conocidos).
pub(super) fn inversa(plan: &Plan) -> Option<ChangeRequest> {
    let f = &plan.funcion;
    let ids = identidad(f);
    let id_param = f.lista.as_ref().map(|l| l.id_param.as_str());
    // Lo que identifica la instancia (el id de la lista va en `instancia`).
    let mut valores: BTreeMap<String, String> = plan
        .valores
        .iter()
        .filter(|(k, _)| ids.contains(*k) && Some(k.as_str()) != id_param)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let anteriores: Vec<(String, String)> = plan
        .campos
        .iter()
        .filter(|c| c.leido)
        .map(|c| (c.nombre.clone(), c.antes.clone()))
        .collect();
    let (instancia, borrar) = match plan.operacion {
        Operacion::Accion => return None,
        Operacion::Editar if anteriores.is_empty() => return None,
        Operacion::Editar => {
            valores.extend(anteriores);
            (plan.instancia.clone(), false)
        }
        Operacion::Crear => (Some(plan.instancia.clone()?), true),
        Operacion::Borrar => {
            valores.extend(anteriores);
            (None, false)
        }
    };
    Some(ChangeRequest {
        funcion: f.name.clone(),
        instancia,
        valores,
        borrar,
    })
}

/// Qué muestra el router respecto a un plan.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Comprobacion {
    /// Coincide; con la instancia nueva si se creó una.
    Coincide(Option<String>),
    /// No hay forma de comprobarlo (sin lectura o sin campos comparables).
    Limitada,
    /// No coincide (motivos, secretos enmascarados).
    Difiere(Vec<String>),
    /// No se pudo leer el estado del router (error transitorio).
    SinLectura(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Lado {
    Nuevo,
    Anterior,
}

/// Compara los campos del plan (lado `lado`) con lo que muestra el router.
/// `Limitada` si el router no muestra ninguno.
pub(super) fn comparar(
    f: &Funcion,
    campos: &[Campo],
    actual: &BTreeMap<String, String>,
    lado: Lado,
) -> Comprobacion {
    let mut comparados = 0;
    let mut fallos = Vec::new();
    for c in campos {
        if lado == Lado::Anterior && !c.leido {
            continue;
        }
        let Some(leido) = buscar(actual, &c.nombre) else {
            continue;
        };
        comparados += 1;
        let esperado = match lado {
            Lado::Nuevo => &c.despues,
            Lado::Anterior => &c.antes,
        };
        if !iguales(f.parametro(&c.nombre), leido, esperado) {
            let ver = |v: &str| {
                if c.secreto {
                    MASCARA.to_owned()
                } else {
                    format!("«{v}»")
                }
            };
            fallos.push(format!(
                "{}: se esperaba {}, el router tiene {}",
                c.nombre,
                ver(esperado),
                ver(leido)
            ));
        }
    }
    if comparados == 0 {
        Comprobacion::Limitada
    } else if fallos.is_empty() {
        Comprobacion::Coincide(None)
    } else {
        Comprobacion::Difiere(fallos)
    }
}

pub(super) fn wan_arriba(w: &[hgu::models::WanConn]) -> bool {
    w.iter().any(|c| c.name == WAN && c.up)
}

pub(super) fn palabra_requerida(f: &Funcion) -> Option<&'static str> {
    match f.tipo {
        Tipo::Accion if f.name == REBOOT => Some(PALABRA_REINICIO),
        Tipo::Accion => Some(PALABRA_ACCION),
        _ => None,
    }
}

/// Diff para mostrar y guardar: secretos enmascarados.
pub(super) fn visible(campos: &[Campo]) -> Diff {
    let ver = |c: &Campo, v: &str| {
        if c.secreto {
            MASCARA.to_owned()
        } else {
            v.to_owned()
        }
    };
    Diff {
        campos: campos
            .iter()
            .map(|c| CampoDiff {
                nombre: c.nombre.clone(),
                antes: ver(c, &c.antes),
                despues: ver(c, &c.despues),
                secreto: c.secreto,
            })
            .collect(),
    }
}

/// Resumen del diff para los registros (secretos enmascarados).
pub(super) fn resumen(campos: &[Campo]) -> String {
    visible(campos)
        .campos
        .iter()
        .map(|c| format!("{}: «{}» → «{}»", c.nombre, c.antes, c.despues))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalogo::Catalogo;

    fn cat() -> Catalogo {
        Catalogo::load().unwrap()
    }

    fn mapa(pares: &[(&str, &str)]) -> BTreeMap<String, String> {
        pares
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn carga_con_la_instancia_completa_y_secretos_sin_tocar() {
        let c = cat();
        let m = Mascara::load().unwrap();
        let f = c.funcion("cbSetWifiAccPt").unwrap();
        let actual = mapa(&[
            ("dpm_cfg_wifi_acc_pt_id", "1"),
            ("ssid", "Casa"),
            ("key_phrase", "clave-actual"),
            ("WMM", "1"),
            ("bssid", "02:00:00:00:00:01"),
        ]);
        let valores = mapa(&[
            ("dpm_cfg_wifi_acc_pt_id", "1"),
            ("ssid", "Otra"),
            ("key_phrase", MASCARA),
            ("wep_key1_data", ""),
        ]);
        let Compuesto {
            efectivos,
            payload,
            campos,
        } = componer(f, &m, Operacion::Editar, &actual, &valores, false);
        let p: BTreeMap<_, _> = payload.into_iter().collect();
        assert_eq!(p["ssid"], "Otra");
        assert_eq!(p["key_phrase"], "clave-actual", "secreto sin tocar");
        // Sin distinguir mayúsculas cuando no hay coincidencia exacta.
        assert_eq!(p["wmm"], "1");
        assert_eq!(p["dpm_cfg_wifi_acc_pt_id"], "1");
        // Ni lo que no es parámetro ni lo que no tiene valor.
        assert!(!p.contains_key("bssid"));
        assert!(!p.contains_key("wep_key1_data"));
        assert!(!p.contains_key("validation_only"));
        assert!(p.values().all(|v| v != MASCARA));
        assert_eq!(efectivos["key_phrase"], "clave-actual");
        assert_eq!(campos.len(), 1);
        assert_eq!(campos[0].nombre, "ssid");

        // Un campo que no es secreto pero llegó enmascarado (p. ej. por la
        // regla de sufijo numérico) tampoco se escribe enmascarado.
        let actual = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("wep_key_idx", "2")]);
        let valores = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("wep_key_idx", MASCARA)]);
        let r = componer(f, &m, Operacion::Editar, &actual, &valores, false);
        assert!(r.campos.is_empty());
        assert!(r.payload.contains(&("wep_key_idx".into(), "2".into())));
        let r = componer(f, &m, Operacion::Editar, &mapa(&[]), &valores, false);
        assert!(r.payload.iter().all(|(_, v)| v != MASCARA));

        // Explícito (reversión): un secreto vacío es el valor a escribir.
        let actual = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "nueva")]);
        let valores = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "")]);
        let r = componer(f, &m, Operacion::Editar, &actual, &valores, true);
        assert!(r.payload.contains(&("key_phrase".into(), String::new())));
        assert_eq!(r.campos.len(), 1);
        let r = componer(f, &m, Operacion::Editar, &actual, &valores, false);
        assert!(r.campos.is_empty(), "sin explícito, vacío = sin tocar");
    }

    #[test]
    fn secreto_cambiado_se_marca() {
        let c = cat();
        let m = Mascara::load().unwrap();
        let f = c.funcion("cbSetWifiAccPt").unwrap();
        let actual = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "vieja")]);
        let valores = mapa(&[("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", "nueva")]);
        let campos = componer(f, &m, Operacion::Editar, &actual, &valores, false).campos;
        assert_eq!(campos.len(), 1);
        assert!(campos[0].secreto && campos[0].leido);
        let d = visible(&campos);
        assert_eq!(
            (d.campos[0].antes.as_str(), d.campos[0].despues.as_str()),
            (MASCARA, MASCARA)
        );
        assert!(!resumen(&campos).contains("nueva"));
        assert!(!resumen(&campos).contains("vieja"));
    }

    #[test]
    fn crear_y_borrar() {
        let c = cat();
        let m = Mascara::load().unwrap();
        let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
        let valores = mapa(&[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("rtm_dhcpv4_svr_pool_static_id", "-1"),
            ("mac_addr", " 02:00:00:00:00:AA "),
            ("ipv4_addr", "192.168.1.60"),
        ]);
        let Compuesto {
            payload, campos, ..
        } = componer(f, &m, Operacion::Crear, &BTreeMap::new(), &valores, false);
        let p: BTreeMap<_, _> = payload.into_iter().collect();
        assert_eq!(p["rtm_dhcpv4_svr_pool_static_id"], "-1");
        assert_eq!(p["mac_addr"], "02:00:00:00:00:AA", "IP/MAC sin espacios");
        let nombres: Vec<_> = campos.iter().map(|c| c.nombre.as_str()).collect();
        assert_eq!(nombres, ["ipv4_addr", "mac_addr"], "sin los ids");

        let actual = mapa(&[
            ("rtm_dhcpv4_svr_pool_static_id", "3"),
            ("adm_state", "1"),
            ("mac_addr", "02:00:00:00:00:aa"),
            ("ipv4_addr", "192.168.1.60"),
        ]);
        let valores = mapa(&[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("rtm_dhcpv4_svr_pool_static_id", "3"),
        ]);
        let Compuesto {
            payload, campos, ..
        } = componer(f, &m, Operacion::Borrar, &actual, &valores, false);
        assert_eq!(
            payload,
            vec![
                ("rtm_dhcpv4_svr_pool_static_id".to_owned(), "3".to_owned()),
                ("rtm_cfg_dhcpv4_svr_pool_id".to_owned(), "0".to_owned()),
                ("del".to_owned(), "DEL".to_owned()),
            ]
        );
        assert_eq!(campos.len(), 3);
        assert!(campos.iter().all(|c| c.despues.is_empty() && c.leido));
    }

    #[test]
    fn inversa_de_cada_operacion() {
        let c = cat();
        let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap().clone();
        let campo = |n: &str, a: &str, d: &str, leido| Campo {
            nombre: n.into(),
            antes: a.into(),
            despues: d.into(),
            secreto: false,
            leido,
        };
        let mut plan = Plan {
            funcion: f,
            operacion: Operacion::Editar,
            instancia: Some("0".into()),
            valores: mapa(&[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("rtm_dhcpv4_svr_pool_static_id", "0"),
                ("ipv4_addr", "192.168.1.84"),
            ]),
            payload: Vec::new(),
            campos: vec![campo("ipv4_addr", "192.168.1.68", "192.168.1.84", true)],
            lectura: Lectura::Ninguna,
            ids_antes: None,
        };
        let r = inversa(&plan).unwrap();
        assert_eq!(r.instancia.as_deref(), Some("0"));
        assert!(!r.borrar);
        assert_eq!(
            r.valores,
            mapa(&[
                ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                ("ipv4_addr", "192.168.1.68")
            ])
        );
        // Sin valores anteriores conocidos no hay inversa.
        plan.campos = vec![campo("ipv4_addr", "", "192.168.1.84", false)];
        assert_eq!(inversa(&plan), None);

        plan.operacion = Operacion::Crear;
        plan.instancia = None;
        assert_eq!(inversa(&plan), None, "sin el id de la instancia creada");
        plan.instancia = Some("5".into());
        let r = inversa(&plan).unwrap();
        assert!(r.borrar);
        assert_eq!(r.instancia.as_deref(), Some("5"));
        assert_eq!(r.valores, mapa(&[("rtm_cfg_dhcpv4_svr_pool_id", "0")]));

        plan.operacion = Operacion::Borrar;
        plan.campos = vec![campo("ipv4_addr", "192.168.1.60", "", true)];
        let r = inversa(&plan).unwrap();
        assert!(!r.borrar);
        assert_eq!(r.instancia, None);
        assert_eq!(r.valores["ipv4_addr"], "192.168.1.60");

        plan.operacion = Operacion::Accion;
        assert_eq!(inversa(&plan), None);
    }

    #[test]
    fn lectura_pareada_sustituye_parametros() {
        let c = cat();
        let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
        let v = mapa(&[
            ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
            ("rtm_dhcpv4_svr_pool_static_id", "4"),
        ]);
        let l = lectura_de(f, &v, Some("4")).unwrap();
        assert_eq!(
            l,
            Lectura::Xml {
                funcion: "cbDhcpV4SvrPoolGetStatic".into(),
                params: vec![
                    ("rtm_cfg_dhcpv4_svr_pool_id".into(), "0".into()),
                    ("rtm_dhcpv4_svr_pool_static_id".into(), "4".into()),
                ],
                clave_id: Some("rtm_dhcpv4_svr_pool_static_id".into()),
            }
        );
        let falta = lectura_de(f, &mapa(&[]), Some("4"));
        assert!(matches!(falta, Err(CambioError::Invalida(_))), "{falta:?}");
        let wl = c.funcion("te_wifi.wl").unwrap();
        assert_eq!(
            lectura_de(wl, &mapa(&[]), None).unwrap(),
            Lectura::Pagina("te_wifi.asp".into())
        );
    }

    #[test]
    fn validacion_de_valores() {
        let c = cat();
        let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
        let p = |n| f.parametro(n).unwrap();
        assert!(validar(p("ipv4_addr"), "192.168.1.5").is_ok());
        assert!(validar(p("ipv4_addr"), "192.168.1.256").is_err());
        assert!(validar(p("mac_addr"), "02:00:00:00:00:aB").is_ok());
        assert!(validar(p("mac_addr"), "02:00:00:00:00").is_err());
        assert!(validar(p("mac_addr"), "02:00:00:00:00:zz").is_err());
        assert!(validar(p("adm_state"), "1").is_ok());
        assert!(validar(p("adm_state"), "2").is_err());
        assert!(validar(p("rtm_cfg_dhcpv4_svr_pool_id"), "x").is_err());
        let w = c.funcion("cbSetWifiAccPt").unwrap();
        assert!(validar(w.parametro("ssid").unwrap(), "a\nb").is_err());
        let e = validar(w.parametro("key_phrase").unwrap(), "a\0b").unwrap_err();
        assert!(!e.contains("a\0b"), "el error no muestra el secreto");
    }

    #[test]
    fn comparar_normaliza_y_enmascara() {
        let c = cat();
        let f = c.funcion("cbSetWifiAccPt").unwrap();
        let campos = vec![
            Campo {
                nombre: "key_phrase".into(),
                antes: "vieja".into(),
                despues: "nueva".into(),
                secreto: true,
                leido: true,
            },
            Campo {
                nombre: "ssid".into(),
                antes: "A".into(),
                despues: "B".into(),
                secreto: false,
                leido: true,
            },
        ];
        let actual = mapa(&[("key_phrase", "otra"), ("ssid", " B ")]);
        let Comprobacion::Difiere(v) = comparar(f, &campos, &actual, Lado::Nuevo) else {
            panic!()
        };
        assert_eq!(v.len(), 1);
        assert!(v[0].contains(MASCARA) && !v[0].contains("otra") && !v[0].contains("nueva"));
        assert_eq!(
            comparar(f, &campos, &mapa(&[("bssid", "x")]), Lado::Nuevo),
            Comprobacion::Limitada
        );
        let anterior = mapa(&[("key_phrase", "vieja"), ("SSID", "A")]);
        assert_eq!(
            comparar(f, &campos, &anterior, Lado::Anterior),
            Comprobacion::Coincide(None)
        );
        let s = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
        assert!(iguales(
            s.parametro("mac_addr"),
            "02:AA:00:00:00:00",
            " 02:aa:00:00:00:00"
        ));
        assert!(!iguales(None, "Casa", "casa"));
    }
}
