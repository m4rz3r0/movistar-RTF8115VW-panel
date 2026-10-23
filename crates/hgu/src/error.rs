//! Errores del cliente del router.

use thiserror::Error;

/// Errores de `Client`. Nunca contienen la contraseña.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// Fallo de red o de transporte HTTP.
    #[error("error de red: {0}")]
    Network(String),
    /// El router rechazó las credenciales.
    #[error("autenticación rechazada: {0}")]
    Auth(&'static str),
    /// La sesión sigue caducada tras volver a iniciar sesión.
    #[error("sesión caducada: {0}")]
    SessionExpired(String),
    /// El router respondió con `Result` distinto de 0.
    #[error("la llamada {name} falló (Result={result}): {error}")]
    CallFailed {
        /// Nombre de la llamada.
        name: String,
        /// Valor de `Result`.
        result: String,
        /// Texto de `Error`, si lo hay.
        error: String,
    },
    /// Todavía no se han dado la dirección y las credenciales del router.
    #[error("router sin configurar")]
    NoConfigurado,
    /// Respuesta que no se esperaba.
    #[error("respuesta inesperada: {0}")]
    Unexpected(String),
    /// Ninguna posición de una lista tiene la instancia que se busca por su
    /// id ([`Client::call_por_id`](crate::Client::call_por_id)).
    #[error("instancia no encontrada: {0}")]
    SinInstancia(String),
}
