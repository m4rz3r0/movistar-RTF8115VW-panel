//! El paso de los DNS de casa: el proveedor que ya está puesto y los DNS propios.

use super::calculo::{Leido, coincide, estado_paso, sin_leer};
use crate::ajustes_router::EstadoPaso;
use ajustes::{CatalogoAjustes, Eleccion, OpcionEleccion, PasoExpandido, ValorLegible, dns_propio};
use std::net::Ipv4Addr;

/// La opción de `e` que tiene entera los valores `leidos` (por campo): el
/// DNS de un proveedor con el secundario de otro (o vacío) no es ninguno.
pub(super) fn opcion_de<'e>(
    leidos: &[(&str, ValorLegible)],
    e: &'e Eleccion,
) -> Option<&'e OpcionEleccion> {
    e.opciones.iter().find(|o| {
        leidos.iter().all(|(campo, v)| {
            o.valores
                .get(*campo)
                .is_some_and(|x| coincide(v, x) == Some(true))
        })
    })
}

/// La opción de la elección `e` que ya tiene el router en los campos del
/// paso `p` (`leer(ajuste, campo)`): el proveedor de DNS que está puesto.
/// `None` si no es ninguna o falta alguna lectura.
pub(super) fn opcion_leida<'e>(
    p: &PasoExpandido,
    leer: &dyn Fn(&str, &str) -> Leido,
    e: &'e Eleccion,
) -> Option<&'e OpcionEleccion> {
    let leidos: Vec<(&str, ValorLegible)> = p
        .destinos
        .iter()
        .map(|d| match leer(&d.ajuste, &d.campo) {
            Leido::Valor(v) => Some((d.campo.as_str(), v)),
            Leido::Fallo => None,
        })
        .collect::<Option<_>>()?;
    opcion_de(&leidos, e)
}

/// El destino del paso `p` cuyo campo declara los resolutores del operador
/// (el DNS principal): su id de campo, su definición y su valor leído.
/// `None` si no hay o no se leyó como texto.
fn principal<'c, 'p>(
    cat: &'c CatalogoAjustes,
    p: &'p PasoExpandido,
    leer: &dyn Fn(&str, &str) -> Leido,
) -> Option<(&'p str, &'c ajustes::Campo, String)> {
    p.destinos.iter().find_map(|d| {
        let def = cat.ajuste(&d.ajuste)?.campo(&d.campo)?;
        if def.del_operador.is_empty() {
            return None;
        }
        match leer(&d.ajuste, &d.campo) {
            Leido::Valor(ValorLegible::Texto(t)) => Some((d.campo.as_str(), def, t)),
            _ => None,
        }
    })
}

/// Los valores leídos de los demás destinos del paso (el DNS secundario),
/// por campo; `None` si alguno no se leyó como texto.
fn demas(
    p: &PasoExpandido,
    leer: &dyn Fn(&str, &str) -> Leido,
    principal: &str,
) -> Option<Vec<(String, String)>> {
    p.destinos
        .iter()
        .filter(|d| d.campo != principal)
        .map(|d| match leer(&d.ajuste, &d.campo) {
            Leido::Valor(ValorLegible::Texto(t)) => Some((d.campo.clone(), t)),
            _ => None,
        })
        .collect()
}

/// ¿Tiene el router DNS propios en los campos del paso `p`? Un DNS
/// principal que no es el router (`ip_router`, vacío o `0.0.0.0`) ni del
/// operador ([`dns_propio`]), y un secundario vacío o que tampoco lo es:
/// con el secundario del operador, parte de las consultas sigue yendo allí.
/// `None` si no se puede saber (sin la IP del router) o el paso no tiene un
/// campo que lo diga.
pub(super) fn propios(
    cat: &CatalogoAjustes,
    p: &PasoExpandido,
    leer: &dyn Fn(&str, &str) -> Leido,
    ip_router: Option<Ipv4Addr>,
) -> Option<bool> {
    let (campo, def, valor) = principal(cat, p, leer)?;
    if dns_propio(def, &valor, ip_router)? {
        for (_, v) in demas(p, leer, campo)? {
            if !v.trim().is_empty() && !dns_propio(def, &v, ip_router)? {
                return Some(false);
            }
        }
        return Some(true);
    }
    Some(false)
}

/// El proveedor de `e` cuyo DNS principal es el del router, con los demás
/// campos vacíos (9.9.9.9 sin secundario): se dice con su nombre, no como
/// «DNS propios».
pub(super) fn solo_principal<'e>(
    cat: &CatalogoAjustes,
    p: &PasoExpandido,
    leer: &dyn Fn(&str, &str) -> Leido,
    e: &'e Eleccion,
) -> Option<&'e OpcionEleccion> {
    let (campo, _, valor) = principal(cat, p, leer)?;
    if !demas(p, leer, campo)?
        .iter()
        .all(|(_, v)| v.trim().is_empty())
    {
        return None;
    }
    e.opciones.iter().find(|o| {
        o.valores
            .get(campo)
            .is_some_and(|x| x.trim() == valor.trim())
    })
}

/// Estado del paso de los DNS (`p`, que elige con `e`): el de
/// [`estado_paso`] (hecho con el par entero de un proveedor) y, si queda
/// pendiente, hecho también con DNS propios ([`propios`]). Si no se puede
/// saber (sin la IP del router), «sin leer».
pub(super) fn estado_dns(
    cat: &CatalogoAjustes,
    p: &PasoExpandido,
    leer: &dyn Fn(&str, &str) -> Leido,
    e: &Eleccion,
    ip_router: Option<Ipv4Addr>,
) -> EstadoPaso {
    let base = estado_paso(p, leer, Some(e));
    if base != EstadoPaso::Pendiente || principal(cat, p, leer).is_none() {
        return base;
    }
    match propios(cat, p, leer, ip_router) {
        Some(true) => EstadoPaso::Hecho,
        Some(false) => EstadoPaso::Pendiente,
        None => sin_leer(p.tema),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ajustes::{Destino, Tema};
    use std::collections::BTreeMap;

    fn destino(ajuste: &str, campo: &str, valor: Option<&str>) -> Destino {
        Destino {
            ajuste: ajuste.into(),
            campo: campo.into(),
            valor: valor.map(str::to_owned),
        }
    }

    fn paso(id: &str, destinos: Vec<Destino>) -> PasoExpandido {
        PasoExpandido {
            paso: id.into(),
            tema: Tema::Sistema,
            elemento: None,
            destinos,
            si: None,
            opcional: false,
            elegir: None,
        }
    }

    fn falla(_: &str, _: &str) -> Leido {
        Leido::Fallo
    }

    /// Vigilancia 5: el DNS solo está hecho con el par entero de un proveedor.
    #[test]
    fn dns_por_pares_de_proveedor() {
        let opcion = |id: &str, principal: &str, secundario: &str| OpcionEleccion {
            id: id.into(),
            defecto: id == "neutro",
            valores: BTreeMap::from([
                ("principal".to_owned(), principal.to_owned()),
                ("secundario".to_owned(), secundario.to_owned()),
            ]),
        };
        let e = Eleccion {
            id: "proveedor_dns".into(),
            opciones: vec![
                opcion("neutro", "9.9.9.9", "149.112.112.112"),
                opcion("rapido", "1.1.1.1", "1.0.0.1"),
            ],
        };
        let dns = PasoExpandido {
            tema: Tema::RedLocal,
            elegir: Some("proveedor_dns".into()),
            ..paso(
                "dns_casa",
                vec![
                    destino("red_local.dns_casa", "principal", None),
                    destino("red_local.dns_casa", "secundario", None),
                ],
            )
        };
        let par = |principal: &'static str, secundario: &'static str| {
            move |_: &str, campo: &str| {
                Leido::Valor(ValorLegible::Texto(
                    if campo == "principal" {
                        principal
                    } else {
                        secundario
                    }
                    .into(),
                ))
            }
        };
        let estado =
            |principal, secundario| estado_paso(&dns, &par(principal, secundario), Some(&e));
        assert_eq!(estado("9.9.9.9", "149.112.112.112"), EstadoPaso::Hecho);
        assert_eq!(estado("1.1.1.1", "1.0.0.1"), EstadoPaso::Hecho);
        assert_eq!(estado("9.9.9.9", ""), EstadoPaso::Pendiente);
        assert_eq!(estado("1.1.1.1", "9.9.9.9"), EstadoPaso::Pendiente);
        assert_eq!(estado("192.168.1.1", "192.168.1.1"), EstadoPaso::Pendiente);
        // El proveedor que está puesto, solo con el par entero.
        let puesto = |principal, secundario| {
            opcion_leida(&dns, &par(principal, secundario), &e).map(|o| o.id.as_str())
        };
        assert_eq!(puesto("1.1.1.1", "1.0.0.1"), Some("rapido"));
        assert_eq!(puesto("9.9.9.9", "149.112.112.112"), Some("neutro"));
        assert_eq!(puesto("1.1.1.1", "9.9.9.9"), None);
        assert_eq!(opcion_leida(&dns, &falla, &e).map(|o| o.id.as_str()), None);
    }

    /// Con el catálogo real: DNS propios dan el paso por hecho; los del
    /// router o del operador, no; sin la IP del router, «sin leer».
    #[test]
    fn dns_propios_con_el_catalogo() {
        let cat = CatalogoAjustes::load().unwrap();
        let e = cat.eleccion(ajustes::ELECCION_DNS).unwrap();
        let p = ajustes::pasos_del_lote(&cat, cat.lote("privacidad").unwrap())
            .into_iter()
            .find(|p| p.paso == "dns_casa")
            .unwrap();
        let par = |principal: &'static str, secundario: &'static str| {
            move |_: &str, campo: &str| {
                let v = if campo == "principal" {
                    principal
                } else {
                    secundario
                };
                Leido::Valor(ValorLegible::Texto(v.into()))
            }
        };
        let router = Some(Ipv4Addr::new(192, 168, 1, 1));
        let estado =
            |principal, secundario, ip| estado_dns(&cat, &p, &par(principal, secundario), e, ip);
        assert_eq!(
            estado("192.168.1.70", "192.0.2.53", router),
            EstadoPaso::Hecho
        );
        assert_eq!(estado("192.0.2.53", "", router), EstadoPaso::Hecho);
        assert_eq!(
            estado("192.168.1.1", "192.168.1.1", router),
            EstadoPaso::Pendiente
        );
        assert_eq!(estado("0.0.0.0", "", router), EstadoPaso::Pendiente);
        assert_eq!(estado("", "", router), EstadoPaso::Pendiente);
        assert_eq!(
            estado("80.58.61.250", "80.58.61.254", router),
            EstadoPaso::Pendiente
        );
        // Principal propio, pero el secundario es del operador o el router:
        // parte de las consultas sigue yendo allí.
        assert_eq!(
            estado("192.168.1.70", "80.58.61.250", router),
            EstadoPaso::Pendiente
        );
        assert_eq!(
            estado("192.168.1.70", "192.168.1.1", router),
            EstadoPaso::Pendiente
        );
        assert_eq!(estado("192.168.1.70", "", router), EstadoPaso::Hecho);
        // Un proveedor entero: hecho, y no son «propios».
        assert_eq!(
            estado("9.9.9.9", "149.112.112.112", None),
            EstadoPaso::Hecho
        );
        assert_eq!(
            propios(&cat, &p, &par("9.9.9.9", "149.112.112.112"), router),
            Some(true)
        );
        assert!(matches!(
            estado("192.168.1.70", "", None),
            EstadoPaso::SinLeer { .. }
        ));
        assert!(matches!(
            estado_dns(&cat, &p, &falla, e, router),
            EstadoPaso::SinLeer { .. }
        ));
    }
}
