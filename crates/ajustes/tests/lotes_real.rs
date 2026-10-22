//! Los lotes del catálogo compilado: sus pasos, cómo se expanden por red y
//! sus textos en los dos idiomas, con el formato de las recomendaciones.

use ajustes::{
    CatalogoAjustes, Destino, Lote, PasoExpandido, clave_lote, clave_paso, clave_proveedor_dns,
    claves_de_lotes, expandir, lotes_del_campo,
};
use i18n::Idioma;
use std::{collections::BTreeSet, fs, path::PathBuf};

mod comun_textos;
use comun_textos::{marcas_de_recomendacion, recomendacion_bien_formada};

fn cargar() -> CatalogoAjustes {
    CatalogoAjustes::load().expect("ajustes y lotes legibles")
}

fn ids_de(lotes: &[Lote]) -> Vec<&str> {
    lotes.iter().map(|l| l.id.as_str()).collect()
}

#[test]
fn lotes_del_catalogo_real() {
    let c = cargar();
    assert_eq!(ids_de(c.lotes()), ["seguridad", "privacidad"]);
    assert_eq!(
        c.lote("seguridad").unwrap().pasos,
        [
            "gestion_remota",
            "aviso_operador",
            "wps",
            "upnp",
            "wpa2",
            "aislar_invitados",
            "espejo",
            "registro"
        ]
    );
    assert_eq!(
        c.lote("privacidad").unwrap().pasos,
        [
            "gestion_remota",
            "aviso_operador",
            "espejo",
            "registro",
            "dns_casa",
            "hora_neutra"
        ]
    );
    let registro = expandir(&c, c.paso("registro").unwrap());
    assert_eq!(
        registro[0].destinos,
        [Destino {
            ajuste: "sistema.registro".into(),
            campo: "envio".into(),
            valor: Some("false".into())
        }]
    );
    let dns = expandir(&c, c.paso("dns_casa").unwrap());
    assert_eq!(dns.len(), 1);
    assert_eq!(dns[0].elegir.as_deref(), Some("proveedor_dns"));
    let campos: Vec<&str> = dns[0].destinos.iter().map(|d| d.campo.as_str()).collect();
    assert_eq!(campos, ["principal", "secundario"]);
    assert!(
        dns[0]
            .destinos
            .iter()
            .all(|d| d.ajuste == "red_local.dns_casa")
    );
    let proveedor = c.eleccion("proveedor_dns").unwrap();
    assert_eq!(proveedor.defecto().id, "quad9");
    assert_eq!(proveedor.opciones.len(), 5);
    let quad9 = proveedor.defecto();
    assert_eq!(quad9.valores["principal"], "9.9.9.9");
    assert_eq!(quad9.valores["secundario"], "149.112.112.112");
    assert_eq!(
        lotes_del_campo(&c, "sistema.registro", "envio"),
        ["seguridad", "privacidad"]
    );
    assert_eq!(
        lotes_del_campo(&c, "red_local.dns_casa", "principal"),
        ["privacidad"]
    );
    let wps = expandir(&c, c.paso("wps").unwrap());
    assert_eq!(
        wps.iter().map(PasoExpandido::clave).collect::<Vec<_>>(),
        ["wps@red-24", "wps@red-5", "wps@invitados"]
    );
    assert_eq!(
        wps[2].destinos,
        [Destino {
            ajuste: "wifi.invitados".into(),
            campo: "wps".into(),
            valor: Some("false".into())
        }]
    );
    assert_eq!(
        lotes_del_campo(&c, "seguridad.gestion_remota", "activa"),
        ["seguridad", "privacidad"]
    );
    assert_eq!(lotes_del_campo(&c, "wifi.red_24", "wps"), ["seguridad"]);
    assert!(lotes_del_campo(&c, "wifi.red_24_avanzado", "aislamiento").is_empty());
    assert!(c.paso("hora_neutra").unwrap().opcional);
}

/// La primera marca tiene que ser una palabra entera: «Recomendado
/// siempre» no dice para quién es.
#[test]
fn recomendacion_exige_la_marca_como_palabra() {
    let es = marcas_de_recomendacion(Idioma::Es);
    let en = marcas_de_recomendacion(Idioma::En);
    assert!(recomendacion_bien_formada(
        "Recomendado si usas X. A favor: a. En contra: b.",
        es
    ));
    assert!(!recomendacion_bien_formada(
        "Recomendado siempre. A favor: a. En contra: b.",
        es
    ));
    assert!(recomendacion_bien_formada(
        "Recommended if you use X. Pros: a. Cons: b.",
        en
    ));
    assert!(!recomendacion_bien_formada(
        "Recommended iffy. Pros: a. Cons: b.",
        en
    ));
}

/// Claves de los mensajes de `mejoras.ftl` en `idioma`.
fn claves_de_mejoras(idioma: Idioma) -> Vec<String> {
    let ruta = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../i18n/locales"))
        .join(idioma.codigo())
        .join("mejoras.ftl");
    fs::read_to_string(&ruta)
        .unwrap_or_else(|e| panic!("{}: {e}", ruta.display()))
        .lines()
        .filter(|l| !l.starts_with([' ', '\t', '#']) && !l.trim().is_empty())
        .filter_map(|l| l.split_once('=').map(|(k, _)| k.trim().to_owned()))
        .collect()
}

/// Cada texto que piden los lotes existe en los dos idiomas, y `mejoras.ftl`
/// no trae textos de lotes, pasos, avisos o consejos que nadie pide.
#[test]
fn claves_de_lotes_en_los_dos_idiomas() {
    const ESTADOS_DE_PASO: [&str; 4] = [
        "mejoras-paso-estado-hecho",
        "mejoras-paso-estado-pendiente",
        "mejoras-paso-estado-sin-leer",
        "mejoras-paso-estado-no-aplica",
    ];
    let c = cargar();
    let claves: BTreeSet<String> = claves_de_lotes(&c).into_iter().collect();
    assert!(
        claves.contains("mejoras-lote-seguridad-titulo"),
        "{claves:?}"
    );
    assert!(claves.contains("mejoras-aviso-tr069"), "{claves:?}");
    // El nombre y la nota de cada proveedor de DNS.
    for o in &c.eleccion("proveedor_dns").unwrap().opciones {
        let k = clave_proveedor_dns(&o.id);
        assert!(claves.contains(&format!("{k}-desc")), "{k}: {claves:?}");
        assert!(claves.contains(&k), "{k}: {claves:?}");
    }
    let de_lotes = [
        "mejoras-lote-",
        "mejoras-paso-",
        "mejoras-aviso-",
        "mejoras-consejo-",
    ];
    for idioma in Idioma::TODOS {
        let faltan: Vec<&String> = claves.iter().filter(|k| !i18n::existe(idioma, k)).collect();
        assert!(faltan.is_empty(), "faltan en {idioma:?}: {faltan:#?}");
        let sobran: Vec<String> = claves_de_mejoras(idioma)
            .into_iter()
            // Los estados de un paso son de la página, no pasos del catálogo.
            .filter(|k| !ESTADOS_DE_PASO.contains(&k.as_str()))
            .filter(|k| de_lotes.iter().any(|p| k.starts_with(p)) && !claves.contains(k))
            .collect();
        assert!(sobran.is_empty(), "sobran en {idioma:?}: {sobran:#?}");
    }
}

/// La descripción de cada lote y de cada paso dice para quién es, qué se
/// gana y qué se pierde, en los dos idiomas.
#[test]
fn textos_de_lotes_con_pros_y_contras() {
    let c = cargar();
    let mut claves: Vec<String> = c
        .lotes()
        .iter()
        .map(|l| format!("{}-desc", clave_lote(&l.id)))
        .collect();
    let pasos: BTreeSet<&str> = c
        .lotes()
        .iter()
        .flat_map(|l| l.pasos.iter().map(String::as_str))
        .collect();
    claves.extend(pasos.iter().map(|p| format!("{}-desc", clave_paso(p))));
    let mut fallos = Vec::new();
    for idioma in Idioma::TODOS {
        for clave in &claves {
            let texto = i18n::tr(idioma, clave, None);
            if !recomendacion_bien_formada(&texto, marcas_de_recomendacion(idioma)) {
                fallos.push(format!("{idioma:?} {clave}: «{texto}»"));
            }
        }
    }
    assert!(
        fallos.is_empty(),
        "textos sin «para quién / a favor / en contra»: {fallos:#?}"
    );
}
