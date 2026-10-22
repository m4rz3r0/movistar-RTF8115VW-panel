//! Tipos del catálogo de ajustes.
//!
//! [`Tema`], [`Nivel`], [`TipoCampo`] y [`ValorLegible`] los usan también las
//! vistas del panel en el navegador. El resto es el formato de
//! `ajustes/*.toml` y `excluidas.toml` (feature `catalogo`).

use serde::{Deserialize, Serialize};

macro_rules! temas {
    ($($v:ident => $slug:literal;)*) => {
        /// Tema de la página de Ajustes: una pestaña por tema.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum Tema {
            $($v,)*
        }

        impl Tema {
            /// Todos los temas, en el orden de las pestañas.
            pub const TODOS: [Tema; [$(Tema::$v),*].len()] = [$(Tema::$v),*];

            /// Identificador corto (el de los TOML, las URL y las claves de
            /// traducción).
            pub fn slug(self) -> &'static str {
                match self { $(Tema::$v => $slug,)* }
            }

            /// Tema a partir de su identificador corto.
            pub fn from_slug(slug: &str) -> Option<Tema> {
                match slug { $($slug => Some(Tema::$v),)* _ => None }
            }
        }
    };
}

temas! {
    Internet => "internet";
    Wifi => "wifi";
    RedLocal => "red_local";
    Puertos => "puertos";
    Seguridad => "seguridad";
    Telefono => "telefono";
    Sistema => "sistema";
}

/// Dónde aparece un ajuste en su página.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Nivel {
    /// Lo esencial: siempre a la vista.
    Basico,
    /// Bajo «Más opciones».
    Avanzado,
    /// Bajo «Más opciones», y no aparece en la web del operador.
    Oculto,
}

/// Qué control muestra un campo y cómo se traduce su valor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoCampo {
    /// Texto libre (límites en bytes UTF-8).
    Texto,
    /// Contraseña o credencial: nunca sale del servidor.
    Secreto,
    /// Sí o no.
    Booleano,
    /// Una de las opciones del campo.
    Opcion,
    /// Entero.
    Numero,
    /// Dirección IPv4.
    Ip,
    /// Dirección MAC.
    Mac,
}

/// Valor de un campo tal como lo ve la persona.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValorLegible {
    /// Texto, número, IP o MAC.
    Texto(String),
    /// Sí o no (ya invertido si el campo lo pide).
    Booleano(bool),
    /// Clave de la opción (`wpa2_wpa3`), no el valor del router.
    Opcion(String),
    /// Valor del router que el catálogo no reconoce (p. ej. de otro firmware).
    Desconocido(String),
    /// Un secreto: el valor no se envía.
    Secreto,
}

/// Clave legible del valor «sin elegir» de un campo `opcion` que lo declara
/// (`sin_eleccion`): la toma del espejo apagado. No se escribe y no cumple un
/// `requiere`.
pub const SIN_ELECCION: &str = "ninguna";

/// Prefijo de las claves de traducción del ajuste `id` (`wifi.red_24` →
/// `ajuste-wifi-red_24`). Lo usan el catálogo y el navegador.
pub fn prefijo_de_ajuste(id: &str) -> String {
    format!("ajuste-{}", id.replace('.', "-"))
}

/// ¿`valor` es uno de los `aceptados` de un campo de texto? Cada uno vale
/// tal cual o, si empieza por `*.`, por cualquier nombre de ese dominio
/// (`*.pool.ntp.org` acepta `es.pool.ntp.org`). Sin distinguir mayúsculas.
pub fn valor_aceptado(aceptados: &[String], valor: &str) -> bool {
    let v = valor.trim().to_ascii_lowercase();
    aceptados.iter().any(|a| {
        let a = a.trim().to_ascii_lowercase();
        match a.strip_prefix("*.") {
            Some(dominio) => v.len() > dominio.len() && v.ends_with(&format!(".{dominio}")),
            None => v == a,
        }
    })
}

/// Otro campo del mismo ajuste que hace falta cuando este toma un valor: al
/// encender el espejo de puerto hay que elegir la toma. Lo comprueban el
/// servidor al guardar y la página junto al campo exigido.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requiere {
    /// Id del campo exigido.
    pub campo: String,
    /// Valor legible de este campo con el que se exige (`true`, una clave
    /// de opción…).
    pub cuando: String,
}

#[cfg(feature = "catalogo")]
pub use catalogo_toml::*;

/// Formato de `ajustes/*.toml` y `excluidas.toml`.
///
/// Una opción que escribe varios parámetros de la misma función declara los
/// extra en el campo (`ademas`, con el elemento de la lectura de cada uno) y
/// les da valor en cada opción:
///
/// ```toml
/// [[ajuste.campo]]
/// id = "seguridad"
/// escribe = { funcion = "cbSetWifiAccPt", parametro = "authentication" }
/// lee = "authentication"
/// tipo = "opcion"
/// ademas = [{ parametro = "encryption", lee = "encryption" }]
/// opciones = [
///   { valor = "3", clave = "wpa2", recomendada = true, ademas = { encryption = "4" } },
/// ]
/// ```
///
/// Un campo que solo se lee no lleva `escribe` y sí `solo_lectura` (la clave
/// del motivo); el único que lleva los dos es un secreto que solo escribe la
/// web básica.
#[cfg(feature = "catalogo")]
mod catalogo_toml {
    use super::{Nivel, Requiere, Tema, TipoCampo};
    use serde::{Deserialize, Deserializer};
    use std::collections::BTreeMap;

    /// Un ajuste: una unidad que la persona entiende, con sus campos.
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Ajuste {
        /// `<tema>.<nombre>`, estable: da las claves de traducción.
        pub id: String,
        pub tema: Tema,
        pub nivel: Nivel,
        /// Efecto común al aplicarlo (`wifi_reinicia` → `efecto-wifi_reinicia`).
        #[serde(default)]
        pub efecto: Option<String>,
        /// Lectura que da el valor actual de todos los campos.
        pub leer: Lectura,
        /// Parámetros que se envían siempre en cada escritura (ids de la
        /// instancia de un ajuste que no es lista).
        #[serde(default)]
        pub fijos: BTreeMap<String, String>,
        /// El ajuste es una lista de instancias (reservas, reglas…).
        #[serde(default)]
        pub lista: bool,
        /// Componente propio de la interfaz, en vez de la tarjeta genérica.
        #[serde(default)]
        pub componente: Option<Componente>,
        /// Columnas de la lista en la página, en orden (spec §5.1): ids de
        /// sus campos o un rango (`externo`: `externo_desde` y
        /// `externo_hasta`); en los dispositivos, los datos de
        /// [`COLUMNAS_DISPOSITIVOS`](crate::COLUMNAS_DISPOSITIVOS).
        #[serde(default)]
        pub columnas: Vec<String>,
        #[serde(default, rename = "campo")]
        pub campos: Vec<Campo>,
    }

    impl Ajuste {
        /// Campo por id.
        pub fn campo(&self, id: &str) -> Option<&Campo> {
            self.campos.iter().find(|c| c.id == id)
        }
    }

    /// Lectura de un ajuste.
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Lectura {
        pub funcion: String,
        /// Parámetros de la lectura (en el TOML, una tabla).
        #[serde(default, deserialize_with = "tabla_a_pares")]
        pub params: Vec<(String, String)>,
    }

    fn tabla_a_pares<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<(String, String)>, D::Error> {
        Ok(BTreeMap::<String, String>::deserialize(d)?
            .into_iter()
            .collect())
    }

    /// Componente propio de la interfaz para un ajuste.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Componente {
        Dispositivos,
        Reservas,
        Puertos,
        ContrasenaRouter,
    }

    impl Componente {
        /// Nombre del componente, el mismo que en el TOML.
        pub fn slug(self) -> &'static str {
            match self {
                Componente::Dispositivos => "dispositivos",
                Componente::Reservas => "reservas",
                Componente::Puertos => "puertos",
                Componente::ContrasenaRouter => "contrasena_router",
            }
        }

        /// Textos propios del componente: finales de claves de traducción
        /// que, tras el prefijo del ajuste (`ajuste-red_local-reservas`),
        /// van en el `.ftl` de su tema.
        pub fn claves(self) -> &'static [&'static str] {
            match self {
                Componente::Dispositivos => &[
                    "col-ip",
                    "col-mac",
                    "sin-nombre",
                    "cable",
                    "wifi24",
                    "wifi5",
                    "invitados",
                    "desconocida",
                    "desconectado",
                    "fijar",
                    "fijar-de",
                    "ya-fija",
                    "no-fijable",
                    "fijar-desconectado",
                    "sin-ip-fija",
                    "conexion-ninguna",
                    "vacia",
                ],
                Componente::Reservas => &[
                    "anadir",
                    "nueva",
                    "cambiar",
                    "borrar-de",
                    "activa-de",
                    "estado-activa",
                    "estado-conectado",
                    "estado-desactivada",
                    "vacia",
                    "error-fuera",
                    "error-router",
                    "error-ip-repetida",
                    "error-mac-repetida",
                    "error-difusion",
                    "error-cambiada",
                ],
                Componente::Puertos => &[
                    "protocolo-ambos",
                    "protocolo-ambos-desc",
                    "paso-igual",
                    "paso-igual-sin-el-primero",
                    "abrir",
                    "nueva",
                    "cambiar",
                    "puerto",
                    "puerto-desc",
                    "dispositivo",
                    "dispositivo-desc",
                    "dispositivo-elegir",
                    "otra-ip",
                    "puerto-distinto",
                    "interno",
                    "interno-desc",
                    "a-ip",
                    "hacia",
                    "estado-abierta",
                    "estado-cerrada",
                    "estado-sin-destino",
                    "destino-perdido-desc",
                    "borrar-de",
                    "activa-de",
                    "vacia",
                    "error-puerto",
                    "error-dispositivo",
                    "error-invertido",
                    "error-distintos",
                    "error-solapa",
                    "error-fuera",
                    "error-cero",
                    "error-cambiada",
                ],
                Componente::ContrasenaRouter => &[],
            }
        }
    }

    /// Un campo del ajuste: un parámetro de una función de escritura (o un
    /// valor que solo se lee).
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Campo {
        /// Id dentro del ajuste (`nombre`, `seguridad`…).
        pub id: String,
        /// Función y parámetro con que se escribe. Falta en un campo que solo
        /// se lee, que entonces lleva `solo_lectura`.
        #[serde(default)]
        pub escribe: Option<Escritura>,
        /// Elemento de la lectura que da su valor.
        pub lee: String,
        pub tipo: TipoCampo,
        #[serde(default)]
        pub validar: Option<Validar>,
        /// Opciones de un campo `opcion` (valor del router ↔ clave legible).
        #[serde(default)]
        pub opciones: Vec<Opcion>,
        /// Otros parámetros de la misma función de escritura que fija cada
        /// opción de un campo `opcion` (p. ej. el cifrado que acompaña a cada
        /// tipo de seguridad WiFi), con el elemento de la lectura que da su
        /// valor. Cada opción les da valor en su propio `ademas`.
        #[serde(default)]
        pub ademas: Vec<Adicional>,
        /// Valores del router para sí y no, en un campo `booleano` (si falta,
        /// `1` y `0`; ver [`Campo::si_no`]).
        #[serde(default)]
        pub booleano: Option<Booleano>,
        /// El parámetro dice lo contrario que la etiqueta («Visible» sobre
        /// `ssid_hide`).
        #[serde(default)]
        pub invertido: bool,
        /// Se puede dejar vacío (p. ej. un DNS secundario).
        #[serde(default)]
        pub opcional: bool,
        /// Clave de traducción del motivo por el que no se puede editar.
        #[serde(default)]
        pub solo_lectura: Option<String>,
        /// Valor legible recomendado (un campo `opcion` marca su opción
        /// recomendada en la propia opción).
        #[serde(default)]
        pub recomendado: Option<String>,
        /// Valor del router que en un campo `opcion` significa que no hay
        /// ninguna elegida (la toma 0 del espejo apagado): se lee como
        /// [`SIN_ELECCION`](super::SIN_ELECCION), no se escribe y no cumple
        /// un `requiere`.
        #[serde(default)]
        pub sin_eleccion: Option<String>,
        /// Otro campo que hace falta cuando este toma un valor
        /// (`requiere = { campo = "puerto", cuando = "true" }`).
        #[serde(default)]
        pub requiere: Option<Requiere>,
        /// Su recomendación solo vale cuando otro campo del ajuste tiene un
        /// valor (`recomendado_si = { campo = "ipv6", cuando = "true" }`: los
        /// anuncios, solo con IPv6 encendido); si no, no se recomienda nada.
        #[serde(default)]
        pub recomendado_si: Option<Requiere>,
        /// Su recomendación es de seguridad (el WPS, el aislamiento de los
        /// invitados): en la tarjeta va antes que las demás y como riesgo.
        #[serde(default)]
        pub seguridad: bool,
        /// En un campo de texto con `recomendado`, otros valores que valen
        /// igual que lo recomendado (otro servidor de hora neutro); ver
        /// [`valor_aceptado`](super::valor_aceptado).
        #[serde(default)]
        pub aceptados: Vec<String>,
        /// Un `booleano` con sus propios textos para sí y no («Encendida» y
        /// «Apagada» en vez de «Sí» y «No»): `<p>-<campo>-true` y
        /// `<p>-<campo>-false`, en el resumen de un cambio y en su valor.
        #[serde(default)]
        pub textos_valor: bool,
    }

    impl Campo {
        /// Valores del router para sí y no (`1` y `0` si el campo no dice
        /// otros).
        pub fn si_no(&self) -> (&str, &str) {
            self.booleano
                .as_ref()
                .map_or(("1", "0"), |b| (b.si.as_str(), b.no.as_str()))
        }

        /// Escritura del campo, si se puede editar: `None` si no tiene
        /// `escribe` o es de solo lectura.
        pub fn escribible(&self) -> Option<&Escritura> {
            self.escribe
                .as_ref()
                .filter(|_| self.solo_lectura.is_none())
        }
    }

    /// Parámetro que acompaña al del campo en cada opción, y el elemento de la
    /// lectura que da su valor.
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Adicional {
        pub parametro: String,
        pub lee: String,
    }

    /// Función de escritura y parámetro de un campo.
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Escritura {
        pub funcion: String,
        pub parametro: String,
    }

    /// Límites: en bytes UTF-8 para textos y secretos, en valor para números;
    /// y, en textos y secretos, si solo admiten ASCII imprimible.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Validar {
        #[serde(default)]
        pub min: Option<i64>,
        #[serde(default)]
        pub max: Option<i64>,
        /// Solo ASCII imprimible (del espacio a `~`), como la frase de una
        /// red WPA. Solo en texto o secreto.
        #[serde(default)]
        pub ascii: bool,
    }

    /// Opción de un campo `opcion`.
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Opcion {
        /// Valor que entiende el router.
        pub valor: String,
        /// Clave legible: da `<prefijo>-<campo>-<clave>` y su `-desc`.
        pub clave: String,
        #[serde(default)]
        pub recomendada: bool,
        /// Aviso al elegirla (`inseguro` → `aviso-inseguro`).
        #[serde(default)]
        pub aviso: Option<String>,
        /// Valor de cada parámetro del `ademas` del campo
        /// (`ademas = { encryption = "4" }`).
        #[serde(default)]
        pub ademas: BTreeMap<String, String>,
    }

    /// Valores del router para sí y no.
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Booleano {
        pub si: String,
        pub no: String,
    }

    /// Un elemento de una pestaña: lo que la persona ve como una cosa (una
    /// red WiFi, una radio) aunque junte varios ajustes. Las referencias a
    /// campos son `<ajuste>.<campo>` (`wifi.red_24.activa`) y las de datos
    /// calculados, `derivado:<nombre>`.
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Elemento {
        /// Id en la URL (`red-24`): minúsculas, cifras y `-`.
        pub id: String,
        pub tema: Tema,
        /// Ids de sus ajustes; cada ajuste va en un solo elemento.
        pub ajustes: Vec<String>,
        /// Papel (`principal`, `invitados`, `radio`): da `ajustes-papel-<papel>`.
        #[serde(default)]
        pub papel: Option<String>,
        /// Campo booleano escribible que lo enciende y lo apaga.
        #[serde(default)]
        pub interruptor: Option<String>,
        /// Hasta seis datos de la tarjeta: campos o derivados.
        #[serde(default)]
        pub datos: Vec<String>,
        /// Regla de estado (`derivado:estado_red`).
        #[serde(default)]
        pub estado: Option<String>,
        /// Glifo de su tarjeta (`reloj`) en vez de los arcos de emisión de
        /// la WiFi; ver [`ICONOS`](crate::ICONOS).
        #[serde(default)]
        pub icono: Option<String>,
    }

    /// Una acción que se aplica a varios elementos de una pestaña a la vez
    /// (`quitar_wps`): pone el campo `campo` de cada uno a `valor` (legible).
    /// En una lista (`lista`), cada instancia es un elemento: pone su campo
    /// o, con `borrar`, la borra (sin `campo` ni `valor`, y con `peligro`).
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Accion {
        /// Id (`quitar_wps`): da `ajustes-accion-<id>`.
        pub id: String,
        pub tema: Tema,
        /// Id del campo, en el ajuste de cada elemento que lo tenga.
        #[serde(default)]
        pub campo: Option<String>,
        /// Valor legible que se escribe (`false`, la clave de una opción…).
        #[serde(default)]
        pub valor: Option<String>,
        /// Lleva el botón de peligro y confirmación.
        #[serde(default)]
        pub peligro: bool,
        /// Id del ajuste de tipo lista cuyas instancias son sus elementos.
        #[serde(default)]
        pub lista: Option<String>,
        /// Borra las instancias marcadas de `lista`.
        #[serde(default)]
        pub borrar: bool,
    }

    /// Función de escritura que ningún ajuste cubre, y por qué.
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Excluida {
        pub funcion: String,
        pub motivo: String,
    }
}
