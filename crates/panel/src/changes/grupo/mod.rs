//! Cambios por grupos de pasos: el orden de aplicación, la reversión en
//! orden inverso y la vigilancia del grupo como una unidad.
//!
//! - Los pasos se escriben en el orden declarado. Cada uno se planifica justo
//!   antes de escribirlo (con lo que muestra el router tras los anteriores) y
//!   se verifica con su lectura pareada.
//! - Si falla el paso k, se revierten en orden inverso los pasos ya escritos
//!   (y el propio k si pudo quedar aplicado en parte). El grupo queda
//!   `fallido` si todas las reversiones se verifican; si no,
//!   `revertido_parcial`, con el detalle de qué paso no volvió.
//! - Se vigila con el plazo del paso de mayor riesgo; si la vigilancia falla,
//!   se revierte el grupo entero en orden inverso.
//! - Deshacer prepara el grupo inverso (pasos inversos en orden inverso).
//! - Los pasos sin cambios o desmarcados se guardan como `omitido`.
//!
//! Un grupo de un paso se comporta exactamente como un cambio del núcleo
//! ([`ChangeManager::preparar`]/[`ChangeManager::confirmar`] lo usan así).
//! Una acción (p. ej. `cbReboot`) solo puede ir sola: no se puede revertir si
//! falla otro paso.

mod aplicar;
mod fallos;

use super::*;

/// Un paso de un grupo: la escritura y la clave i18n de su resumen humano
/// (`None`: el nombre de la función).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paso {
    pub req: ChangeRequest,
    pub etiqueta: Option<String>,
}

/// Petición de un grupo de pasos. `titulo`: clave i18n del resumen del grupo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrupoRequest {
    pub pasos: Vec<Paso>,
    pub titulo: Option<String>,
}

/// Un paso preparado: su diff para la pantalla (las credenciales
/// enmascaradas y los datos sensibles en claro, `plan::para_mostrar`) o
/// `sin_cambios` si el router ya está así (se omitirá al confirmar).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasoPreparado {
    /// Posición en el grupo (desde 0); es la que se desmarca al confirmar.
    pub indice: usize,
    pub diff: Diff,
    pub sin_cambios: bool,
    pub funcion: String,
    pub etiqueta: Option<String>,
}

/// Resultado de [`ChangeManager::preparar_grupo`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparadoGrupo {
    /// Token para [`ChangeManager::confirmar_grupo`] (un solo uso, 10 min).
    pub token: String,
    pub pasos: Vec<PasoPreparado>,
    /// El mayor de los pasos con cambios: marca la vigilancia del grupo.
    pub riesgo: Riesgo,
}

/// Estado de un paso de un grupo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstadoPaso {
    /// Aún no se ha escrito (o el grupo se cortó antes de llegar a él).
    Pendiente,
    /// Sin cambios o desmarcado: no se escribe.
    Omitido,
    /// Escrito (o a punto), sin verificar.
    Aplicando,
    /// Escrito y verificado (o sin forma de comprobarlo).
    Aplicado,
    /// No se aplicó; o no se pudo revertir sin saber si se aplicó.
    Fallido,
    /// Revertido y verificado.
    Revertido,
}

impl EstadoPaso {
    /// Nombre en la base de datos.
    pub fn as_str(self) -> &'static str {
        match self {
            EstadoPaso::Pendiente => "pendiente",
            EstadoPaso::Omitido => "omitido",
            EstadoPaso::Aplicando => "aplicando",
            EstadoPaso::Aplicado => "aplicado",
            EstadoPaso::Fallido => "fallido",
            EstadoPaso::Revertido => "revertido",
        }
    }

    pub(super) fn parse(s: &str) -> Option<EstadoPaso> {
        [
            EstadoPaso::Pendiente,
            EstadoPaso::Omitido,
            EstadoPaso::Aplicando,
            EstadoPaso::Aplicado,
            EstadoPaso::Fallido,
            EstadoPaso::Revertido,
        ]
        .into_iter()
        .find(|e| e.as_str() == s)
    }
}

/// Un paso de un cambio del historial (secretos enmascarados).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasoCambio {
    /// Posición en el grupo (desde 0).
    pub orden: i64,
    pub funcion: String,
    pub instancia: Option<String>,
    /// Clave i18n del resumen humano (`None`: la función).
    pub etiqueta: Option<String>,
    pub estado: EstadoPaso,
    pub diff: Diff,
    /// Por qué falló el paso o su reversión.
    pub error: Option<String>,
}

/// Un paso escrito (o que pudo llegar a escribirse) de un grupo en curso.
pub(super) struct PasoEscrito {
    pub(super) orden: usize,
    pub(super) plan: Plan,
    /// Se verificó (`aplicado`); si no, no se sabe si llegó.
    pub(super) verificado: bool,
}

/// Lo que hace falta para vigilar (y, si falla, revertir) un grupo aplicado.
pub(super) struct Ejecucion {
    pub(super) id: i64,
    /// Pasos aplicados, en orden.
    pub(super) pasos: Vec<PasoEscrito>,
    pub(super) base: Base,
    pub(super) nota: Option<Nota>,
    pub(super) riesgo: Riesgo,
    /// El grupo tiene más de un paso (no omitido): el detalle de un fallo
    /// nombra el paso y una reversión a medias es `revertido_parcial`.
    pub(super) multi: bool,
}

/// Fallo de un paso al aplicarlo.
pub(super) struct FalloPaso {
    pub(super) error: CambioError,
    /// El router pudo quedar con el paso aplicado (en parte): se revierte
    /// con los demás.
    pub(super) revertir: bool,
}

impl FalloPaso {
    /// El paso no se aplicó (no se escribió, el router lo rechazó o sigue
    /// mostrando lo de antes).
    fn no_aplicado(error: CambioError) -> FalloPaso {
        FalloPaso {
            error,
            revertir: false,
        }
    }
}

/// Por qué no se pudo revertir un paso.
pub(super) enum ErrorReversion {
    /// Sin valores anteriores (o sin la instancia creada) no hay inversa.
    SinInversa,
    Fallo(String),
}

impl std::fmt::Display for ErrorReversion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorReversion::SinInversa => {
                f.write_str("no se puede revertir: valores anteriores desconocidos")
            }
            ErrorReversion::Fallo(e) => write!(f, "no se pudo revertir: {e}"),
        }
    }
}

impl ErrorReversion {
    /// El texto legible (y, aparte, lo técnico) para el Historial.
    pub(super) fn nota(&self) -> Nota {
        match self {
            ErrorReversion::SinInversa => Nota::clave(nota::ERROR_SIN_INVERSA),
            ErrorReversion::Fallo(e) => Nota::clave(nota::ERROR_SIN_REVERTIR).tecnico(e.clone()),
        }
    }
}

/// Detalle de las reversiones fallidas: en un grupo de un paso, el motivo
/// tal cual (como en el núcleo); en uno de varios, cada paso con su motivo
/// (por su número, nunca por su función).
pub(super) fn detalle_reversion(multi: bool, fallos: &[(usize, String, ErrorReversion)]) -> Nota {
    if !multi && let [(_, _, e)] = fallos {
        return e.nota();
    }
    Nota::juntar(fallos.iter().map(|(o, _, e)| e.nota().del_paso(*o)))
        .unwrap_or_else(|| Nota::clave(nota::ERROR_SIN_REVERTIR))
}

/// Un paso guardado, para retomar un grupo.
pub(super) struct PasoGuardado {
    pub(super) orden: usize,
    pub(super) funcion: String,
    pub(super) estado: EstadoPaso,
    /// Tiene un error guardado: falló, o falló su reversión (el grupo se
    /// estaba revirtiendo y hay que terminar de revertirlo).
    pub(super) con_error: bool,
    /// Plan (solo de los pasos escritos), o por qué no se puede rehacer.
    pub(super) plan: Result<Plan, String>,
}
