//! El token de un cambio preparado: a quién va ligado y si quedaría sin nada que aplicar.

use super::*;

impl ChangeManager {
    /// Liga el cambio preparado con `token` al usuario del panel `usuario`:
    /// solo él lo confirma; otro recibe `TokenInvalido` y el token sigue
    /// valiendo. Un token sin ligar no lo confirma nadie: lo ligan las
    /// server functions que preparan, con el usuario de la sesión, y quien
    /// use el gestor directamente.
    pub fn ligar(&self, token: &str, usuario: &str) {
        if let Some(p) = lock(&self.pendientes).get_mut(token) {
            p.usuario = Some(usuario.to_owned());
        }
    }

    /// ¿Se quedaría sin escribir nada el grupo preparado con `token` sin los
    /// pasos de `desmarcados`? Es decir, todos los pasos con cambios llegan
    /// desmarcados. No consume el token; con uno que no vale, `false` (lo
    /// dice [`ChangeManager::confirmar_grupo`]).
    pub fn nada_que_aplicar(&self, token: &str, desmarcados: &[usize]) -> bool {
        lock(&self.pendientes).get(token).is_some_and(|v| {
            v.sin_cambios
                .iter()
                .enumerate()
                .all(|(i, sin)| *sin || desmarcados.contains(&i))
        })
    }
}
