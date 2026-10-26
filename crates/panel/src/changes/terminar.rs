//! «Todo bien, terminar»: quien hizo un cambio da por buena su vigilancia
//! antes de que acabe el plazo.
//!
//! Solo mientras el gestor está de verdad vigilando (no al esperar la
//! vuelta de un reinicio ni al revertir), solo quien lo hizo y solo si la
//! última comprobación salió bien: con el router sin responder, terminar
//! podría dejar puesto justo lo que lo dejó sin responder. El cambio queda
//! `aplicado` con una nota que dice quién terminó la vigilancia, y la cola
//! sigue con el siguiente.

use super::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// Una petición de [`ChangeManager::terminar`]: quién la hace y por dónde
/// se le responde (`Ok` cuando el cambio ya está guardado como aplicado).
pub(super) type Terminada = (String, oneshot::Sender<Result<(), ErrorTerminar>>);

/// La vigilancia en curso, para poder terminarla.
pub(super) struct EnVigilancia {
    pub(super) id: i64,
    /// Fallos seguidos en las comprobaciones hasta ahora.
    pub(super) seguidos: Arc<AtomicU32>,
    terminar: Option<oneshot::Sender<Terminada>>,
}

/// Por qué no se puede terminar una vigilancia.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorTerminar {
    /// El cambio no se está vigilando (ya terminó, se está revirtiendo, se
    /// espera la vuelta de un reinicio…).
    NoSeVigila,
    /// Lo hizo otro usuario del panel.
    OtroUsuario,
    /// La última comprobación falló: el router no responde o internet se
    /// ha caído.
    Fallando,
}

impl ErrorTerminar {
    /// Clave i18n del motivo.
    pub fn clave(self) -> &'static str {
        match self {
            ErrorTerminar::NoSeVigila => "cambio-terminar-no-vigila",
            ErrorTerminar::OtroUsuario => "cambio-terminar-otro-usuario",
            ErrorTerminar::Fallando => "cambio-terminar-fallando",
        }
    }
}

/// Al salir de la vigilancia (también si la tarea se aborta), el hueco se
/// vacía si sigue siendo el de este cambio.
pub(super) struct FinVigilancia<'a> {
    m: &'a ChangeManager,
    id: i64,
}

impl Drop for FinVigilancia<'_> {
    fn drop(&mut self) {
        let mut v = lock(&self.m.vigilancia);
        if v.as_ref().is_some_and(|v| v.id == self.id) {
            *v = None;
        }
    }
}

impl ChangeManager {
    /// Empieza a vigilar el cambio `id`: desde ahora se puede terminar.
    /// Devuelve el contador de fallos, por dónde llega la petición de
    /// terminar y lo que vacía el hueco al acabar.
    pub(super) fn abrir_vigilancia(
        &self,
        id: i64,
    ) -> (
        Arc<AtomicU32>,
        oneshot::Receiver<Terminada>,
        FinVigilancia<'_>,
    ) {
        let (tx, rx) = oneshot::channel();
        let seguidos = Arc::new(AtomicU32::new(0));
        *lock(&self.vigilancia) = Some(EnVigilancia {
            id,
            seguidos: seguidos.clone(),
            terminar: Some(tx),
        });
        (seguidos, rx, FinVigilancia { m: self, id })
    }

    /// Vuelve a aceptar peticiones de terminar la vigilancia de `id` (tras
    /// rechazar una porque la comprobación en curso falló).
    pub(super) fn rearmar_vigilancia(&self, id: i64) -> Option<oneshot::Receiver<Terminada>> {
        let mut v = lock(&self.vigilancia);
        let v = v.as_mut().filter(|v| v.id == id)?;
        let (tx, rx) = oneshot::channel();
        v.terminar = Some(tx);
        Some(rx)
    }

    /// ¿Puede `usuario` terminar ya la vigilancia del cambio `id`? (lo que
    /// enseña el botón; [`ChangeManager::terminar`] lo vuelve a comprobar).
    pub async fn terminable(&self, id: i64, usuario: &str) -> bool {
        let vigilando = lock(&self.vigilancia).as_ref().is_some_and(|v| v.id == id);
        vigilando
            && self
                .cambio(id)
                .await
                .is_some_and(|c| c.usuario.as_deref() == Some(usuario))
    }

    /// Termina ya la vigilancia del cambio `id`, que hizo `usuario`: queda
    /// `aplicado`, con la nota de quién la terminó. Vuelve cuando ya está
    /// guardado.
    pub async fn terminar(&self, id: i64, usuario: &str) -> Result<(), ErrorTerminar> {
        let c = self.cambio(id).await.ok_or(ErrorTerminar::NoSeVigila)?;
        if c.estado != Estado::Vigilando {
            return Err(ErrorTerminar::NoSeVigila);
        }
        if c.usuario.as_deref() != Some(usuario) {
            return Err(ErrorTerminar::OtroUsuario);
        }
        let (hecho, esperar) = oneshot::channel();
        {
            let mut v = lock(&self.vigilancia);
            let v = v
                .as_mut()
                .filter(|v| v.id == id)
                .ok_or(ErrorTerminar::NoSeVigila)?;
            if v.seguidos.load(Ordering::SeqCst) > 0 {
                return Err(ErrorTerminar::Fallando);
            }
            let tx = v.terminar.take().ok_or(ErrorTerminar::NoSeVigila)?;
            tx.send((usuario.to_owned(), hecho))
                .map_err(|_| ErrorTerminar::NoSeVigila)?;
        }
        // La tarea del cambio lo guarda enseguida; si no, la vigilancia ya
        // había acabado por su cuenta.
        match tokio::time::timeout(Duration::from_secs(10), esperar).await {
            Ok(Ok(r)) => r,
            _ => Err(ErrorTerminar::NoSeVigila),
        }
    }
}
