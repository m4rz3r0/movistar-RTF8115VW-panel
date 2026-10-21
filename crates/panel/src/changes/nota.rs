//! Notas y errores del gestor («verificación limitada», «el router no hizo
//! el cambio»…). Se guardan (columnas `nota` y `error` de `changes` y
//! `change_steps`) como claves de traducción con sus argumentos, no como
//! texto: el Historial las pinta en el idioma de quien lo mira. Lo técnico
//! (el texto del router, los parámetros) va aparte, para los «Detalles
//! técnicos» y el registro. Las filas de antes guardan el texto en
//! castellano y se muestran tal cual.
//!
//! **Las claves están guardadas en las bases de datos de los usuarios: no se
//! renombran ni se borran** (como mucho, se cambia su texto en los `.ftl`).

use i18n::{FluentArgs, Idioma, tr};
use serde::{Deserialize, Serialize};

/// Marca de una nota guardada como claves (el texto de una fila antigua no
/// empieza nunca así).
const PREFIJO: &str = "i18n:";

pub(crate) const VERIFICACION_LIMITADA: &str = "nota-verificacion-limitada";
pub(crate) const REINICIO_SIN_RESPUESTA: &str = "nota-reinicio-sin-respuesta";
pub(crate) const SIN_CONFIRMAR: &str = "nota-sin-confirmar";
pub(crate) const RELEIDO_APLICADA: &str = "nota-releido-aplicada";
pub(crate) const SIN_CAMPOS_ESCRITOS: &str = "nota-sin-campos-escritos";
pub(crate) const SIN_RELEER: &str = "nota-sin-releer";
pub(crate) const WAN_CAIDA: &str = "nota-wan-caida";
pub(crate) const RETOMADA: &str = "nota-retomada";
/// «paso 2: …», para la nota de un paso de un grupo.
const PASO: &str = "nota-paso";
/// «paso 2 (UDP): …», con el nombre del paso.
const PASO_NOMBRE: &str = "nota-paso-nombre";

// Errores (columna `error`). Lo técnico, con [`Nota::tecnico`].
pub(crate) const ERROR_ROUTER: &str = "cambio-error-router";
pub(crate) const ERROR_VERIFICACION: &str = "cambio-error-verificacion";
pub(crate) const ERROR_PREPARAR: &str = "cambio-error-preparar";
pub(crate) const ERROR_REVERTIDO: &str = "cambio-error-revertido";
pub(crate) const ERROR_SIN_REVERTIR: &str = "cambio-error-sin-revertir";
pub(crate) const ERROR_SIN_INVERSA: &str = "cambio-error-sin-inversa";
pub(crate) const ERROR_ROUTER_NO_RESPONDE: &str = "cambio-error-router-no-responde";
pub(crate) const ERROR_INTERNET_CAIDA: &str = "cambio-error-internet-caida";
pub(crate) const ERROR_REINICIO_SIN_VUELTA: &str = "cambio-error-reinicio-sin-vuelta";
pub(crate) const ERROR_ACCION: &str = "cambio-error-accion";
pub(crate) const ERROR_INTERRUMPIDO: &str = "cambio-error-interrumpido";
pub(crate) const ERROR_NINGUN_PASO: &str = "cambio-error-ningun-paso";
pub(crate) const ERROR_SIN_PASOS: &str = "cambio-error-sin-pasos";
pub(crate) const ERROR_NO_APLICADO: &str = "cambio-error-no-aplicado";
pub(crate) const ERROR_REINICIO_INCIERTO: &str = "cambio-error-reinicio-incierto";
pub(crate) const ERROR_SIN_RESPUESTA_TRAS_PANEL: &str = "cambio-error-sin-respuesta-tras-panel";
pub(crate) const ERROR_SIN_COMPROBAR: &str = "cambio-error-sin-comprobar";
pub(crate) const ERROR_ANTES_DE_APLICAR: &str = "cambio-error-antes-de-aplicar";
pub(crate) const ERROR_SIN_CAMBIO: &str = "cambio-error-sin-cambio";
pub(crate) const ERROR_PASO_ILEGIBLE: &str = "cambio-error-paso-ilegible";

/// Una nota: uno o varios trozos, que se pintan separados por «; ».
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nota(Vec<Trozo>);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Trozo {
    clave: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    args: Vec<(String, String)>,
    /// El paso del grupo al que se refiere (desde 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    paso: Option<usize>,
    /// Clave del nombre de ese paso («TCP»), si lo tiene.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    nombre_paso: Option<String>,
    /// Lo técnico (el texto del router, los parámetros): solo para los
    /// «Detalles técnicos» y el registro.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tecnico: Option<String>,
}

impl Nota {
    /// Una nota sin argumentos.
    pub fn clave(clave: &str) -> Nota {
        Nota::con(clave, &[])
    }

    /// Una nota con sus argumentos (`$nombre` en el `.ftl`).
    pub fn con(clave: &str, args: &[(&str, String)]) -> Nota {
        Nota(vec![Trozo {
            clave: clave.to_owned(),
            args: args
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
            paso: None,
            nombre_paso: None,
            tecnico: None,
        }])
    }

    /// El error de un fallo del gestor: el texto legible según su tipo y,
    /// aparte, lo técnico.
    pub(crate) fn de_error(e: &super::CambioError) -> Nota {
        use super::CambioError as E;
        match e {
            E::Router(t) => Nota::clave(ERROR_ROUTER).tecnico(t.clone()),
            E::Verificacion(v) => Nota::clave(ERROR_VERIFICACION).tecnico(v.join("; ")),
            e => Nota::clave(ERROR_PREPARAR).tecnico(e.to_string()),
        }
    }

    /// Lo técnico de este trozo (el último de la nota).
    pub fn tecnico(mut self, detalle: impl Into<String>) -> Nota {
        if let Some(t) = self.0.last_mut() {
            t.tecnico = Some(detalle.into());
        }
        self
    }

    /// Esta nota y, detrás, `otra`.
    pub fn y(mut self, otra: Nota) -> Nota {
        self.0.extend(otra.0);
        self
    }

    /// La nota del paso `orden` (desde 0) de un grupo.
    pub fn del_paso(self, orden: usize) -> Nota {
        self.del_paso_nombrado(orden, None)
    }

    /// La nota del paso `orden` (desde 0) de un grupo, con la clave de su
    /// nombre si la tiene (nunca el nombre de la función).
    pub fn del_paso_nombrado(mut self, orden: usize, nombre: Option<&str>) -> Nota {
        for t in &mut self.0 {
            if t.paso.is_none() {
                t.paso = Some(orden + 1);
                t.nombre_paso = nombre.map(str::to_owned);
            }
        }
        self
    }

    /// Las notas juntas, en orden; `None` si no hay ninguna.
    pub fn juntar(notas: impl IntoIterator<Item = Nota>) -> Option<Nota> {
        notas.into_iter().reduce(Nota::y)
    }

    /// Lo que se guarda en la columna `nota`.
    pub fn guardar(&self) -> String {
        // Una lista de cadenas y números siempre se serializa.
        let json = serde_json::to_string(self).unwrap_or_default();
        format!("{PREFIJO}{json}")
    }

    fn leer(guardada: &str) -> Option<Nota> {
        guardada
            .strip_prefix(PREFIJO)
            .and_then(|j| serde_json::from_str::<Nota>(j).ok())
    }

    /// El texto de una nota guardada, en `idioma`, sin lo técnico. Una fila
    /// antigua (o lo que no se pueda leer como claves) se muestra tal cual.
    pub fn texto(guardada: &str, idioma: Idioma) -> String {
        match Nota::leer(guardada) {
            Some(n) => n.en(idioma),
            None => guardada.to_owned(),
        }
    }

    /// Lo técnico de una nota guardada (`None` si no lo tiene o es una fila
    /// antigua, cuyo texto ya lo lleva).
    pub fn detalle(guardada: &str, idioma: Idioma) -> Option<String> {
        Nota::leer(guardada)?.tecnicos(idioma)
    }

    /// El texto en `idioma`, sin lo técnico.
    pub fn en(&self, idioma: Idioma) -> String {
        self.0
            .iter()
            .map(|t| {
                let mut args = FluentArgs::new();
                for (k, v) in &t.args {
                    args.set(k.clone(), v.clone());
                }
                Nota::con_paso(t, tr(idioma, &t.clave, Some(&args)), idioma)
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn tecnicos(&self, idioma: Idioma) -> Option<String> {
        let v: Vec<String> = self
            .0
            .iter()
            .filter_map(|t| {
                let d = t.tecnico.clone()?;
                Some(match t.paso {
                    Some(n) => {
                        let mut a = FluentArgs::new();
                        a.set("n", n);
                        a.set("nota", d);
                        tr(idioma, PASO, Some(&a))
                    }
                    None => d,
                })
            })
            .collect();
        (!v.is_empty()).then(|| v.join("; "))
    }

    fn con_paso(t: &Trozo, texto: String, idioma: Idioma) -> String {
        let Some(n) = t.paso else {
            return texto;
        };
        let mut a = FluentArgs::new();
        a.set("n", n);
        a.set("nota", texto);
        match &t.nombre_paso {
            Some(k) => {
                a.set("nombre", tr(idioma, k, None));
                tr(idioma, PASO_NOMBRE, Some(&a))
            }
            None => tr(idioma, PASO, Some(&a)),
        }
    }

    /// Para el registro y para quien confirmó: en castellano, con lo
    /// técnico entre corchetes.
    pub fn para_registro(&self) -> String {
        let texto = self.en(Idioma::Es);
        match self.tecnicos(Idioma::Es) {
            Some(d) => format!("{texto} [{d}]"),
            None => texto,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_claves_existen_en_los_dos_idiomas() {
        for clave in [
            VERIFICACION_LIMITADA,
            REINICIO_SIN_RESPUESTA,
            SIN_CONFIRMAR,
            RELEIDO_APLICADA,
            SIN_CAMPOS_ESCRITOS,
            SIN_RELEER,
            WAN_CAIDA,
            RETOMADA,
            PASO,
            PASO_NOMBRE,
            ERROR_ROUTER,
            ERROR_VERIFICACION,
            ERROR_PREPARAR,
            ERROR_REVERTIDO,
            ERROR_SIN_REVERTIR,
            ERROR_SIN_INVERSA,
            ERROR_ROUTER_NO_RESPONDE,
            ERROR_INTERNET_CAIDA,
            ERROR_REINICIO_SIN_VUELTA,
            ERROR_ACCION,
            ERROR_INTERRUMPIDO,
            ERROR_NINGUN_PASO,
            ERROR_SIN_PASOS,
            ERROR_NO_APLICADO,
            ERROR_REINICIO_INCIERTO,
            ERROR_SIN_RESPUESTA_TRAS_PANEL,
            ERROR_SIN_COMPROBAR,
            ERROR_ANTES_DE_APLICAR,
            ERROR_SIN_CAMBIO,
            ERROR_PASO_ILEGIBLE,
        ] {
            for idioma in Idioma::TODOS {
                assert!(i18n::existe(idioma, clave), "{clave} en {idioma:?}");
            }
        }
    }

    #[test]
    fn se_pinta_en_el_idioma_de_quien_mira() {
        let nota = Nota::con(SIN_CONFIRMAR, &[("error", "sin respuesta".into())])
            .y(Nota::clave(RELEIDO_APLICADA));
        let guardada = nota.guardar();
        assert_eq!(
            Nota::texto(&guardada, Idioma::Es),
            "el router no confirmó la escritura (sin respuesta); releído: aplicada"
        );
        assert_eq!(
            Nota::texto(&guardada, Idioma::En),
            "the router didn't confirm the write (sin respuesta); read again: applied"
        );
    }

    #[test]
    fn la_nota_de_un_paso_lleva_su_numero_y_no_la_funcion() {
        let nota = Nota::juntar([
            Nota::clave(RETOMADA),
            Nota::clave(VERIFICACION_LIMITADA).del_paso(1),
        ])
        .unwrap();
        let es = Nota::texto(&nota.guardar(), Idioma::Es);
        assert_eq!(
            es,
            "vigilancia retomada tras un reinicio del panel; paso 2: verificación limitada: \
             el router no muestra los campos escritos"
        );
        let en = Nota::texto(&nota.guardar(), Idioma::En);
        assert_eq!(
            en,
            "watching resumed after the panel restarted; step 2: limited check: the router \
             doesn't show the fields written"
        );
        assert!(!es.contains("cb") && !en.contains("cb"));
        // Un paso ya numerado no se renumera.
        let otra = Nota::clave(WAN_CAIDA).del_paso(0).del_paso(4);
        assert!(Nota::texto(&otra.guardar(), Idioma::Es).starts_with("paso 1: "));
    }

    #[test]
    fn un_error_separa_lo_tecnico_y_nombra_el_paso() {
        let e = super::super::CambioError::Router("la llamada cbSetNatRule falló".into());
        let nota = Nota::de_error(&e)
            .del_paso_nombrado(1, Some("ajuste-puertos-reglas-protocolo-udp"))
            .y(Nota::clave(ERROR_REVERTIDO));
        let g = nota.guardar();
        for idioma in Idioma::TODOS {
            let texto = Nota::texto(&g, idioma);
            assert!(!texto.contains("cb"), "{texto}");
            assert!(texto.contains("UDP"), "{texto}");
            let d = Nota::detalle(&g, idioma).unwrap();
            assert!(d.contains("cbSetNatRule"), "{d}");
        }
        assert_eq!(
            Nota::texto(&g, Idioma::En),
            "step 2 (UDP): the router didn't make the change; the previous values were put back"
        );
        assert!(
            nota.para_registro()
                .contains("[paso 2: la llamada cbSetNatRule falló]")
        );
        // Una fila antigua no tiene detalle aparte: ya va en su texto.
        assert_eq!(
            Nota::detalle("paso 2 (cbSetNatRule): falló", Idioma::Es),
            None
        );
    }

    #[test]
    fn una_fila_antigua_se_muestra_tal_cual() {
        let antigua = "vigilancia retomada tras un reinicio del panel; verificación limitada";
        assert_eq!(Nota::texto(antigua, Idioma::En), antigua);
        // Con la marca pero sin claves que leer, también tal cual.
        assert_eq!(Nota::texto("i18n:{roto", Idioma::En), "i18n:{roto");
        assert_eq!(Nota::juntar([]), None);
    }
}
