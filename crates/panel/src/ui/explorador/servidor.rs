//! Parte de servidor del explorador: catálogo, lecturas enmascaradas,
//! formularios a partir de la lectura pareada, paso de los cambios por el
//! gestor y revelado de secretos. La llaman las server functions de
//! [`super::datos`].

use super::*;
use crate::{
    changes::{CambioError, ChangeManager, ChangeRequest, Estado, buscar, identidad},
    state::AppState,
};
use axum::http::StatusCode;
use catalogo::{Area, Funcion, Origen, Riesgo, Tipo, TipoParam};
use hgu::xml::XmlNode;
use std::{
    collections::{BTreeMap, HashSet},
    fmt,
    sync::Arc,
};

/// Páginas que disparan acciones al cargarse: nunca se cargan, digan lo
/// que digan el catálogo o la petición.
const PAGINAS_PROHIBIDAS: &[&str] = &["syslog.asp", "backup_restore.asp"];

pub fn app_state() -> Result<AppState, ServerFnError> {
    use_context::<AppState>().ok_or_else(|| ServerFnError::new("falta el estado del panel"))
}

/// Marca la respuesta `no-store`: puede llevar secretos (una lectura con sus
/// valores, el detalle de un cambio).
pub(crate) fn sin_guardar() {
    use axum::http::{HeaderValue, header};
    if let Some(resp) = use_context::<leptos_axum::ResponseOptions>() {
        resp.insert_header(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
}

pub(crate) fn con_estado(status: StatusCode, msg: impl Into<String>) -> ServerFnError {
    if let Some(resp) = use_context::<leptos_axum::ResponseOptions>() {
        resp.set_status(status);
    }
    ServerFnError::new(msg.into())
}

pub(crate) fn invalida(msg: impl Into<String>) -> ServerFnError {
    con_estado(StatusCode::BAD_REQUEST, msg)
}

/// La nota (o el error) de un cambio en el idioma de quien la mira (las de
/// antes, que guardan el texto, tal cual).
pub(crate) fn nota_legible(guardada: &str) -> String {
    crate::changes::Nota::texto(guardada, crate::ui::i18n::idioma_actual())
}

/// Lo técnico de un error guardado (el texto del router), para los
/// «Detalles técnicos».
pub(crate) fn error_tecnico(guardado: &str) -> Option<String> {
    crate::changes::Nota::detalle(guardado, crate::ui::i18n::idioma_actual())
}

/// Registra el error con un id y devuelve un mensaje genérico con ese id.
pub(crate) fn interno(e: impl fmt::Display) -> ServerFnError {
    let mut id = [0u8; 4];
    if getrandom::fill(&mut id).is_err() {
        id = (crate::series::now_ms() as u32).to_be_bytes();
    }
    let id: String = id.iter().map(|b| format!("{b:02x}")).collect();
    tracing::error!(id, "error interno en el explorador: {e}");
    con_estado(
        StatusCode::INTERNAL_SERVER_ERROR,
        crate::t!("comun-error-interno-id", id = id),
    )
}

pub(crate) fn gestor(state: &AppState) -> Result<Arc<ChangeManager>, ServerFnError> {
    state
        .cambios()
        .ok_or_else(|| interno("el gestor de cambios no está instalado"))
}

/// Liga el cambio preparado con `token` al usuario de la sesión
/// ([`ChangeManager::ligar`]): solo él lo confirma. Lo llaman las server
/// functions que preparan un cambio.
pub(crate) async fn ligar_a_la_sesion(token: &str) -> Result<(), ServerFnError> {
    let yo = crate::auth::require_user().await?;
    gestor(&app_state()?)?.ligar(token, &yo.username);
    Ok(())
}

/// El error de un motivo del gestor con texto propio
/// ([`crate::changes::motivo_con_texto`]), en el idioma de la petición.
pub(crate) fn error_con_texto(e: &CambioError) -> Option<ServerFnError> {
    let (codigo, clave) = crate::changes::motivo_con_texto(e)?;
    let status = StatusCode::from_u16(codigo).unwrap_or(StatusCode::BAD_REQUEST);
    Some(con_estado(status, crate::ui::i18n::traducir(clave, None)))
}

/// Error del gestor en el idioma de la petición y con qué hacer. El detalle
/// que trae el gestor (`m`) está en español.
pub(crate) fn error_cambio(e: CambioError) -> ServerFnError {
    use CambioError as E;
    if let Some(r) = error_con_texto(&e) {
        return r;
    }
    let (status, msg) = match e {
        E::Bloqueada(m) if m == crate::changes::BLOQUEO_SOLO_LECTURA => {
            (StatusCode::FORBIDDEN, crate::t!("panel-solo-lectura"))
        }
        E::Bloqueada(m) if m == crate::changes::BLOQUEO_SECRETO_TAPADO => (
            StatusCode::FORBIDDEN,
            crate::t!("cambio-error-secreto-tapado"),
        ),
        E::Bloqueada(m) => (
            StatusCode::FORBIDDEN,
            crate::t!("biblioteca-error-bloqueada", motivo = m),
        ),
        E::SinCambios => (
            StatusCode::BAD_REQUEST,
            crate::t!("biblioteca-error-sin-cambios"),
        ),
        E::EnCurso => (StatusCode::CONFLICT, crate::t!("biblioteca-error-en-curso")),
        E::Router(m) => (
            StatusCode::BAD_GATEWAY,
            crate::t!("biblioteca-error-router", detalle = m),
        ),
        E::Verificacion(v) => (
            StatusCode::BAD_GATEWAY,
            crate::t!("biblioteca-error-verificacion", detalle = v.join("; ")),
        ),
        E::PalabraIncorrecta => (
            StatusCode::BAD_REQUEST,
            crate::t!("biblioteca-error-palabra"),
        ),
        E::TokenInvalido => (StatusCode::BAD_REQUEST, crate::t!("biblioteca-error-token")),
        E::Invalida(m) => (
            StatusCode::BAD_REQUEST,
            crate::t!("biblioteca-error-invalida", detalle = m),
        ),
        E::InstanciaCambiada(m) => (
            StatusCode::CONFLICT,
            crate::t!("biblioteca-error-router", detalle = m),
        ),
    };
    con_estado(status, msg)
}

pub(crate) fn funcion<'a>(state: &'a AppState, nombre: &str) -> Result<&'a Funcion, ServerFnError> {
    state.catalogo.funcion(nombre).ok_or_else(|| {
        con_estado(
            StatusCode::NOT_FOUND,
            crate::t!("biblioteca-error-no-existe-funcion", nombre = nombre),
        )
    })
}

// ---- catálogo

fn tipo(t: Tipo) -> TipoFuncion {
    match t {
        Tipo::Lectura => TipoFuncion::Lectura,
        Tipo::Escritura => TipoFuncion::Escritura,
        Tipo::Accion => TipoFuncion::Accion,
    }
}

fn origen(o: Origen) -> OrigenFuncion {
    match o {
        Origen::Basica => OrigenFuncion::Basica,
        Origen::Avanzada => OrigenFuncion::Avanzada,
        Origen::Oculta => OrigenFuncion::Oculta,
        Origen::Incompleta => OrigenFuncion::Incompleta,
    }
}

pub fn riesgo(r: Riesgo) -> RiesgoCambio {
    match r {
        Riesgo::Bajo => RiesgoCambio::Bajo,
        Riesgo::Medio => RiesgoCambio::Medio,
        Riesgo::Alto => RiesgoCambio::Alto,
    }
}

fn tipo_campo(t: TipoParam) -> TipoCampo {
    match t {
        TipoParam::Ip => TipoCampo::Ip,
        TipoParam::Mac => TipoCampo::Mac,
        TipoParam::Bool => TipoCampo::Bool,
        TipoParam::Enum => TipoCampo::Enum,
        TipoParam::Int => TipoCampo::Int,
        TipoParam::Text => TipoCampo::Text,
        TipoParam::Secret => TipoCampo::Secret,
    }
}

/// Motivo por el que el panel no escribe con `f` (catálogo o bloqueos de
/// la v1 del gestor). Las lecturas solo por el catálogo.
fn bloqueo(state: &AppState, f: &Funcion) -> Option<String> {
    if let Some(m) = &f.bloqueada {
        return Some(m.clone());
    }
    if f.tipo == Tipo::Lectura {
        return None;
    }
    match state.cambios().map(|g| g.escribible(&f.name)) {
        Some(Err(CambioError::Bloqueada(m))) => Some(m),
        _ => None,
    }
}

fn resumen(state: &AppState, f: &Funcion) -> FuncionResumen {
    FuncionResumen {
        nombre: f.name.clone(),
        area: f.area.slug().to_owned(),
        descripcion: f.descripcion.clone(),
        tipo: tipo(f.tipo),
        origen: origen(f.origen),
        riesgo: riesgo(f.riesgo),
        bloqueada: bloqueo(state, f),
    }
}

pub fn areas(state: &AppState) -> Vec<AreaVista> {
    Area::TODAS
        .iter()
        .map(|a| AreaVista {
            slug: a.slug().to_owned(),
            etiqueta: a.etiqueta().to_owned(),
            funciones: state
                .catalogo
                .por_area(*a)
                .map(|f| resumen(state, f))
                .collect(),
        })
        .collect()
}

/// Nombre sin el sufijo de id (`_id` o `_0i`), si lo tiene.
fn base_de_id(nombre: &str) -> Option<&str> {
    nombre
        .strip_suffix("_id")
        .or_else(|| nombre.strip_suffix("_0i"))
}

/// Valor con el que se rellena un parámetro de lectura o una clave:
/// `ALL` en `all` y `0` en los ids, salvo el de una lista que admite
/// `all=ALL` (vacío para listarlas todas) y sus alternativas (`x_0i`
/// frente a `x_id`: se rellena solo el id de la lista).
fn inicial(f: &Funcion, nombre: &str) -> String {
    if nombre == "all" {
        return "ALL".into();
    }
    let Some(base) = base_de_id(nombre) else {
        return String::new();
    };
    match &f.lista {
        Some(l) if l.id_param == nombre && l.all => String::new(),
        Some(l) if l.id_param != nombre && base_de_id(&l.id_param) == Some(base) => String::new(),
        _ => "0".into(),
    }
}

fn param_vista(f: &Funcion, p: &catalogo::Parametro) -> ParamVista {
    ParamVista {
        nombre: p.nombre.clone(),
        etiqueta: p.etiqueta.clone(),
        tipo: tipo_campo(p.tipo),
        opciones: p.opciones.clone(),
        inicial: inicial(f, &p.nombre),
    }
}

pub fn detalle(state: &AppState, nombre: &str) -> Result<FuncionDetalle, ServerFnError> {
    let f = funcion(state, nombre)?;
    let ident = identidad(f);
    let id_lista = f.lista.as_ref().map(|l| l.id_param.as_str());
    // Lo que no puede cortar el acceso no se vigila; la WiFi, según por
    // dónde llegue el panel (aquí se cuenta como si se vigilara).
    let vigila = f.acceso != Some(catalogo::Acceso::NoCorta);
    let vigilancia_s = state
        .cambios()
        .map_or(0, |g| g.tiempos().plazo(f.riesgo, vigila).as_secs());
    Ok(FuncionDetalle {
        resumen: resumen(state, f),
        etiqueta_area: f.area.etiqueta().to_owned(),
        pagina: f.pagina.clone(),
        parametros: f.parametros.iter().map(|p| param_vista(f, p)).collect(),
        lectura_pareada: f.lectura_pareada.as_ref().map(|lp| {
            let area = state
                .catalogo
                .funcion(&lp.funcion)
                .map_or(f.area, |l| l.area);
            (lp.funcion.clone(), area.slug().to_owned())
        }),
        lista: f.lista.as_ref().map(|l| ListaVista {
            id_param: l.id_param.clone(),
            borrar: l.del,
        }),
        claves: f
            .parametros
            .iter()
            .filter(|p| ident.contains(&p.nombre) && Some(p.nombre.as_str()) != id_lista)
            .map(|p| param_vista(f, p))
            .collect(),
        vigilancia_s,
        lectura: modo_lectura(f).map(drop),
    })
}

// ---- enmascarado

/// Nombres de parámetros secretos del catálogo (en minúsculas).
fn secretos_catalogo(catalogo: &catalogo::Catalogo) -> HashSet<String> {
    catalogo
        .todas()
        .iter()
        .flat_map(|f| &f.parametros)
        .filter(|p| p.tipo == TipoParam::Secret)
        .map(|p| p.nombre.to_ascii_lowercase())
        .collect()
}

/// Qué se censura en pantalla: los secretos de `secretos.toml` más los del
/// catálogo. Los datos sensibles (usuarios, URL, números) se ven en claro.
pub(crate) struct Ocultar {
    mascara: Arc<catalogo::mask::Mascara>,
    catalogo: HashSet<String>,
}

impl Ocultar {
    pub(crate) fn new(state: &AppState) -> Ocultar {
        Ocultar::de(state.mascara.clone(), &state.catalogo)
    }

    fn de(mascara: Arc<catalogo::mask::Mascara>, catalogo: &catalogo::Catalogo) -> Ocultar {
        Ocultar {
            mascara,
            catalogo: secretos_catalogo(catalogo),
        }
    }

    /// ¿Va censurado en pantalla? Solo un secreto (por la máscara o el
    /// catálogo).
    pub(crate) fn oculto(&self, nombre: &str, valor: &str) -> bool {
        self.mascara.oculta_en_pantalla(nombre, valor)
            || self.catalogo.contains(&nombre.to_ascii_lowercase())
    }

    /// Valor de una lectura: un secreto va con su valor, para el campo
    /// censurado con el ojo (el XML crudo, que se lee como texto, va
    /// enmascarado aparte).
    fn valor(&self, nombre: &str, valor: &str) -> Valor {
        if self.oculto(nombre, valor) {
            Valor::Clave(valor.to_owned())
        } else {
            Valor::Texto(valor.to_owned())
        }
    }
}

// ---- lecturas

enum ModoLectura {
    Xml,
    Pagina,
}

/// Única puerta de las lecturas: solo funciones de lectura del catálogo,
/// no bloqueadas, y nunca las páginas que disparan acciones.
fn modo_lectura(f: &Funcion) -> Result<ModoLectura, String> {
    if let Some(m) = &f.bloqueada {
        return Err(crate::t!("biblioteca-error-bloqueada", motivo = m.as_str()));
    }
    if PAGINAS_PROHIBIDAS.contains(&f.name.as_str()) {
        return Err(crate::t!("biblioteca-error-pagina-accion"));
    }
    if f.tipo != Tipo::Lectura {
        return Err(crate::t!(
            "biblioteca-error-no-lectura",
            nombre = f.name.as_str()
        ));
    }
    if f.name.starts_with("cb") {
        Ok(ModoLectura::Xml)
    } else if f.name.ends_with(".asp") {
        Ok(ModoLectura::Pagina)
    } else {
        Err(
            "Lectura de la web básica con una respuesta que el panel no sabe \
             interpretar (ni, por tanto, enmascarar): no se ejecuta desde el explorador."
                .into(),
        )
    }
}

/// Campos hoja de un elemento con su ruta (`a.b`).
type Campos<'a> = Vec<(String, &'a XmlNode)>;

/// Campos hoja de `n` con su ruta (`a.b`), en orden.
fn aplanar<'a>(n: &'a XmlNode, prefijo: &str, out: &mut Campos<'a>) {
    let mut vistos: BTreeMap<&str, usize> = BTreeMap::new();
    for c in &n.children {
        if c.tag == "Result" || c.tag == "ErrorString" {
            continue;
        }
        let i = vistos.entry(c.tag.as_str()).or_default();
        let nombre = if *i == 0 {
            format!("{prefijo}{}", c.tag)
        } else {
            format!("{prefijo}{}[{i}]", c.tag)
        };
        *i += 1;
        if c.children.is_empty() {
            out.push((nombre, c));
        } else {
            aplanar(c, &format!("{nombre}."), out);
        }
    }
}

/// El XML de `n`, sangrado, para leerlo en pantalla: con los secretos
/// enmascarados o, con `o = None`, tal cual (para «Mostrar claves»).
fn xml_legible(o: Option<&Ocultar>, n: &XmlNode, nivel: usize, out: &mut String) {
    let sangria = "  ".repeat(nivel);
    let escapar = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    if n.children.is_empty() {
        let texto = if o.is_some_and(|o| o.oculto(&n.tag, &n.text)) {
            MASCARA.to_owned()
        } else {
            escapar(&n.text)
        };
        out.push_str(&format!("{sangria}<{0}>{texto}</{0}>\n", n.tag));
    } else {
        out.push_str(&format!("{sangria}<{}>\n", n.tag));
        for c in &n.children {
            xml_legible(o, c, nivel + 1, out);
        }
        out.push_str(&format!("{sangria}</{}>\n", n.tag));
    }
}

/// Identificador de una fila: el valor de `id` si lo tiene, o `#n`.
fn id_fila(campos: &[(String, &XmlNode)], id: Option<&str>, n: usize) -> String {
    id.and_then(|id| campos.iter().find(|(k, _)| k == id))
        .map(|(_, v)| v.text.clone())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("#{n}"))
}

fn id_lista(f: &Funcion) -> Option<&str> {
    f.lista.as_ref().map(|l| l.id_param.as_str())
}

fn vista_xml(o: &Ocultar, f: &Funcion, n: &XmlNode) -> Vista {
    let mut pares = Vec::new();
    let mut grupos: Vec<(String, Vec<Campos<'_>>)> = Vec::new();
    for c in &n.children {
        if c.tag == "Result" || c.tag == "ErrorString" {
            continue;
        }
        if c.children.is_empty() {
            pares.push(Par {
                nombre: c.tag.clone(),
                valor: o.valor(&c.tag, &c.text),
            });
            continue;
        }
        let mut campos = Vec::new();
        aplanar(c, "", &mut campos);
        match grupos.iter_mut().find(|(t, _)| *t == c.tag) {
            Some((_, filas)) => filas.push(campos),
            None => grupos.push((c.tag.clone(), vec![campos])),
        }
    }
    let grupos = grupos
        .into_iter()
        .map(|(nombre, filas)| {
            let mut columnas: Vec<String> = Vec::new();
            for fila in &filas {
                for (k, _) in fila {
                    if !columnas.contains(k) {
                        columnas.push(k.clone());
                    }
                }
            }
            let total = filas.len();
            let filas = filas
                .iter()
                .take(MAX_INSTANCIAS)
                .enumerate()
                .map(|(i, fila)| Fila {
                    id: id_fila(fila, id_lista(f), i),
                    valores: columnas
                        .iter()
                        .map(|c| match fila.iter().find(|(k, _)| k == c) {
                            Some((_, v)) => o.valor(&v.tag, &v.text),
                            None => Valor::Ausente,
                        })
                        .collect(),
                })
                .collect();
            Grupo {
                nombre,
                columnas,
                filas,
                total,
            }
        })
        .collect();
    let mut crudo = String::new();
    xml_legible(Some(o), n, 0, &mut crudo);
    let mut con_claves = String::new();
    xml_legible(None, n, 0, &mut con_claves);
    Vista {
        pares,
        grupos,
        // Solo si hay algo censurado: es lo que muestra «Mostrar claves».
        crudo_con_claves: (con_claves != crudo).then_some(con_claves),
        crudo: Some(crudo),
    }
}

/// Respuesta en claro de una lectura (solo en memoria).
enum Respuesta {
    Xml(XmlNode),
    Pagina(BTreeMap<String, String>),
}

async fn ejecutar(
    state: &AppState,
    f: &Funcion,
    params: &[(String, String)],
) -> Result<Respuesta, ServerFnError> {
    let modo = modo_lectura(f).map_err(|m| con_estado(StatusCode::FORBIDDEN, m))?;
    for (k, v) in params {
        if f.parametro(k).is_none() {
            return Err(invalida(crate::t!(
                "biblioteca-error-param-desconocido",
                funcion = f.name.as_str(),
                param = k.as_str()
            )));
        }
        if v.chars().any(char::is_control) {
            return Err(invalida(format!(
                "{k}: {}",
                crate::t!("biblioteca-validar-control")
            )));
        }
    }
    let c = &state.hgu;
    let sin_router = |e: hgu::Error| {
        con_estado(
            StatusCode::BAD_GATEWAY,
            crate::t!("biblioteca-error-lectura-router", error = e.to_string()),
        )
    };
    match modo {
        ModoLectura::Xml => {
            let p: Vec<(&str, &str)> = params
                .iter()
                .filter(|(_, v)| !v.trim().is_empty())
                .map(|(k, v)| (k.as_str(), v.trim()))
                .collect();
            c.call(&f.name, &p)
                .await
                .map(Respuesta::Xml)
                .map_err(sin_router)
        }
        ModoLectura::Pagina => {
            let mut vars = hgu::fmt::page_vars(&c.basic_page(&f.name).await.map_err(sin_router)?);
            vars.remove("sessionKey");
            Ok(Respuesta::Pagina(vars))
        }
    }
}

pub async fn leer(
    state: &AppState,
    nombre: &str,
    params: &[(String, String)],
) -> Result<Vista, ServerFnError> {
    let f = funcion(state, nombre)?;
    let o = Ocultar::new(state);
    Ok(match ejecutar(state, f, params).await? {
        Respuesta::Xml(n) => vista_xml(&o, f, &n),
        Respuesta::Pagina(vars) => Vista {
            pares: vars
                .iter()
                .map(|(k, v)| Par {
                    nombre: k.clone(),
                    valor: o.valor(k, v),
                })
                .collect(),
            grupos: Vec::new(),
            crudo: None,
            crudo_con_claves: None,
        },
    })
}

// ---- formularios

fn mapa(pares: Vec<(String, String)>) -> BTreeMap<String, String> {
    pares.into_iter().collect()
}

/// Claves de instancia aceptables para `f` (sin el id de la lista).
fn claves_validas(
    f: &Funcion,
    claves: Vec<(String, String)>,
) -> Result<BTreeMap<String, String>, ServerFnError> {
    let ident = identidad(f);
    let id = id_lista(f);
    for (k, v) in &claves {
        if !ident.contains(k) || Some(k.as_str()) == id {
            return Err(invalida(crate::t!(
                "biblioteca-error-no-identifica",
                param = k.as_str(),
                funcion = f.name.as_str()
            )));
        }
        if let Some(p) = f.parametro(k) {
            validar_campo(tipo_campo(p.tipo), &p.opciones, v)
                .map_err(|e| invalida(format!("{}: {e}", p.etiqueta)))?;
        }
    }
    Ok(mapa(claves))
}

pub async fn formulario(
    state: &AppState,
    nombre: &str,
    instancia: Option<String>,
    claves: Vec<(String, String)>,
) -> Result<FormVista, ServerFnError> {
    let f = funcion(state, nombre)?;
    if f.tipo == Tipo::Lectura {
        return Err(invalida(crate::t!(
            "biblioteca-error-lectura-sin-formulario",
            nombre = nombre
        )));
    }
    let g = gestor(state)?;
    g.escribible(nombre).map_err(error_cambio)?;
    let claves = claves_validas(f, claves)?;
    let operacion = match (f.tipo, &f.lista, &instancia) {
        (Tipo::Accion, _, _) => Operacion::Accion,
        (_, Some(_), None) => Operacion::Crear,
        _ => Operacion::Editar,
    };
    let o = Ocultar::new(state);
    // Qué campos se pueden rellenar: en una edición, los que muestra la
    // lectura pareada (el gestor rechaza cambiar uno que no muestra); al
    // crear, los que muestran las instancias que ya hay.
    let actual = match operacion {
        Operacion::Editar => g
            .valores_actuales(nombre, instancia.as_deref(), &claves)
            .await
            .map_err(error_cambio)?
            .unwrap_or_default(),
        Operacion::Crear => {
            let filas = g.instancias(nombre, &claves).await.map_err(error_cambio)?;
            filas
                .into_iter()
                .flatten()
                .map(|(k, _)| (k, String::new()))
                .collect()
        }
        Operacion::Accion => BTreeMap::new(),
    };
    let ident = identidad(f);
    let mut campos = Vec::new();
    let mut omitidos = Vec::new();
    for p in &f.parametros {
        if ident.contains(&p.nombre) && operacion != Operacion::Accion {
            continue;
        }
        let cur = buscar(&actual, &p.nombre);
        // Al crear o ejecutar, un sí/no empieza en «no»: lo que se ve
        // (interruptor apagado) es lo que se envía.
        let nuevo = || {
            if p.tipo == TipoParam::Bool {
                "0".to_owned()
            } else {
                String::new()
            }
        };
        let mut sin_leer = false;
        let valor = match (operacion, cur) {
            (Operacion::Accion, _) => nuevo(),
            (Operacion::Crear, Some(_)) => nuevo(),
            (Operacion::Crear, None) if actual.is_empty() => nuevo(),
            // Un secreto, con su valor para el campo censurado con el ojo
            // (la respuesta va con `no-store`); el navegador lo devuelve
            // como MASCARA si no se toca. Si el router no deja leerlo, vacío.
            (Operacion::Editar, Some(v)) if p.tipo == TipoParam::Secret => {
                sin_leer =
                    crate::changes::plan::enmascarado(v) || (p.tapado && v.trim().is_empty());
                if sin_leer {
                    String::new()
                } else {
                    v.to_owned()
                }
            }
            (Operacion::Editar, Some(v)) if o.oculto(&p.nombre, v) => MASCARA.to_owned(),
            (Operacion::Editar, Some(v)) => v.to_owned(),
            (_, None) => {
                omitidos.push(p.nombre.clone());
                continue;
            }
        };
        campos.push(CampoForm {
            param: param_vista(f, p),
            valor,
            sin_leer,
        });
    }
    Ok(FormVista {
        funcion: f.name.clone(),
        operacion,
        instancia,
        claves: claves.into_iter().collect(),
        campos,
        omitidos,
    })
}

pub async fn instancias(
    state: &AppState,
    nombre: &str,
    claves: Vec<(String, String)>,
) -> Result<ListaInstancias, ServerFnError> {
    let f = funcion(state, nombre)?;
    let g = gestor(state)?;
    g.escribible(nombre).map_err(error_cambio)?;
    let claves = claves_validas(f, claves)?;
    let filas = g.instancias(nombre, &claves).await.map_err(error_cambio)?;
    let o = Ocultar::new(state);
    let mut columnas: Vec<String> = Vec::new();
    for fila in &filas {
        for k in fila.keys() {
            if !columnas.contains(k) {
                columnas.push(k.clone());
            }
        }
    }
    // El id primero.
    if let Some(id) = id_lista(f)
        && let Some(i) = columnas.iter().position(|c| c == id)
    {
        let c = columnas.remove(i);
        columnas.insert(0, c);
    }
    let total = filas.len();
    Ok(ListaInstancias {
        filas: filas
            .iter()
            .take(MAX_INSTANCIAS)
            .enumerate()
            .map(|(i, fila)| Fila {
                id: id_lista(f)
                    .and_then(|id| buscar(fila, id))
                    .filter(|v| !v.is_empty())
                    .map_or_else(|| format!("#{i}"), str::to_owned),
                valores: columnas
                    .iter()
                    .map(|c| fila.get(c).map_or(Valor::Ausente, |v| o.valor(c, v)))
                    .collect(),
            })
            .collect(),
        columnas,
        total,
    })
}

// ---- cambios

/// Valor del diff para mostrar: opciones y sí/no con su etiqueta.
fn valor_diff(p: Option<&catalogo::Parametro>, v: String) -> Valor {
    if v.is_empty() {
        return Valor::Ausente;
    }
    match p.map(|p| (p.tipo, p)) {
        Some((TipoParam::Enum, p)) => Valor::Opcion {
            etiqueta: p
                .opciones
                .iter()
                .find(|(o, _)| *o == v)
                .map(|(_, e)| e.clone()),
            valor: v,
        },
        Some((TipoParam::Bool, _)) => Valor::Opcion {
            etiqueta: match v.as_str() {
                "1" => Some("Sí".into()),
                "0" => Some("No".into()),
                _ => None,
            },
            valor: v,
        },
        _ => Valor::Texto(v),
    }
}

/// Campo del diff del gestor para la interfaz. Secreto si lo dice el gestor
/// o si la vista de lectura lo enmascararía (nombre o valor).
///
/// `f` es `None` si la función ya no está en el catálogo (un cambio antiguo
/// del historial): sin etiquetas, y los valores como texto.
pub(crate) fn campo_cambio(
    f: Option<&Funcion>,
    o: &Ocultar,
    c: crate::changes::CampoDiff,
) -> CampoCambio {
    let secreto = c.secreto || o.oculto(&c.nombre, &c.antes) || o.oculto(&c.nombre, &c.despues);
    let p = f.and_then(|f| f.parametro(&c.nombre));
    let valor = |v: String| {
        if secreto {
            Valor::Secreto
        } else {
            valor_diff(p, v)
        }
    };
    CampoCambio {
        etiqueta: p.map_or_else(|| c.nombre.clone(), |p| p.etiqueta.clone()),
        antes: valor(c.antes),
        despues: valor(c.despues),
        nombre: c.nombre,
    }
}

pub async fn preparar(state: &AppState, p: Peticion) -> Result<PreparadoVista, ServerFnError> {
    let f = funcion(state, &p.funcion)?;
    let g = gestor(state)?;
    // Por el nombre, antes que nada: una función bloqueada no se escribe
    // desde la Biblioteca ni con la carga que el gestor admite a la regla
    // «sin ping» del cortafuegos (esa solo llega desde Seguridad).
    g.escribible(&p.funcion).map_err(error_cambio)?;
    // Validación de tipo de los valores nuevos (el gestor vuelve a validar
    // lo que cambia): un MASCARA o un secreto vacío es "sin tocar".
    for (k, v) in &p.valores {
        let Some(param) = f.parametro(k) else {
            return Err(invalida(crate::t!(
                "biblioteca-error-param-desconocido",
                funcion = f.name.as_str(),
                param = k.as_str()
            )));
        };
        let sin_tocar = v == MASCARA || (param.tipo == TipoParam::Secret && v.is_empty());
        if !sin_tocar && !v.trim().is_empty() {
            validar_campo(tipo_campo(param.tipo), &param.opciones, v)
                .map_err(|e| invalida(format!("{}: {e}", param.etiqueta)))?;
        }
    }
    let req = ChangeRequest {
        funcion: p.funcion.clone(),
        instancia: p.instancia.clone(),
        valores: mapa(p.valores),
        borrar: p.borrar,
        identidad: Default::default(),
    };
    let prep = g.preparar(req).await.map_err(error_cambio)?;
    Ok(vista_preparado(state, &g, prep))
}

/// Cambio preparado del gestor para la interfaz (secretos enmascarados,
/// opciones con etiqueta).
pub(crate) fn vista_preparado(
    state: &AppState,
    g: &ChangeManager,
    prep: crate::changes::Preparado,
) -> PreparadoVista {
    let o = Ocultar::new(state);
    // Cada campo con las etiquetas de la función de su paso (deshacer un
    // grupo junta varios).
    let campos = prep
        .pasos
        .into_iter()
        .flat_map(|p| {
            let f = state.catalogo.funcion(&p.funcion);
            p.diff
                .campos
                .into_iter()
                .map(|c| campo_cambio(f, &o, c))
                .collect::<Vec<_>>()
        })
        .collect();
    PreparadoVista {
        token: prep.token,
        funcion: prep.funcion,
        instancia: prep.instancia,
        borrar: prep.borrar,
        campos,
        riesgo: riesgo(prep.riesgo),
        vigilancia_s: g.tiempos().plazo(prep.riesgo, prep.vigila).as_secs(),
        palabra: prep.palabra,
        aviso: None,
    }
}

pub async fn confirmar(
    state: &AppState,
    token: &str,
    palabra: Option<&str>,
    usuario: &str,
) -> Result<i64, ServerFnError> {
    let g = gestor(state)?;
    let palabra = palabra.map(str::trim).filter(|p| !p.is_empty());
    g.confirmar(token, palabra, usuario)
        .await
        .map_err(error_cambio)
}

pub(crate) fn estado(e: Estado) -> EstadoDeCambio {
    match e {
        Estado::Preparado => EstadoDeCambio::Preparado,
        Estado::Aplicando => EstadoDeCambio::Aplicando,
        Estado::Vigilando => EstadoDeCambio::Vigilando,
        Estado::Aplicado => EstadoDeCambio::Aplicado,
        Estado::Revertido => EstadoDeCambio::Revertido,
        Estado::Fallido => EstadoDeCambio::Fallido,
        Estado::RevertidoParcial => EstadoDeCambio::RevertidoParcial,
    }
}

/// El estado del cambio `id` para `usuario` (que es quien puede terminar su
/// vigilancia, si la hizo él).
pub async fn estado_cambio(
    state: &AppState,
    id: i64,
    usuario: &str,
) -> Result<CambioVista, ServerFnError> {
    let g = gestor(state)?;
    let c = g.cambio(id).await.ok_or_else(|| {
        con_estado(
            StatusCode::NOT_FOUND,
            crate::t!("biblioteca-error-no-existe-cambio", id = id),
        )
    })?;
    let restante_s = (c.estado == Estado::Vigilando).then(|| {
        let fin = c.updated + g.tiempos().vigilancia(c.riesgo).as_secs() as i64;
        (fin - crate::series::now()).max(0)
    });
    Ok(CambioVista {
        id: c.id,
        estado: estado(c.estado),
        error: c.error.as_deref().map(nota_legible),
        nota: c.nota.as_deref().map(nota_legible),
        restante_s,
        terminable: c.estado == Estado::Vigilando && g.terminable(id, usuario).await,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecturas_prohibidas_y_permitidas() {
        let c = catalogo::Catalogo::load().unwrap();
        for n in [
            "syslog.asp",
            "backup_restore.asp",
            "cbRestoreConfig",
            "cbReboot",
        ] {
            let f = c.funcion(n).unwrap();
            assert!(modo_lectura(f).is_err(), "{n}");
        }
        for n in ["te_ppp_pm.cmd?action=retrieve", "te_wifirefresh.wl"] {
            assert!(modo_lectura(c.funcion(n).unwrap()).is_err(), "{n}");
        }
        assert!(matches!(
            modo_lectura(c.funcion("te_wifi.asp").unwrap()),
            Ok(ModoLectura::Pagina)
        ));
        assert!(matches!(
            modo_lectura(c.funcion("cbGetSummary").unwrap()),
            Ok(ModoLectura::Xml)
        ));
        // Ninguna lectura del catálogo carga una página prohibida.
        for f in c.todas() {
            if PAGINAS_PROHIBIDAS.contains(&f.name.as_str()) {
                assert!(modo_lectura(f).is_err());
            }
        }
    }

    #[test]
    fn valores_iniciales_de_lectura() {
        let c = catalogo::Catalogo::load().unwrap();
        let f = c.funcion("cbDhcpV4SvrPoolGetStatic").unwrap();
        assert_eq!(inicial(f, "all"), "ALL");
        assert_eq!(inicial(f, "rtm_cfg_dhcpv4_svr_pool_id"), "0");
        // El id de una lista con `all=ALL` se deja vacío para listarlas.
        assert_eq!(inicial(f, "rtm_dhcpv4_svr_pool_static_id"), "");
        let w = c.funcion("cbGetWanIPIntf").unwrap();
        assert_eq!(inicial(w, "ip_intf_0i"), "0");
        let n = c.funcion("cbGetNatPortMapping").unwrap();
        assert_eq!(inicial(n, "dmz"), "");
        // Dos formas de dar la misma instancia: solo el id de la lista.
        let a = c.funcion("cbGetWifiAccPt").unwrap();
        assert_eq!(inicial(a, "dpm_cfg_wifi_acc_pt_id"), "0");
        assert_eq!(inicial(a, "dpm_cfg_wifi_acc_pt_0i"), "");
    }

    fn diff(nombre: &str, antes: &str, despues: &str) -> crate::changes::CampoDiff {
        crate::changes::CampoDiff {
            nombre: nombre.into(),
            antes: antes.into(),
            despues: despues.into(),
            secreto: false,
            sensible: false,
        }
    }

    #[test]
    fn diff_con_etiquetas_de_opciones() {
        let c = catalogo::Catalogo::load().unwrap();
        let o = Ocultar::de(Arc::new(catalogo::mask::Mascara::load().unwrap()), &c);
        let f = c.funcion("cbDhcpV4SvrPoolSetStatic").unwrap();
        let d = campo_cambio(Some(f), &o, diff("adm_state", "0", "1"));
        assert_eq!(d.etiqueta, "Activado");
        assert_eq!(
            d.antes,
            Valor::Opcion {
                valor: "0".into(),
                etiqueta: Some("Desactivado".into())
            }
        );
        assert_eq!(
            d.despues,
            Valor::Opcion {
                valor: "1".into(),
                etiqueta: Some("Activado".into())
            }
        );
        let d = campo_cambio(Some(f), &o, diff("adm_state", "7", ""));
        assert_eq!(
            d.antes,
            Valor::Opcion {
                valor: "7".into(),
                etiqueta: None
            }
        );
        assert_eq!(d.despues, Valor::Ausente);
        assert_eq!(Valor::texto_opcion("7", None), "(valor desconocido: 7)");
        assert_eq!(Valor::texto_opcion("1", Some("Sí")), "Sí (1)");
        let w = c.funcion("cbSetWifiAccPt").unwrap();
        let d = campo_cambio(Some(w), &o, diff("ssid_hide", "0", "1"));
        assert_eq!(
            d.despues,
            Valor::Opcion {
                valor: "1".into(),
                etiqueta: Some("Sí".into())
            }
        );
        let d = campo_cambio(Some(w), &o, diff("ssid", "A", "B"));
        assert_eq!(d.despues, Valor::Texto("B".into()));
    }

    #[test]
    fn diff_enmascara_lo_que_la_lectura_enmascara() {
        let c = catalogo::Catalogo::load().unwrap();
        let o = Ocultar::de(Arc::new(catalogo::mask::Mascara::load().unwrap()), &c);
        let w = c.funcion("cbSetWifiAccPt").unwrap();
        // Nombre sensible con sufijo numérico y valor de texto: el gestor no
        // lo marca, pero la lectura lo enmascara.
        let d = campo_cambio(Some(w), &o, diff("wep_key_idx", "1", "clave"));
        assert!(o.oculto("wep_key_idx", "clave"));
        assert_eq!((d.antes, d.despues), (Valor::Secreto, Valor::Secreto));
        // Parámetro secreto del catálogo aunque el gestor no lo marque.
        let d = campo_cambio(Some(w), &o, diff("key_phrase", "a", "b"));
        assert_eq!(d.despues, Valor::Secreto);
        let d = campo_cambio(Some(w), &o, diff("wep_key_idx", "1", "2"));
        assert!(matches!(d.despues, Valor::Opcion { .. }));
    }

    #[test]
    fn aplana_y_enmascara_anidados() {
        let n = hgu::xml::parse_response(
            "cbX",
            "<Response><cbX><a>1</a><e><id>3</id><key_phrase>s3cr3t</key_phrase>\
             <sub><password>p</password><x>y</x></sub><sub><x>z</x></sub></e>\
             <Result>0</Result></cbX></Response>",
        )
        .unwrap();
        let mut campos = Vec::new();
        aplanar(n.get("e").unwrap(), "", &mut campos);
        let nombres: Vec<_> = campos.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            nombres,
            ["id", "key_phrase", "sub.password", "sub.x", "sub[1].x"]
        );
    }
}
