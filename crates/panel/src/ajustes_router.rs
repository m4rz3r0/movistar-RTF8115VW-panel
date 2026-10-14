//! Ajustes del router por tema: lectura con valores legibles
//! ([`leer_tema`]) y escritura de un ajuste como un grupo de cambios del
//! gestor ([`preparar_ajuste`], [`confirmar_ajuste`], [`deshacer_ajuste`]),
//! con un resumen legible «antes → después» ([`GrupoVista`]).
//!
//! Los tipos de aquí los usan las páginas de Ajustes en el navegador. Solo
//! llevan ids, claves de traducción y valores legibles: ningún nombre de la
//! API salvo en [`DetallesTecnicos`], que la interfaz pinta únicamente dentro
//! de `<details class="detalles-tecnicos">`. Un secreto llega como
//! [`ValorLegible::Secreto`]: su valor nunca sale del servidor.

use crate::ui::explorador::RiesgoCambio;
use ajustes::{Nivel, TipoCampo, ValorLegible};
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Los ajustes de un tema, leídos del router.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemaVista {
    /// Identificador corto del tema (`wifi`).
    pub tema: String,
    /// El panel está en modo solo lectura (firmware sin confirmar): los
    /// ajustes se ven pero no se cambian.
    pub solo_lectura: bool,
    /// En el orden del catálogo.
    pub ajustes: Vec<AjusteVista>,
}

/// Un ajuste con su valor actual.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AjusteVista {
    /// `<tema>.<nombre>` (`wifi.red_24`).
    pub id: String,
    /// Prefijo de sus claves de traducción (`ajuste-wifi-red_24`): con
    /// `-titulo`, `-desc` y `-ayuda`.
    pub clave: String,
    pub nivel: Nivel,
    /// El mayor de las funciones con que se escribe (bajo si no escribe).
    pub riesgo: RiesgoCambio,
    /// Clave de traducción del efecto al aplicarlo (`efecto-wifi_red`).
    pub efecto: Option<String>,
    /// Es una lista: sus valores están en [`AjusteVista::instancias`] y
    /// [`AjusteVista::campos`] es la plantilla de una instancia nueva.
    pub lista: bool,
    /// Componente propio de la interfaz (`reservas`), si no usa la tarjeta
    /// genérica.
    pub componente: Option<String>,
    pub estado: EstadoLectura,
    /// Campos con su valor. En una lista, la plantilla de una instancia
    /// nueva (valor vacío). Vacío si no se pudo leer.
    pub campos: Vec<CampoVista>,
    /// Instancias de una lista, en el orden del router.
    pub instancias: Vec<InstanciaVista>,
    pub tecnico: DetallesTecnicos,
}

/// Si se pudo leer el valor actual de un ajuste.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoLectura {
    Leido,
    /// No se pudo leer: el motivo, traducido y sin secretos.
    SinLeer(String),
}

/// Un campo con su valor actual y lo que la página necesita para pintarlo y
/// validarlo sin el catálogo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampoVista {
    pub id: String,
    /// Prefijo de sus claves de traducción (`ajuste-wifi-red_24-seguridad`):
    /// sola es la etiqueta; con `-desc`, su descripción; con `-<opción>` y
    /// `-<opción>-desc`, las de cada opción.
    pub clave: String,
    pub tipo: TipoCampo,
    pub valor: ValorLegible,
    /// Opciones de un campo `opcion`, en el orden del catálogo.
    pub opciones: Vec<OpcionVista>,
    /// Límites: en bytes UTF-8 para textos y secretos, en valor para números.
    pub min: Option<i64>,
    pub max: Option<i64>,
    /// Solo admite ASCII imprimible (la contraseña de una red WiFi).
    pub ascii: bool,
    /// Se puede dejar vacío.
    pub opcional: bool,
    /// Clave de traducción del motivo por el que no se puede editar.
    pub solo_lectura: Option<String>,
    /// Valor legible recomendado: la clave de la opción recomendada en un
    /// campo `opcion` (`wpa2`), `true`/`false` en un booleano o el texto.
    pub recomendado: Option<String>,
    /// Clave de traducción del porqué de la recomendación (para quién es, a
    /// favor y en contra): `<clave>-recomendado`. Solo si hay recomendación.
    pub motivo_recomendado: Option<String>,
}

/// Una opción de un campo `opcion`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpcionVista {
    /// Clave legible (`wpa2`): el valor que se envía al guardar y el final de
    /// sus claves de traducción.
    pub clave: String,
    pub recomendada: bool,
    /// Clave de traducción del aviso al elegirla (`aviso-wifi_abierta`).
    pub aviso: Option<String>,
}

/// Una instancia de una lista (una regla, una reserva…).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanciaVista {
    /// Id de la instancia en el router (su posición si el router no da uno).
    pub id: String,
    pub campos: Vec<CampoVista>,
}

/// Funciones y parámetros del router detrás de un ajuste. Lo único con
/// nombres de la API: solo se pinta dentro de los detalles técnicos plegados.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetallesTecnicos {
    /// Función de lectura con sus parámetros.
    pub lectura: String,
    /// Funciones de escritura, sin repetir.
    pub escrituras: Vec<String>,
    /// Pares campo → parámetro (el que escribe o, si solo se lee, el
    /// elemento de la lectura; después, los que fija cada opción).
    pub parametros: Vec<(String, String)>,
}

/// Lo que la página pide guardar de un ajuste.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeticionAjuste {
    /// Id del ajuste (`wifi.red_24`).
    pub ajuste: String,
    /// Id de la instancia que se edita o borra en una lista; `None` crea
    /// una nueva. No cuenta en un ajuste que no es lista.
    pub instancia: Option<String>,
    /// Borra la instancia (solo en una lista).
    pub borrar: bool,
    /// Campos que se cambian: id del campo → valor legible (la clave de una
    /// opción, `true`/`false`, el texto). Un secreto sin tocar no va.
    pub valores: BTreeMap<String, String>,
}

/// Resumen de un ajuste preparado para guardarlo: lo que cambia en cada paso
/// y lo que hace falta para confirmarlo. Sin nombres de la API y con los
/// secretos como [`MASCARA`](crate::ui::explorador::MASCARA).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrupoVista {
    /// Token para [`confirmar_ajuste`] (un solo uso, 10 min).
    pub token: String,
    /// Clave de traducción del título del ajuste (`ajuste-wifi-red_24-titulo`).
    pub titulo: String,
    /// Un paso por función de escritura, en orden.
    pub pasos: Vec<PasoVista>,
    /// El mayor de los pasos con cambios: marca la vigilancia.
    pub riesgo: RiesgoCambio,
    /// Clave de traducción del efecto al aplicarlo (`efecto-wifi_red`).
    pub efecto: Option<String>,
    /// Segundos que se vigila el router tras aplicarlo.
    pub vigilancia_s: u64,
}

/// Un paso del grupo: los campos de una misma escritura del router.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasoVista {
    /// Posición en el grupo (desde 0): la que se desmarca al confirmar.
    pub indice: usize,
    /// El router ya está así: no se escribirá.
    pub sin_cambios: bool,
    pub campos: Vec<CambioLegible>,
}

/// Un campo que cambia, con sus valores legibles. Si es `secreto`, `antes`
/// y `despues` valen [`MASCARA`](crate::ui::explorador::MASCARA).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CambioLegible {
    /// Clave de traducción del campo (`ajuste-wifi-red_24-nombre`).
    pub campo: String,
    /// Valor legible: la clave de una opción (con `campo` da su texto,
    /// `<campo>-<opción>`), `true`/`false` en un sí/no o el texto tal cual.
    /// Un valor que el panel no reconoce va tal cual lo da el router.
    pub antes: String,
    pub despues: String,
    pub secreto: bool,
    /// Tipo del campo: dice cómo se pintan `antes` y `despues`.
    pub tipo: TipoCampo,
}

/// Prepara el guardado de un ajuste: lee el router, calcula lo que cambia
/// en cada paso y devuelve el resumen con su token. Errores con el mensaje
/// traducido: 409 si ya está así o hay otro cambio en curso, 403 en solo
/// lectura, 400 si un valor no vale. Exige sesión.
#[server(endpoint = "preparar_ajuste", input = Json)]
pub async fn preparar_ajuste(p: PeticionAjuste) -> Result<GrupoVista, ServerFnError> {
    servidor::preparar_ajuste(p).await
}

/// Aplica el ajuste preparado con `token` sin los pasos de `desmarcados`
/// (índices). Devuelve el id del cambio para seguirlo con
/// [`estado_cambio`](crate::ui::explorador::estado_cambio).
#[server(endpoint = "confirmar_ajuste", input = Json)]
pub async fn confirmar_ajuste(
    token: String,
    desmarcados: Vec<usize>,
) -> Result<i64, ServerFnError> {
    servidor::confirmar_ajuste(token, desmarcados).await
}

/// Prepara el cambio que deshace `cambio` (uno aplicado desde Ajustes) y
/// devuelve su resumen, que se confirma con [`confirmar_ajuste`].
#[server(endpoint = "deshacer_ajuste", input = Json)]
pub async fn deshacer_ajuste(cambio: i64) -> Result<GrupoVista, ServerFnError> {
    servidor::deshacer_ajuste(cambio).await
}

/// Los ajustes del tema `tema` (`wifi`, `puertos`…) con su valor actual
/// del router. Cada lectura distinta se hace una sola vez; si una falla,
/// solo sus ajustes quedan [`EstadoLectura::SinLeer`]. 404 si el tema no
/// existe. Exige sesión; la respuesta va con `no-store`.
#[server(endpoint = "leer_tema", input = Json)]
pub async fn leer_tema(tema: String) -> Result<TemaVista, ServerFnError> {
    servidor::leer_tema(tema).await
}

#[cfg(feature = "ssr")]
pub(crate) mod servidor {
    use super::*;
    use crate::{
        changes::{
            BLOQUEO_SOLO_LECTURA, CambioError, ChangeManager, ChangeRequest, Diff, GrupoRequest,
            Paso, PasoPreparado, buscar,
        },
        instalar::servidor::fallo,
        instancia,
        panel_ajustes::servidor::preparar,
        state::AppState,
        t,
        ui::explorador::{
            MASCARA,
            servidor::{error_cambio, gestor, riesgo},
        },
    };
    use ajustes::{Ajuste, Campo, ErrorValor, Tema, a_legible, a_router, prefijo_i18n};
    use axum::http::StatusCode;
    use catalogo::{Catalogo, Riesgo};
    use hgu::xml::XmlNode;
    use std::collections::HashMap;

    /// Elementos de una lectura: nombre → valor.
    type Leidos = BTreeMap<String, String>;
    /// Una lectura del router: función y parámetros.
    type Lectura = (String, Vec<(String, String)>);

    pub(super) async fn leer_tema(tema: String) -> Result<TemaVista, ServerFnError> {
        let (state, _) = preparar().await?;
        let tema = Tema::from_slug(&tema)
            .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-tema")))?;
        let mut lecturas: HashMap<Lectura, Option<XmlNode>> = HashMap::new();
        let mut ajustes = Vec::new();
        for a in state.ajustes.por_tema(tema) {
            let clave = (a.leer.funcion.clone(), a.leer.params.clone());
            if !lecturas.contains_key(&clave) {
                let nodo = leer(&state, a).await;
                lecturas.insert(clave.clone(), nodo);
            }
            ajustes.push(vista(&state.catalogo, a, lecturas[&clave].as_ref()));
        }
        Ok(TemaVista {
            tema: tema.slug().to_owned(),
            solo_lectura: instancia::solo_lectura(&state.db).await,
            ajustes,
        })
    }

    /// Respuesta de la lectura del ajuste; `None` (y se registra) si falla.
    async fn leer(state: &AppState, a: &Ajuste) -> Option<XmlNode> {
        let params: Vec<(&str, &str)> = a
            .leer
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        state
            .hgu
            .call(&a.leer.funcion, &params)
            .await
            .inspect_err(|e| {
                tracing::warn!(
                    "ajustes: no se puede leer {} ({}): {e}",
                    a.leer.funcion,
                    a.id
                );
            })
            .ok()
    }

    /// Vista de un ajuste con la respuesta de su lectura (`None` si falló).
    fn vista(f: &Catalogo, a: &Ajuste, nodo: Option<&XmlNode>) -> AjusteVista {
        let prefijo = prefijo_i18n(a);
        let sin_leer = || {
            (
                EstadoLectura::SinLeer(t!("ajustes-error-lectura")),
                Vec::new(),
                Vec::new(),
            )
        };
        let (estado, campos, instancias) = match nodo {
            None => sin_leer(),
            Some(n) if a.lista => match lista(f, a, &prefijo, n) {
                Some(instancias) => {
                    let plantilla = a
                        .campos
                        .iter()
                        .map(|c| campo(&prefijo, c, valor_inicial(c)))
                        .collect();
                    (EstadoLectura::Leido, plantilla, instancias)
                }
                None => {
                    tracing::warn!(
                        "ajustes: {} ({}) trae instancias sin su id: no se ofrecen",
                        a.id,
                        a.leer.funcion
                    );
                    sin_leer()
                }
            },
            Some(n) => {
                let mut leidos = Leidos::new();
                hojas(n, &mut leidos);
                let campos = a
                    .campos
                    .iter()
                    .map(|c| campo(&prefijo, c, a_legible(c, &leidos)))
                    .collect();
                (EstadoLectura::Leido, campos, Vec::new())
            }
        };
        AjusteVista {
            id: a.id.clone(),
            nivel: a.nivel,
            riesgo: riesgo(ajustes::riesgo(a, f)),
            efecto: a.efecto.as_ref().map(|e| format!("efecto-{e}")),
            lista: a.lista,
            componente: a.componente.map(|c| c.slug().to_owned()),
            estado,
            campos,
            instancias,
            tecnico: tecnico(a),
            clave: prefijo,
        }
    }

    /// Instancias de una lista con sus valores. El id de cada una es el del
    /// router (el elemento de [`id_instancia`]); solo en una lista que no se
    /// escribe (como los usuarios del router) cae a su posición. En una que
    /// se escribe, `None` si alguna instancia no trae el suyo: la Tarea 7
    /// escribe por ese id, y una posición tomada por id editaría o borraría
    /// otra instancia.
    fn lista(f: &Catalogo, a: &Ajuste, prefijo: &str, n: &XmlNode) -> Option<Vec<InstanciaVista>> {
        let se_escribe = a.campos.iter().any(|c| c.escribible().is_some());
        let id_param = id_instancia(f, a);
        let mut elementos: Vec<&str> = a
            .campos
            .iter()
            .flat_map(|c| std::iter::once(&c.lee).chain(c.ademas.iter().map(|ad| &ad.lee)))
            .map(String::as_str)
            .collect();
        elementos.extend(id_param);
        instancias(n, &elementos)
            .into_iter()
            .enumerate()
            .map(|(i, leidos)| {
                let id = match id_param
                    .and_then(|p| leidos.get(p))
                    .filter(|v| !v.is_empty())
                {
                    Some(id) => id.clone(),
                    None if !se_escribe => i.to_string(),
                    None => return None,
                };
                let campos = a
                    .campos
                    .iter()
                    .map(|c| campo(prefijo, c, a_legible(c, &leidos)))
                    .collect();
                Some(InstanciaVista { id, campos })
            })
            .collect()
    }

    /// Valor con que empieza un campo en la plantilla de una instancia
    /// nueva: la opción recomendada o la primera; en un sí/no, el
    /// recomendado o no; en el resto, el recomendado o vacío. Nunca
    /// [`ValorLegible::Desconocido`] (no hay valor del router) ni
    /// [`ValorLegible::Secreto`] (no hay nada oculto).
    fn valor_inicial(c: &Campo) -> ValorLegible {
        match c.tipo {
            TipoCampo::Opcion => c
                .opciones
                .iter()
                .find(|o| o.recomendada)
                .or_else(|| c.opciones.first())
                .map_or_else(
                    || ValorLegible::Texto(String::new()),
                    |o| ValorLegible::Opcion(o.clave.clone()),
                ),
            TipoCampo::Booleano => ValorLegible::Booleano(c.recomendado.as_deref() == Some("true")),
            _ => ValorLegible::Texto(c.recomendado.clone().unwrap_or_default()),
        }
    }

    /// Vista de un campo con su valor. Las claves de traducción siguen las
    /// de `ajustes::claves_requeridas`.
    fn campo(prefijo: &str, c: &Campo, valor: ValorLegible) -> CampoVista {
        let clave = format!("{prefijo}-{}", c.id);
        let recomendado = c
            .opciones
            .iter()
            .find(|o| o.recomendada)
            .map(|o| o.clave.clone())
            .or_else(|| c.recomendado.clone());
        CampoVista {
            id: c.id.clone(),
            tipo: c.tipo,
            valor,
            opciones: c
                .opciones
                .iter()
                .map(|o| OpcionVista {
                    clave: o.clave.clone(),
                    recomendada: o.recomendada,
                    aviso: o.aviso.as_ref().map(|av| format!("aviso-{av}")),
                })
                .collect(),
            min: c.validar.and_then(|v| v.min),
            max: c.validar.and_then(|v| v.max),
            ascii: c.validar.is_some_and(|v| v.ascii),
            opcional: c.opcional,
            solo_lectura: c.solo_lectura.clone(),
            motivo_recomendado: recomendado.as_ref().map(|_| format!("{clave}-recomendado")),
            recomendado,
            clave,
        }
    }

    /// Elementos hoja de `n` a cualquier profundidad; si uno se repite, vale
    /// el primero.
    fn hojas(n: &XmlNode, out: &mut Leidos) {
        for c in &n.children {
            if c.tag == "Result" || c.tag == "ErrorString" {
                continue;
            }
            if c.children.is_empty() {
                out.entry(c.tag.clone()).or_insert_with(|| c.text.clone());
            } else {
                hojas(c, out);
            }
        }
    }

    /// Elementos de cada instancia de una lista, en orden. Normalmente cada
    /// instancia es un elemento con hijos. Si la respuesta trae los valores
    /// seguidos sin elemento que los agrupe (como los pares nombre/papel de
    /// los usuarios del router), solo cuentan las hojas de `elementos` (los
    /// que leen los campos y el id) y empieza una instancia cada vez que se
    /// repite una. Supone que el router da los elementos de cada instancia
    /// juntos y antes que los de la siguiente; si a una le faltase alguno,
    /// ese campo se vería vacío o desconocido, sin mezclarse con otra.
    fn instancias(n: &XmlNode, elementos: &[&str]) -> Vec<Leidos> {
        let grupos: Vec<&XmlNode> = n
            .children
            .iter()
            .filter(|c| !c.children.is_empty())
            .collect();
        if !grupos.is_empty() {
            return grupos
                .into_iter()
                .map(|g| {
                    let mut leidos = Leidos::new();
                    hojas(g, &mut leidos);
                    leidos
                })
                .collect();
        }
        let mut out = Vec::new();
        let mut actual = Leidos::new();
        for c in &n.children {
            let es_suyo =
                c.children.is_empty() && elementos.iter().any(|e| e.eq_ignore_ascii_case(&c.tag));
            if !es_suyo {
                continue;
            }
            if actual.contains_key(&c.tag) {
                out.push(std::mem::take(&mut actual));
            }
            actual.insert(c.tag.clone(), c.text.clone());
        }
        if !actual.is_empty() {
            out.push(actual);
        }
        out
    }

    /// Elemento que da el id de cada instancia de una lista: el parámetro de
    /// instancia de su función de escritura o, si no escribe, el de su
    /// lectura.
    fn id_instancia<'f>(f: &'f Catalogo, a: &Ajuste) -> Option<&'f str> {
        a.campos
            .iter()
            .filter_map(|c| c.escribe.as_ref())
            .map(|e| e.funcion.as_str())
            .chain(std::iter::once(a.leer.funcion.as_str()))
            .filter_map(|nombre| f.funcion(nombre))
            .find_map(|func| func.lista.as_ref())
            .map(|l| l.id_param.as_str())
    }

    fn tecnico(a: &Ajuste) -> DetallesTecnicos {
        let mut lectura = a.leer.funcion.clone();
        if !a.leer.params.is_empty() {
            let params: Vec<String> = a
                .leer
                .params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            lectura = format!("{lectura} ({})", params.join(", "));
        }
        let mut escrituras: Vec<String> = Vec::new();
        let mut parametros = Vec::new();
        for c in &a.campos {
            match &c.escribe {
                Some(e) => {
                    if !escrituras.contains(&e.funcion) {
                        escrituras.push(e.funcion.clone());
                    }
                    parametros.push((c.id.clone(), e.parametro.clone()));
                }
                None => parametros.push((c.id.clone(), c.lee.clone())),
            }
            parametros.extend(
                c.ademas
                    .iter()
                    .map(|ad| (c.id.clone(), ad.parametro.clone())),
            );
        }
        DetallesTecnicos {
            lectura,
            escrituras,
            parametros,
        }
    }

    // ------------------------------------------------------------ guardar

    /// Pares `(parámetro, valor)` que escribe el campo `c` con el legible
    /// `v`, validado; `None` si es un secreto sin tocar ([`MASCARA`]).
    fn pares(c: &Campo, v: &str) -> Result<Option<Vec<(String, String)>>, ErrorValor> {
        if c.tipo == TipoCampo::Secreto && v == MASCARA {
            return Ok(None);
        }
        a_router(c, v).map(Some)
    }

    /// Grupo de cambios que guarda `p` en el ajuste `a`: un paso por función
    /// de escritura de los campos tocados, en el orden de los campos (al
    /// borrar, de todos los campos que se escriben). Cada paso lleva los
    /// `fijos` del ajuste y los parámetros de sus campos tocados en formato
    /// del router; lo no tocado lo completa el gestor con lo que lea. Los
    /// ids de `p.valores` que no son campos del ajuste no cuentan (los
    /// rechaza quien llama). Sin campos tocados, el grupo no tiene pasos.
    pub(crate) fn grupo_de(a: &Ajuste, p: &PeticionAjuste) -> Result<GrupoRequest, ErrorValor> {
        let titulo = format!("{}-titulo", prefijo_i18n(a));
        let mut pasos: Vec<(String, BTreeMap<String, String>)> = Vec::new();
        for c in &a.campos {
            let tocados = match p.valores.get(&c.id) {
                Some(v) if !p.borrar => pares(c, v)?,
                _ => None,
            };
            let Some(e) = c.escribible() else {
                continue;
            };
            if tocados.is_none() && !p.borrar {
                continue;
            }
            let i = match pasos.iter().position(|(f, _)| *f == e.funcion) {
                Some(i) => i,
                None => {
                    pasos.push((e.funcion.clone(), a.fijos.clone()));
                    pasos.len() - 1
                }
            };
            pasos[i].1.extend(tocados.unwrap_or_default());
        }
        Ok(GrupoRequest {
            pasos: pasos
                .into_iter()
                .map(|(funcion, valores)| Paso {
                    req: ChangeRequest {
                        funcion,
                        instancia: p.instancia.clone().filter(|_| a.lista),
                        valores,
                        borrar: p.borrar && a.lista,
                    },
                    etiqueta: Some(titulo.clone()),
                })
                .collect(),
            titulo: Some(titulo),
        })
    }

    /// En un ajuste que no es lista pero escribe con una función de lista
    /// (un pool DHCP concreto), la instancia de cada paso es el id de la
    /// lista que fijan sus `fijos`: sin ella, el gestor crearía una
    /// instancia nueva. `Err` con la función si los `fijos` no lo traen.
    fn instancias_fijas(f: &Catalogo, a: &Ajuste, g: &mut GrupoRequest) -> Result<(), String> {
        if a.lista {
            return Ok(());
        }
        for paso in &mut g.pasos {
            let Some(l) = f.funcion(&paso.req.funcion).and_then(|f| f.lista.as_ref()) else {
                continue;
            };
            paso.req.instancia = Some(
                a.fijos
                    .get(&l.id_param)
                    .cloned()
                    .ok_or_else(|| paso.req.funcion.clone())?,
            );
        }
        Ok(())
    }

    /// Texto legible de un valor para el resumen.
    fn texto(v: ValorLegible) -> String {
        match v {
            ValorLegible::Texto(t) | ValorLegible::Opcion(t) | ValorLegible::Desconocido(t) => t,
            ValorLegible::Booleano(b) => b.to_string(),
            ValorLegible::Secreto => MASCARA.to_owned(),
        }
    }

    /// Cambio legible del campo `c` según el diff de su paso, o `None` si
    /// ninguno de sus parámetros cambia. Un parámetro del campo que no está
    /// en el diff no cambia: su valor lo da `completar` (lo pedido o lo que
    /// muestra el router), así una opción con `ademas` se reconoce entera.
    fn cambio_legible(
        prefijo: &str,
        c: &Campo,
        diff: &Diff,
        completar: &dyn Fn(&str) -> Option<String>,
    ) -> Option<CambioLegible> {
        let e = c.escribe.as_ref()?;
        let params: Vec<(&str, &str)> = std::iter::once((e.parametro.as_str(), c.lee.as_str()))
            .chain(
                c.ademas
                    .iter()
                    .map(|ad| (ad.parametro.as_str(), ad.lee.as_str())),
            )
            .collect();
        let en_diff = |param: &str| {
            diff.campos
                .iter()
                .find(|d| d.nombre.eq_ignore_ascii_case(param))
        };
        if !params.iter().any(|(param, _)| en_diff(param).is_some()) {
            return None;
        }
        let campo = format!("{prefijo}-{}", c.id);
        let secreto = c.tipo == TipoCampo::Secreto
            || params
                .iter()
                .any(|(param, _)| en_diff(param).is_some_and(|d| d.secreto));
        if secreto {
            return Some(CambioLegible {
                campo,
                antes: MASCARA.to_owned(),
                despues: MASCARA.to_owned(),
                secreto: true,
                tipo: c.tipo,
            });
        }
        let (mut antes, mut despues) = (Leidos::new(), Leidos::new());
        for (param, lee) in params {
            let (a, d) = match en_diff(param) {
                Some(d) => (d.antes.clone(), d.despues.clone()),
                None => {
                    let v = completar(param).unwrap_or_default();
                    (v.clone(), v)
                }
            };
            antes.insert(lee.to_owned(), a);
            despues.insert(lee.to_owned(), d);
        }
        Some(CambioLegible {
            campo,
            antes: texto(a_legible(c, &antes)),
            despues: texto(a_legible(c, &despues)),
            secreto: false,
            tipo: c.tipo,
        })
    }

    /// Resumen de un grupo preparado del ajuste `a`. `completar(paso,
    /// parámetro)` da el valor de un parámetro que no cambia en ese paso.
    fn vista_grupo(
        g: &ChangeManager,
        a: &Ajuste,
        token: String,
        pasos: &[PasoPreparado],
        r: Riesgo,
        completar: &dyn Fn(usize, &str) -> Option<String>,
    ) -> GrupoVista {
        let prefijo = prefijo_i18n(a);
        GrupoVista {
            token,
            titulo: format!("{prefijo}-titulo"),
            pasos: pasos
                .iter()
                .map(|paso| PasoVista {
                    indice: paso.indice,
                    sin_cambios: paso.sin_cambios,
                    campos: a
                        .campos
                        .iter()
                        .filter(|c| {
                            c.escribe
                                .as_ref()
                                .is_some_and(|e| e.funcion == paso.funcion)
                        })
                        .filter_map(|c| {
                            cambio_legible(&prefijo, c, &paso.diff, &|param| {
                                completar(paso.indice, param)
                            })
                        })
                        .collect(),
                })
                .collect(),
            riesgo: riesgo(r),
            efecto: a.efecto.as_ref().map(|e| format!("efecto-{e}")),
            vigilancia_s: g.tiempos().vigilancia(r).as_secs(),
        }
    }

    /// Error del gestor al guardar un ajuste, con el mensaje traducido y sin
    /// nombres de la API (el detalle técnico va al registro).
    fn error_ajuste(e: CambioError) -> ServerFnError {
        use CambioError as E;
        match e {
            E::SinCambios => fallo(StatusCode::CONFLICT, t!("ajustes-error-sin-cambios")),
            E::EnCurso => fallo(StatusCode::CONFLICT, t!("biblioteca-error-en-curso")),
            E::Bloqueada(m) if m == BLOQUEO_SOLO_LECTURA => {
                fallo(StatusCode::FORBIDDEN, t!("ajustes-solo-lectura"))
            }
            E::Bloqueada(m) => {
                tracing::warn!("ajustes: escritura bloqueada: {m}");
                fallo(StatusCode::FORBIDDEN, t!("ajustes-error-bloqueada"))
            }
            E::Router(_) | E::Verificacion(_) => {
                tracing::warn!("ajustes: el router no hizo el cambio: {e}");
                fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-router"))
            }
            E::Invalida(_) => {
                tracing::warn!("ajustes: petición no válida: {e}");
                fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
            }
            e => error_cambio(e),
        }
    }

    /// Mensaje traducido de un valor que no vale, con la etiqueta del campo.
    fn error_valor(prefijo: &str, c: &Campo, e: &ErrorValor) -> ServerFnError {
        let mut args = i18n::FluentArgs::new();
        if let ErrorValor::Corto { min } | ErrorValor::FueraDeRango { min: Some(min), .. } = e {
            args.set("min", *min);
        }
        if let ErrorValor::Largo { max } | ErrorValor::FueraDeRango { max: Some(max), .. } = e {
            args.set("max", *max);
        }
        let traducir = crate::ui::i18n::traducir;
        let etiqueta = traducir(&format!("{prefijo}-{}", c.id), None);
        let msg = traducir(e.clave_i18n(), Some(&args));
        fallo(StatusCode::BAD_REQUEST, format!("{etiqueta}: {msg}"))
    }

    /// Ajuste `id` del catálogo (404 si no existe).
    fn ajuste_por_id<'a>(state: &'a AppState, id: &str) -> Result<&'a Ajuste, ServerFnError> {
        state
            .ajustes
            .ajuste(id)
            .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-ajuste")))
    }

    /// Un cambio a la vez: con otro aplicándose o vigilándose no se prepara
    /// nada (409), tampoco en otro ajuste que escribe la misma instancia.
    fn libre(g: &ChangeManager) -> Result<(), ServerFnError> {
        match g.activo() {
            Some(_) => Err(error_ajuste(CambioError::EnCurso)),
            None => Ok(()),
        }
    }

    pub(super) async fn preparar_ajuste(p: PeticionAjuste) -> Result<GrupoVista, ServerFnError> {
        let (state, _) = preparar().await?;
        let a = ajuste_por_id(&state, &p.ajuste)?;
        let prefijo = prefijo_i18n(a);
        for (id, v) in &p.valores {
            let c = a
                .campo(id)
                .ok_or_else(|| fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-campo")))?;
            pares(c, v).map_err(|e| error_valor(&prefijo, c, &e))?;
        }
        let g = gestor(&state)?;
        libre(&g)?;
        let mut grupo = grupo_de(a, &p).map_err(|e| {
            tracing::warn!("ajustes: {}: valor no válido tras validarlo: {e}", a.id);
            fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
        })?;
        instancias_fijas(&state.catalogo, a, &mut grupo).map_err(|funcion| {
            tracing::warn!(
                "ajustes: {}: los fijos no dan la instancia de {funcion}",
                a.id
            );
            fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
        })?;
        if grupo.pasos.is_empty() {
            return Err(error_ajuste(CambioError::SinCambios));
        }
        let pedidos: Vec<BTreeMap<String, String>> =
            grupo.pasos.iter().map(|p| p.req.valores.clone()).collect();
        let prep = g.preparar_grupo(grupo).await.map_err(error_ajuste)?;
        Ok(vista_grupo(
            &g,
            a,
            prep.token,
            &prep.pasos,
            prep.riesgo,
            &|i, param| buscar(&pedidos[i], param).map(str::to_owned),
        ))
    }

    pub(super) async fn confirmar_ajuste(
        token: String,
        desmarcados: Vec<usize>,
    ) -> Result<i64, ServerFnError> {
        let (state, user) = preparar().await?;
        let g = gestor(&state)?;
        g.confirmar_grupo(&token, &desmarcados, &user.username)
            .await
            .map_err(error_ajuste)
    }

    /// Ajuste e instancia del cambio `id`, hecho desde Ajustes: por su
    /// título o, si es un deshacer (que no lo lleva), por el del cambio que
    /// deshace.
    async fn ajuste_del_cambio<'a>(
        state: &'a AppState,
        g: &ChangeManager,
        id: i64,
    ) -> Option<(&'a Ajuste, Option<String>)> {
        let mut actual = id;
        // Una cadena de deshacer no debería ser larga; el tope evita un
        // bucle si el historial tuviera un ciclo.
        for _ in 0..32 {
            let c = g.cambio(actual).await?;
            if let Some(titulo) = &c.titulo {
                let a = state
                    .ajustes
                    .todos()
                    .iter()
                    .find(|a| format!("{}-titulo", prefijo_i18n(a)) == *titulo)?;
                let instancia = c.pasos.iter().find_map(|p| p.instancia.clone());
                return Some((a, instancia.or(c.instancia)));
            }
            actual = g.enlaces_deshacer(actual).await.1?;
        }
        None
    }

    pub(super) async fn deshacer_ajuste(cambio: i64) -> Result<GrupoVista, ServerFnError> {
        let (state, _) = preparar().await?;
        let g = gestor(&state)?;
        let (a, instancia) = ajuste_del_cambio(&state, &g, cambio)
            .await
            .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-cambio")))?;
        libre(&g)?;
        let prep = g.deshacer(cambio).await.map_err(error_ajuste)?;
        // Lo que no cambia en cada paso, como lo muestra ahora el router
        // (una lectura por paso con cambios).
        let mut actuales: Vec<BTreeMap<String, String>> = Vec::new();
        for paso in &prep.pasos {
            let leidos = if paso.sin_cambios {
                None
            } else {
                g.valores_actuales(&paso.funcion, instancia.as_deref(), &a.fijos)
                    .await
                    .inspect_err(|e| {
                        tracing::warn!("ajustes: {}: no se puede leer para el resumen: {e}", a.id);
                    })
                    .ok()
                    .flatten()
            };
            actuales.push(leidos.unwrap_or_default());
        }
        Ok(vista_grupo(
            &g,
            a,
            prep.token,
            &prep.pasos,
            prep.riesgo,
            &|i, param| buscar(&actuales[i], param).map(str::to_owned),
        ))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ajustes::CatalogoAjustes;

        fn peticion(ajuste: &str, valores: &[(&str, &str)], borrar: bool) -> PeticionAjuste {
            PeticionAjuste {
                ajuste: ajuste.into(),
                instancia: Some("7".into()),
                borrar,
                valores: valores
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect(),
            }
        }

        fn mapa(pares: &[(&str, &str)]) -> BTreeMap<String, String> {
            pares
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect()
        }

        #[test]
        fn grupo_de_un_ajuste_de_varios_campos() {
            let cat = CatalogoAjustes::load().unwrap();
            let a = cat.ajuste("wifi.red_5").unwrap();
            let p = peticion(
                "wifi.red_5",
                &[("nombre", "casa-5"), ("seguridad", "abierta")],
                false,
            );
            let g = grupo_de(a, &p).unwrap();
            assert_eq!(g.titulo.as_deref(), Some("ajuste-wifi-red_5-titulo"));
            assert_eq!(g.pasos.len(), 1, "{g:?}");
            let paso = &g.pasos[0];
            assert_eq!(paso.etiqueta.as_deref(), Some("ajuste-wifi-red_5-titulo"));
            assert_eq!(paso.req.funcion, "cbSetWifiAccPt");
            assert_eq!(paso.req.instancia, None, "no es una lista");
            assert!(!paso.req.borrar);
            assert_eq!(
                paso.req.valores,
                mapa(&[
                    ("dpm_cfg_wifi_acc_pt_id", "1"),
                    ("ssid", "casa-5"),
                    ("authentication", "0"),
                    ("encryption", "0"),
                ])
            );
        }

        #[test]
        fn grupo_de_sin_secreto_sin_tocar_ni_campos_sin_tocar() {
            let cat = CatalogoAjustes::load().unwrap();
            let a = cat.ajuste("wifi.red_24").unwrap();
            let p = peticion(
                "wifi.red_24",
                &[("nombre", "casa"), ("contrasena", MASCARA)],
                false,
            );
            let g = grupo_de(a, &p).unwrap();
            assert_eq!(
                g.pasos[0].req.valores,
                mapa(&[("dpm_cfg_wifi_acc_pt_id", "0"), ("ssid", "casa")])
            );
            // Nada tocado: ningún paso.
            let g = grupo_de(a, &peticion("wifi.red_24", &[], false)).unwrap();
            assert!(g.pasos.is_empty());
        }

        #[test]
        fn grupo_de_valida_cada_valor() {
            let cat = CatalogoAjustes::load().unwrap();
            let a = cat.ajuste("wifi.red_24").unwrap();
            let largo = "x".repeat(33);
            let e = grupo_de(a, &peticion("wifi.red_24", &[("nombre", &largo)], false));
            assert_eq!(e.unwrap_err(), ErrorValor::Largo { max: 32 });
            let e = grupo_de(a, &peticion("wifi.red_24", &[("seguridad", "wep")], false));
            assert_eq!(e.unwrap_err(), ErrorValor::Opcion);
            // Un campo de solo lectura no se escribe.
            let u = cat.ajuste("seguridad.usuarios_router").unwrap();
            let e = grupo_de(u, &peticion("x", &[("papel", "x")], false));
            assert_eq!(e.unwrap_err(), ErrorValor::SoloLectura);
        }

        #[test]
        fn grupo_de_una_lista_con_su_instancia_y_borrar() {
            let cat = CatalogoAjustes::load().unwrap();
            let a = cat.ajuste("red_local.reservas").unwrap();
            let g = grupo_de(a, &peticion("x", &[("ip", "192.0.2.9")], false)).unwrap();
            assert_eq!(g.pasos.len(), 1);
            assert_eq!(g.pasos[0].req.instancia.as_deref(), Some("7"));
            assert_eq!(
                g.pasos[0].req.valores,
                mapa(&[
                    ("ipv4_addr", "192.0.2.9"),
                    ("rtm_cfg_dhcpv4_svr_pool_id", "0")
                ])
            );
            // Borrar: cada función de escritura, solo con los fijos.
            let g = grupo_de(a, &peticion("x", &[("ip", "192.0.2.9")], true)).unwrap();
            assert_eq!(g.pasos.len(), 1);
            assert!(g.pasos[0].req.borrar);
            assert_eq!(
                g.pasos[0].req.valores,
                mapa(&[("rtm_cfg_dhcpv4_svr_pool_id", "0")])
            );
        }

        #[test]
        fn instancia_fija_de_un_ajuste_sobre_una_funcion_de_lista() {
            let f = Catalogo::load().unwrap();
            let cat = CatalogoAjustes::load().unwrap();
            let a = cat.ajuste("red_local.dhcp").unwrap();
            let mut g = grupo_de(a, &peticion("x", &[("activo", "false")], false)).unwrap();
            assert_eq!(g.pasos[0].req.instancia, None);
            instancias_fijas(&f, a, &mut g).unwrap();
            assert_eq!(g.pasos[0].req.instancia.as_deref(), Some("0"));
            // Sin el id en los fijos, no se adivina (crearía otra instancia).
            let mut sin_id = a.clone();
            sin_id.fijos.clear();
            let mut g = grupo_de(&sin_id, &peticion("x", &[("activo", "false")], false)).unwrap();
            assert_eq!(
                instancias_fijas(&f, &sin_id, &mut g).unwrap_err(),
                "cbSetDhcpv4ServerPool"
            );
        }

        #[test]
        fn resumen_legible_de_una_opcion_con_ademas() {
            let cat = CatalogoAjustes::load().unwrap();
            let a = cat.ajuste("wifi.red_24").unwrap();
            let c = a.campo("seguridad").unwrap();
            // Solo cambia `authentication`; `encryption` (igual) lo completa
            // lo pedido.
            let diff = Diff {
                campos: vec![crate::changes::CampoDiff {
                    nombre: "authentication".into(),
                    antes: "99".into(),
                    despues: "3".into(),
                    secreto: false,
                }],
            };
            let pedido = mapa(&[("authentication", "3"), ("encryption", "4")]);
            let completar = |p: &str| buscar(&pedido, p).map(str::to_owned);
            let l = cambio_legible("ajuste-wifi-red_24", c, &diff, &completar).unwrap();
            assert_eq!(l.campo, "ajuste-wifi-red_24-seguridad");
            assert_eq!(l.antes, "99", "desconocido: tal cual");
            assert_eq!(l.despues, "wpa2");
            assert_eq!(l.tipo, TipoCampo::Opcion);
            // Un campo cuyo parámetro no cambia no sale.
            let nombre = a.campo("nombre").unwrap();
            assert_eq!(
                cambio_legible("ajuste-wifi-red_24", nombre, &diff, &completar),
                None
            );
        }
    }
}
