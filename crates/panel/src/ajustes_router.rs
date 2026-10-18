//! Ajustes del router por tema: lectura con valores legibles
//! ([`leer_tema`]) y escritura de un ajuste como un grupo de cambios del
//! gestor ([`preparar_ajuste`], [`confirmar_ajuste`], [`deshacer_ajuste`]),
//! con un resumen legible «antes → después» ([`GrupoVista`]).
//!
//! Los tipos de aquí los usan las páginas de Ajustes en el navegador. Solo
//! llevan ids, claves de traducción y valores legibles: ningún nombre de la
//! API salvo en [`DetallesTecnicos`], que la interfaz pinta únicamente dentro
//! de `<details class="detalles-tecnicos">`. Un secreto llega como
//! [`ValorLegible::Secreto`]: su valor solo sale del servidor al pedirlo con
//! [`revelar_ajuste`] (con reautenticación, anotado y con `no-store`).

use crate::t;
use crate::ui::explorador::RiesgoCambio;
use ajustes::{Nivel, Requiere, TipoCampo, ValorLegible};
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;

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
    /// Secreto que el router da tapado al leer: no se puede «Mostrar», y
    /// guardar cualquier campo que escriba con él exige escribirlo.
    #[serde(default)]
    pub tapado: bool,
    /// Valor legible recomendado: la clave de la opción recomendada en un
    /// campo `opcion` (`wpa2`), `true`/`false` en un booleano o el texto.
    pub recomendado: Option<String>,
    /// Clave de traducción del porqué de la recomendación (para quién es, a
    /// favor y en contra): `<clave>-recomendado`. Solo si hay recomendación.
    pub motivo_recomendado: Option<String>,
    /// Otro campo del ajuste que hace falta cuando este toma un valor (la
    /// toma del espejo al encenderlo): la página lo pide junto a ese campo.
    pub requiere: Option<Requiere>,
    /// El campo `opcion` tiene un valor «sin elegir» ([`ajustes::SIN_ELECCION`],
    /// la toma del espejo apagado): se ve como una opción más, pero no se
    /// escribe ni cuenta como elegida.
    pub sin_eleccion: bool,
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

/// Lo que la página pide guardar de un ajuste. Su `Debug` no enseña los
/// valores (pueden ser contraseñas), solo los ids de los campos.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Lo que la página espera que tenga la instancia que edita o borra
    /// (id del campo → valor legible): si el router ya tiene otra cosa
    /// (otra sesión la cambió), 409. Por ahora lo comprueban las reservas,
    /// que al editar o borrar exigen su MAC (sin ella, 400).
    #[serde(default)]
    pub esperados: BTreeMap<String, String>,
}

impl std::fmt::Debug for PeticionAjuste {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeticionAjuste")
            .field("ajuste", &self.ajuste)
            .field("instancia", &self.instancia)
            .field("borrar", &self.borrar)
            .field("valores", &self.valores.keys().collect::<Vec<_>>())
            .field("esperados", &self.esperados.keys().collect::<Vec<_>>())
            .finish()
    }
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
    /// A qué se refiere, si el título no basta (el dispositivo de una IP fija).
    pub detalle: Option<String>,
}

/// Un paso del grupo: los campos de una misma escritura del router.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasoVista {
    /// Posición en el grupo (desde 0): la que se desmarca al confirmar.
    pub indice: usize,
    /// El router ya está así: no se escribirá.
    pub sin_cambios: bool,
    pub campos: Vec<CambioLegible>,
    /// Clave de traducción de su título («TCP»); sin ella, «Paso N».
    #[serde(default)]
    pub titulo: Option<String>,
    /// Clave de traducción de una nota bajo el título (lo que no se repite).
    #[serde(default)]
    pub nota: Option<String>,
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

/// Un dispositivo que el router conoce en la red de casa.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispositivoVista {
    /// Nombre que da el dispositivo; vacío si no da ninguno.
    pub nombre: String,
    pub ip: String,
    pub mac: String,
    pub conexion: Conexion,
    /// Conectado ahora.
    pub activo: bool,
    /// Id de su reserva (IP fija), si tiene.
    pub reserva: Option<String>,
    /// La IP de su reserva (puede no ser la que tiene ahora).
    pub ip_fija: Option<String>,
}

/// Por dónde está conectado un dispositivo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Conexion {
    /// Por cable: la interfaz del router tal como la da (`tipo:id`). Qué
    /// toma física es cada id está sin comprobar en el router real, así que
    /// la página solo dice «por cable».
    Cable(String),
    Wifi24,
    Wifi5,
    /// Asociado a una red de invitados.
    Invitados,
    /// No se sabe: no está conectado ahora o no se pudo leer la WiFi.
    Desconocida,
}

/// Por qué no vale una reserva (IP fija).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorReserva {
    /// La IP no es de la red de casa.
    FueraDeLaRed,
    /// La IP es la del router.
    IpDelRouter,
    /// Otra reserva ya tiene esa IP.
    IpRepetida,
    /// El dispositivo (su MAC) ya tiene otra reserva.
    MacRepetida,
    /// La primera o la última dirección de la red, que son de toda la red.
    Difusion,
}

impl ErrorReserva {
    /// Campo de la reserva junto al que va el mensaje.
    pub fn campo(self) -> &'static str {
        match self {
            ErrorReserva::MacRepetida => "mac",
            _ => "ip",
        }
    }

    /// Mensaje traducido; `router` es la IP del router (la cita al decir
    /// que una IP no es de la red).
    pub fn texto(self, router: Ipv4Addr) -> String {
        match self {
            ErrorReserva::FueraDeLaRed => t!(
                "ajuste-red_local-reservas-error-fuera",
                router = router.to_string()
            ),
            ErrorReserva::IpDelRouter => t!("ajuste-red_local-reservas-error-router"),
            ErrorReserva::IpRepetida => t!("ajuste-red_local-reservas-error-ip-repetida"),
            ErrorReserva::MacRepetida => t!("ajuste-red_local-reservas-error-mac-repetida"),
            ErrorReserva::Difusion => t!("ajuste-red_local-reservas-error-difusion"),
        }
    }
}

/// MAC comparable: normalizada si es válida; si no, en minúsculas.
pub(crate) fn mac_comparable(mac: &str) -> String {
    ajustes::normalizar_mac(mac.trim()).unwrap_or_else(|| mac.trim().to_ascii_lowercase())
}

/// ¿Vale la reserva de `mac` a `ip` en la red `lan` (IP y máscara del
/// router), con las reservas `existentes` (`(id, mac, ip)`)? Al editar la
/// reserva `editando`, la suya no cuenta como repetida. La MAC y la IP ya
/// tienen buen formato (lo comprueba `validar_legible`).
pub fn validar_reserva(
    mac: &str,
    ip: Ipv4Addr,
    lan: (Ipv4Addr, Ipv4Addr),
    existentes: &[(String, String, String)],
    editando: Option<&str>,
) -> Result<(), ErrorReserva> {
    let (router, mascara) = lan;
    let (m, n) = (u32::from(mascara), u32::from(ip));
    if n & m != u32::from(router) & m {
        return Err(ErrorReserva::FueraDeLaRed);
    }
    if ip == router {
        return Err(ErrorReserva::IpDelRouter);
    }
    let anfitrion = n & !m;
    if anfitrion == 0 || anfitrion == !m {
        return Err(ErrorReserva::Difusion);
    }
    let otras = || {
        existentes
            .iter()
            .filter(|(id, _, _)| Some(id.as_str()) != editando)
    };
    if otras().any(|(_, _, otra)| otra.trim().parse::<Ipv4Addr>().ok() == Some(ip)) {
        return Err(ErrorReserva::IpRepetida);
    }
    let mac = mac_comparable(mac);
    if otras().any(|(_, otra, _)| mac_comparable(otra) == mac) {
        return Err(ErrorReserva::MacRepetida);
    }
    Ok(())
}

/// ¿Hay que validar la reserva que pide `p` ([`validar_reserva`])? Una
/// nueva, o una que cambia de MAC o de IP o se activa. Desactivarla o
/// borrarla, no: así se puede apagar una reserva que ya no valdría (su IP
/// quedó fuera de la red, o la comparte con otra) sin corregirla antes.
pub fn valida_la_reserva(p: &PeticionAjuste) -> bool {
    if p.borrar {
        return false;
    }
    p.instancia.is_none()
        || p.valores.contains_key("mac")
        || p.valores.contains_key("ip")
        || p.valores.get("activa").map(|v| v.trim()) == Some("true")
}

/// Valor del campo «protocolo» de una regla nueva que abre el puerto en TCP
/// y en UDP: el router solo admite uno por regla, así que el servidor crea
/// dos (una de cada) en un único grupo de cambios.
pub const AMBOS: &str = "ambos";

/// Protocolo de una regla de apertura de puertos. `Tcp` y `Udp` son las
/// opciones `tcp` y `udp` del catálogo (su valor del router sale de ahí);
/// `Ambos` solo existe al crear: son dos reglas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocolo {
    Tcp,
    Udp,
    Ambos,
}

impl Protocolo {
    /// Del valor legible del campo (`tcp`, `udp` o [`AMBOS`]).
    pub fn de_clave(clave: &str) -> Option<Protocolo> {
        match clave.trim() {
            "tcp" => Some(Protocolo::Tcp),
            "udp" => Some(Protocolo::Udp),
            AMBOS => Some(Protocolo::Ambos),
            _ => None,
        }
    }

    /// Las opciones del catálogo que cubre.
    pub fn claves(self) -> &'static [&'static str] {
        match self {
            Protocolo::Tcp => &["tcp"],
            Protocolo::Udp => &["udp"],
            Protocolo::Ambos => &["tcp", "udp"],
        }
    }

    /// ¿Comparten algún protocolo?
    fn comparte(self, otro: Protocolo) -> bool {
        self.claves().iter().any(|c| otro.claves().contains(c))
    }
}

/// Una regla de apertura de puertos: lo que llega de internet por los
/// puertos `externo` (desde, hasta) va a los `interno` del dispositivo `ip`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglaPuertos {
    /// Id en el router; `None` si es nueva.
    pub id: Option<String>,
    pub nombre: String,
    pub protocolo: Protocolo,
    pub externo: (u16, u16),
    pub interno: (u16, u16),
    pub ip: Ipv4Addr,
    pub activa: bool,
}

/// Por qué no vale una regla de apertura de puertos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorRegla {
    /// El último puerto de un rango es menor que el primero.
    RangoInvertido,
    /// El rango del dispositivo no tiene el mismo tamaño que el de internet.
    RangosDistintos,
    /// Otra regla ya abre alguno de esos puertos con el mismo protocolo.
    Solapa,
    /// La IP no es la de un dispositivo de la red de casa.
    FueraDeLaRed,
    /// El puerto 0 no existe.
    PuertoCero,
}

impl ErrorRegla {
    /// Campo de la regla junto al que va el mensaje: el puerto de internet
    /// (`externo_desde`) o el dispositivo (`destino`).
    pub fn campo(self) -> &'static str {
        match self {
            ErrorRegla::FueraDeLaRed => "destino",
            _ => "externo_desde",
        }
    }

    /// Mensaje traducido; `router` es la IP del router.
    pub fn texto(self, router: Ipv4Addr) -> String {
        match self {
            ErrorRegla::RangoInvertido => t!("ajuste-puertos-reglas-error-invertido"),
            ErrorRegla::RangosDistintos => t!("ajuste-puertos-reglas-error-distintos"),
            ErrorRegla::Solapa => t!("ajuste-puertos-reglas-error-solapa"),
            ErrorRegla::FueraDeLaRed => t!(
                "ajuste-puertos-reglas-error-fuera",
                router = router.to_string()
            ),
            ErrorRegla::PuertoCero => t!("ajuste-puertos-reglas-error-cero"),
        }
    }
}

/// ¿Vale la regla `r` con las reglas `existentes` en la red `lan` (IP y
/// máscara del router)? Por orden: ningún puerto es 0; cada rango va de
/// menor a mayor; los dos rangos tienen el mismo tamaño; la IP es la de un
/// dispositivo de la red (ni la del router ni la primera o la última de la
/// red); ninguna otra regla (la propia, por su id, no cuenta) abre alguno
/// de esos puertos de internet con el mismo protocolo.
pub fn validar_regla(
    r: &ReglaPuertos,
    existentes: &[ReglaPuertos],
    lan: (Ipv4Addr, Ipv4Addr),
) -> Result<(), ErrorRegla> {
    let (ext, int) = (r.externo, r.interno);
    if [ext.0, ext.1, int.0, int.1].contains(&0) {
        return Err(ErrorRegla::PuertoCero);
    }
    if ext.0 > ext.1 || int.0 > int.1 {
        return Err(ErrorRegla::RangoInvertido);
    }
    if ext.1 - ext.0 != int.1 - int.0 {
        return Err(ErrorRegla::RangosDistintos);
    }
    let (router, mascara) = lan;
    let (m, n) = (u32::from(mascara), u32::from(r.ip));
    let anfitrion = n & !m;
    if n & m != u32::from(router) & m || r.ip == router || anfitrion == 0 || anfitrion == !m {
        return Err(ErrorRegla::FueraDeLaRed);
    }
    let solapa = existentes
        .iter()
        .filter(|o| r.id.is_none() || o.id != r.id)
        .any(|o| o.protocolo.comparte(r.protocolo) && o.externo.0 <= ext.1 && ext.0 <= o.externo.1);
    if solapa {
        return Err(ErrorRegla::Solapa);
    }
    Ok(())
}

/// ¿El dispositivo de destino `ip` ya no está en la red? Cierto si no es la
/// IP de ningún dispositivo que conozca el router (conectado o no) ni la de
/// ninguna IP fija (`reservas`: `(id, mac, ip)`).
pub fn destino_inexistente(
    ip: &str,
    dispositivos: &[DispositivoVista],
    reservas: &[(String, String, String)],
) -> bool {
    let ip = ip.trim();
    !dispositivos.iter().any(|d| d.ip.trim() == ip)
        && !reservas.iter().any(|(_, _, r)| r.trim() == ip)
}

/// Valor legible del campo `id` de una instancia (vacío si falta).
pub fn valor_de(campos: &[CampoVista], id: &str) -> String {
    match campos.iter().find(|c| c.id == id).map(|c| &c.valor) {
        Some(ValorLegible::Texto(t) | ValorLegible::Opcion(t) | ValorLegible::Desconocido(t)) => {
            t.trim().to_owned()
        }
        Some(ValorLegible::Booleano(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// La regla de una instancia leída de `puertos.reglas`; `None` si un puerto
/// o la IP no lo son. Un protocolo que no es TCP ni UDP queda como
/// [`Protocolo::Ambos`]: no se sabe con cuál choca.
pub fn regla_de(i: &InstanciaVista) -> Option<ReglaPuertos> {
    let v = |id: &str| valor_de(&i.campos, id);
    let puerto = |id: &str| v(id).parse::<u16>().ok();
    Some(ReglaPuertos {
        id: Some(i.id.clone()),
        nombre: v("nombre"),
        // Un protocolo que el panel no conoce puede ser cualquiera: cuenta
        // como los dos (al comprobar si otra regla choca con ella).
        protocolo: Protocolo::de_clave(&v("protocolo"))
            .filter(|p| *p != Protocolo::Ambos)
            .unwrap_or(Protocolo::Ambos),
        externo: (puerto("externo_desde")?, puerto("externo_hasta")?),
        interno: (puerto("interno_desde")?, puerto("interno_hasta")?),
        ip: v("destino").parse().ok()?,
        activa: v("activa") == "true",
    })
}

/// Campos de una regla que la identifican: la página los envía como
/// `esperados` al editarla o borrarla, y el servidor comprueba que el
/// router aún los tiene (si no, 409).
pub const IDENTIFICAN_REGLA: [&str; 3] = ["protocolo", "externo_desde", "destino"];

/// ¿Hay que validar la regla que pide `p` ([`validar_regla`])? Una nueva, o
/// una que cambia algo más que el nombre o se activa. Desactivarla, cambiarle
/// el nombre o borrarla, no: así se puede apagar una regla que ya no
/// valdría sin corregirla antes.
pub fn valida_la_regla(p: &PeticionAjuste) -> bool {
    if p.borrar {
        return false;
    }
    p.instancia.is_none()
        || p.valores.iter().any(|(k, v)| match k.as_str() {
            "nombre" => false,
            "activa" => v.trim() == "true",
            _ => true,
        })
}

/// Texto traducido de un valor que no vale, con sus límites: el mismo en el
/// servidor y en la página.
pub fn texto_error_valor(e: &ajustes::ErrorValor) -> String {
    let mut args = i18n::FluentArgs::new();
    let (min, max) = e.limites();
    if let Some(min) = min {
        args.set("min", min);
    }
    if let Some(max) = max {
        args.set("max", max);
    }
    crate::ui::i18n::traducir(e.clave_i18n(), Some(&args))
}

/// Cuánto vale una lectura del router para mostrar: la caché del servidor
/// (`crate::lecturas`) y la precarga de temas de la página.
pub const CADUCIDAD_LECTURAS: std::time::Duration = std::time::Duration::from_secs(30);

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
/// del router. Cada lectura distinta se hace una sola vez, de la caché
/// corta si está ([`crate::lecturas`]) y, las que faltan, seguidas en una
/// tanda; si una falla, solo sus ajustes quedan [`EstadoLectura::SinLeer`].
/// 404 si el tema no existe. Exige sesión; la respuesta va con `no-store`.
#[server(endpoint = "leer_tema", input = Json)]
pub async fn leer_tema(tema: String) -> Result<TemaVista, ServerFnError> {
    servidor::leer_tema(tema).await
}

/// Valor en claro del secreto `campo` del ajuste `ajuste` (en una lista, de
/// su instancia `instancia`), leído del router en ese momento: nunca de la
/// caché de lecturas. Con `contrasena` vacía, solo si la sesión tiene abierta
/// la ventana de reautenticación (si no, 401 con
/// [`crate::auth::ERROR_REAUTENTICACION`]); con la contraseña del panel, la
/// comprueba con el freno del login y abre la ventana. Queda anotado, sin el
/// valor, en `reveals` (con el ajuste y el campo). La respuesta va con
/// `no-store`. 400 si el campo no es un secreto del ajuste; 404 si el ajuste
/// no existe.
#[server(endpoint = "revelar_ajuste", input = Json)]
pub async fn revelar_ajuste(
    ajuste: String,
    campo: String,
    instancia: Option<String>,
    #[server(default)] contrasena: String,
) -> Result<String, ServerFnError> {
    servidor::revelar_ajuste(ajuste, campo, instancia, contrasena).await
}

/// Los dispositivos que el router conoce en la red de casa, los conectados
/// primero y por IP, con su conexión y su reserva (lecturas para mostrar,
/// como [`leer_tema`]). Exige sesión; la respuesta va con `no-store`.
#[server(endpoint = "leer_dispositivos", input = Json)]
pub async fn leer_dispositivos() -> Result<Vec<DispositivoVista>, ServerFnError> {
    servidor::leer_dispositivos().await
}

/// IP y máscara del router en la red de casa (`None` si no da ninguna
/// legible): la página valida con ellas una reserva antes de enviarla (el
/// servidor la vuelve a validar leyendo el router). Lectura para mostrar,
/// como [`leer_tema`].
#[server(endpoint = "leer_red_casa", input = Json)]
pub async fn leer_red_casa() -> Result<Option<(Ipv4Addr, Ipv4Addr)>, ServerFnError> {
    servidor::leer_red_casa().await
}

/// Las IP de las IP fijas del router (solo eso: es lo que usa la página):
/// Puertos las usa para saber si el dispositivo de destino de una regla
/// sigue en la red ([`destino_inexistente`]). Lectura para mostrar, como
/// [`leer_tema`].
#[server(endpoint = "leer_reservas", input = Json)]
pub async fn leer_reservas() -> Result<Vec<String>, ServerFnError> {
    servidor::leer_reservas().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_de_la_peticion_no_ensena_los_valores() {
        let p = PeticionAjuste {
            ajuste: "wifi.red_24".into(),
            instancia: None,
            borrar: false,
            valores: [("contrasena".to_owned(), "clave-de-prueba-9".to_owned())].into(),
            esperados: BTreeMap::new(),
        };
        let d = format!("{p:?}");
        assert!(!d.contains("clave-de-prueba-9"), "{d}");
        assert!(d.contains("wifi.red_24") && d.contains("contrasena"), "{d}");
    }
}

#[cfg(feature = "ssr")]
pub(crate) mod servidor {
    use super::*;
    use crate::{
        changes::{
            BLOQUEO_SECRETO_TAPADO, BLOQUEO_SOLO_LECTURA, CambioError, ChangeManager,
            ChangeRequest, Diff, GrupoRequest, Paso, PasoPreparado, buscar,
        },
        instalar::servidor::fallo,
        instancia,
        lecturas::Clave,
        panel_ajustes::servidor::preparar,
        state::AppState,
        t,
        ui::explorador::{
            MASCARA,
            servidor::{error_cambio, gestor, riesgo, usuario_para_revelar},
        },
    };
    use ajustes::{
        Ajuste, Campo, Componente, ErrorValor, Tema, a_legible, a_router, falta_requerido,
        prefijo_i18n,
    };
    use axum::http::StatusCode;
    use catalogo::{Catalogo, Riesgo};
    use hgu::xml::XmlNode;
    use std::collections::HashMap;

    /// Elementos de una lectura: nombre → valor.
    type Leidos = BTreeMap<String, String>;

    /// Lectura de un ajuste.
    fn clave(a: &Ajuste) -> Clave {
        Clave::new(&a.leer.funcion, &a.leer.params)
    }

    pub(super) async fn leer_tema(tema: String) -> Result<TemaVista, ServerFnError> {
        let (state, _) = preparar().await?;
        let tema = Tema::from_slug(&tema)
            .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-tema")))?;
        let del_tema: Vec<&Ajuste> = state.ajustes.por_tema(tema).collect();
        let claves: Vec<Clave> = del_tema.iter().map(|a| clave(a)).collect();
        let leidas = state
            .lecturas
            .leer(&state.hgu, &state.mascara, &claves)
            .await;
        let ajustes = del_tema
            .iter()
            .zip(&claves)
            .map(|(a, c)| {
                let nodo = leidas.get(c).and_then(|r| r.as_ref().ok());
                vista(&state.catalogo, a, nodo)
            })
            .collect();
        Ok(TemaVista {
            tema: tema.slug().to_owned(),
            solo_lectura: instancia::solo_lectura(&state.db).await,
            ajustes,
        })
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
                        .map(|c| campo(f, &prefijo, c, valor_inicial(c)))
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
                    .map(|c| campo(f, &prefijo, c, a_legible(c, &leidos)))
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
        Some(
            instancias_con_id(f, a, n)?
                .into_iter()
                .map(|(id, leidos)| InstanciaVista {
                    id,
                    campos: a
                        .campos
                        .iter()
                        .map(|c| campo(f, prefijo, c, a_legible(c, &leidos)))
                        .collect(),
                })
                .collect(),
        )
    }

    /// Instancias de una lista con su id (ver [`lista`]) y sus elementos tal
    /// cual los da el router.
    fn instancias_con_id(f: &Catalogo, a: &Ajuste, n: &XmlNode) -> Option<Vec<(String, Leidos)>> {
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
                Some((id, leidos))
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
    /// ¿El campo `c` escribe un secreto que el router da tapado al leer?
    pub(crate) fn tapado(f: &Catalogo, c: &Campo) -> bool {
        c.tipo == TipoCampo::Secreto
            && c.escribible().is_some_and(|e| {
                f.funcion(&e.funcion)
                    .and_then(|func| func.parametro(&e.parametro))
                    .is_some_and(|p| p.tapado)
            })
    }

    fn campo(f: &Catalogo, prefijo: &str, c: &Campo, valor: ValorLegible) -> CampoVista {
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
            tapado: tapado(f, c),
            motivo_recomendado: recomendado.as_ref().map(|_| format!("{clave}-recomendado")),
            recomendado,
            requiere: c.requiere.clone(),
            sin_eleccion: c.sin_eleccion.is_some(),
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
    /// `v`, validado; `None` si es un secreto sin tocar ([`MASCARA`]). Un
    /// campo de solo lectura no vale ni con la máscara: la página no lo
    /// envía.
    fn pares(c: &Campo, v: &str) -> Result<Option<Vec<(String, String)>>, ErrorValor> {
        if c.escribible().is_none() {
            return Err(ErrorValor::SoloLectura);
        }
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
                    titulo: None,
                    nota: None,
                })
                .collect(),
            riesgo: riesgo(r),
            efecto: a.efecto.as_ref().map(|e| format!("efecto-{e}")),
            vigilancia_s: g.tiempos().vigilancia(r).as_secs(),
            detalle: None,
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
            E::Bloqueada(m) if m == BLOQUEO_SECRETO_TAPADO => {
                fallo(StatusCode::FORBIDDEN, t!("cambio-error-secreto-tapado"))
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
        let etiqueta = crate::ui::i18n::traducir(&format!("{prefijo}-{}", c.id), None);
        fallo(
            StatusCode::BAD_REQUEST,
            format!("{etiqueta}: {}", texto_error_valor(e)),
        )
    }

    /// Mensaje traducido de un campo que hace falta y no viene (400), junto
    /// a la etiqueta del campo que falta: «<falta>: hace falta para <pide>».
    fn error_requiere(prefijo: &str, pide: &Campo, falta: &Campo) -> ServerFnError {
        let traducir = crate::ui::i18n::traducir;
        let etiqueta = |c: &Campo| traducir(&format!("{prefijo}-{}", c.id), None);
        let mut args = i18n::FluentArgs::new();
        args.set("campo", etiqueta(pide));
        let msg = traducir("ajustes-error-requiere", Some(&args));
        fallo(
            StatusCode::BAD_REQUEST,
            format!("{}: {msg}", etiqueta(falta)),
        )
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

    // --------------------------------------------------- red local

    /// Lectura del router que falla al leer los dispositivos o la red: 502 con
    /// el mensaje de siempre (el detalle va al registro).
    fn sin_leer(que: &str, e: hgu::Error) -> ServerFnError {
        tracing::warn!("ajustes: no se puede leer {que}: {e}");
        fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-lectura"))
    }

    /// Lectura de las estaciones asociadas a la radio `radio`.
    fn clave_estaciones(radio: &str) -> Clave {
        Clave::new("cbListWifiAccPtAssociation", &[("dpm_cfg_wifi_id", radio)])
    }

    /// Lista de dispositivos del router.
    fn clave_hosts() -> Clave {
        Clave::new::<&str, &str>("cbGetLanHostList", &[])
    }

    /// Reservas del servidor DHCP.
    fn clave_reservas() -> Clave {
        Clave::new("cbDhcpV4SvrPoolGetStatic", &hgu::read::POOL)
    }

    /// IP y máscara del router en la red de casa.
    fn clave_red_casa() -> Clave {
        Clave::new::<&str, &str>("cbGetLanIPv4Addr", &[])
    }

    /// Lecturas de [`leer_dispositivos`]: la lista, las reservas y las
    /// estaciones de cada radio.
    fn claves_dispositivos() -> Vec<Clave> {
        let mut claves = vec![clave_hosts(), clave_reservas()];
        claves.extend(hgu::read::RADIOS_POR_DEFECTO.map(clave_estaciones));
        claves
    }

    /// Conexión de cada MAC asociada a la WiFi (en minúsculas), por los
    /// puntos de acceso de cada radio: la radio 0 es la de 2,4 GHz y la 1
    /// la de 5 GHz; en cada una, el punto de acceso con su mismo id es la
    /// red principal y los demás son de invitados. `None` si no se pudo
    /// leer alguna radio.
    fn asociados(
        leidas: &HashMap<Clave, Result<XmlNode, hgu::Error>>,
    ) -> Option<HashMap<String, Conexion>> {
        let mut out = HashMap::new();
        for radio in hgu::read::RADIOS_POR_DEFECTO {
            let n = leidas.get(&clave_estaciones(radio))?.as_ref().ok()?;
            for st in hgu::models::WifiAssoc::parse_all(n) {
                let conexion = match (st.ap_id == radio, radio) {
                    (false, _) => Conexion::Invitados,
                    (true, "0") => Conexion::Wifi24,
                    (true, _) => Conexion::Wifi5,
                };
                out.insert(mac_comparable(&st.mac), conexion);
            }
        }
        Some(out)
    }

    /// Orden de la lista: los conectados primero y, dentro, por IP (las que
    /// no son IPv4, al final).
    fn orden(d: &DispositivoVista) -> (bool, u32, String) {
        let ip = d.ip.parse::<Ipv4Addr>().map_or(u32::MAX, u32::from);
        (!d.activo, ip, d.ip.clone())
    }

    /// Respuesta de la lectura `clave` de `leidas`; si falló, 502 con el
    /// mensaje de siempre (el fallo ya lo registró la caché de lecturas).
    fn leida(
        leidas: &mut HashMap<Clave, Result<XmlNode, hgu::Error>>,
        clave: &Clave,
    ) -> Result<XmlNode, ServerFnError> {
        match leidas.remove(clave) {
            Some(Ok(n)) => Ok(n),
            _ => Err(fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-lectura"))),
        }
    }

    pub(super) async fn leer_dispositivos() -> Result<Vec<DispositivoVista>, ServerFnError> {
        let (state, _) = preparar().await?;
        let mut leidas = state
            .lecturas
            .leer(&state.hgu, &state.mascara, &claves_dispositivos())
            .await;
        let wifi = asociados(&leidas);
        let hosts = hgu::models::LanHost::parse_all(&leida(&mut leidas, &clave_hosts())?);
        let reservas = hgu::models::StaticLease::parse_all(&leida(&mut leidas, &clave_reservas())?);
        let mut lista: Vec<DispositivoVista> = hosts
            .into_iter()
            .map(|h| {
                let conexion = match &wifi {
                    _ if !h.active => Conexion::Desconocida,
                    None => Conexion::Desconocida,
                    Some(w) => w
                        .get(&mac_comparable(&h.mac))
                        .cloned()
                        .unwrap_or_else(|| Conexion::Cable(h.iface.clone())),
                };
                let fija = reservas
                    .iter()
                    .find(|r| mac_comparable(&r.mac) == mac_comparable(&h.mac));
                let (reserva, ip_fija) = (fija.map(|r| r.id.clone()), fija.map(|r| r.ip.clone()));
                DispositivoVista {
                    nombre: h.hostname,
                    ip: h.ip,
                    mac: h.mac,
                    conexion,
                    activo: h.active,
                    reserva,
                    ip_fija,
                }
            })
            .collect();
        lista.sort_by_key(orden);
        Ok(lista)
    }

    pub(super) async fn leer_red_casa() -> Result<Option<(Ipv4Addr, Ipv4Addr)>, ServerFnError> {
        let (state, _) = preparar().await?;
        let clave = clave_red_casa();
        let mut leidas = state
            .lecturas
            .leer(&state.hgu, &state.mascara, std::slice::from_ref(&clave))
            .await;
        let n = leida(&mut leidas, &clave)?;
        Ok(hgu::read::lan_ipv4_de(&n))
    }

    pub(super) async fn leer_reservas() -> Result<Vec<String>, ServerFnError> {
        let (state, _) = preparar().await?;
        let clave = clave_reservas();
        let mut leidas = state
            .lecturas
            .leer(&state.hgu, &state.mascara, std::slice::from_ref(&clave))
            .await;
        let n = leida(&mut leidas, &clave)?;
        Ok(hgu::models::StaticLease::parse_all(&n)
            .into_iter()
            .map(|r| r.ip)
            .collect())
    }

    /// Nombre del dispositivo con la MAC `mac` en la lista del router, si lo
    /// da (`None` también si no se puede leer: solo es para el resumen).
    async fn nombre_del_dispositivo(state: &AppState, mac: &str) -> Option<String> {
        let hosts = hgu::read::lan_hosts(&state.hgu)
            .await
            .inspect_err(|e| tracing::warn!("ajustes: no se pueden leer los dispositivos: {e}"))
            .ok()?;
        let mac = mac_comparable(mac);
        hosts
            .into_iter()
            .find(|h| mac_comparable(&h.mac) == mac)
            .map(|h| h.hostname)
            .filter(|n| !n.trim().is_empty())
    }

    /// Comprueba una reserva de `p` con lo que tiene ahora el router y
    /// devuelve el nombre de su dispositivo, si se conoce (para el resumen).
    /// - Al editar o borrar, la reserva tiene que existir (400), la página
    ///   tiene que decir qué MAC espera (`esperados`; si no, 400) y tiene que
    ///   ser esa (si no, 409: otra sesión la cambió).
    /// - Si hay que validarla ([`valida_la_reserva`]), como la página
    ///   ([`validar_reserva`]), con la red y las reservas del router; lo que
    ///   no se cambia es lo de la reserva. Una nueva necesita MAC e IP.
    ///   Error 400 junto al campo. Los formatos ya están validados.
    async fn comprobar_reserva(
        state: &AppState,
        a: &Ajuste,
        p: &PeticionAjuste,
    ) -> Result<Option<String>, ServerFnError> {
        let prefijo = prefijo_i18n(a);
        let reservas = hgu::read::static_leases(&state.hgu)
            .await
            .map_err(|e| sin_leer("las reservas", e))?;
        let actual = match &p.instancia {
            None => None,
            Some(id) => Some(
                reservas
                    .iter()
                    .find(|r| r.id == *id)
                    .ok_or_else(|| fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida")))?,
            ),
        };
        // Al editar o borrar, la MAC que espera la página es obligatoria:
        // sin ella no se sabe si la reserva sigue siendo la misma.
        if let Some(r) = actual {
            let esperada = p.esperados.get("mac").ok_or_else(|| {
                tracing::warn!("ajustes: {}: editar o borrar sin la MAC esperada", a.id);
                fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
            })?;
            if mac_comparable(&r.mac) != mac_comparable(esperada) {
                return Err(fallo(
                    StatusCode::CONFLICT,
                    t!("ajuste-red_local-reservas-error-cambiada"),
                ));
            }
        }
        let valor = |campo: &str, de_la_reserva: Option<&str>| {
            p.valores
                .get(campo)
                .filter(|_| !p.borrar)
                .map(|v| v.trim().to_owned())
                .or_else(|| de_la_reserva.map(str::to_owned))
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    let c = a.campo(campo).expect("campo de la reserva");
                    error_valor(&prefijo, c, &ErrorValor::Vacio)
                })
        };
        let mac = valor("mac", actual.map(|r| r.mac.as_str()))?;
        let detalle = nombre_del_dispositivo(state, &mac).await;
        if !valida_la_reserva(p) {
            return Ok(detalle);
        }
        let ip = valor("ip", actual.map(|r| r.ip.as_str()))?;
        let ip: Ipv4Addr = ip.parse().map_err(|_| {
            let c = a.campo("ip").expect("campo de la reserva");
            error_valor(&prefijo, c, &ErrorValor::Ip)
        })?;
        let lan = hgu::read::lan_ipv4(&state.hgu)
            .await
            .map_err(|e| sin_leer("la red de casa", e))?
            .ok_or_else(|| {
                tracing::warn!("ajustes: el router no da su IP en la red de casa");
                fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-lectura"))
            })?;
        let existentes: Vec<(String, String, String)> =
            reservas.into_iter().map(|r| (r.id, r.mac, r.ip)).collect();
        validar_reserva(&mac, ip, lan, &existentes, p.instancia.as_deref()).map_err(|e| {
            let etiqueta = crate::ui::i18n::traducir(&format!("{prefijo}-{}", e.campo()), None);
            fallo(
                StatusCode::BAD_REQUEST,
                format!("{etiqueta}: {}", e.texto(lan.0)),
            )
        })?;
        Ok(detalle)
    }

    // --------------------------------------------------- puertos

    /// Peticiones en que se parte `p`: una regla de puertos nueva con
    /// «Ambos» ([`AMBOS`]) son dos, una TCP y una UDP, porque el router
    /// admite un solo protocolo por regla; van en el mismo grupo (un
    /// diálogo, un deshacer). El resto, `p` sola.
    pub(crate) fn partes(a: &Ajuste, p: &PeticionAjuste) -> Vec<PeticionAjuste> {
        let ambos = p.valores.get("protocolo").map(|v| v.trim()) == Some(AMBOS);
        if a.componente != Some(Componente::Puertos) || p.borrar || !ambos {
            return vec![p.clone()];
        }
        Protocolo::Ambos
            .claves()
            .iter()
            .map(|clave| {
                let mut q = p.clone();
                q.valores.insert("protocolo".into(), (*clave).to_owned());
                q
            })
            .collect()
    }

    /// Comprueba una regla de puertos de `p` con lo que tiene ahora el
    /// router y devuelve su nombre (para el resumen).
    /// - «Ambos» solo vale al crear (400).
    /// - Al editar o borrar, la regla tiene que existir (400), la página
    ///   tiene que decir lo que la identifica ([`IDENTIFICAN_REGLA`] en
    ///   `esperados`; si falta algo, 400) y tiene que coincidir (si no, 409:
    ///   otra sesión la cambió).
    /// - Si hay que validarla ([`valida_la_regla`]), como la página
    ///   ([`validar_regla`]), con la red y las reglas del router; lo que no
    ///   se cambia es lo de la regla. Una nueva necesita todos sus campos.
    ///   Error 400 junto al campo. Los formatos ya están validados.
    async fn comprobar_regla(
        state: &AppState,
        a: &Ajuste,
        p: &PeticionAjuste,
    ) -> Result<Option<String>, ServerFnError> {
        let prefijo = prefijo_i18n(a);
        let invalida = |motivo: &str| {
            tracing::warn!("ajustes: {}: {motivo}", a.id);
            fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
        };
        let ambos = p.valores.get("protocolo").map(|v| v.trim()) == Some(AMBOS);
        if ambos && p.instancia.is_some() {
            return Err(invalida("«ambos» solo al crear una regla"));
        }
        let params: Vec<(&str, &str)> = a
            .leer
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let n = state
            .hgu
            .call(&a.leer.funcion, &params)
            .await
            .map_err(|e| sin_leer("las reglas de puertos", e))?;
        let instancias = lista(&state.catalogo, a, &prefijo, &n).ok_or_else(|| {
            tracing::warn!("ajustes: {}: reglas sin su id", a.id);
            fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-lectura"))
        })?;
        let actual = match &p.instancia {
            None => None,
            Some(id) => Some(
                instancias
                    .iter()
                    .find(|i| i.id == *id)
                    .ok_or_else(|| invalida("la regla no existe"))?,
            ),
        };
        // Al editar o borrar, lo que identifica la regla es obligatorio:
        // sin ello no se sabe si sigue siendo la misma.
        if let Some(i) = actual {
            for campo in IDENTIFICAN_REGLA {
                let esperado = p
                    .esperados
                    .get(campo)
                    .ok_or_else(|| invalida("editar o borrar sin lo que identifica la regla"))?;
                if esperado.trim() != valor_de(&i.campos, campo) {
                    return Err(fallo(
                        StatusCode::CONFLICT,
                        t!("ajuste-puertos-reglas-error-cambiada"),
                    ));
                }
            }
        }
        let valor = |campo: &str| {
            p.valores
                .get(campo)
                .filter(|_| !p.borrar)
                .map(|v| v.trim().to_owned())
                .or_else(|| actual.map(|i| valor_de(&i.campos, campo)))
                .filter(|v| !v.is_empty())
        };
        let detalle = valor("nombre");
        if !valida_la_regla(p) {
            return Ok(detalle);
        }
        let campo = |id: &str| a.campo(id).expect("campo de la regla");
        let exigir = |id: &str| {
            valor(id).ok_or_else(|| error_valor(&prefijo, campo(id), &ErrorValor::Vacio))
        };
        let puerto = |id: &str| {
            exigir(id)?.parse::<u16>().map_err(|_| {
                let c = campo(id);
                let (min, max) = (c.validar.and_then(|v| v.min), c.validar.and_then(|v| v.max));
                error_valor(&prefijo, c, &ErrorValor::FueraDeRango { min, max })
            })
        };
        exigir("nombre")?;
        let regla = ReglaPuertos {
            id: p.instancia.clone(),
            nombre: detalle.clone().unwrap_or_default(),
            protocolo: Protocolo::de_clave(&exigir("protocolo")?)
                .ok_or_else(|| error_valor(&prefijo, campo("protocolo"), &ErrorValor::Opcion))?,
            externo: (puerto("externo_desde")?, puerto("externo_hasta")?),
            interno: (puerto("interno_desde")?, puerto("interno_hasta")?),
            ip: exigir("destino")?
                .parse()
                .map_err(|_| error_valor(&prefijo, campo("destino"), &ErrorValor::Ip))?,
            activa: valor("activa").as_deref() == Some("true"),
        };
        let existentes: Vec<ReglaPuertos> = instancias.iter().filter_map(regla_de).collect();
        let lan = hgu::read::lan_ipv4(&state.hgu)
            .await
            .map_err(|e| sin_leer("la red de casa", e))?
            .ok_or_else(|| {
                tracing::warn!("ajustes: el router no da su IP en la red de casa");
                fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-lectura"))
            })?;
        validar_regla(&regla, &existentes, lan).map_err(|e| {
            let etiqueta = crate::ui::i18n::traducir(&format!("{prefijo}-{}", e.campo()), None);
            fallo(
                StatusCode::BAD_REQUEST,
                format!("{etiqueta}: {}", e.texto(lan.0)),
            )
        })?;
        Ok(detalle)
    }

    pub(super) async fn preparar_ajuste(p: PeticionAjuste) -> Result<GrupoVista, ServerFnError> {
        let (state, _) = preparar().await?;
        let a = ajuste_por_id(&state, &p.ajuste)?;
        if p.borrar && !a.lista {
            return Err(fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida")));
        }
        let prefijo = prefijo_i18n(a);
        let partes = partes(a, &p);
        let campos = partes
            .iter()
            .flat_map(|q| &q.valores)
            .map(|(id, v)| {
                a.campo(id)
                    .map(|c| (c, v))
                    .ok_or_else(|| fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-campo")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Antes que los valores: «sin elegir» en el campo exigido es que
        // falta, no una opción que no vale.
        if !p.borrar
            && let Some((pide, falta)) = falta_requerido(a, &p.valores)
        {
            return Err(error_requiere(&prefijo, pide, falta));
        }
        for (c, v) in campos {
            pares(c, v).map_err(|e| error_valor(&prefijo, c, &e))?;
        }
        let g = gestor(&state)?;
        libre(&g)?;
        let detalle = match a.componente {
            Some(Componente::Reservas) => comprobar_reserva(&state, a, &p).await?,
            Some(Componente::Puertos) => comprobar_regla(&state, a, &p).await?,
            _ => None,
        };
        let mut grupo = GrupoRequest {
            pasos: Vec::new(),
            titulo: None,
        };
        // Con varias partes («TCP y UDP»), cada paso se titula con su
        // protocolo.
        let mut titulos: Vec<Option<String>> = Vec::new();
        for q in &partes {
            let g = grupo_de(a, q).map_err(|e| {
                tracing::warn!("ajustes: {}: valor no válido tras validarlo: {e}", a.id);
                fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
            })?;
            let titulo = (partes.len() > 1)
                .then(|| q.valores.get("protocolo"))
                .flatten()
                .map(|v| format!("{prefijo}-protocolo-{v}"));
            titulos.extend(std::iter::repeat_n(titulo, g.pasos.len()));
            grupo.pasos.extend(g.pasos);
            grupo.titulo = g.titulo;
        }
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
        exigir_tapados(&state.catalogo, a, &partes, &grupo)?;
        let pedidos: Vec<BTreeMap<String, String>> =
            grupo.pasos.iter().map(|p| p.req.valores.clone()).collect();
        let prep = g.preparar_grupo(grupo).await.map_err(error_ajuste)?;
        let mut vista = vista_grupo(&g, a, prep.token, &prep.pasos, prep.riesgo, &|i, param| {
            buscar(&pedidos[i], param).map(str::to_owned)
        });
        titular_partes(&mut vista.pasos, &titulos, &prefijo);
        Ok(GrupoVista { detalle, ..vista })
    }

    /// Cada paso que escribe una función con un secreto tapado al leer lo
    /// reenvía: tiene que venir escrito en la petición (si no, 400 junto al
    /// campo, con el motivo). Al borrar no se reenvía.
    fn exigir_tapados(
        f: &Catalogo,
        a: &Ajuste,
        partes: &[PeticionAjuste],
        g: &GrupoRequest,
    ) -> Result<(), ServerFnError> {
        let prefijo = prefijo_i18n(a);
        for c in a.campos.iter().filter(|c| tapado(f, c)) {
            let Some(e) = c.escribible() else {
                continue;
            };
            let reenvia = g
                .pasos
                .iter()
                .any(|p| p.req.funcion == e.funcion && !p.req.borrar);
            let escrito = partes.iter().all(|q| {
                q.valores
                    .get(&c.id)
                    .is_some_and(|v| !v.is_empty() && v != MASCARA)
            });
            if reenvia && !escrito {
                let etiqueta = crate::ui::i18n::traducir(&format!("{prefijo}-{}", c.id), None);
                return Err(fallo(
                    StatusCode::BAD_REQUEST,
                    format!("{etiqueta}: {}", t!("ajustes-secreto-tapado-pedir")),
                ));
            }
        }
        Ok(())
    }

    /// Pone a cada paso su título (`titulos`, por índice). Los pasos
    /// titulados después del primero solo enseñan lo que cambia respecto a
    /// él (en «TCP y UDP», el protocolo), con una nota que lo dice.
    fn titular_partes(pasos: &mut [PasoVista], titulos: &[Option<String>], prefijo: &str) {
        let Some(primero) = pasos.first().map(|p| p.campos.clone()) else {
            return;
        };
        for (i, paso) in pasos.iter_mut().enumerate() {
            paso.titulo = titulos.get(paso.indice).cloned().flatten();
            if i > 0 && paso.titulo.is_some() {
                paso.campos.retain(|c| !primero.contains(c));
                paso.nota = Some(format!("{prefijo}-paso-igual"));
            }
        }
    }

    // --------------------------------------------------- secretos

    /// Elemento `nombre` de una lectura: primero exacto, luego sin distinguir
    /// mayúsculas.
    fn leido<'a>(leidos: &'a Leidos, nombre: &str) -> Option<&'a str> {
        leidos
            .get(nombre)
            .or_else(|| {
                leidos
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(nombre))
                    .map(|(_, v)| v)
            })
            .map(String::as_str)
    }

    pub(super) async fn revelar_ajuste(
        ajuste: String,
        campo: String,
        instancia: Option<String>,
        contrasena: String,
    ) -> Result<String, ServerFnError> {
        let (state, user) = preparar().await?;
        let a = ajuste_por_id(&state, &ajuste)?;
        // Uno tapado al leer no tiene nada que mostrar.
        let c = a
            .campo(&campo)
            .filter(|c| c.tipo == TipoCampo::Secreto && !tapado(&state.catalogo, c))
            .ok_or_else(|| fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-campo")))?;
        if a.lista != instancia.is_some() {
            return Err(fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida")));
        }
        let user = usuario_para_revelar(&state, user, &contrasena).await?;
        // Del router, no de la caché: lo que se ve es lo que tiene ahora.
        let params: Vec<(&str, &str)> = a
            .leer
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let n = state
            .hgu
            .call(&a.leer.funcion, &params)
            .await
            .map_err(|e| sin_leer("un secreto", e))?;
        let leidos = match &instancia {
            None => {
                let mut leidos = Leidos::new();
                hojas(&n, &mut leidos);
                Some(leidos)
            }
            Some(id) => instancias_con_id(&state.catalogo, a, &n)
                .into_iter()
                .flatten()
                .find(|(i, _)| i == id)
                .map(|(_, leidos)| leidos),
        };
        let valor = leidos
            .as_ref()
            .and_then(|l| leido(l, &c.lee))
            .map(str::to_owned)
            .ok_or_else(|| fallo(StatusCode::NOT_FOUND, t!("ajustes-error-secreto-sin-valor")))?;
        sqlx::query(
            "INSERT INTO reveals (ts, user_id, funcion, campo, instancia, ajuste) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(crate::series::now())
        .bind(user.id)
        .bind(&a.leer.funcion)
        .bind(&c.id)
        .bind(&instancia)
        .bind(&a.id)
        .execute(&state.db)
        .await
        .map_err(crate::instalar::servidor::interno)?;
        tracing::info!(
            usuario = %user.username,
            ajuste = %a.id,
            campo = %c.id,
            instancia = instancia.as_deref().unwrap_or("-"),
            "secreto mostrado desde Ajustes"
        );
        Ok(valor)
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
    pub(crate) async fn ajuste_del_cambio<'a>(
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
        // Antes que «otro cambio en curso»: un cambio que aún se vigila (el
        // activo) o que ya se deshizo no se puede deshacer.
        if !g.se_puede_deshacer(cambio).await {
            return Err(fallo(
                StatusCode::BAD_REQUEST,
                t!("ajustes-error-no-deshacer"),
            ));
        }
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
                esperados: BTreeMap::new(),
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

        /// La caché guarda las lecturas enmascaradas: lo que se pinta con
        /// ellas (cada ajuste del catálogo, los dispositivos, las reservas,
        /// las estaciones WiFi y la red de casa) es igual que con la
        /// respuesta en claro del router.
        #[tokio::test]
        async fn lo_enmascarado_se_pinta_igual_que_lo_leido() {
            let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/../hgu/fixtures");
            let sim =
                router_sim::Sim::start(router_sim::SimConfig::new("u", "clave", fixtures)).await;
            let mut cfg = hgu::ClientConfig::new(&sim.base_url, "u", "clave");
            cfg.min_gap = std::time::Duration::from_millis(1);
            let c = hgu::Client::new(cfg);
            let f = Catalogo::load().unwrap();
            let m = catalogo::mask::Mascara::load().unwrap();
            let ajustes = CatalogoAjustes::load().unwrap();
            let leer = async |clave: &Clave| {
                let p: Vec<(&str, &str)> = clave
                    .params
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                c.call(&clave.funcion, &p).await.unwrap()
            };
            for a in ajustes.todos() {
                let n = leer(&clave(a)).await;
                assert_eq!(
                    vista(&f, a, Some(&n)),
                    vista(&f, a, Some(&m.enmascarar(&n))),
                    "{}",
                    a.id
                );
            }
            use hgu::models::{LanHost, StaticLease, WifiAssoc};
            let n = leer(&clave_hosts()).await;
            assert!(!LanHost::parse_all(&n).is_empty());
            assert_eq!(
                LanHost::parse_all(&n),
                LanHost::parse_all(&m.enmascarar(&n))
            );
            let n = leer(&clave_reservas()).await;
            assert!(!StaticLease::parse_all(&n).is_empty());
            assert_eq!(
                StaticLease::parse_all(&n),
                StaticLease::parse_all(&m.enmascarar(&n))
            );
            for radio in hgu::read::RADIOS_POR_DEFECTO {
                let n = leer(&clave_estaciones(radio)).await;
                assert_eq!(
                    WifiAssoc::parse_all(&n),
                    WifiAssoc::parse_all(&m.enmascarar(&n))
                );
            }
            let n = leer(&clave_red_casa()).await;
            assert!(hgu::read::lan_ipv4_de(&n).is_some());
            assert_eq!(
                hgu::read::lan_ipv4_de(&n),
                hgu::read::lan_ipv4_de(&m.enmascarar(&n))
            );
        }
    }
}
