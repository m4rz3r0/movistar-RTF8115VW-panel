//! Las otras redes de invitados (puntos de acceso 3 a 7), que el panel no muestra: si están separadas de la red de casa.
//!
//! Salen de las listas de puntos de acceso de las dos radios, que ya se
//! leen para la nota de la radio (`claves_de_listas`). En el router real
//! (2026-10-23) los puntos de acceso 2 a 7 son redes de invitados: la 2 es
//! la que muestra el panel y las demás solo se ven aquí.

use super::super::super::lectura::instancias;
use super::*;
use crate::ajustes_router::RecomendacionVista;

/// El primer punto de acceso de invitados (la red de invitados del panel).
const PRIMERA_INVITADA: u32 = 2;
/// El último punto de acceso de invitados.
const ULTIMA_INVITADA: u32 = 7;
/// Ajuste y campo a los que apunta la recomendación (solo informa).
const AJUSTE: &str = "wifi.invitados_avanzado";
const CAMPO: &str = "separada";

/// Un punto de acceso de invitados según las listas.
struct Invitada {
    id: u32,
    ssid: String,
    encendida: bool,
    /// `None` si la lectura no lo dice.
    separada: Option<bool>,
}

/// Los puntos de acceso de invitados (ids 2 a 7) de las listas de las dos
/// radios, por id, y si se leyeron todas las listas (cada radio trae la
/// mitad de las redes: si falta una, faltan las suyas). `None` si no se
/// pudo leer ninguna lista.
fn invitadas(ctx: &Contexto) -> Option<(Vec<Invitada>, bool)> {
    let claves = claves_de_listas(ctx.del_tema);
    let listas: Vec<&XmlNode> = claves.iter().filter_map(|(_, c)| ctx.leida(c)).collect();
    if listas.is_empty() {
        return None;
    }
    let completas = listas.len() == claves.len();
    let elementos = [ID_PUNTO, "ssid", "adm_state", "guest_networking"];
    let mut out: Vec<Invitada> = listas
        .into_iter()
        .flat_map(|l| instancias(l, &elementos))
        .filter_map(|l| {
            let id: u32 = l.get(ID_PUNTO)?.trim().parse().ok()?;
            (PRIMERA_INVITADA..=ULTIMA_INVITADA)
                .contains(&id)
                .then(|| Invitada {
                    id,
                    ssid: l
                        .get("ssid")
                        .map(|s| s.trim().to_owned())
                        .unwrap_or_default(),
                    encendida: si_no(&l, "adm_state") == Some(true),
                    separada: si_no(&l, "guest_networking"),
                })
        })
        .collect();
    out.sort_by_key(|i| i.id);
    out.dedup_by_key(|i| i.id);
    Some((out, completas))
}

/// `derivado:otras_invitadas`: cuántas de las otras redes de invitados
/// (ids 3 a 7) están separadas de la red de casa («5 de 5 separadas»).
/// `None` si falta alguna de las listas, si no traen ninguna o si de
/// alguna no se sabe si está separada: una cifra parcial parecería
/// completa.
pub(in super::super) fn otras_invitadas(ctx: &Contexto) -> Option<ValorLegible> {
    let (todas, completas) = invitadas(ctx)?;
    let otras: Vec<Invitada> = todas
        .into_iter()
        .filter(|i| i.id > PRIMERA_INVITADA)
        .collect();
    if !completas || otras.is_empty() || otras.iter().any(|i| i.separada.is_none()) {
        return None;
    }
    let separadas = otras.iter().filter(|i| i.separada == Some(true)).count();
    Some(ValorLegible::Texto(t!(
        "ajustes-dato-otras_invitadas-valor",
        n = separadas,
        total = otras.len()
    )))
}

/// Nombre de una red de invitados en un texto: su nombre y su número, o
/// solo el número si no tiene nombre.
fn nombre(i: &Invitada) -> String {
    if i.ssid.is_empty() {
        t!("ajustes-wifi-invitada-numero", id = i.id)
    } else {
        t!(
            "ajustes-wifi-invitada-nombre",
            nombre = i.ssid.clone(),
            id = i.id
        )
    }
}

/// Recomendación (solo informa: el panel no puede cambiarlo) si alguna red
/// de invitados (id 2 o más) está encendida y no está separada de la red de
/// casa. Es un riesgo: quien se conecte a ella llega a los dispositivos de
/// casa. Si falta la lista de una radio, avisa solo de las redes que se
/// ven: las de la otra radio no se dan por separadas, solo no se nombran (y
/// el dato `otras_invitadas` sale sin cifra).
pub(in super::super) fn invitada_sin_separar(ctx: &Contexto) -> Option<RecomendacionVista> {
    let (todas, _) = invitadas(ctx)?;
    let sin_separar: Vec<String> = todas
        .iter()
        .filter(|i| i.encendida && i.separada == Some(false))
        .map(nombre)
        .collect();
    if sin_separar.is_empty() {
        return None;
    }
    Some(RecomendacionVista {
        problema: t!(
            "ajustes-problema-wifi-invitada-sin-separar",
            n = sin_separar.len(),
            redes = super::super::super::cortafuegos::enumerar(&sin_separar)
        ),
        texto: t!("ajustes-wifi-invitada-sin-separar-recomendado"),
        riesgo: true,
        boton: String::new(),
        ajuste: AJUSTE.to_owned(),
        campo: CAMPO.to_owned(),
        valor: String::new(),
        lotes: Vec::new(),
        informativa: true,
        // Las pone `fuentes::en_recomendacion`.
        fuentes: Vec::new(),
        respaldo: None,
    })
}
