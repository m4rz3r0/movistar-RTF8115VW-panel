//! Comprobar las credenciales de otro usuario de la web del router: el inicio de sesión básico con las del panel y solo el avanzado con las suyas.
//!
//! En el router real, la página básica (`te_acceso_router.cgi`) solo admite
//! el usuario fijo de la web (el del panel): su formulario no tiene campo
//! de usuario, va oculto. Otro usuario solo se puede comprobar en el
//! inicio de sesión avanzado (`cbCheckLogin`), que responde `Result=4` si
//! el usuario o la contraseña no valen.

use super::Client;
use crate::error::Error;

/// Usuario y contraseña de un inicio de sesión (en claro).
#[derive(Clone, Copy)]
pub struct Credenciales<'a> {
    pub usuario: &'a str,
    pub contrasena: &'a str,
}

impl std::fmt::Debug for Credenciales<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credenciales")
            .field("usuario", &self.usuario)
            .field("contrasena", &"***")
            .finish()
    }
}

impl Client {
    /// Inicia sesión con `basica` en la página básica y con `avanzada` en
    /// `cbCheckLogin`, sin leer nada más: comprueba que `avanzada` entra.
    /// Usa la sesión de este cliente (que conviene que sea uno aparte y
    /// cerrarlo con [`Client::logout`] al acabar). Error si el router
    /// rechaza alguna de las dos.
    pub async fn entrar_como(
        &self,
        basica: Credenciales<'_>,
        avanzada: Credenciales<'_>,
    ) -> Result<(), Error> {
        let mut st = self.shared.state.lock().await;
        Self::invalidate(&mut st);
        self.login_basico(&mut st, basica.usuario, basica.contrasena)
            .await?;
        if self
            .login_avanzado(&mut st, avanzada.usuario, avanzada.contrasena)
            .await?
        {
            Ok(())
        } else {
            Err(Error::SessionExpired(
                "el router cerró la sesión básica al entrar".into(),
            ))
        }
    }
}
