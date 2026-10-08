//! Tipos del catálogo de funciones (`funciones.toml`).

use serde::{Deserialize, Serialize};

/// Una función de gestión del router: una llamada `cb*.xml` de la interfaz
/// avanzada, una URL `*.cmd|.cgi|.wl` de la básica o una página `.asp`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Funcion {
    /// Nombre con el que se invoca: `cbGetWifi`, `te_wifi.wl`,
    /// `te_puertos.cmd?action=add`, `restoreinfo.cgi`, `upload/cfgupload.asp`…
    pub name: String,
    /// Área del explorador.
    pub area: Area,
    /// De qué parte de la interfaz del router sale.
    pub origen: Origen,
    /// Lectura, escritura o acción.
    pub tipo: Tipo,
    /// Página de la que se saca el `sessionKey` (y donde vive la función).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagina: Option<String>,
    /// Qué hace, en español.
    pub descripcion: String,
    /// Parámetros que acepta.
    #[serde(default, rename = "parametro", skip_serializing_if = "Vec::is_empty")]
    pub parametros: Vec<Parametro>,
    /// Lectura que devuelve lo que esta función escribe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lectura_pareada: Option<LecturaPareada>,
    /// Cómo se enumeran y seleccionan las instancias, si es una lista.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lista: Option<Lista>,
    /// Riesgo de usarla (determina la vigilancia tras un cambio).
    pub riesgo: Riesgo,
    /// Motivo por el que el panel no la invoca nunca.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bloqueada: Option<String>,
}

impl Funcion {
    /// Parámetro por nombre.
    pub fn parametro(&self, nombre: &str) -> Option<&Parametro> {
        self.parametros.iter().find(|p| p.nombre == nombre)
    }
}

macro_rules! area {
    ($($v:ident => $slug:literal, $etq:literal;)*) => {
        /// Área funcional (lateral del explorador).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum Area {
            $(
                #[doc = $etq]
                $v,
            )*
        }

        impl Area {
            /// Todas las áreas, en el orden del explorador.
            pub const TODAS: &[Area] = &[$(Area::$v),*];

            /// Identificador corto (el mismo que en `funciones.toml` y en las URL).
            pub fn slug(&self) -> &'static str {
                match self { $(Area::$v => $slug,)* }
            }

            /// Área a partir de su identificador corto.
            pub fn from_slug(slug: &str) -> Option<Area> {
                match slug { $($slug => Some(Area::$v),)* _ => None }
            }

            /// Nombre para mostrar.
            pub fn etiqueta(&self) -> &'static str {
                match self { $(Area::$v => $etq,)* }
            }
        }
    };
}

area! {
    Wan => "wan", "WAN";
    LanDhcp => "lan_dhcp", "LAN/DHCP";
    Wifi => "wifi", "WiFi";
    Nat => "nat", "NAT";
    Cortafuegos => "cortafuegos", "Cortafuegos";
    Dns => "dns", "DNS";
    Ipv6 => "ipv6", "IPv6";
    Multicast => "multicast", "Multicast/IGMP";
    Qos => "qos", "QoS";
    Voip => "voip", "VoIP";
    Tr069 => "tr069", "TR-069";
    Usuarios => "usuarios", "Usuarios";
    Sistema => "sistema", "Sistema";
    Ocultas => "ocultas", "Ocultas";
}

/// De qué parte de la interfaz del router sale la función.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origen {
    /// Interfaz básica (`te_*.asp`).
    Basica,
    /// Interfaz avanzada enlazada desde el menú.
    Avanzada,
    /// Existe y funciona, pero ningún menú la enlaza (o está comentada).
    Oculta,
    /// Restos sin terminar: la página existe pero sus llamadas no responden.
    Incompleta,
}

/// Qué hace la función con el router.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tipo {
    /// Solo lee.
    Lectura,
    /// Cambia configuración que una lectura pareada permite comprobar.
    Escritura,
    /// Dispara algo (reinicio, guardado, exportación…) sin estado comprobable.
    Accion,
}

/// Riesgo de usar la función.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Riesgo {
    /// Vigilancia de 30 s.
    Bajo,
    /// Vigilancia de 2 min.
    Medio,
    /// Vigilancia de 5 min.
    Alto,
}

/// Parámetro de una función.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parametro {
    /// Nombre tal como se envía.
    pub nombre: String,
    /// Etiqueta para el formulario.
    pub etiqueta: String,
    /// Tipo del valor.
    pub tipo: TipoParam,
    /// Valores posibles `(valor, etiqueta)` de un `Enum`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opciones: Vec<(String, String)>,
    /// La página lo edita en varios campos `<nombre>-0..-n`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partido: Option<Partido>,
}

/// Tipo del valor de un parámetro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoParam {
    /// Dirección IPv4 (o máscara).
    Ip,
    /// Dirección MAC.
    Mac,
    /// `0`/`1` (casilla).
    Bool,
    /// Uno de [`Parametro::opciones`].
    Enum,
    /// Entero.
    Int,
    /// Texto libre.
    Text,
    /// Secreto: nunca se muestra ni se registra.
    Secret,
}

/// Campo partido como lo hace `beforeSendForm` (tabs.js).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Partido {
    /// IPv4 en `-0..-3`, unida con `.`.
    Ip4,
    /// MAC en `-0..-5`, unida con `:`.
    Mac6,
}

/// Lectura que devuelve lo que escribe una función.
///
/// En `params`, un valor `{nombre}` se sustituye por el valor del parámetro
/// `nombre` de la escritura (normalmente el id de la instancia).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LecturaPareada {
    /// Función de lectura.
    pub funcion: String,
    /// Parámetros de la lectura.
    #[serde(default)]
    pub params: Vec<(String, String)>,
}

/// Cómo se enumeran las instancias de una lista.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lista {
    /// Parámetro que identifica la instancia (`*_id` o `*_0i`).
    pub id_param: String,
    /// Admite `all=ALL` para listar todas.
    #[serde(default)]
    pub all: bool,
    /// Admite `del=DEL` para borrar una instancia.
    #[serde(default)]
    pub del: bool,
}
