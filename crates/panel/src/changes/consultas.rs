//! Consultas del gestor: qué se puede escribir, valores actuales e historial.

use super::persistencia::a_cambio;
use super::*;

impl ChangeManager {
    /// ¿Se puede escribir `nombre`? Mismas reglas que [`ChangeManager::preparar`]
    /// (catálogo y bloqueos de la v1), sin leer el router.
    pub fn escribible(&self, nombre: &str) -> Result<(), CambioError> {
        self.funcion_escribible(nombre).map(drop)
    }

    /// Valores que muestra ahora la lectura pareada de `funcion` para la
    /// instancia (`valores` lleva los parámetros que la identifican). Son los
    /// mismos con los que [`ChangeManager::preparar`] calcula el diff. `None`
    /// si la función no tiene lectura comprobable.
    pub async fn valores_actuales(
        &self,
        funcion: &str,
        instancia: Option<&str>,
        valores: &BTreeMap<String, String>,
    ) -> Result<Option<BTreeMap<String, String>>, CambioError> {
        let f = self.funcion_escribible(funcion)?;
        match lectura_de(f, valores, instancia)? {
            Lectura::Ninguna => Ok(None),
            l => self.leer(&l).await.map(Some).map_err(|e| {
                CambioError::Router(format!("no se puede leer el estado actual: {e}"))
            }),
        }
    }

    /// Instancias de la lista de `funcion` según su lectura pareada (sin el
    /// id, con `all=ALL`), como las ve el gestor al verificar una creación.
    pub async fn instancias(
        &self,
        funcion: &str,
        valores: &BTreeMap<String, String>,
    ) -> Result<Vec<BTreeMap<String, String>>, CambioError> {
        let f = self.funcion_escribible(funcion)?;
        if f.lista.is_none() {
            return Err(CambioError::Invalida(format!("{funcion} no es una lista")));
        }
        // El id de la lectura se quita al listar: cualquier valor sirve.
        let l = lectura_de(f, valores, Some("-1"))?;
        self.listar(&l)
            .await
            .map_err(|e| CambioError::Router(format!("no se pueden listar las instancias: {e}")))
    }

    /// Un cambio del historial.
    pub async fn cambio(&self, id: i64) -> Option<Cambio> {
        let f = self.fila(id).await?;
        a_cambio(&f, &self.filas_pasos(id).await)
    }

    /// ¿Crea o borra usuarios del router el cambio `id`? Deshacerlo también
    /// los toca, así que exige reautenticación, como hacerlo.
    pub async fn es_de_usuarios_el_cambio(&self, id: i64) -> bool {
        self.cambio(id).await.is_some_and(|c| {
            super::es_de_usuarios(&c.funcion)
                || c.pasos.iter().any(|p| super::es_de_usuarios(&p.funcion))
        })
    }

    /// Los secretos que cambiaron en cada paso del cambio `id`, con su valor
    /// anterior y el nuevo en claro: `(orden, parámetro, antes, después,
    /// tapado)` (`None` si ese lado no se guardó: al crear no hay anterior;
    /// al borrar, nuevo; ni el anterior de uno que el router da tapado al
    /// leer, que es el vacío de la lectura). Descifra solo este cambio. Para el detalle de un cambio en el
    /// Historial, que los enseña censurados con el ojo; un paso que no se
    /// puede descifrar (otra clave) queda fuera y se anota, sin valores.
    pub async fn secretos_en_claro(
        &self,
        id: i64,
    ) -> Vec<(i64, String, Option<String>, Option<String>, bool)> {
        let mut out = Vec::new();
        for f in self.filas_pasos(id).await {
            let orden: i64 = f.get("orden");
            let funcion: String = f.get("funcion");
            let func = self.state.catalogo.funcion(&funcion);
            let diff: Vec<CampoDiff> =
                serde_json::from_str(&f.get::<String, _>("diff")).unwrap_or_default();
            let abrir = |col: &str| -> Option<BTreeMap<String, String>> {
                let blob = f.get::<Option<Vec<u8>>, _>(col)?;
                let m = self
                    .state
                    .history_key
                    .open(&blob)
                    .and_then(|plano| serde_json::from_slice(&plano).ok());
                if m.is_none() {
                    tracing::warn!(
                        cambio = id,
                        orden,
                        "no se puede descifrar {col} (¿otra clave?)"
                    );
                }
                m
            };
            let (antes, despues) = (
                abrir("valores_anteriores_cifrados").unwrap_or_default(),
                abrir("valores_nuevos_cifrados").unwrap_or_default(),
            );
            for d in diff.into_iter().filter(|d| d.secreto) {
                let tapado = func
                    .and_then(|f| f.parametro(&d.nombre))
                    .is_some_and(|p| p.tapado);
                let a = antes.get(&d.nombre).cloned().filter(|_| !tapado);
                let n = despues.get(&d.nombre).cloned();
                if a.is_some() || n.is_some() {
                    out.push((orden, d.nombre, a, n, tapado));
                }
            }
        }
        out
    }

    /// Los `limite` cambios más recientes, del último al primero.
    pub async fn historial(&self, limite: u32) -> Vec<Cambio> {
        match sqlx::query("SELECT * FROM changes ORDER BY id DESC LIMIT ?")
            .bind(limite)
            .fetch_all(&self.state.db)
            .await
        {
            Ok(filas) => {
                let mut out = Vec::with_capacity(filas.len());
                for f in &filas {
                    let pasos = self.filas_pasos(f.get("id")).await;
                    out.extend(a_cambio(f, &pasos));
                }
                out
            }
            Err(e) => {
                tracing::error!("no se puede leer el historial: {e}");
                Vec::new()
            }
        }
    }
}
