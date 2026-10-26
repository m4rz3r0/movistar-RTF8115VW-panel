//! Persistencia de los cambios y sus pasos en la base de datos.

use super::especiales::funcion_enviada;
use super::*;

impl ChangeManager {
    /// Lo que se guarda de un plan (secretos cifrados aparte).
    fn datos_plan(&self, plan: &Plan) -> DatosPlan {
        let clave = &self.state.history_key;
        // JSON de los valores no secretos y, aparte, los secretos cifrados.
        let partir = |pares: Vec<(&str, &str, bool)>| {
            let mut publico = BTreeMap::new();
            let mut secreto = BTreeMap::new();
            for (k, v, s) in pares {
                if s { &mut secreto } else { &mut publico }.insert(k, v);
            }
            let cifrado = (!secreto.is_empty()).then(|| clave.seal(json(&secreto).as_bytes()));
            (json(&publico), cifrado)
        };
        let (anteriores, anteriores_cifrados) = partir(
            plan.campos
                .iter()
                .filter(|c| c.leido)
                .map(|c| (c.nombre.as_str(), c.antes.as_str(), c.secreto))
                .collect(),
        );
        let (nuevos, nuevos_cifrados) = partir(
            plan.campos
                .iter()
                .map(|c| (c.nombre.as_str(), c.despues.as_str(), c.secreto))
                .collect(),
        );
        let m = &self.state.mascara;
        let f = &plan.funcion;
        let solicitud: BTreeMap<&str, &str> = plan
            .valores
            .iter()
            .filter(|(k, v)| {
                !es_secreto_param(f, m, k)
                    && !m.oculta(k, v)
                    && !plan.campos.iter().any(|c| c.secreto && &c.nombre == *k)
            })
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        DatosPlan {
            operacion: plan.operacion.as_str(),
            solicitud: json(&solicitud),
            diff: json(&visible(&plan.campos).campos),
            anteriores,
            anteriores_cifrados,
            nuevos,
            nuevos_cifrados,
            ids_antes: plan.ids_antes.as_ref().map(json),
        }
    }

    /// Guarda un grupo nuevo (`aplicando`; si esperó en la cola, en su fila) con todos sus pasos: el principal
    /// con su plan (`aplicando`), y los demás `omitido` o `pendiente` (sin
    /// valores: se guardan al llegar a ellos).
    pub(super) async fn insertar(&self, g: &NuevoGrupo<'_>) -> Result<i64, sqlx::Error> {
        let (i0, plan) = g.principal;
        let d = self.datos_plan(plan);
        let wan_antes = match g.base {
            Base::Wan(up) => Some(up),
            Base::Uptime(_) => None,
        };
        let now = series::now();
        let mut tx = self.state.db.begin().await?;
        // Un cambio que esperó en la cola ya tiene su fila.
        let id: i64 = if let Some(id) = g.fila {
            Self::ocupar_fila(&mut tx, id, g, &d, wan_antes).await?;
            id
        } else {
            sqlx::query_scalar(
                "INSERT INTO changes (ts, updated, user_id, usuario, funcion, instancia, riesgo, \
             estado, operacion, diff, anteriores, anteriores_cifrados, solicitud, nuevos, \
             nuevos_cifrados, wan_antes, ids_antes, titulo) VALUES (?, ?, \
             (SELECT id FROM users WHERE username = ?), \
             ?, ?, ?, ?, 'aplicando', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
            )
            .bind(now)
            .bind(now)
            .bind(g.usuario)
            .bind(g.usuario)
            .bind(funcion_enviada(plan))
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
            .fetch_one(&mut *tx)
            .await?
        };
        for (i, paso) in g.pasos.iter().enumerate() {
            let (funcion, instancia, datos, estado) = if i == i0 {
                (
                    funcion_enviada(plan),
                    &plan.instancia,
                    &d,
                    EstadoPaso::Aplicando,
                )
            } else {
                (
                    paso.req.funcion.as_str(),
                    &paso.req.instancia,
                    &DatosPlan::vacio(),
                    g.estados[i],
                )
            };
            sqlx::query(
                "INSERT INTO cambio_pasos (cambio_id, orden, funcion, instancia, etiqueta, \
                 operacion, solicitud, diff, valores_nuevos, valores_nuevos_cifrados, \
                 valores_anteriores, valores_anteriores_cifrados, ids_antes, estado) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(i as i64)
            .bind(funcion)
            .bind(instancia)
            .bind(&paso.etiqueta)
            .bind(datos.operacion)
            .bind(&datos.solicitud)
            .bind(&datos.diff)
            .bind(datos.nuevos.as_bytes())
            .bind(&datos.nuevos_cifrados)
            .bind(datos.anteriores.as_bytes())
            .bind(&datos.anteriores_cifrados)
            .bind(&datos.ids_antes)
            .bind(estado.as_str())
            .execute(&mut *tx)
            .await?;
        }
        if let Some(o) = g.deshace {
            sqlx::query("UPDATE changes SET deshecho_por = ? WHERE id = ?")
                .bind(id)
                .bind(o)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(id)
    }

    /// Guarda el plan de un paso que se va a escribir (`aplicando`).
    pub(super) async fn guardar_paso(
        &self,
        id: i64,
        orden: usize,
        plan: &Plan,
    ) -> Result<(), sqlx::Error> {
        let d = self.datos_plan(plan);
        sqlx::query(
            "UPDATE cambio_pasos SET funcion = ?, instancia = ?, operacion = ?, solicitud = ?, \
             diff = ?, \
             valores_nuevos = ?, valores_nuevos_cifrados = ?, valores_anteriores = ?, \
             valores_anteriores_cifrados = ?, ids_antes = ?, estado = 'aplicando' \
             WHERE cambio_id = ? AND orden = ?",
        )
        .bind(funcion_enviada(plan))
        .bind(&plan.instancia)
        .bind(d.operacion)
        .bind(&d.solicitud)
        .bind(&d.diff)
        .bind(d.nuevos.as_bytes())
        .bind(&d.nuevos_cifrados)
        .bind(d.anteriores.as_bytes())
        .bind(&d.anteriores_cifrados)
        .bind(&d.ids_antes)
        .bind(id)
        .bind(orden as i64)
        .execute(&self.state.db)
        .await
        .map(drop)
    }

    /// Cambia el estado; `error` y `nota` solo se escriben si son `Some`.
    pub(super) async fn marcar(
        &self,
        id: i64,
        estado: Estado,
        error: Option<&Nota>,
        nota: Option<&Nota>,
    ) {
        let nota = nota.map(Nota::guardar);
        let guardado = error.map(Nota::guardar);
        let r = sqlx::query(
            "UPDATE changes SET estado = ?, updated = ?, error = COALESCE(?, error), \
             nota = COALESCE(?, nota) WHERE id = ?",
        )
        .bind(estado.as_str())
        .bind(series::now())
        .bind(guardado.as_deref())
        .bind(nota.as_deref())
        .bind(id)
        .execute(&self.state.db)
        .await;
        if let Err(e) = r {
            tracing::error!(cambio = id, "no se puede guardar el estado {estado:?}: {e}");
        }
        match error {
            Some(e) => tracing::warn!(
                cambio = id,
                estado = estado.as_str(),
                "{}",
                e.para_registro()
            ),
            None => tracing::info!(
                cambio = id,
                estado = estado.as_str(),
                nota = nota
                    .as_deref()
                    .map_or_else(String::new, |n| Nota::texto(n, i18n::Idioma::Es)),
                "cambio actualizado"
            ),
        }
    }

    /// Cambia el estado de un paso; `error` solo se escribe si es `Some`.
    pub(super) async fn marcar_paso(
        &self,
        id: i64,
        orden: usize,
        estado: EstadoPaso,
        error: Option<&Nota>,
    ) {
        let error = error.map(Nota::guardar);
        let r = sqlx::query(
            "UPDATE cambio_pasos SET estado = ?, error = COALESCE(?, error) \
             WHERE cambio_id = ? AND orden = ?",
        )
        .bind(estado.as_str())
        .bind(error.as_deref())
        .bind(id)
        .bind(orden as i64)
        .execute(&self.state.db)
        .await;
        if let Err(e) = r {
            tracing::error!(
                cambio = id,
                paso = orden + 1,
                "no se puede guardar el estado {estado:?}: {e}"
            );
        }
    }

    /// La instancia creada por el paso `orden` (también la del cambio si es
    /// su primer paso escrito).
    pub(super) async fn fijar_instancia(&self, id: i64, orden: usize, instancia: &str) {
        let r = async {
            sqlx::query("UPDATE cambio_pasos SET instancia = ? WHERE cambio_id = ? AND orden = ?")
                .bind(instancia)
                .bind(id)
                .bind(orden as i64)
                .execute(&self.state.db)
                .await?;
            sqlx::query(
                "UPDATE changes SET instancia = ?1 WHERE id = ?2 AND ?3 = (SELECT MIN(orden) \
                 FROM cambio_pasos WHERE cambio_id = ?2 AND estado NOT IN ('omitido', \
                 'pendiente'))",
            )
            .bind(instancia)
            .bind(id)
            .bind(orden as i64)
            .execute(&self.state.db)
            .await
        }
        .await;
        if let Err(e) = r {
            tracing::error!(cambio = id, "no se puede guardar la instancia nueva: {e}");
        }
    }

    pub(super) async fn fila(&self, id: i64) -> Option<sqlx::sqlite::SqliteRow> {
        sqlx::query("SELECT * FROM changes WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.state.db)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(cambio = id, "no se puede leer el cambio: {e}");
                None
            })
    }

    /// Pasos de un cambio, en orden.
    pub(super) async fn filas_pasos(&self, id: i64) -> Vec<sqlx::sqlite::SqliteRow> {
        sqlx::query("SELECT * FROM cambio_pasos WHERE cambio_id = ? ORDER BY orden")
            .bind(id)
            .fetch_all(&self.state.db)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(cambio = id, "no se pueden leer los pasos: {e}");
                Vec::new()
            })
    }

    /// Plan de un paso guardado (sin carga: solo para comprobar, revertir o
    /// deshacer). Los secretos se descifran con la clave del historial.
    pub(super) fn reconstruir(&self, fila: &sqlx::sqlite::SqliteRow) -> Result<Plan, String> {
        let nombre: String = fila.get("funcion");
        let f = self
            .state
            .catalogo
            .funcion(&nombre)
            .ok_or_else(|| format!("la función {nombre} ya no está en el catálogo"))?
            .clone();
        let op =
            Operacion::parse(&fila.get::<String, _>("operacion")).ok_or("operación desconocida")?;
        let instancia: Option<String> = fila.get("instancia");
        let mapa = |col: &str| -> Result<BTreeMap<String, String>, String> {
            serde_json::from_slice(&fila.get::<Vec<u8>, _>(col))
                .map_err(|e| format!("{col} no válido: {e}"))
        };
        let abrir = |col: &str, m: &mut BTreeMap<String, String>| -> Result<(), String> {
            if let Some(blob) = fila.get::<Option<Vec<u8>>, _>(col) {
                let plano = self
                    .state
                    .history_key
                    .open(&blob)
                    .ok_or_else(|| format!("no se puede descifrar {col} (¿otra clave?)"))?;
                let s: BTreeMap<String, String> =
                    serde_json::from_slice(&plano).map_err(|e| format!("{col}: {e}"))?;
                m.extend(s);
            }
            Ok(())
        };
        let mut anteriores = mapa("valores_anteriores")?;
        abrir("valores_anteriores_cifrados", &mut anteriores)?;
        let mut nuevos = mapa("valores_nuevos")?;
        abrir("valores_nuevos_cifrados", &mut nuevos)?;
        let diff: Vec<CampoDiff> = serde_json::from_str(&fila.get::<String, _>("diff"))
            .map_err(|e| format!("diff no válido: {e}"))?;
        let campos: Vec<Campo> = diff
            .into_iter()
            .map(|d| Campo {
                antes: anteriores.get(&d.nombre).cloned().unwrap_or_default(),
                despues: nuevos.get(&d.nombre).cloned().unwrap_or_default(),
                leido: anteriores.contains_key(&d.nombre),
                secreto: d.secreto,
                nombre: d.nombre,
            })
            .collect();
        let mut valores: BTreeMap<String, String> =
            serde_json::from_str(&fila.get::<String, _>("solicitud"))
                .map_err(|e| format!("solicitud no válida: {e}"))?;
        if op != Operacion::Borrar {
            valores.extend(campos.iter().map(|c| (c.nombre.clone(), c.despues.clone())));
        }
        let lectura = lectura_de(&f, &valores, instancia.as_deref()).map_err(|e| e.to_string())?;
        Ok(Plan {
            funcion: f,
            operacion: op,
            instancia,
            valores,
            payload: Vec::new(),
            campos,
            lectura,
            ids_antes: fila
                .get::<Option<String>, _>("ids_antes")
                .and_then(|j| serde_json::from_str(&j).ok()),
        })
    }
}

/// Lo que se guarda de un plan.
pub(in crate::changes) struct DatosPlan {
    pub(in crate::changes) operacion: &'static str,
    /// JSON de los valores enviados no secretos.
    pub(in crate::changes) solicitud: String,
    /// JSON del diff (secretos enmascarados).
    pub(in crate::changes) diff: String,
    /// JSON de los valores no secretos y, aparte, los secretos cifrados.
    pub(in crate::changes) anteriores: String,
    pub(in crate::changes) anteriores_cifrados: Option<Vec<u8>>,
    pub(in crate::changes) nuevos: String,
    pub(in crate::changes) nuevos_cifrados: Option<Vec<u8>>,
    pub(in crate::changes) ids_antes: Option<String>,
}

impl DatosPlan {
    /// Un paso al que aún no se ha llegado (o que se omite).
    fn vacio() -> DatosPlan {
        DatosPlan {
            operacion: Operacion::Editar.as_str(),
            solicitud: "{}".into(),
            diff: "[]".into(),
            anteriores: "{}".into(),
            anteriores_cifrados: None,
            nuevos: "{}".into(),
            nuevos_cifrados: None,
            ids_antes: None,
        }
    }
}

/// Un grupo que empieza a aplicarse.
pub(super) struct NuevoGrupo<'a> {
    /// El primer paso que cambia algo (posición y plan).
    pub(super) principal: (usize, &'a Plan),
    pub(super) pasos: &'a [Paso],
    /// Estado inicial de cada paso (el principal se guarda `aplicando`).
    pub(super) estados: &'a [EstadoPaso],
    pub(super) titulo: Option<&'a str>,
    pub(super) riesgo: Riesgo,
    pub(super) usuario: &'a str,
    pub(super) base: Base,
    pub(super) deshace: Option<i64>,
    /// Su fila `en_cola`, si esperó en la cola.
    pub(super) fila: Option<i64>,
}

pub(super) fn riesgo_str(r: Riesgo) -> &'static str {
    match r {
        Riesgo::Bajo => "bajo",
        Riesgo::Medio => "medio",
        Riesgo::Alto => "alto",
    }
}

pub(super) fn json<T: Serialize + ?Sized>(v: &T) -> String {
    serde_json::to_string(v).expect("mapas de cadenas serializables")
}

pub(super) fn nuevo_token() -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("generador aleatorio del sistema");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn a_paso(f: &sqlx::sqlite::SqliteRow) -> Option<PasoCambio> {
    Some(PasoCambio {
        orden: f.get("orden"),
        funcion: f.get("funcion"),
        instancia: f.get("instancia"),
        etiqueta: f.get("etiqueta"),
        estado: EstadoPaso::parse(&f.get::<String, _>("estado"))?,
        diff: Diff {
            campos: serde_json::from_str(&f.get::<String, _>("diff")).unwrap_or_default(),
        },
        error: f.get("error"),
    })
}

/// Un cambio con sus pasos. Función e instancia: las de su primer paso
/// escrito; diff: los de todos los pasos juntos.
pub(super) fn a_cambio(
    f: &sqlx::sqlite::SqliteRow,
    pasos: &[sqlx::sqlite::SqliteRow],
) -> Option<Cambio> {
    let riesgo = match f.get::<String, _>("riesgo").as_str() {
        "bajo" => Riesgo::Bajo,
        "medio" => Riesgo::Medio,
        _ => Riesgo::Alto,
    };
    let pasos: Vec<PasoCambio> = pasos.iter().filter_map(a_paso).collect();
    let principal = pasos
        .iter()
        .find(|p| !matches!(p.estado, EstadoPaso::Omitido | EstadoPaso::Pendiente));
    let diff = if pasos.is_empty() {
        Diff {
            campos: serde_json::from_str(&f.get::<String, _>("diff")).unwrap_or_default(),
        }
    } else {
        Diff {
            campos: pasos
                .iter()
                .flat_map(|p| p.diff.campos.iter().cloned())
                .collect(),
        }
    };
    Some(Cambio {
        id: f.get("id"),
        ts: f.get("ts"),
        updated: f.get("updated"),
        funcion: principal.map_or_else(|| f.get("funcion"), |p| p.funcion.clone()),
        instancia: principal.map_or_else(|| f.get("instancia"), |p| p.instancia.clone()),
        riesgo,
        estado: Estado::parse(&f.get::<String, _>("estado"))?,
        diff,
        error: f.get("error"),
        nota: f.get("nota"),
        usuario: f.get("usuario"),
        titulo: f.get("titulo"),
        pasos,
    })
}
