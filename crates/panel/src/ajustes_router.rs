//! Ajustes del router por tema: lectura con valores legibles
//! ([`leer_tema`]).
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
        instalar::servidor::fallo, instancia, panel_ajustes::servidor::preparar, state::AppState,
        t, ui::explorador::servidor::riesgo,
    };
    use ajustes::{Ajuste, Campo, Tema, a_legible, prefijo_i18n};
    use axum::http::StatusCode;
    use catalogo::Catalogo;
    use hgu::xml::XmlNode;
    use std::collections::{BTreeMap, HashMap};

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
}
