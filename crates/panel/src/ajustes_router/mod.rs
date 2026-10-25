//! Ajustes del router por tema: lectura con valores legibles
//! ([`leer_tema`]) y escritura de un ajuste como un grupo de cambios del
//! gestor ([`preparar_ajuste`], [`confirmar_ajuste`], [`deshacer_ajuste`]),
//! con un resumen legible «antes → después» ([`GrupoVista`]).
//!
//! Los tipos de aquí los usan las páginas de Ajustes en el navegador. Solo
//! llevan ids, claves de traducción y valores legibles: ningún nombre de la
//! API salvo en [`DetallesTecnicos`], que la interfaz pinta únicamente dentro
//! de `<details class="detalles-tecnicos">`. Un secreto llega como
//! [`ValorLegible::Secreto`](ajustes::ValorLegible::Secreto), y su valor aparte, en
//! [`CampoVista::valor_secreto`] (la página lo enseña censurado, con un ojo).

mod mejoras;
mod reglas;
mod reservas;
mod vistas;

pub use mejoras::*;
pub use reglas::*;
pub use reservas::*;
pub use vistas::*;

use leptos::prelude::*;
use leptos::server_fn::codec::Json;

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

/// Prepara una acción sobre varios elementos del tema (`quitar_wps` en dos
/// redes): un único grupo del gestor con un paso por elemento, que se
/// confirma con [`confirmar_ajuste`] y se deshace entero con
/// [`deshacer_ajuste`]. Un elemento que ya tiene el valor sale como paso
/// sin cambios («ya estaba así») y no se escribe. Errores con el mensaje
/// traducido: 400 con más de [`MAX_ELEMENTOS_ACCION`] elementos o si la
/// acción no se aplica a alguno; 409 si ninguno cambia o hay otro cambio
/// en curso; 403 en solo lectura. Exige sesión.
#[server(endpoint = "preparar_accion", input = Json)]
pub async fn preparar_accion(
    tema: String,
    accion: String,
    elementos: Vec<String>,
) -> Result<GrupoVista, ServerFnError> {
    servidor::preparar_accion(tema, accion, elementos).await
}

/// Prepara el guardado del detalle de un elemento: los cambios de varios
/// ajustes suyos (lo esencial y lo avanzado de una red) en un solo grupo,
/// con un paso por escritura titulado con su ajuste, que se confirma con
/// [`confirmar_ajuste`] y se deshace entero. Errores como
/// [`preparar_ajuste`]; 400 con una lista o un componente propio.
#[server(endpoint = "preparar_elemento", input = Json)]
pub async fn preparar_elemento(
    peticiones: Vec<PeticionAjuste>,
) -> Result<GrupoVista, ServerFnError> {
    servidor::preparar_elemento(peticiones).await
}

/// Prepara «No responder al ping desde internet»: la regla propia del panel
/// en el cortafuegos (ICMP `echo_request` descartado en el filtro de
/// entrada IPv4 de la conexión a internet), que se confirma con
/// [`confirmar_ajuste`] y se deshace con [`deshacer_ajuste`] (borra solo su
/// regla). 409 si el cortafuegos no se puede leer, la regla ya está, la
/// función espera su validación en el router real o hay otro cambio en
/// curso; 403 en solo lectura. Exige sesión.
#[server(endpoint = "preparar_sin_ping", input = Json)]
pub async fn preparar_sin_ping() -> Result<GrupoVista, ServerFnError> {
    servidor::preparar_sin_ping().await
}

/// Como mucho, elementos en una acción sobre varios.
pub const MAX_ELEMENTOS_ACCION: usize = 20;

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

/// La página del tema `tema` (`wifi`, `puertos`…) entera: el resumen, los
/// elementos con su estado, sus datos, su recomendación principal y el
/// detalle de sus ajustes con el valor actual del router, y las acciones
/// sobre varios elementos. Cada lectura distinta se hace una sola vez, de
/// la caché corta si está ([`crate::lecturas`]) y, las que faltan, seguidas
/// en una tanda; si una falla, solo los elementos que la usan quedan
/// [`EstadoLectura::SinLeer`]. 404 si el tema no existe. Exige sesión; la
/// respuesta va con `no-store`.
///
/// Con `fresca` («Volver a leer»), lo de este tema no sale de la caché.
#[server(endpoint = "leer_tema", input = Json)]
pub async fn leer_tema(
    tema: String,
    #[server(default)] fresca: bool,
) -> Result<PaginaTema, ServerFnError> {
    servidor::leer_tema(tema, fresca).await
}

/// Lo que ha cambiado desde la foto de partida (la lectura del asistente
/// de instalación), con el router de ahora (lecturas para mostrar, como
/// [`leer_tema`]). Sin foto, [`ComparacionVista::momento`] es `None`. Los
/// secretos se comparan aquí y solo viajan los que difieren, para su campo
/// con el ojo. Exige sesión; la respuesta va con `no-store`.
#[server(endpoint = "comparar_con_partida", input = Json)]
pub async fn comparar_con_partida() -> Result<ComparacionVista, ServerFnError> {
    servidor::comparar_con_partida().await
}

/// Prepara el cambio que vuelve el campo `campo` del ajuste `ajuste` (o,
/// sin campo, todos los que se pueden) al valor de la foto de partida, como
/// [`preparar_ajuste`]: se confirma con [`confirmar_ajuste`] y el resumen
/// enmascara los secretos. Las listas no se restauran (400), ni un valor
/// que el panel no reconoce o un secreto que el router no dejaba leer.
/// `clave` es la que el router no deja leer y va en la misma escritura
/// ([`Diferencia::Valor`]`::pide`): sin ella, el router no escribe.
#[server(endpoint = "preparar_volver_a_partida", input = Json)]
pub async fn preparar_volver_a_partida(
    ajuste: String,
    campo: Option<String>,
    instancia: Option<String>,
    clave: Option<String>,
) -> Result<GrupoVista, ServerFnError> {
    servidor::preparar_volver_a_partida(ajuste, campo, instancia, clave).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// El valor de un secreto no sale en el `Debug` de su campo (ni en el de
    /// lo que lo lleva, como una comparación con la foto de partida).
    #[test]
    fn debug_del_campo_no_ensena_el_secreto() {
        let c = CampoVista {
            id: "contrasena".into(),
            clave: "ajuste-wifi-red_5-contrasena".into(),
            tipo: ajustes::TipoCampo::Secreto,
            valor: ajustes::ValorLegible::Secreto,
            opciones: Vec::new(),
            min: None,
            max: None,
            ascii: true,
            opcional: false,
            solo_lectura: None,
            tapado: false,
            valor_secreto: Some("clave-de-prueba-9".into()),
            recomendado: None,
            motivo_recomendado: None,
            aceptados: Vec::new(),
            requiere: None,
            sin_eleccion: false,
            fuentes: Vec::new(),
            respaldo: None,
        };
        let d = Diferencia::Valor {
            partida: Box::new(c.clone()),
            actual: Box::new(c),
            restaurable: true,
            pide: None,
        };
        let texto = format!("{d:?}");
        assert!(!texto.contains("clave-de-prueba-9"), "{texto}");
        assert!(
            texto.contains("contrasena") && texto.contains("(oculto)"),
            "{texto}"
        );
    }

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
pub(crate) mod servidor;
