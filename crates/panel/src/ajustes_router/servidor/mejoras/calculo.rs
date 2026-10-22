//! Cálculo puro del estado de cada paso y de cada lote de mejoras, y del contador.

use crate::ajustes_router::{EstadoLote, EstadoPaso};
use crate::t;
use ajustes::{Eleccion, PasoExpandido, Tema, ValorLegible, partir_referencia};
use std::collections::BTreeSet;

/// Lo leído de un campo: su valor legible o una lectura que falló.
pub(super) enum Leido {
    Valor(ValorLegible),
    Fallo,
}

/// ¿Vale `v` lo que dice `esperado` (legible)? `None` si el router no da
/// el dato (un valor vacío que el panel no reconoce). Otro valor que no
/// reconoce no es el esperado.
fn coincide(v: &ValorLegible, esperado: &str) -> Option<bool> {
    match v {
        ValorLegible::Desconocido(t) if t.is_empty() => None,
        ValorLegible::Desconocido(_) | ValorLegible::Secreto => Some(false),
        ValorLegible::Booleano(b) => Some(b.to_string() == esperado),
        ValorLegible::Opcion(t) | ValorLegible::Texto(t) => Some(t == esperado),
    }
}

/// «Sin leer» porque falló la lectura de la pestaña `tema`.
fn sin_leer(tema: Tema) -> EstadoPaso {
    let pestana = crate::ui::i18n::traducir(&format!("ajustes-tema-{}", tema.slug()), None);
    EstadoPaso::SinLeer {
        motivo: t!("mejoras-motivo-sin-leer", pestana = pestana),
    }
}

/// «Sin leer» porque el router no da el dato.
fn sin_dato() -> EstadoPaso {
    EstadoPaso::SinLeer {
        motivo: t!("mejoras-motivo-sin-dato"),
    }
}

/// Estado del paso `p` con lo leído (`leer(ajuste, campo)`) y, si su valor
/// sale de una elección, la elección `e`:
/// - con una condición que no se cumple, «no aplica»;
/// - si falla una lectura que necesita o el router no da el dato, «sin
///   leer» con el motivo;
/// - si cada campo tiene su valor (con una elección, el de alguna de sus
///   opciones, entera), «hecho»; si no, «pendiente».
pub(super) fn estado_paso(
    p: &PasoExpandido,
    leer: &dyn Fn(&str, &str) -> Leido,
    e: Option<&Eleccion>,
) -> EstadoPaso {
    let valor = |ajuste: &str, campo: &str| match leer(ajuste, campo) {
        Leido::Fallo => Err(sin_leer(p.tema)),
        Leido::Valor(ValorLegible::Desconocido(t)) if t.is_empty() => Err(sin_dato()),
        Leido::Valor(v) => Ok(v),
    };
    if let Some(si) = &p.si {
        let Some((a, c)) = partir_referencia(&si.campo) else {
            return sin_leer(p.tema);
        };
        match valor(a, c) {
            Err(estado) => return estado,
            Ok(v) if coincide(&v, &si.valor) != Some(true) => return EstadoPaso::NoAplica,
            Ok(_) => {}
        }
    }
    let mut leidos: Vec<(&str, ValorLegible)> = Vec::with_capacity(p.destinos.len());
    for d in &p.destinos {
        match valor(&d.ajuste, &d.campo) {
            Ok(v) => leidos.push((&d.campo, v)),
            Err(estado) => return estado,
        }
    }
    let hecho = match (&p.elegir, e) {
        // Una opción entera: el DNS de un proveedor con el secundario de
        // otro (o vacío) no es ninguno.
        (Some(_), Some(e)) => e.opciones.iter().any(|o| {
            leidos.iter().all(|(campo, v)| {
                o.valores
                    .get(*campo)
                    .is_some_and(|x| coincide(v, x) == Some(true))
            })
        }),
        (Some(_), None) => false,
        (None, _) => p.destinos.iter().zip(&leidos).all(|(d, (_, v))| {
            d.valor
                .as_deref()
                .is_some_and(|x| coincide(v, x) == Some(true))
        }),
    };
    if hecho {
        EstadoPaso::Hecho
    } else {
        EstadoPaso::Pendiente
    }
}

/// ¿Cuenta el paso en «x de y» y en el contador? Los opcionales no.
fn cuenta(p: &PasoExpandido) -> bool {
    !p.opcional
}

/// Estado de un lote con el de sus pasos: «x de y pendientes» entre los
/// que cuentan, hechos o pendientes; «aplicado» si no falta ninguno y se
/// leyeron todos. Sin pendientes leídos y con pasos sin leer, «sin leer»:
/// con el motivo del primero si no se pudo leer ninguno, y si no, con
/// cuántos faltan.
pub(super) fn estado_lote(pasos: &[(PasoExpandido, EstadoPaso)]) -> EstadoLote {
    let leidos: Vec<&EstadoPaso> = pasos
        .iter()
        .filter(|(p, e)| cuenta(p) && matches!(e, EstadoPaso::Hecho | EstadoPaso::Pendiente))
        .map(|(_, e)| e)
        .collect();
    let total = leidos.len() as u32;
    let pendientes = leidos
        .iter()
        .filter(|e| ***e == EstadoPaso::Pendiente)
        .count() as u32;
    // Lo no leído no se da por hecho: sin pendientes leídos, un lote con
    // pasos sin leer no está aplicado.
    let sin_leer: Vec<&String> = pasos
        .iter()
        .filter(|(p, _)| cuenta(p))
        .filter_map(|(_, e)| match e {
            EstadoPaso::SinLeer { motivo } => Some(motivo),
            _ => None,
        })
        .collect();
    if pendientes == 0
        && let Some(primero) = sin_leer.first()
    {
        let motivo = if total == 0 {
            (*primero).clone()
        } else {
            t!("mejoras-motivo-pasos-sin-leer", n = sin_leer.len())
        };
        return EstadoLote::SinLeer { motivo };
    }
    if pendientes == 0 {
        EstadoLote::Aplicado
    } else {
        EstadoLote::Pendientes { pendientes, total }
    }
}

/// Pasos pendientes sin repetir (por su clave: uno compartido por dos lotes
/// cuenta una vez) entre los que cuentan. `None` si no se pudo leer ninguno
/// de los que cuentan: el contador no inventa.
pub(super) fn pendientes_unicos(pasos: &[(PasoExpandido, EstadoPaso)]) -> Option<u32> {
    let contados = || pasos.iter().filter(|(p, _)| cuenta(p));
    let algo_leido = contados().any(|(_, e)| !matches!(e, EstadoPaso::SinLeer { .. }));
    if !algo_leido {
        return None;
    }
    let claves: BTreeSet<String> = contados()
        .filter(|(_, e)| *e == EstadoPaso::Pendiente)
        .map(|(p, _)| p.clave())
        .collect();
    Some(claves.len() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ajustes::{Condicion, Destino, OpcionEleccion};
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

    /// El espejo: `sistema.diagnostico.espejo` a `false`.
    fn espejo() -> PasoExpandido {
        paso(
            "espejo",
            vec![destino("sistema.diagnostico", "espejo", Some("false"))],
        )
    }

    fn con(v: ValorLegible) -> impl Fn(&str, &str) -> Leido {
        move |_, _| Leido::Valor(v.clone())
    }

    fn falla(_: &str, _: &str) -> Leido {
        Leido::Fallo
    }

    fn es_sin_leer(e: &EstadoPaso) -> bool {
        matches!(e, EstadoPaso::SinLeer { motivo } if !motivo.is_empty())
    }

    #[test]
    fn paso_hecho_pendiente_y_no_aplica() {
        let p = espejo();
        let hecho = con(ValorLegible::Booleano(false));
        assert_eq!(estado_paso(&p, &hecho, None), EstadoPaso::Hecho);
        let pendiente = con(ValorLegible::Booleano(true));
        assert_eq!(estado_paso(&p, &pendiente, None), EstadoPaso::Pendiente);

        // Aislar invitados con la red de invitados apagada.
        let aislar = PasoExpandido {
            tema: Tema::Wifi,
            si: Some(Condicion {
                campo: "wifi.invitados.activa".into(),
                valor: "true".into(),
            }),
            ..paso(
                "aislar_invitados",
                vec![destino(
                    "wifi.invitados_avanzado",
                    "aislamiento",
                    Some("true"),
                )],
            )
        };
        let leer =
            |a: &str, _: &str| Leido::Valor(ValorLegible::Booleano(a == "wifi.invitados_avanzado"));
        assert_eq!(estado_paso(&aislar, &leer, None), EstadoPaso::NoAplica);
        // Encendida y sin aislar: pendiente.
        let leer = |a: &str, _: &str| Leido::Valor(ValorLegible::Booleano(a == "wifi.invitados"));
        assert_eq!(estado_paso(&aislar, &leer, None), EstadoPaso::Pendiente);
    }

    #[test]
    fn desconocido_vacio_es_sin_leer_y_otro_es_pendiente() {
        let p = espejo();
        let vacio = con(ValorLegible::Desconocido(String::new()));
        let e = estado_paso(&p, &vacio, None);
        assert!(es_sin_leer(&e), "{e:?}");
        assert_eq!(
            e,
            EstadoPaso::SinLeer {
                motivo: t!("mejoras-motivo-sin-dato")
            }
        );
        let raro = con(ValorLegible::Desconocido("7".into()));
        assert_eq!(estado_paso(&p, &raro, None), EstadoPaso::Pendiente);
    }

    /// El motivo nombra la pestaña, nunca la lectura de la API.
    #[test]
    fn lectura_fallida_no_cuenta() {
        let e = estado_paso(&espejo(), &falla, None);
        let EstadoPaso::SinLeer { motivo } = &e else {
            panic!("{e:?}");
        };
        assert!(motivo.contains(&t!("ajustes-tema-sistema")), "{motivo}");
        assert!(!motivo.contains("cb"), "{motivo}");
        let pasos = vec![
            (espejo(), e.clone()),
            (paso("upnp", Vec::new()), EstadoPaso::Pendiente),
        ];
        assert_eq!(
            estado_lote(&pasos),
            EstadoLote::Pendientes {
                pendientes: 1,
                total: 1
            }
        );
        assert_eq!(pendientes_unicos(&pasos), Some(1));
    }

    #[test]
    fn paso_compartido_cuenta_una_vez() {
        let pasos = vec![
            (espejo(), EstadoPaso::Pendiente),
            (paso("upnp", Vec::new()), EstadoPaso::Hecho),
            // El mismo paso en el otro lote.
            (espejo(), EstadoPaso::Pendiente),
        ];
        assert_eq!(pendientes_unicos(&pasos), Some(1));
        let por_red = |red: &str| PasoExpandido {
            elemento: Some(red.into()),
            ..paso("wps", Vec::new())
        };
        let pasos = vec![
            (por_red("red-24"), EstadoPaso::Pendiente),
            (por_red("red-5"), EstadoPaso::Pendiente),
        ];
        assert_eq!(pendientes_unicos(&pasos), Some(2), "una por red");
    }

    #[test]
    fn nada_leido_no_da_contador() {
        let motivo = || EstadoPaso::SinLeer { motivo: "x".into() };
        let pasos = vec![(espejo(), motivo()), (paso("upnp", Vec::new()), motivo())];
        assert_eq!(pendientes_unicos(&pasos), None);
        assert_eq!(pendientes_unicos(&[]), None);
        // Todo leído y hecho: 0, que no es «sin contador por no leer».
        let pasos = vec![(espejo(), EstadoPaso::Hecho)];
        assert_eq!(pendientes_unicos(&pasos), Some(0));
    }

    #[test]
    fn opcional_no_cuenta() {
        let hora = PasoExpandido {
            opcional: true,
            ..paso("hora_neutra", Vec::new())
        };
        let pasos = vec![
            (espejo(), EstadoPaso::Hecho),
            (hora.clone(), EstadoPaso::Pendiente),
        ];
        assert_eq!(estado_lote(&pasos), EstadoLote::Aplicado);
        assert_eq!(pendientes_unicos(&pasos), Some(0));
        // Solo el opcional leído: no hay nada que contar.
        let pasos = vec![
            (espejo(), EstadoPaso::SinLeer { motivo: "x".into() }),
            (hora, EstadoPaso::Pendiente),
        ];
        assert_eq!(pendientes_unicos(&pasos), None);
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
    }

    #[test]
    fn lote_sin_leer_si_ninguno_se_lee() {
        let a = estado_paso(&espejo(), &falla, None);
        let pasos = vec![
            (espejo(), a.clone()),
            (paso("upnp", Vec::new()), a.clone()),
            (
                PasoExpandido {
                    opcional: true,
                    ..paso("hora_neutra", Vec::new())
                },
                EstadoPaso::Pendiente,
            ),
        ];
        let EstadoPaso::SinLeer { motivo } = a else {
            unreachable!()
        };
        assert_eq!(estado_lote(&pasos), EstadoLote::SinLeer { motivo });
    }

    /// Lo no leído no se da por hecho: sin pendientes leídos y con pasos
    /// sin leer, el lote no está aplicado, y el motivo dice cuántos faltan.
    #[test]
    fn lote_con_pasos_sin_leer_y_lo_demas_hecho_no_esta_aplicado() {
        let sin_leer = estado_paso(&espejo(), &falla, None);
        let pasos = vec![
            (paso("upnp", Vec::new()), EstadoPaso::Hecho),
            (espejo(), sin_leer.clone()),
            (paso("registro", Vec::new()), sin_leer),
        ];
        assert_eq!(
            estado_lote(&pasos),
            EstadoLote::SinLeer {
                motivo: t!("mejoras-motivo-pasos-sin-leer", n = 2)
            }
        );
        assert_eq!(pendientes_unicos(&pasos), Some(0));
        // Con un pendiente leído, sigue siendo «x de y».
        let mut pasos = pasos;
        pasos[0].1 = EstadoPaso::Pendiente;
        assert_eq!(
            estado_lote(&pasos),
            EstadoLote::Pendientes {
                pendientes: 1,
                total: 1
            }
        );
    }
}
