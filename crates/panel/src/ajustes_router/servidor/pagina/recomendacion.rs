//! La recomendación principal de cada elemento y qué pasa ahora con su campo.

use super::*;

/// Valor legible de un campo como se escribe (`true`, la clave de una
/// opción, el texto); `None` si es un secreto o el panel no lo reconoce.
fn legible(v: &ValorLegible) -> Option<String> {
    match v {
        ValorLegible::Texto(t) | ValorLegible::Opcion(t) => Some(t.clone()),
        ValorLegible::Booleano(b) => Some(b.to_string()),
        ValorLegible::Desconocido(_) | ValorLegible::Secreto => None,
    }
}

/// El campo tiene recomendación y otro valor que el panel reconoce (y que no
/// vale igual, como otro servidor de hora neutro): uno que no reconoce no es
/// una recomendación pendiente (no se sabe qué tiene).
pub(in super::super) fn pendiente(c: &CampoVista) -> bool {
    c.tipo != TipoCampo::Secreto
        && match (&c.recomendado, legible(&c.valor)) {
            (Some(r), Some(v)) => v != *r && !ajustes::valor_aceptado(&c.aceptados, &v),
            _ => false,
        }
}

/// Texto legible del valor actual de un campo (como en la página).
fn texto_actual(c: &CampoVista) -> String {
    match &c.valor {
        ValorLegible::Booleano(b) => crate::ui::grupo::texto_booleano(&c.clave, *b),
        ValorLegible::Opcion(o) => traducir(&format!("{}-{o}", c.clave), None),
        ValorLegible::Texto(t) | ValorLegible::Desconocido(t) => t.clone(),
        ValorLegible::Secreto => String::new(),
    }
}

/// Claves del texto de lo que pasa ahora con el campo `c` del ajuste
/// `ajuste`, de la más concreta a la menos: la de su valor
/// (`ajustes-problema-<tema>-<campo>-<valor>`, «El WPS está activado: …») y
/// la del campo con el valor como argumento
/// (`ajustes-problema-<tema>-<campo>`, para un texto libre como un servidor).
fn claves_problema(ajuste: &str, c: &CampoVista) -> Vec<String> {
    let tema = ajuste.split_once('.').map_or(ajuste, |(t, _)| t);
    let campo = format!("ajustes-problema-{tema}-{}", c.id);
    let mut claves: Vec<String> = legible(&c.valor)
        .map(|v| format!("{campo}-{v}"))
        .into_iter()
        .collect();
    claves.push(campo);
    claves
}

/// Qué pasa ahora con un campo que no tiene lo recomendado: su texto propio
/// ([`claves_problema`]) o, sin él, el valor que tiene («Ahora: WPS, Sí.»).
fn problema(ajuste: &str, c: &CampoVista) -> String {
    let idioma = crate::ui::i18n::idioma_actual();
    let actual = texto_actual(c);
    match claves_problema(ajuste, c)
        .into_iter()
        .find(|k| i18n::existe(idioma, k))
    {
        Some(k) => {
            let mut args = ::i18n::FluentArgs::new();
            args.set("valor", actual);
            traducir(&k, Some(&args))
        }
        None => ahora(c, actual),
    }
}

/// «Ahora: …» genérico. Un sí/no sin textos propios se dice con su nombre
/// («Ahora: UPnP activado.», «Ahora no: Anuncios del router.»), no con
/// «, Sí»; lo demás, con su valor.
fn ahora(c: &CampoVista, actual: String) -> String {
    let idioma = crate::ui::i18n::idioma_actual();
    let campo = traducir(&c.clave, None);
    match c.valor {
        ValorLegible::Booleano(b) if !i18n::existe(idioma, &format!("{}-{b}", c.clave)) => {
            if b {
                t!("ajustes-recomendacion-ahora-si", campo = campo)
            } else {
                t!("ajustes-recomendacion-ahora-no", campo = campo)
            }
        }
        _ => t!("ajustes-recomendacion-ahora", campo = campo, valor = actual),
    }
}

/// La recomendación pendiente más importante de los ajustes de un
/// elemento: primero las de seguridad o de una escritura de riesgo alto;
/// después, el resto en el orden del catálogo. Solo de campos editables y
/// que el botón puede cambiar solos: ni los de una lista (sus campos son la
/// plantilla de una instancia nueva) ni los de un ajuste con una clave que
/// el router no deja leer (guardarlo la exige; se cambia desde el detalle).
pub(super) fn recomendacion(
    f: &Catalogo,
    cat: &CatalogoAjustes,
    acciones: &[&ajustes::Accion],
    ajustes: &[AjusteVista],
) -> Option<RecomendacionVista> {
    let candidatas: Vec<(&AjusteVista, &CampoVista, bool)> = ajustes
        .iter()
        .filter(|v| !v.lista && !v.campos.iter().any(|c| c.tapado))
        .flat_map(|v| v.campos.iter().map(move |c| (v, c)))
        .filter(|(_, c)| c.solo_lectura.is_none() && pendiente(c))
        .map(|(v, c)| {
            let del_catalogo = cat.ajuste(&v.id).and_then(|a| a.campo(&c.id));
            let alto = del_catalogo
                .and_then(Campo::escribible)
                .and_then(|w| f.funcion(&w.funcion))
                .is_some_and(|func| func.riesgo == Riesgo::Alto);
            (v, c, alto || del_catalogo.is_some_and(|x| x.seguridad))
        })
        .collect();
    let (v, c, riesgo) = candidatas
        .iter()
        .find(|(_, _, riesgo)| *riesgo)
        .or_else(|| candidatas.first())?;
    let valor = c.recomendado.clone()?;
    let boton = acciones
        .iter()
        .find(|a| a.campo.as_ref() == Some(&c.id) && a.valor.as_ref() == Some(&valor))
        .map_or_else(
            || t!("ajustes-recomendacion-aplicar"),
            |a| traducir(&clave_accion(&a.id), None),
        );
    Some(RecomendacionVista {
        problema: problema(&v.id, c),
        texto: c
            .motivo_recomendado
            .as_deref()
            .map(|k| traducir(k, None))
            .unwrap_or_default(),
        riesgo: *riesgo,
        boton,
        lotes: ajustes::lotes_del_campo(cat, &v.id, &c.id)
            .into_iter()
            .map(ajustes::clave_lote)
            .collect(),
        ajuste: v.id.clone(),
        campo: c.id.clone(),
        valor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn campo(tipo: TipoCampo, valor: ValorLegible, recomendado: Option<&str>) -> CampoVista {
        CampoVista {
            id: "wps".into(),
            clave: "ajuste-wifi-red_24-wps".into(),
            tipo,
            valor,
            opciones: Vec::new(),
            min: None,
            max: None,
            ascii: false,
            opcional: false,
            solo_lectura: None,
            tapado: false,
            valor_secreto: None,
            recomendado: recomendado.map(str::to_owned),
            motivo_recomendado: None,
            aceptados: Vec::new(),
            requiere: None,
            sin_eleccion: false,
        }
    }

    /// Cada recomendación de Internet, Seguridad, Teléfono y Sistema que
    /// puede salir en una tarjeta dice qué pasa ahora con su propio texto
    /// (no «Ahora: …»), en los dos idiomas. No salen las de una lista ni
    /// las de un ajuste que exige una clave que el router no deja leer.
    #[test]
    fn cada_recomendacion_dice_que_pasa() {
        let cat = CatalogoAjustes::load().unwrap();
        let f = Catalogo::load().unwrap();
        let temas = [
            Tema::Internet,
            Tema::Seguridad,
            Tema::Telefono,
            Tema::Sistema,
        ];
        let mut vistas = 0;
        for a in cat
            .todos()
            .iter()
            .filter(|a| temas.contains(&a.tema) && !a.lista)
        {
            if a.campos
                .iter()
                .any(|c| super::super::super::lectura::tapado(&f, c))
            {
                continue;
            }
            for c in a.campos.iter().filter(|c| c.escribible().is_some()) {
                // Los valores que no son lo recomendado.
                let otros: Vec<ValorLegible> = match (&c.tipo, &c.recomendado) {
                    (TipoCampo::Booleano, Some(r)) => {
                        vec![ValorLegible::Booleano(r != "true")]
                    }
                    (TipoCampo::Opcion, _) if c.opciones.iter().any(|o| o.recomendada) => c
                        .opciones
                        .iter()
                        .filter(|o| !o.recomendada)
                        .map(|o| ValorLegible::Opcion(o.clave.clone()))
                        .collect(),
                    (_, Some(_)) => vec![ValorLegible::Texto("hora.example".into())],
                    _ => Vec::new(),
                };
                for valor in otros {
                    vistas += 1;
                    let mut v = campo(c.tipo, valor, None);
                    v.id = c.id.clone();
                    for idioma in i18n::Idioma::TODOS {
                        assert!(
                            claves_problema(&a.id, &v)
                                .iter()
                                .any(|k| i18n::existe(idioma, k)),
                            "{}.{} = {:?} en {idioma:?}",
                            a.id,
                            c.id,
                            v.valor
                        );
                    }
                }
            }
        }
        assert!(vistas >= 5, "{vistas}");
    }

    /// Solo cuenta como pendiente un valor reconocido distinto del
    /// recomendado: uno que el panel no reconoce no es ni recomendación ni
    /// riesgo.
    #[test]
    fn un_valor_no_reconocido_no_es_pendiente() {
        let b = TipoCampo::Booleano;
        assert!(pendiente(&campo(
            b,
            ValorLegible::Booleano(true),
            Some("false")
        )));
        assert!(!pendiente(&campo(
            b,
            ValorLegible::Booleano(false),
            Some("false")
        )));
        assert!(!pendiente(&campo(b, ValorLegible::Booleano(true), None)));
        let raro = ValorLegible::Desconocido("7".into());
        assert!(!pendiente(&campo(b, raro.clone(), Some("false"))));
        assert!(!pendiente(&campo(TipoCampo::Opcion, raro, Some("wpa2"))));
        let secreto = campo(TipoCampo::Secreto, ValorLegible::Secreto, Some("x"));
        assert!(!pendiente(&secreto));
    }
}
