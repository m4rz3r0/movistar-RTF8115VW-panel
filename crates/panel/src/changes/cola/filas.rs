//! Las filas de la cola en la base de datos: `en_cola` mientras esperan,
//! `cancelado` si no llegan a aplicarse.

use super::*;
use crate::changes::persistencia::{NuevoGrupo, json, riesgo_str};

impl ChangeManager {
    /// Guarda `t` `en_cola`, con lo que mostró cada paso al prepararlo (los
    /// valores, al llegarle el turno). Sin ningún paso que aplicar,
    /// [`CambioError::SinCambios`] y no se guarda nada.
    pub(super) async fn insertar_en_cola(&self, t: &Turno) -> Result<i64, CambioError> {
        let p = &t.pendiente;
        let huella = |i: usize| {
            p.huella
                .get(i)
                .and_then(Option::as_ref)
                .filter(|_| !t.desmarcados.contains(&i))
        };
        let Some(i0) = (0..p.pasos.len()).find(|i| huella(*i).is_some()) else {
            return Err(CambioError::SinCambios);
        };
        let principal = huella(i0).expect("buscado arriba");
        let riesgo = (0..p.pasos.len())
            .filter(|i| huella(*i).is_some())
            .filter_map(|i| self.state.catalogo.funcion(&p.pasos[i].req.funcion))
            .map(|f| f.riesgo)
            .max()
            .unwrap_or(Riesgo::Bajo);
        let guardar = async {
            let now = series::now();
            let mut tx = self.state.db.begin().await?;
            let id: i64 = sqlx::query_scalar(
                "INSERT INTO changes (ts, updated, user_id, usuario, funcion, instancia, riesgo, \
                 estado, operacion, diff, anteriores, titulo) VALUES (?, ?, \
                 (SELECT id FROM users WHERE username = ?), ?, ?, ?, ?, 'en_cola', ?, ?, '{}', \
                 ?) RETURNING id",
            )
            .bind(now)
            .bind(now)
            .bind(&t.usuario)
            .bind(&t.usuario)
            .bind(&p.pasos[i0].req.funcion)
            .bind(&principal.instancia)
            .bind(riesgo_str(riesgo))
            .bind(principal.operacion.as_str())
            .bind(json(&principal.diff.campos))
            .bind(&p.titulo)
            .fetch_one(&mut *tx)
            .await?;
            for (i, paso) in p.pasos.iter().enumerate() {
                let (estado, operacion, diff) = match huella(i) {
                    Some(h) => (
                        EstadoPaso::Pendiente,
                        h.operacion.as_str(),
                        json(&h.diff.campos),
                    ),
                    None => (
                        EstadoPaso::Omitido,
                        Operacion::Editar.as_str(),
                        "[]".to_owned(),
                    ),
                };
                sqlx::query(
                    "INSERT INTO cambio_pasos (cambio_id, orden, funcion, instancia, etiqueta, \
                     operacion, diff, valores_nuevos, valores_anteriores, estado) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(id)
                .bind(i as i64)
                .bind(&paso.req.funcion)
                .bind(&paso.req.instancia)
                .bind(&paso.etiqueta)
                .bind(operacion)
                .bind(diff)
                .bind("{}".as_bytes())
                .bind("{}".as_bytes())
                .bind(estado.as_str())
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            Ok::<i64, sqlx::Error>(id)
        };
        guardar
            .await
            .map_err(|e| CambioError::Router(format!("no se puede guardar el cambio: {e}")))
    }

    /// Al llegarle el turno: la fila `en_cola` `id` pasa a ser la del
    /// cambio que se aplica (`aplicando`, con sus datos de ahora); sus
    /// pasos se vuelven a guardar. Error si ya no estaba en la cola.
    pub(in crate::changes) async fn ocupar_fila(
        tx: &mut sqlx::SqliteConnection,
        id: i64,
        g: &NuevoGrupo<'_>,
        d: &crate::changes::persistencia::DatosPlan,
        wan_antes: Option<bool>,
    ) -> Result<(), sqlx::Error> {
        let (_, plan) = g.principal;
        let now = series::now();
        let hecho = sqlx::query(
            "UPDATE changes SET ts = ?, updated = ?, funcion = ?, instancia = ?, riesgo = ?, \
             estado = 'aplicando', operacion = ?, diff = ?, anteriores = ?, \
             anteriores_cifrados = ?, solicitud = ?, nuevos = ?, nuevos_cifrados = ?, \
             wan_antes = ?, ids_antes = ?, titulo = ? WHERE id = ? AND estado = 'en_cola'",
        )
        .bind(now)
        .bind(now)
        .bind(crate::changes::especiales::funcion_enviada(plan))
        .bind(&plan.instancia)
        .bind(riesgo_str(g.riesgo))
        .bind(d.operacion)
        .bind(&d.diff)
        .bind(&d.anteriores)
        .bind(&d.anteriores_cifrados)
        .bind(&d.solicitud)
        .bind(&d.nuevos)
        .bind(&d.nuevos_cifrados)
        .bind(wan_antes)
        .bind(&d.ids_antes)
        .bind(g.titulo)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if hecho.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        sqlx::query("DELETE FROM cambio_pasos WHERE cambio_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        Ok(())
    }

    /// La fila `id`, si sigue `en_cola`, queda `cancelado` con `motivo`
    /// (y sus pasos pendientes, también).
    pub(in crate::changes) async fn cancelar_en_cola(&self, id: i64, motivo: &Nota) {
        let r = async {
            let mut tx = self.state.db.begin().await?;
            let hecho = sqlx::query(
                "UPDATE changes SET estado = 'cancelado', updated = ?, error = ? \
                 WHERE id = ? AND estado = 'en_cola'",
            )
            .bind(series::now())
            .bind(motivo.guardar())
            .bind(id)
            .execute(&mut *tx)
            .await?;
            if hecho.rows_affected() > 0 {
                sqlx::query(
                    "UPDATE cambio_pasos SET estado = 'cancelado' \
                     WHERE cambio_id = ? AND estado = 'pendiente'",
                )
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            Ok::<bool, sqlx::Error>(hecho.rows_affected() > 0)
        };
        match r.await {
            Ok(true) => tracing::info!(
                cambio = id,
                "cambio de la cola cancelado: {}",
                motivo.para_registro()
            ),
            Ok(false) => {}
            Err(e) => tracing::error!(cambio = id, "no se puede cancelar el cambio en cola: {e}"),
        }
    }

    /// Al arrancar: lo que quedó `en_cola` (el panel se detuvo con cambios
    /// esperando) queda `cancelado`; nunca se aplica a ciegas.
    pub(in crate::changes) async fn cancelar_cola_al_arrancar(&self) {
        let ids: Vec<i64> =
            match sqlx::query_scalar("SELECT id FROM changes WHERE estado = 'en_cola' ORDER BY id")
                .fetch_all(&self.state.db)
                .await
            {
                Ok(ids) => ids,
                Err(e) => {
                    tracing::error!("no se pueden leer los cambios en cola: {e}");
                    return;
                }
            };
        let nota = Nota::clave(nota::COLA_INTERRUMPIDA);
        for id in ids {
            self.cancelar_en_cola(id, &nota).await;
        }
    }
}
