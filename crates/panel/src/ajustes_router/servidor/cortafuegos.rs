//! Lado del servidor del cortafuegos: «No responder al ping desde internet» (preparar y su resumen) y lo que dice su tarjeta.
//!
//! La regla la pone y la quita el gestor de cambios (ver
//! `changes::especiales::cortafuegos`): aquí se prepara el alta con la
//! carga exacta, se resume el cambio (también el de su deshacer) y se
//! calcula, con la lectura de la página, si el router responde al ping, la
//! recomendación (solo informa) y si la acción está disponible.

use super::derivados::Contexto;
use super::*;
use crate::ajustes_router::{AccionPropia, CambioLegible, PasoVista, RecomendacionVista};
use crate::changes::SIN_PING_YA_PUESTA;
use crate::ui::comun::EstadoVisual;
use crate::ui::i18n::traducir;
use crate::validacion_real::{Funcion3, sin_validar};
use hgu::cortafuegos::{self as cf, ALTA, Abierto, Filtro, Servicio};

/// El ajuste de las reglas.
pub(super) const AJUSTE: &str = "seguridad.cortafuegos";
/// Título de los cambios en el Historial: el del ajuste.
const TITULO: &str = "ajuste-seguridad-cortafuegos-titulo";
/// Id de la acción propia del elemento.
pub(crate) const SIN_PING: &str = "sin_ping";
/// El campo del resumen del cambio: la regla puesta o no.
const CAMPO: &str = "ajuste-seguridad-cortafuegos-sin-ping";

/// Las reglas con la huella «sin ping» de los filtros de entrada IPv4.
fn con_huella(f: &[Filtro]) -> impl Iterator<Item = &cf::Regla> {
    f.iter()
        .filter(|x| x.tipo == "0" && x.version == "0")
        .flat_map(|x| &x.reglas)
        .filter(|r| cf::es_sin_ping(r))
}

/// ¿Hay ya una regla con la huella «sin ping», activa o no? Con ella el
/// alta da 409 ([`SIN_PING_YA_PUESTA`]), así que la acción no se ofrece.
fn hay_regla(f: &[Filtro]) -> bool {
    con_huella(f).next().is_some()
}

/// ¿Está puesta y activa la regla «sin ping»? Una desactivada en el router
/// no cuenta para el estado.
pub(super) fn puesta(f: &[Filtro]) -> bool {
    con_huella(f).any(|r| r.enable == "1")
}

/// El ping que llega de internet, por versión: abierto desde cualquier
/// origen. En IPv4 solo cuenta sin la regla del panel; con ella no se
/// afirma nada (su efecto está aún sin validar en el router real, y la
/// regla no toca IPv6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Ping {
    pub v4: bool,
    pub v6: bool,
    /// La regla del panel está puesta.
    pub regla: bool,
}

/// Los servicios abiertos por los filtros de la versión `v` (0 IPv4, 1
/// IPv6).
fn abiertos_en(f: &[Filtro], v: &str) -> Vec<Abierto> {
    let solo: Vec<Filtro> = f.iter().filter(|x| x.version == v).cloned().collect();
    cf::abiertos(&solo)
}

/// ¿Abre `a` el ping a cualquier origen?
fn ping_abierto(a: &[Abierto]) -> bool {
    a.iter()
        .any(|a| a.servicio == Servicio::Ping && a.desde_cualquiera)
}

/// El ping de [`Ping`] según los filtros leídos.
pub(super) fn ping(f: &[Filtro]) -> Ping {
    let regla = puesta(f);
    Ping {
        v4: !regla && ping_abierto(&abiertos_en(f, "0")),
        v6: ping_abierto(&abiertos_en(f, "1")),
        regla,
    }
}

/// Estado de la tarjeta: en ámbar si responde al ping por alguna versión;
/// con la regla del panel puesta y nada más abierto, solo eso (sin decir
/// que no responde).
pub(super) fn estado(p: Ping) -> (EstadoVisual, &'static str) {
    match (p.v4, p.v6) {
        (true, true) => (EstadoVisual::Atencion, "ajustes-estado-cortafuegos-ping"),
        (true, false) => (
            EstadoVisual::Atencion,
            "ajustes-estado-cortafuegos-ping-ipv4",
        ),
        (false, true) => (
            EstadoVisual::Atencion,
            "ajustes-estado-cortafuegos-ping-ipv6",
        ),
        (false, false) if p.regla => (
            EstadoVisual::Emite,
            "ajustes-estado-cortafuegos-regla-puesta",
        ),
        (false, false) => (EstadoVisual::Emite, "ajustes-estado-cortafuegos-sin-ping"),
    }
}

/// Nombre legible de un servicio.
fn nombre(s: Servicio) -> String {
    match s {
        Servicio::Ping => t!("ajuste-seguridad-cortafuegos-servicio-ping"),
        Servicio::Web => t!("ajuste-seguridad-cortafuegos-servicio-web"),
        Servicio::Ssh => t!("ajuste-seguridad-cortafuegos-servicio-ssh"),
        Servicio::GestionOperador => t!("ajuste-seguridad-cortafuegos-servicio-gestion"),
    }
}

/// «a», «a y b», «a, b y c».
fn enumerar(nombres: &[String]) -> String {
    match nombres {
        [] => String::new(),
        [uno] => uno.clone(),
        [antes @ .., ultimo] => format!(
            "{} {} {ultimo}",
            antes.join(", "),
            t!("ajuste-seguridad-cortafuegos-y")
        ),
    }
}

/// Qué pasa ahora: el ping abierto a cualquiera, por versión, y los
/// servicios abiertos (la web, el SSH, la gestión remota del operador o el
/// ping solo desde redes concretas) desde cualquier sitio o solo desde
/// redes concretas. Vacío si nada está abierto.
fn problema(f: &[Filtro], p: Ping) -> Vec<String> {
    let mut out = Vec::new();
    match (p.v4, p.v6) {
        (true, true) => out.push(t!("ajustes-problema-seguridad-cortafuegos-ping")),
        (true, false) => out.push(t!("ajustes-problema-seguridad-cortafuegos-ping-ipv4")),
        (false, true) => out.push(t!("ajustes-problema-seguridad-cortafuegos-ping-ipv6")),
        (false, false) => {}
    }
    // El ping abierto a cualquiera ya va arriba; solo desde redes
    // concretas, con los demás.
    let servicios: Vec<Abierto> = cf::abiertos(f)
        .into_iter()
        .filter(|a| a.servicio != Servicio::Ping || !a.desde_cualquiera)
        .collect();
    let de = |cualquiera: bool| -> String {
        let n: Vec<String> = servicios
            .iter()
            .filter(|a| a.desde_cualquiera == cualquiera)
            .map(|a| nombre(a.servicio))
            .collect();
        enumerar(&n)
    };
    let cualquiera = de(true);
    if !cualquiera.is_empty() {
        out.push(t!(
            "ajustes-problema-seguridad-cortafuegos-servicios-cualquiera",
            servicios = cualquiera
        ));
    }
    let redes = de(false);
    if !redes.is_empty() {
        out.push(t!(
            "ajustes-problema-seguridad-cortafuegos-servicios-redes",
            servicios = redes
        ));
    }
    out
}

/// Los filtros de la lectura del cortafuegos de la página (`None` si no se
/// pudo leer).
pub(super) fn filtros(ctx: &Contexto) -> Option<Vec<Filtro>> {
    ctx.nodo(AJUSTE).map(cf::de_xml)
}

/// Si `e` es el elemento del cortafuegos, su recomendación (solo informa:
/// se aplica con la acción propia) y «No responder al ping desde internet»
/// con el motivo si no está disponible (sin leer, sin validar en el router
/// real o ya puesta).
pub(super) fn del_elemento(
    ctx: &Contexto,
    e: &ajustes::Elemento,
    leido: bool,
) -> Option<(Option<RecomendacionVista>, AccionPropia)> {
    if !e.ajustes.iter().any(|a| a == AJUSTE) {
        return None;
    }
    let accion = |motivo: Option<String>| AccionPropia {
        id: SIN_PING.to_owned(),
        motivo,
    };
    let Some(f) = filtros(ctx).filter(|_| leido) else {
        return Some((
            None,
            accion(Some(t!("ajuste-seguridad-cortafuegos-sin-leer"))),
        ));
    };
    let motivo = match sin_validar(Funcion3::SinPing) {
        Some(m) => Some(traducir(m, None)),
        None if hay_regla(&f) => Some(t!("ajuste-seguridad-cortafuegos-ya-puesta")),
        None => None,
    };
    let p = ping(&f);
    let problema = problema(&f, p);
    // La regla del panel solo arregla el ping de IPv4; lo demás, a revisar.
    let mut texto = Vec::new();
    if p.v4 {
        texto.push(t!("ajuste-seguridad-cortafuegos-sin-ping-recomendado"));
    }
    if p.v6 || problema.len() > usize::from(p.v4) {
        texto.push(t!("ajuste-seguridad-cortafuegos-revisar-recomendado"));
    }
    let recomendacion = (!problema.is_empty()).then(|| RecomendacionVista {
        problema: problema.join(" "),
        texto: texto.join(" "),
        riesgo: false,
        boton: String::new(),
        ajuste: AJUSTE.to_owned(),
        campo: String::new(),
        valor: String::new(),
        lotes: Vec::new(),
        informativa: true,
    });
    Some((recomendacion, accion(motivo)))
}

/// Resumen de un cambio de la regla «sin ping» (también el de su
/// deshacer): un paso que la pone o la quita. Quitarla no es «borrar» algo
/// de la página (`borra`, que cerraría el detalle del elemento): es volver
/// a como estaba.
pub(crate) fn vista(
    g: &ChangeManager,
    token: String,
    pasos: &[PasoPreparado],
    r: Riesgo,
) -> GrupoVista {
    GrupoVista {
        token,
        titulo: TITULO.into(),
        pasos: pasos
            .iter()
            .map(|p| {
                let pone = p.diff.campos.iter().any(|c| !c.despues.is_empty());
                PasoVista {
                    indice: p.indice,
                    sin_cambios: p.sin_cambios,
                    campos: vec![CambioLegible {
                        campo: CAMPO.into(),
                        antes: (!pone).to_string(),
                        despues: pone.to_string(),
                        secreto: false,
                        tipo: TipoCampo::Booleano,
                    }],
                    ..PasoVista::default()
                }
            })
            .collect(),
        riesgo: riesgo(r),
        efecto: None,
        vigilancia_s: g.tiempos().vigilancia(r).as_secs(),
        detalle: None,
        aviso: None,
        borra: false,
        proveedor_dns: None,
    }
}

/// Error 409 con el texto de `clave`.
fn conflicto(clave: &str) -> ServerFnError {
    fallo(StatusCode::CONFLICT, traducir(clave, None))
}

pub(in crate::ajustes_router) async fn preparar_sin_ping() -> Result<GrupoVista, ServerFnError> {
    let (state, _) = preparar().await?;
    if let Some(m) = sin_validar(Funcion3::SinPing) {
        return Err(conflicto(m));
    }
    let g = gestor(&state)?;
    libre(&g)?;
    let filtro = match g.filtro_de_internet().await {
        Ok(f) => f,
        Err(CambioError::Router(e)) => {
            tracing::warn!("cortafuegos: {e}");
            return Err(conflicto("ajuste-seguridad-cortafuegos-sin-leer"));
        }
        Err(CambioError::Invalida(e)) => {
            tracing::warn!("cortafuegos: {e}");
            return Err(conflicto("ajuste-seguridad-cortafuegos-sin-filtro"));
        }
        Err(e) => return Err(error_ajuste(e)),
    };
    if filtro.reglas.iter().any(cf::es_sin_ping) {
        return Err(conflicto(SIN_PING_YA_PUESTA));
    }
    let req = ChangeRequest {
        funcion: ALTA.into(),
        instancia: None,
        valores: cf::carga_sin_ping(&filtro)
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
        borrar: false,
        identidad: BTreeMap::new(),
    };
    let grupo = GrupoRequest {
        pasos: vec![Paso {
            req,
            etiqueta: Some(TITULO.into()),
        }],
        titulo: Some(TITULO.into()),
    };
    let prep = g.preparar_grupo(grupo).await.map_err(error_ajuste)?;
    Ok(vista(&g, prep.token, &prep.pasos, prep.riesgo))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filtros() -> Vec<Filtro> {
        cf::analizar(include_str!(
            "../../../../hgu/fixtures/paginas/scbidirectionalflt_ls.asp"
        ))
        .unwrap()
    }

    /// La regla del panel en el filtro IPv4 de la copia saneada.
    fn con_regla(mut f: Vec<Filtro>) -> Vec<Filtro> {
        let mut r = f[0].reglas[8].clone();
        (r.rid, r.action, r.icmp_type) = ("16".into(), "0".into(), "25".into());
        f[0].reglas.push(r);
        f
    }

    #[test]
    fn el_ping_por_version() {
        let f = filtros();
        let p = ping(&f);
        assert_eq!((p.v4, p.v6, p.regla), (true, true, false));
        assert_eq!(estado(p).1, "ajustes-estado-cortafuegos-ping");
        // Con la regla, IPv6 sigue abierto: no se dice que no responda.
        let p = ping(&con_regla(f.clone()));
        assert_eq!((p.v4, p.v6, p.regla), (false, true, true));
        assert_eq!(estado(p).1, "ajustes-estado-cortafuegos-ping-ipv6");
        // Sin el ping de IPv6, solo «regla puesta», sin afirmar su efecto.
        let mut sin_v6 = con_regla(f);
        sin_v6[1].reglas.retain(|r| r.icmp_type != "55");
        let p = ping(&sin_v6);
        assert_eq!((p.v4, p.v6, p.regla), (false, false, true));
        assert_eq!(estado(p).1, "ajustes-estado-cortafuegos-regla-puesta");
    }

    /// Una regla con la huella pero desactivada no cuenta para el estado
    /// (el ping de IPv4 sigue abierto), aunque sí impide otra alta.
    #[test]
    fn la_regla_desactivada_no_cuenta() {
        let mut f = con_regla(filtros());
        f[0].reglas.last_mut().unwrap().enable = "0".into();
        let p = ping(&f);
        assert_eq!((p.v4, p.regla), (true, false));
        assert!(hay_regla(&f));
    }
}
