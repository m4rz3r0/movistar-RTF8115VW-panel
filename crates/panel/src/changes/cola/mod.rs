//! Cola de cambios: un cambio confirmado mientras otro se aplica o se
//! vigila no se rechaza; espera su turno.
//!
//! - Al confirmar con el hueco ocupado por la tarea de un cambio, el token
//!   se gasta y el cambio se guarda `en_cola` (con lo que mostró al
//!   prepararlo): ya no caduca. Como mucho [`MAX_COLA`] esperando; después,
//!   [`CambioError::ColaLlena`].
//! - Cuando el cambio en curso queda `aplicado` (al acabar la vigilancia,
//!   también si se termina a mano), la misma tarea sigue con el primero de
//!   la cola. Justo antes de escribir, con el sondeo en pausa, vuelve a
//!   calcular lo que va a cambiar: si no sale igual que al revisarlo (el
//!   router cambió), no se escribe nada y queda `cancelado`.
//! - Si un cambio no queda aplicado (se revirtió, falló, se canceló), la
//!   cola se para: los que esperaban quedan `cancelado` con el motivo («no
//!   se aplicó porque el cambio anterior se deshizo»…).
//! - Quien encoló un cambio puede quitarlo mientras espera
//!   ([`ChangeManager::quitar_de_la_cola`]).
//! - La cola vive en memoria: si el panel se detiene o se reinicia, lo que
//!   esperaba queda `cancelado` (nunca se aplica a ciegas al arrancar).
//! - Detrás de un cambio de credenciales del router
//!   ([`ChangeManager::excluir_cambios`]) no se encola: `EnCurso`, como
//!   antes.

mod filas;

use super::*;
use std::collections::VecDeque;

/// Cuántos cambios pueden esperar en la cola.
pub const MAX_COLA: usize = 5;

/// Lo que un paso iba a cambiar al prepararlo: si el router sigue igual,
/// al volver a calcularlo sale lo mismo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::changes) struct Huella {
    pub(in crate::changes) diff: Diff,
    pub(in crate::changes) operacion: Operacion,
    pub(in crate::changes) instancia: Option<String>,
}

impl Huella {
    pub(in crate::changes) fn de(plan: &Plan) -> Huella {
        Huella {
            diff: visible(&plan.campos),
            operacion: plan.operacion,
            instancia: plan.instancia.clone(),
        }
    }
}

/// Un cambio confirmado que va a aplicarse.
pub(in crate::changes) struct Turno {
    /// Su fila `en_cola`; `None` si no pasó por la cola.
    pub(in crate::changes) fila: Option<i64>,
    pub(in crate::changes) pendiente: Pendiente,
    pub(in crate::changes) desmarcados: Vec<usize>,
    pub(in crate::changes) usuario: String,
}

/// El hueco del gestor: el cambio en curso y la cola detrás de él.
#[derive(Default)]
pub(in crate::changes) struct Hueco {
    /// `Some(None)`: reservado y aún sin id.
    pub(in crate::changes) activo: Option<Option<i64>>,
    /// Lo tiene la tarea de un cambio, que al terminar sigue con la cola:
    /// se puede encolar detrás.
    pub(in crate::changes) con_cola: bool,
    /// Los que esperan, en orden.
    pub(in crate::changes) cola: VecDeque<Turno>,
    /// Los que se están guardando para entrar en la cola: cuentan para el
    /// tope.
    encolando: usize,
}

impl Hueco {
    /// ¿Cabe otro cambio? Con el hueco libre, sí; ocupado, si se puede
    /// encolar y la cola no está llena.
    fn admite(&self) -> Result<(), CambioError> {
        match self.activo {
            None => Ok(()),
            Some(_) if !self.con_cola => Err(CambioError::EnCurso),
            Some(_) if self.cola.len() + self.encolando >= MAX_COLA => Err(CambioError::ColaLlena),
            Some(_) => Ok(()),
        }
    }
}

/// Cómo entra un cambio que se confirma.
pub(in crate::changes) enum Entrada {
    /// El hueco estaba libre: se aplica ya.
    Ahora(Reserva),
    /// A la cola (ya cuenta para el tope).
    Cola,
}

/// Por qué no se puede quitar un cambio de la cola.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorQuitar {
    /// No está esperando (ya se está aplicando, ya terminó o no existe).
    NoEnCola,
    /// Lo encoló otro usuario del panel.
    OtroUsuario,
}

impl ErrorQuitar {
    /// Clave i18n del motivo.
    pub fn clave(self) -> &'static str {
        match self {
            ErrorQuitar::NoEnCola => "cambio-quitar-no-en-cola",
            ErrorQuitar::OtroUsuario => "cambio-quitar-otro-usuario",
        }
    }
}

/// Respuesta para quien confirmó: el id del cambio o por qué no se aplicó.
type Aviso = oneshot::Sender<Result<i64, CambioError>>;

impl ChangeManager {
    /// ¿Se puede preparar un cambio ahora? Con otro en curso, sí mientras
    /// quepa en la cola; con la cola llena, [`CambioError::ColaLlena`]; con
    /// el gestor apartado (credenciales del router), `EnCurso`.
    pub fn admite_cambio(&self) -> Result<(), CambioError> {
        lock(&self.hueco).admite()
    }

    /// Ids de los cambios que esperan en la cola, en orden.
    pub fn en_cola(&self) -> Vec<i64> {
        lock(&self.hueco)
            .cola
            .iter()
            .filter_map(|t| t.fila)
            .collect()
    }

    /// Puesto (desde 1) del cambio `id` en la cola y quién lo encoló.
    pub fn puesto_en_cola(&self, id: i64) -> Option<(usize, String)> {
        let h = lock(&self.hueco);
        h.cola
            .iter()
            .enumerate()
            .find(|(_, t)| t.fila == Some(id))
            .map(|(i, t)| (i + 1, t.usuario.clone()))
    }

    /// Reserva el hueco o un sitio en la cola.
    pub(in crate::changes) fn entrar(self: &Arc<Self>) -> Result<Entrada, CambioError> {
        let mut h = lock(&self.hueco);
        if h.activo.is_none() {
            h.activo = Some(None);
            h.con_cola = true;
            return Ok(Entrada::Ahora(Reserva {
                m: self.clone(),
                suelta: false,
            }));
        }
        h.admite()?;
        h.encolando += 1;
        Ok(Entrada::Cola)
    }

    /// Devuelve el sitio de la cola reservado con [`Self::entrar`] sin
    /// llegar a encolar nada.
    pub(in crate::changes) fn devolver_sitio(&self) {
        let mut h = lock(&self.hueco);
        h.encolando = h.encolando.saturating_sub(1);
    }

    /// Encola `t` (con su sitio ya reservado): lo guarda `en_cola` y
    /// devuelve su id. Si mientras se guardaba el hueco quedó libre, se
    /// aplica ya, como sin cola.
    pub(in crate::changes) async fn encolar(
        self: &Arc<Self>,
        mut t: Turno,
    ) -> Result<i64, CambioError> {
        let fila = self.insertar_en_cola(&t).await;
        enum Destino {
            Cola(i64),
            Ahora(Reserva, Box<Turno>),
            Apartado(i64),
        }
        let destino = {
            let mut h = lock(&self.hueco);
            h.encolando = h.encolando.saturating_sub(1);
            let id = fila?;
            t.fila = Some(id);
            if h.activo.is_some() && h.con_cola {
                tracing::info!(cambio = id, usuario = t.usuario, "cambio en cola");
                h.cola.push_back(t);
                Destino::Cola(id)
            } else if h.activo.is_none() {
                // El hueco quedó libre mientras se guardaba: se aplica ya.
                h.activo = Some(Some(id));
                h.con_cola = true;
                let reserva = Reserva {
                    m: self.clone(),
                    suelta: false,
                };
                Destino::Ahora(reserva, Box::new(t))
            } else {
                Destino::Apartado(id)
            }
        };
        match destino {
            Destino::Cola(id) => Ok(id),
            Destino::Ahora(reserva, t) => {
                let (tx, rx) = oneshot::channel();
                self.lanzar(reserva, *t, Some(tx));
                rx.await.unwrap_or_else(|_| Err(interrumpida()))
            }
            // Lo apartó un cambio de credenciales: no se encola detrás.
            Destino::Apartado(id) => {
                self.cancelar_en_cola(id, &Nota::clave(nota::COLA_INTERRUMPIDA))
                    .await;
                Err(CambioError::EnCurso)
            }
        }
    }

    /// Aplica `t` y, si queda aplicado, sigue con la cola; todo en su
    /// propia tarea (si quien confirmó se va, sigue hasta un estado final).
    pub(in crate::changes) fn lanzar(
        self: &Arc<Self>,
        reserva: Reserva,
        t: Turno,
        aviso: Option<Aviso>,
    ) {
        let m = self.clone();
        let h = tokio::spawn(async move {
            let estado = m.turno(&t, &reserva, aviso).await;
            m.seguir_la_cola(reserva, estado).await;
        });
        *lock(&self.tarea) = Some(h.abort_handle());
    }

    /// Aplica y vigila un cambio. Devuelve su estado final.
    async fn turno(&self, t: &Turno, reserva: &Reserva, aviso: Option<Aviso>) -> Estado {
        match self
            .aplicar_grupo(&t.pendiente, &t.desmarcados, &t.usuario, reserva, t.fila)
            .await
        {
            Ok(ej) => {
                if let Some(a) = aviso {
                    let _ = a.send(Ok(ej.id));
                }
                self.seguir(ej).await
            }
            Err(e) => {
                // Sin llegar a escribir: lo que esperaba en la cola queda
                // cancelado con el motivo (si llegó a escribir, ya tiene su
                // estado final).
                if let Some(id) = t.fila {
                    let nota = match &e {
                        CambioError::InstanciaCambiada(_) => {
                            Nota::clave(nota::COLA_ROUTER_CAMBIADO).tecnico(e.to_string())
                        }
                        e => Nota::clave(nota::COLA_NO_PREPARADO).y(Nota::de_error(e)),
                    };
                    self.cancelar_en_cola(id, &nota).await;
                }
                if let Some(a) = aviso {
                    let _ = a.send(Err(e));
                }
                Estado::Fallido
            }
        }
    }

    /// Tras un cambio que acabó en `anterior`: si quedó aplicado, aplica el
    /// siguiente de la cola, y así hasta vaciarla; si no, cancela los que
    /// esperan. Después suelta el hueco.
    pub(in crate::changes) async fn seguir_la_cola(
        self: &Arc<Self>,
        mut reserva: Reserva,
        mut anterior: Estado,
    ) {
        loop {
            self.state.lecturas.invalidar();
            let siguiente = {
                let mut h = lock(&self.hueco);
                match h.cola.pop_front() {
                    Some(t) if anterior == Estado::Aplicado => {
                        h.activo = Some(t.fila);
                        Ok(t)
                    }
                    primero => {
                        let mut resto: Vec<Turno> = primero.into_iter().collect();
                        resto.extend(h.cola.drain(..));
                        h.activo = None;
                        h.con_cola = false;
                        Err(resto)
                    }
                }
            };
            match siguiente {
                Ok(t) => {
                    tracing::info!(cambio = t.fila, "le llega el turno al cambio en cola");
                    anterior = self.turno(&t, &reserva, None).await;
                }
                Err(resto) => {
                    // El hueco ya está libre: al soltarla no se toca.
                    reserva.suelta = true;
                    drop(reserva);
                    let nota = Nota::clave(if anterior == Estado::Revertido {
                        nota::COLA_ANTERIOR_DESHECHO
                    } else {
                        nota::COLA_ANTERIOR_NO_APLICADO
                    });
                    for t in resto {
                        if let Some(id) = t.fila {
                            self.cancelar_en_cola(id, &nota).await;
                        }
                    }
                    return;
                }
            }
        }
    }

    /// Quita de la cola el cambio `id`, que encoló `usuario`: queda
    /// `cancelado`, con la nota de quién lo quitó.
    pub async fn quitar_de_la_cola(&self, id: i64, usuario: &str) -> Result<(), ErrorQuitar> {
        {
            let mut h = lock(&self.hueco);
            let i = h
                .cola
                .iter()
                .position(|t| t.fila == Some(id))
                .ok_or(ErrorQuitar::NoEnCola)?;
            if h.cola[i].usuario != usuario {
                return Err(ErrorQuitar::OtroUsuario);
            }
            h.cola.remove(i);
        }
        tracing::info!(cambio = id, usuario, "cambio quitado de la cola");
        let nota = Nota::con(nota::COLA_QUITADO, &[("usuario", usuario.to_owned())]);
        self.cancelar_en_cola(id, &nota).await;
        Ok(())
    }

    /// La tarea de la cola se cortó con `cola` esperando: quedan
    /// `cancelado` (en segundo plano: se llama al soltar la reserva).
    pub(in crate::changes) fn cancelar_al_cortarse(self: &Arc<Self>, cola: VecDeque<Turno>) {
        let ids: Vec<i64> = cola.into_iter().filter_map(|t| t.fila).collect();
        if ids.is_empty() {
            return;
        }
        // Sin runtime (el proceso se acaba), quedan `en_cola` y se cancelan
        // al arrancar.
        let Ok(rt) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let m = self.clone();
        rt.spawn(async move {
            let nota = Nota::clave(nota::COLA_INTERRUMPIDA);
            for id in ids {
                m.cancelar_en_cola(id, &nota).await;
            }
        });
    }

    /// Justo antes de escribir un cambio que esperó en la cola: el panel
    /// sigue admitiendo cambios y cada paso cambiaría lo mismo que al
    /// revisarlo. Si no, [`CambioError::InstanciaCambiada`] (o el error al
    /// prepararlo) y no se escribe nada.
    pub(in crate::changes) async fn comprobar_huella(
        &self,
        p: &Pendiente,
        desmarcados: &[usize],
    ) -> Result<(), CambioError> {
        if crate::instancia::solo_lectura(&self.state.db).await {
            return Err(CambioError::Bloqueada(BLOQUEO_SOLO_LECTURA.into()));
        }
        for (i, paso) in p.pasos.iter().enumerate() {
            if desmarcados.contains(&i) {
                continue;
            }
            let ahora = match self.planificar(&paso.req, p.explicito).await {
                Ok(plan) => Some(Huella::de(&plan)),
                Err(CambioError::SinCambios) => None,
                Err(e) => return Err(e),
            };
            if p.huella.get(i) != Some(&ahora) {
                return Err(CambioError::InstanciaCambiada(format!(
                    "paso {}: el router ha cambiado desde que se revisó el cambio",
                    i + 1
                )));
            }
        }
        Ok(())
    }
}

fn interrumpida() -> CambioError {
    CambioError::Router("la aplicación del cambio se interrumpió".into())
}
