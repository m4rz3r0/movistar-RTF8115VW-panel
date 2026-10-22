//! Tipos y constantes del gestor de cambios.

use super::*;

/// Tiempos del gestor.
#[derive(Clone, Debug)]
pub struct Tiempos {
    /// Vigilancia de un cambio de riesgo bajo (30 s).
    pub bajo: Duration,
    /// Riesgo medio (2 min).
    pub medio: Duration,
    /// Riesgo alto (5 min); también la espera máxima de la vuelta tras un
    /// reinicio y el plazo para conseguir revertir.
    pub alto: Duration,
    /// Entre comprobaciones de la vigilancia (5 s).
    pub intervalo: Duration,
    /// Fallos seguidos que provocan la reversión (3).
    pub fallos_para_revertir: u32,
    /// Espera máxima a que el sondeo se detenga antes de escribir (30 s).
    pub pausa_max: Duration,
}

impl Default for Tiempos {
    fn default() -> Self {
        Tiempos {
            bajo: Duration::from_secs(30),
            medio: Duration::from_secs(120),
            alto: Duration::from_secs(300),
            intervalo: Duration::from_secs(5),
            fallos_para_revertir: 3,
            pausa_max: Duration::from_secs(30),
        }
    }
}

/// Resultado de releer tras una escritura sin respuesta.
pub(super) enum SinRespuesta {
    /// Hay que seguir (vigilar), con esta comprobación y esta nota.
    Aplicada(Comprobacion, Nota),
    /// Fallido, con el motivo: el router sigue como antes (no se aplicó) o,
    /// al crear, no se sabe qué instancia es la del cambio.
    Fallido(String),
    /// El router no muestra ni lo de antes ni lo nuevo (pudo quedar a
    /// medias): fallido, y hay que revertir (motivo).
    OtrosValores(String),
    /// Al crear, el router no muestra lo de antes pero tampoco se sabe qué
    /// instancia es la del cambio: pudo crearse algo (motivo).
    CreacionIncierta(String),
}

impl Tiempos {
    /// Duración de la vigilancia según el riesgo.
    pub fn vigilancia(&self, riesgo: Riesgo) -> Duration {
        match riesgo {
            Riesgo::Bajo => self.bajo,
            Riesgo::Medio => self.medio,
            Riesgo::Alto => self.alto,
        }
    }
}

/// Petición de cambio.
///
/// En las funciones con `lista`, `instancia` es el id de la instancia que se
/// edita o borra (`borrar`); `None` crea una nueva (id `-1`). Los campos
/// IP/MAC partidos llegan unidos con su nombre base.
///
/// Su `Debug` enseña los nombres de los valores, no los valores (pueden ser
/// secretos).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeRequest {
    pub funcion: String,
    pub instancia: Option<String>,
    pub valores: BTreeMap<String, String>,
    pub borrar: bool,
    /// Al editar o borrar una instancia de una lista: lo que la identifica
    /// (elemento leído → valor del router), tal como era al preparar. Justo
    /// antes de escribir el paso, la instancia con ese id tiene que seguir
    /// teniéndolo; si no (el router renumeró sus instancias, otra sesión la
    /// cambió), el paso no se escribe. Vacío: sin comprobación.
    #[serde(default)]
    pub identidad: BTreeMap<String, String>,
}

impl std::fmt::Debug for ChangeRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeRequest")
            .field("funcion", &self.funcion)
            .field("instancia", &self.instancia)
            .field("valores", &self.valores.keys().collect::<Vec<_>>())
            .field("borrar", &self.borrar)
            .field("identidad", &self.identidad.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Un campo del diff. Si `secreto`, `antes` y `despues` valen [`MASCARA`]
/// (y su `Debug` los tapa aunque no fuera así).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampoDiff {
    pub nombre: String,
    pub antes: String,
    pub despues: String,
    pub secreto: bool,
    /// Dato sensible en claro (un usuario, una URL): solo en el diff para la
    /// pantalla de un cambio preparado, nunca en el guardado (ahí va como
    /// `secreto`). Su `Debug` también lo tapa.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sensible: bool,
}

impl std::fmt::Debug for CampoDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let valor = |v: &str| {
            if self.secreto || self.sensible {
                "***".to_owned()
            } else {
                v.to_owned()
            }
        };
        f.debug_struct("CampoDiff")
            .field("nombre", &self.nombre)
            .field("antes", &valor(&self.antes))
            .field("despues", &valor(&self.despues))
            .field("secreto", &self.secreto)
            .finish()
    }
}

/// Diferencias entre el router y la petición.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diff {
    pub campos: Vec<CampoDiff>,
}

/// Resultado de [`ChangeManager::preparar`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preparado {
    /// Token para [`ChangeManager::confirmar`] (un solo uso, 10 min).
    pub token: String,
    pub diff: Diff,
    pub riesgo: Riesgo,
    /// Palabra que hay que escribir para confirmar (acciones).
    pub palabra: Option<String>,
    /// Lo que se va a escribir: función, instancia y si es un borrado (lo
    /// que hace falta para presentarlo sin tener la petición a mano, p. ej.
    /// al deshacer).
    pub funcion: String,
    pub instancia: Option<String>,
    pub borrar: bool,
    /// Los pasos del grupo (uno, salvo al deshacer un grupo). `diff` junta
    /// los de todos; `funcion`, `instancia` y `borrar` son los del primero
    /// que cambia algo.
    pub pasos: Vec<PasoPreparado>,
}

/// Errores del gestor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CambioError {
    /// La función está bloqueada (motivo).
    Bloqueada(String),
    /// La petición no cambia nada.
    SinCambios,
    /// Hay otro cambio en curso.
    EnCurso,
    /// El router (o el panel) no pudo hacer el cambio.
    Router(String),
    /// El router no muestra lo que se escribió.
    Verificacion(Vec<String>),
    /// Falta la palabra de confirmación o no coincide.
    PalabraIncorrecta,
    /// Token desconocido, caducado o ya usado.
    TokenInvalido,
    /// Petición no válida (función, parámetro o valor).
    Invalida(String),
    /// La instancia que iba a editarse o borrarse ya no es la misma que al
    /// preparar (`ChangeRequest::identidad`): el motivo.
    InstanciaCambiada(String),
}

impl std::fmt::Display for CambioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CambioError::Bloqueada(m) => write!(f, "función bloqueada: {m}"),
            CambioError::SinCambios => f.write_str("no hay nada que cambiar"),
            CambioError::EnCurso => f.write_str("hay otro cambio en curso"),
            CambioError::Router(e) => write!(f, "{e}"),
            CambioError::Verificacion(v) => {
                write!(f, "el router no muestra el cambio: {}", v.join("; "))
            }
            CambioError::PalabraIncorrecta => f.write_str("la palabra de confirmación no coincide"),
            CambioError::TokenInvalido => {
                f.write_str("el cambio preparado ha caducado o ya se usó; revísalo de nuevo")
            }
            CambioError::Invalida(e) => write!(f, "petición no válida: {e}"),
            CambioError::InstanciaCambiada(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CambioError {}

/// Estado de un cambio en el historial.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Estado {
    Preparado,
    Aplicando,
    Vigilando,
    Aplicado,
    Revertido,
    Fallido,
    /// Grupo que falló y cuya reversión no se pudo completar: el detalle
    /// (`error`) dice qué pasos siguen aplicados.
    RevertidoParcial,
}

impl Estado {
    /// Nombre en la base de datos.
    pub fn as_str(self) -> &'static str {
        match self {
            Estado::Preparado => "preparado",
            Estado::Aplicando => "aplicando",
            Estado::Vigilando => "vigilando",
            Estado::Aplicado => "aplicado",
            Estado::Revertido => "revertido",
            Estado::Fallido => "fallido",
            Estado::RevertidoParcial => "revertido_parcial",
        }
    }

    pub(super) fn parse(s: &str) -> Option<Estado> {
        [
            Estado::Preparado,
            Estado::Aplicando,
            Estado::Vigilando,
            Estado::Aplicado,
            Estado::Revertido,
            Estado::Fallido,
            Estado::RevertidoParcial,
        ]
        .into_iter()
        .find(|e| e.as_str() == s)
    }
}

/// Un cambio del historial (secretos enmascarados): un grupo de pasos.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cambio {
    pub id: i64,
    /// Creación y última actualización (segundos Unix).
    pub ts: i64,
    pub updated: i64,
    /// Función e instancia del primer paso que se escribió.
    pub funcion: String,
    pub instancia: Option<String>,
    /// El mayor de los pasos.
    pub riesgo: Riesgo,
    pub estado: Estado,
    /// Los diffs de los pasos que no se omitieron, juntos.
    pub diff: Diff,
    pub error: Option<String>,
    pub nota: Option<String>,
    pub usuario: Option<String>,
    /// Clave i18n del resumen del grupo.
    pub titulo: Option<String>,
    pub pasos: Vec<PasoCambio>,
}

/// Referencia para la vigilancia, tomada antes de escribir.
#[derive(Clone, Copy, Debug)]
pub(super) enum Base {
    /// Si la WAN estaba arriba.
    Wan(bool),
    /// Tiempo encendido antes de `cbReboot`.
    Uptime(u64),
}

pub(super) struct Pendiente {
    pub(super) pasos: Vec<Paso>,
    /// Pasos que no cambiaban nada al preparar: se omiten.
    pub(super) sin_cambios: Vec<bool>,
    pub(super) titulo: Option<String>,
    pub(super) caduca: Instant,
    /// Valores exactos (deshacer): sin "secreto vacío = sin tocar" ni
    /// validación de lo que vino del router.
    pub(super) explicito: bool,
    /// Cambio que deshace, si es un deshacer.
    pub(super) deshace: Option<i64>,
}

/// Lo que el gestor usa del estado del servidor (sin el propio gestor, para
/// que `AppState` pueda guardarlo sin ciclos).
pub(super) struct Recursos {
    pub(super) db: SqlitePool,
    pub(super) hgu: hgu::Client,
    pub(super) catalogo: Arc<Catalogo>,
    pub(super) mascara: Arc<Mascara>,
    pub(super) poller: PollerHandle,
    pub(super) history_key: Arc<HistoryKey>,
    /// Conexión a internet que vigila la red de seguridad WAN.
    pub(super) red: RedActual,
    /// Caché de lecturas para mostrar: se invalida al verificar y al
    /// terminar cada cambio (las escrituras ya la invalidan solas).
    pub(super) lecturas: Arc<CacheLecturas>,
}

impl Recursos {
    /// ¿Hay internet? (ver [`plan::wan_arriba`]): la conexión que vigila
    /// ([`RedActual::wan_en`]) y si está arriba. Error si el router no
    /// responde.
    pub(super) async fn wan_vigilada(&self) -> Result<(Option<String>, bool), hgu::Error> {
        let conns = read::wan(&self.hgu).await?;
        let wan = self.red.wan_en(&conns);
        let arriba = wan_arriba(&conns, wan.as_deref());
        Ok((wan, arriba))
    }
}

/// Fallo de un intento de reversión.
pub(super) enum FalloReversion {
    /// Transitorio (el router no responde o no se pudo releer): se reintenta.
    Reintentar(String),
    /// Determinista: no tiene sentido reintentar.
    Definitivo(String),
}

/// Funciones que cambian la IP LAN del router (bloqueadas en v1).
pub(super) const CAMBIAN_IP_LAN: &[&str] = &[
    "cbSetLanIPv4Addr",
    "cbSetLanIPv4Intf",
    "te_red_local.cgi",
    "te_mapa_red_local.cgi",
];
/// Motivo de [`CambioError::Bloqueada`] con el panel en solo lectura
/// (firmware no probado sin confirmar). Es la clave i18n del texto: la
/// interfaz lo traduce.
pub const BLOQUEO_SOLO_LECTURA: &str = "panel-solo-lectura";
pub(super) const BLOQUEO_IP_LAN: &str = "bloqueada en v1: cambiar la IP del router corta la conexión del \
                              propio panel; hazlo desde la web del router";
/// Motivo de [`CambioError::Bloqueada`] cuando la escritura reenviaría un
/// secreto que el router da vacío o tapado (al completar la carga con lo
/// leído): podría borrar esa clave. Es la clave i18n del texto. Se quitará
/// cuando la fase 3 compruebe qué hace el router real con ese reenvío.
pub const BLOQUEO_SECRETO_TAPADO: &str = "cambio-error-secreto-tapado";
/// Motivo de [`CambioError::Invalida`] al deshacer un cambio en el que solo
/// cambió lo que el router no deja leer: no hay valor anterior al que volver.
pub const DESHACER_SOLO_TAPADOS: &str =
    "solo cambió una clave que el router no deja leer: no hay valor anterior al que volver";
pub(super) const BLOQUEO_SECRETOS_BASICA: &str = "bloqueada en v1: falta validar con el router real el \
                                       formato de los secretos de la web básica";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_sin_valores_ni_secretos() {
        let r = ChangeRequest {
            funcion: "cbSetWifiAccPt".into(),
            instancia: None,
            valores: [("key_phrase".to_owned(), "clave-en-claro".to_owned())].into(),
            borrar: false,
            identidad: Default::default(),
        };
        let d = format!("{r:?}");
        assert!(
            !d.contains("clave-en-claro") && d.contains("key_phrase"),
            "{d}"
        );
        let c = CampoDiff {
            nombre: "key_phrase".into(),
            antes: "clave-de-antes".into(),
            despues: "clave-en-claro".into(),
            secreto: true,
            sensible: false,
        };
        let d = format!("{c:?}");
        assert!(
            !d.contains("clave-en-claro") && !d.contains("clave-de-antes"),
            "{d}"
        );
        let c = CampoDiff {
            secreto: false,
            ..c
        };
        assert!(
            format!("{c:?}").contains("clave-en-claro"),
            "lo que no es secreto, sí"
        );
        // Un dato sensible en claro (el diff para la pantalla) tampoco sale
        // en un registro.
        let c = CampoDiff {
            nombre: "acs_username".into(),
            antes: "usuario-de-antes".into(),
            despues: "usuario-nuevo".into(),
            secreto: false,
            sensible: true,
        };
        let d = format!("{c:?}");
        assert!(
            !d.contains("usuario-nuevo") && !d.contains("usuario-de-antes"),
            "{d}"
        );
        // Y no se guarda marcado: el diff guardado no cambia de forma.
        let guardado = CampoDiff {
            sensible: false,
            ..c
        };
        assert!(
            !serde_json::to_string(&guardado)
                .unwrap()
                .contains("sensible")
        );
    }
}
